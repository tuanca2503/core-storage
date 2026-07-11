# CLI Plan — Kế hoạch thiết kế CLI (`core-store`)

> Tài liệu tổng hợp thiết kế CLI cho Storage Engine (tiếp theo `plan.md`).

## 1. Mục tiêu

- CLI (`core-store`) chỉ là **một client** của Storage Service, tương tự Web/API Server.
- CLI **không chứa business logic**, không thao tác trực tiếp với ổ đĩa.
- Mọi thao tác của CLI đều được gửi qua **TCP** tới Storage Service để xử lý.

## 2. Vị trí của CLI trong kiến trúc tổng thể

```
             +----------------------+
             |  Storage Service     |
             |----------------------|
             | TCP Listener         |
             | Command Router       |
             | Storage Engine       |
             +----------▲-----------+
                        │
              TCP Protocol
                        │
        +---------------+---------------+
        |                               |
        ▼                               ▼
   core-store CLI                 Web/API Server
```

- CLI và Web Server dùng **chung một protocol** và **chung một `StorageClient`**.
- Storage Engine (chunking, segment, metadata, WAL...) đã được định nghĩa ở `plan.md` — CLI không cần biết chi tiết tầng này.

## 3. Nguyên tắc thiết kế

- Một giao thức duy nhất cho mọi client (CLI, Web Server, SDK sau này).
- CLI chỉ là **wrapper mỏng** quanh `StorageClient`.
- Không truyền dữ liệu binary qua tham số command line.
- Dữ liệu luôn được truyền bằng **stream**, không load hết vào memory.

## 4. Các nhóm lệnh CLI

### Upload

Từ file:
```
core-store put movie.mp4
```
CLI mở file và stream trực tiếp tới Storage Service.

Từ stdin:
```
cat movie.mp4 | core-store put -
```
Dấu `-` đại diện cho stdin — cho phép pipe dữ liệu từ lệnh khác vào thẳng CLI.

### Download

```
core-store get <object-id> output.mp4
```
hoặc ghi ra stdout:
```
core-store cat <object-id>
```

### Metadata

```
core-store list
core-store stat <object-id>
core-store verify <object-id>
core-store delete <object-id>
```

### Disk

```
core-store disk list
core-store disk info <disk-id>
core-store disk enable <disk-id>
core-store disk disable <disk-id>
core-store disk readonly <disk-id>
core-store disk writable <disk-id>
```
Các lệnh này chỉ cập nhật metadata của ổ (`disk.json` / metadata DB trong `plan.md`) thông qua Storage Service — CLI không ghi trực tiếp xuống đĩa.

### Maintenance

```
core-store repair
core-store compact
core-store scrub
core-store recover
```
Tương ứng với các cơ chế Compaction/GC, checksum scrubbing và recovery đã mô tả trong `plan.md`.

## 5. StorageClient API (dùng chung cho mọi client)

```
put(reader)
get(id)
delete(id)
stat(id)
list()
```

- Nguồn dữ liệu đầu vào (`reader`) có thể là: file, stdin, network stream, hoặc memory buffer.
- `StorageClient` chỉ làm việc với **byte stream**, không quan tâm dữ liệu đến từ đâu.
- Vì CLI và Web Server đều dùng chung API này, hành vi upload/download luôn nhất quán giữa các client.

## 6. TCP Protocol (tổng quan)

Luồng ví dụ cho lệnh PUT:

```
Client
  │
  ├── PUT
  ├── Metadata
  ├── ACK
  ├── Chunk
  ├── Chunk
  ├── Chunk
  └── END
        │
        ▼
    Object ID
```

Các bước: Client gửi lệnh `PUT` kèm metadata → Storage Service trả `ACK` → Client stream lần lượt các `Chunk` → Client gửi `END` báo kết thúc → Storage Service trả về `Object ID`.

> Chi tiết đầy đủ về wire format sẽ nằm trong một tài liệu riêng — xem mục "Việc cần làm tiếp theo".

## 7. Phân chia trách nhiệm

| Thành phần | Trách nhiệm |
|---|---|
| CLI / Web Server | Nhận input từ người dùng, gọi `StorageClient`, hiển thị kết quả |
| Storage Service | Chunking, Scheduling, WAL, Metadata, Recovery |

CLI và Web Server **không biết cách lưu dữ liệu**. Toàn bộ logic lưu trữ nằm trong Storage Service. Nhờ vậy, hệ thống chỉ có **một pipeline upload/download duy nhất**, dù dữ liệu đến từ CLI, Web hay SDK nào khác sau này.

## 8. Việc cần làm tiếp theo

- [ ] Viết `protocol.md` — định nghĩa chi tiết TCP protocol:
  - Frame format (Header + Payload)
  - Command ID
  - Versioning
  - Authentication (nếu có)
  - Streaming protocol
  - Chunk packet format
  - ACK/NACK
  - Error code
  - Resume upload
  - Heartbeat/Ping
  - Compression/Encryption flags
- [ ] Định nghĩa cách CLI biết địa chỉ/port của Storage Service (config file, biến môi trường, hay tham số `--host`)
- [ ] Định nghĩa quy ước exit code / error message cho CLI
- [ ] Định nghĩa format output cho `list`, `stat` (text thường hay hỗ trợ `--json`)
