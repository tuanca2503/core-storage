# Plan: Kiến trúc truy cập SQLite (Server / Client)

## 1. Mục tiêu

Thiết kế lớp truy cập SQLite theo đúng mô hình đã dùng cho crate `api` (tách `server` và `client`):

- **Server**: sở hữu cả kết nối ghi lẫn đọc, chạy 24/7, có quản lý lifetime (start/stop, reconnect, drain khi shutdown).
- **Client**: chỉ có khả năng đọc — không có method ghi nào tồn tại trong API bề mặt, đảm bảo an toàn kiểu ngay từ compile-time (không phải chỉ do quy ước).

## 2. Nguyên tắc nền tảng (đã thống nhất)

### 2.1. Chỉ 1 writer tại 1 thời điểm

SQLite (kể cả WAL mode) chỉ cho phép 1 writer ở mức file. Dồn toàn bộ ghi qua **1 queue + 1 thread duy nhất** xử lý — tránh `SQLITE_BUSY` do nhiều connection tranh ghi.

### 2.2. Không share 1 `Connection` object giữa nhiều thread để đọc song song

`rusqlite::Connection` là `Send` nhưng không `Sync` — nhiều thread không thể cùng gọi `.query()` trên chung 1 instance. Bọc `Arc<Mutex<Connection>>` sẽ vô tình biến đọc song song thành tuần tự (mọi thread phải chờ lock). Giải pháp đúng: **connection pool**, mỗi thread lấy 1 connection riêng.

### 2.3. Bắt buộc bật WAL mode

Không bật WAL, writer sẽ khóa toàn bộ database, chặn hết reader trong lúc ghi — phá vỡ mục tiêu "đọc không cần chờ". Với WAL: 1 writer + nhiều reader chạy đồng thời, không chặn nhau.

```rust
conn.pragma_update(None, "journal_mode", "WAL")?;
```

## 3. Kiến trúc tổng quan (giống crate `api`)

```
db/
  src/
    lib.rs
    server.rs     // Sở hữu write queue + read pool, quản lý lifetime, chạy 24/7
    client.rs      // Chỉ có read pool, KHÔNG có method ghi nào
    write_queue.rs // Hàng đợi ghi + thread xử lý tuần tự
    pragma.rs      // Cấu hình pragma dùng chung (WAL, busy_timeout, synchronous)
```

| | Server | Client |
|---|---|---|
| Kết nối ghi | ✅ Có (1 connection, qua queue + thread riêng) | ❌ Không tồn tại trong API |
| Kết nối đọc | ✅ Có (connection pool) | ✅ Có (connection pool) |
| Quản lý lifetime | ✅ Start/stop, reconnect, graceful shutdown | Không cần — chỉ mở/đóng pool đơn giản |
| Chạy 24/7 | ✅ Đúng, giữ thread ghi sống suốt vòng đời process | Không bắt buộc — có thể mở theo nhu cầu |

## 4. Server

### 4.1. Trách nhiệm

- Mở và giữ **1 write connection** sống suốt vòng đời server, xử lý tuần tự qua queue.
- Mở và giữ **1 read pool** (nhiều connection đọc), phục vụ đọc song song thật.
- Quản lý lifecycle: khởi động, dừng an toàn (đảm bảo queue được xử lý hết trước khi đóng), tự phục hồi khi connection lỗi.

### 4.2. Write path — queue + thread duy nhất

```rust
pub struct WriteQueue {
    tx: mpsc::Sender<WriteJob>,
    handle: Option<thread::JoinHandle<()>>,
}

struct WriteJob {
    sql: String,
    params: Vec<Value>,
    reply: oneshot::Sender<BaseResult<()>>, // để caller biết kết quả, nếu cần
}

impl WriteQueue {
    pub fn start(db_path: PathBuf) -> BaseResult<Self> {
        let (tx, rx) = mpsc::channel::<WriteJob>();

        let handle = thread::spawn(move || {
            let conn = Connection::open(&db_path).expect("open db failed");
            apply_pragmas(&conn); // WAL, busy_timeout, synchronous

            while let Ok(job) = rx.recv() {
                let result = conn
                    .execute(&job.sql, params_from_iter(job.params.iter()))
                    .map(|_| ())
                    .map_err(BaseError::from);
                let _ = job.reply.send(result);
            }
            // rx.recv() trả Err khi tx bị drop toàn bộ → thread tự kết thúc, phục vụ graceful shutdown
        });

        Ok(Self { tx, handle: Some(handle) })
    }

    pub fn submit(&self, sql: String, params: Vec<Value>) -> BaseResult<()> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx.send(WriteJob { sql, params, reply: reply_tx })
            .map_err(|_| BaseError::system_error("write queue closed", Codes::Internal))?;
        reply_rx.recv().map_err(|_| BaseError::system_error("writer thread dropped", Codes::Internal))?
    }
}
```

### 4.3. Read path — connection pool

```rust
pub struct ReadPool {
    pool: r2d2::Pool<SqliteConnectionManager>,
}

impl ReadPool {
    pub fn new(db_path: PathBuf) -> BaseResult<Self> {
        let manager = SqliteConnectionManager::file(&db_path)
            .with_init(|conn| {
                apply_pragmas(conn);
                Ok(())
            });
        let pool = r2d2::Pool::builder()
            .max_size(8) // tùy chỉnh theo tải thực tế
            .build(manager)?;
        Ok(Self { pool })
    }

    pub fn get(&self) -> BaseResult<PooledConnection<SqliteConnectionManager>> {
        self.pool.get().map_err(BaseError::from)
    }
}
```

### 4.4. Struct `Server` — gộp cả 2, quản lý lifetime

```rust
pub struct Server {
    write_queue: WriteQueue,
    read_pool: ReadPool,
}

impl Server {
    pub fn start(db_path: PathBuf) -> BaseResult<Self> {
        Ok(Self {
            write_queue: WriteQueue::start(db_path.clone())?,
            read_pool: ReadPool::new(db_path)?,
        })
    }

    pub fn write(&self, sql: String, params: Vec<Value>) -> BaseResult<()> {
        self.write_queue.submit(sql, params)
    }

    pub fn read(&self) -> BaseResult<PooledConnection<SqliteConnectionManager>> {
        self.read_pool.get()
    }

    /// Dừng an toàn: đóng sender để writer thread tự thoát sau khi xử lý hết job đang chờ trong queue
    pub fn shutdown(mut self) -> BaseResult<()> {
        drop(self.write_queue.tx.clone());
        if let Some(handle) = self.write_queue.handle.take() {
            handle.join().map_err(|_| BaseError::system_error("writer thread panicked", Codes::Internal))?;
        }
        Ok(())
    }
}
```

> Lưu ý: `shutdown()` chỉ thực sự kết thúc thread khi **tất cả** bản sao của `tx` bị drop (không chỉ 1 clone) — cần đảm bảo không còn `Sender` nào khác đang sống ở nơi khác khi gọi shutdown.

## 5. Client

Chỉ expose read pool — **không có bất kỳ method ghi nào tồn tại trong struct này**, nên không cần dựa vào "quy ước không được ghi", mà compiler tự chặn:

```rust
pub struct Client {
    read_pool: ReadPool,
}

impl Client {
    pub fn connect(db_path: PathBuf) -> BaseResult<Self> {
        Ok(Self { read_pool: ReadPool::new(db_path)? })
    }

    pub fn read(&self) -> BaseResult<PooledConnection<SqliteConnectionManager>> {
        self.read_pool.get()
    }

    // Không có fn write(...) — cố tình không định nghĩa, đảm bảo an toàn ở mức API/type
}
```

Nơi khác dùng `Client` sẽ **không thể** gọi bất kỳ hàm ghi nào — lỗi "method not found" ngay lúc biên dịch nếu cố tình thử, không phải lỗi runtime hay quy ước bằng miệng.

## 6. Pragma cấu hình dùng chung

```rust
pub fn apply_pragmas(conn: &Connection) {
    conn.pragma_update(None, "journal_mode", "WAL").expect("set WAL failed");
    conn.pragma_update(None, "busy_timeout", 5000).expect("set busy_timeout failed"); // ms, chờ thay vì fail ngay khi gặp lock
    conn.pragma_update(None, "synchronous", "NORMAL").expect("set synchronous failed"); // cân bằng durability/throughput với WAL
}
```

| Pragma | Giá trị đề xuất | Ý nghĩa |
|---|---|---|
| `journal_mode` | `WAL` | Bắt buộc — cho phép nhiều reader + 1 writer đồng thời |
| `busy_timeout` | `5000` (ms) | Khi gặp lock tạm thời, chờ thay vì trả lỗi `SQLITE_BUSY` ngay lập tức |
| `synchronous` | `NORMAL` | Với WAL, `NORMAL` là mức khuyến nghị chuẩn (an toàn khi crash OS, chỉ mất dữ liệu nếu mất điện đúng lúc checkpoint — chấp nhận được cho hầu hết use-case; nếu cần durability tuyệt đối, đổi thành `FULL`, đánh đổi throughput) |

## 7. Vòng đời (lifetime) của Server — điểm khác biệt chính so với Client

- **Khởi động**: mở write connection (giữ sống suốt runtime), mở read pool, áp pragma cho cả 2.
- **Chạy 24/7**: writer thread block chờ trên `rx.recv()`, không tốn CPU khi rảnh; read pool cấp connection theo nhu cầu, tự tái sử dụng.
- **Lỗi/reconnect**: nếu write connection gặp lỗi nghiêm trọng (I/O error, corrupt), writer thread nên thoát vòng lặp và emit lỗi ra ngoài (qua channel/log) để tầng gọi quyết định restart `Server` — không nên tự động retry vô hạn trong im lặng.
- **Shutdown**: đóng `Sender`, đợi writer thread xử lý hết job còn tồn đọng trong queue rồi mới thoát hẳn — đảm bảo không mất ghi đang chờ xử lý.

Client không cần các bước này — chỉ mở pool khi cần, không giữ state "phải chạy liên tục", đóng lại bất cứ lúc nào mà không ảnh hưởng tính toàn vẹn dữ liệu (vì client chưa bao giờ ghi).

## 8. Việc còn để mở

- Retry/backoff policy khi `busy_timeout` vẫn hết hạn (trả lỗi ra sao, có tự thử lại ở tầng `submit()` không).
- Cơ chế health-check định kỳ cho `Server` (ping connection ghi còn sống không) nếu cần giám sát chủ động thay vì chỉ phát hiện lỗi khi có request.
- Số lượng connection tối đa trong `read_pool` (`max_size`) — cần benchmark theo tải thực tế thay vì con số mặc định 8 ở trên.