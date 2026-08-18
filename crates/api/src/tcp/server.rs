//server.rs
use std::io;
use std::sync::Arc;

use tokio::io::{AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Mutex, Semaphore, mpsc, watch};

use crate::tcp::{BufferPool, Message, MessageType, TransferEvents};

pub struct Server {
    pub tx: watch::Sender<bool>,
    pub handle: tokio::task::JoinHandle<io::Result<()>>,
}

impl Server {
    pub fn start(
        port: u64,
        max_concurrent_clients: usize,
        events: Arc<dyn TransferEvents>,
        buffer_pool: Arc<BufferPool>,
    ) -> Self {
        let (tx, _) = watch::channel(false);
        let mut shutdown_rx = tx.subscribe();

        let handle = tokio::spawn(async move {
            let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
            let semaphore = Arc::new(Semaphore::new(max_concurrent_clients));

            loop {
                tokio::select! {
                    accept_result = listener.accept() => {
                        let Ok((socket, addr)) = accept_result else { continue };
                        let _ = socket.set_nodelay(true);
                        let permit = Arc::clone(&semaphore).acquire_owned().await.expect("semaphore không bao giờ bị đóng");
                        let (handler, buffer_pool) = (events.clone(), buffer_pool.clone());

                        tokio::spawn(async move {
                            let _permit = permit;
                            if let Err(e) = Server::handle(socket,  handler, buffer_pool).await {
                                println!("Lỗi xử lý client {addr}: {e}");
                            }
                        });
                    }

                    _ = shutdown_rx.changed() => break,
                }
            }

            Ok(())
        });
        Self { tx, handle }
    }

    /// Gửi tín hiệu dừng nhận kết nối mới.
    /// Lưu ý: các client đang kết nối vẫn tự chạy tới khi họ đóng, không bị ngắt cưỡng bức.
    pub async fn shutdown(self) {
        drop(self.tx);
        let _ = self.handle.await;
    }

    async fn handle(
        socket: TcpStream,
        events: Arc<dyn TransferEvents>,
        buffer_pool: Arc<BufferPool>,
    ) -> io::Result<()> {
        let (reader, mut writer) = socket.into_split();
        let mut reader = BufReader::new(reader);
        let msg = Message::from_reader(&mut reader).await?;
        match msg.message_type {
            MessageType::New => {
                let obj = msg.as_object()?;
                let total_size = obj.total_size;
                let uuid = obj.external_id.to_string();
                events.on_new(obj).await?;
                Message::stream(&uuid).send(&mut writer).await?;
                // TODO start new obj here
                // loop here
                let mut chunk_index: u64 = 0;
                let mut filled: usize = 0;
                let mut bytes_received: u64 = 0;

                let mut buf = match buffer_pool.acquire_timeout().await {
                    Some(buf) => buf,
                    None => {
                        Message::error("not enough PooledBuffer")
                            .send(&mut writer)
                            .await?;
                        return Ok(());
                    }
                };

                while bytes_received < total_size {
                    let remaining_total = (total_size - bytes_received) as usize;
                    let capacity = buf.len() - filled;
                    let read_len = remaining_total.min(capacity);

                    Message::read_data_chunk(&mut reader, &mut buf[filled..filled + read_len])
                        .await?;

                    filled += read_len;
                    bytes_received += read_len as u64;

                    if bytes_received == total_size {
                        // Chunk cuối: gửi thẳng buf hiện tại, KHÔNG cần lấy buffer mới nữa vì loop sẽ kết thúc.
                        // Phần buf sau vị trí `filled` có thể là rác từ lần dùng trước của pool,
                        // nhưng không sao vì on_chunk luôn nhận kèm `filled` để biết đọc tới đâu là đủ.
                        // events
                        //     .on_chunk(chunk_index, buf, filled)
                        //     .await?;
                        break;
                    }

                    if filled == buf.len() {
                        let next = match buffer_pool.acquire_timeout().await {
                            Some(buf) => buf,
                            None => {
                                Message::error("not enough PooledBuffer")
                                    .send(&mut writer)
                                    .await?;
                                return Ok(());
                            }
                        };
                        let full = std::mem::replace(&mut buf, next);

                        // events
                        //     .on_chunk(chunk_index, full, filled)
                        //     .await?;

                        chunk_index += 1;
                        filled = 0;
                    }
                }

                events.on_complete(&uuid).await?;
            }
            MessageType::Resume => {
                let uuid = msg.get_string()?;
                let resume_size = events.on_resume(&uuid).await?;
                Message::stream(resume_size.to_string())
                    .send(&mut writer)
                    .await?;
                // TODO resume obj here
                // loop here

                events.on_complete(&uuid).await?;
            }
            MessageType::Close => {
                // TODO: when user proactively close the connection or call cancle
                let uuid = msg.get_string()?;
                events.on_close(&uuid).await?;
            }
            _ => {
                Message::error("unsupport").send(&mut writer).await?;
                return Ok(());
            }
        }

        Message::success().send(&mut writer).await?;

        // let (filename, total_size) = Server::read_info_payload(&mut reader, payload_len).await?;

        // let conn_id = Uuid::now_v7();
        // let ok = events.on_info(conn_id, filename, total_size).await;
        // Server::write_ack(&mut writer, ok.is_ok()).await?;
        // ok.map_err(Server::protocol_err)?;

        // // ---- DATA: tích luỹ tới CHUNK_SIZE hoặc hết total_size -> gọi on_chunk ----
        // let mut chunk_index = 0u64;
        // let mut bytes_received = 0u64;
        // let mut buf = buffer_pool.acquire().await;
        // let mut filled = 0usize;

        // while bytes_received < total_size {
        //     // Giới hạn độ dài đọc theo CẢ 2 mốc: chỗ trống còn lại trong buffer VÀ
        //     // số byte DATA còn thiếu theo total_size. Thiếu vế sau sẽ over-read ở
        //     // chunk cuối: client thường gửi liền tay msg END ngay sau DATA, TCP
        //     // không có ranh giới message nên read() có thể nuốt nhầm vài byte đầu
        //     // của END vào buffer DATA.
        //     let remaining_total = (total_size - bytes_received) as usize;
        //     let capacity = buf.len() - filled;
        //     let read_len = remaining_total.min(capacity);

        //     let n = tokio::time::timeout(
        //         read_timeout,
        //         reader.read(&mut buf[filled..filled + read_len]),
        //     )
        //     .await
        //     .map_err(|_| Server::protocol_err("timeout đọc DATA"))??;
        //     if n == 0 {
        //         return Err(Server::protocol_err("client đóng giữa chừng"));
        //     }
        //     filled += n;
        //     bytes_received += n as u64;

        //     if filled == buf.len() || bytes_received == total_size {
        //         let full = std::mem::replace(&mut buf, buffer_pool.acquire().await);
        //         events
        //             .on_chunk(conn_id, chunk_index, full, filled)
        //             .await
        //             .map_err(Server::protocol_err)?;
        //         chunk_index += 1;
        //         filled = 0;
        //     }
        // }

        // // ---- END ----
        // let (msg_type, payload_len) = Server::read_msg_header(&mut reader).await?;
        // if msg_type != MSG_END {
        //     return Err(Server::protocol_err("thiếu END"));
        // }
        // let mut hash = vec![0u8; payload_len as usize];
        // reader.read_exact(&mut hash).await?;

        // let result = events.on_complete(conn_id, hash).await;
        // Server::write_ack(&mut writer, result.is_ok()).await?;
        Ok(())
    }

    /// Đọc header cố định 5 byte: [msg_type: 1][payload_len: u32 BE].
    /// Dùng `read_exact` vì `read()` không đảm bảo trả đủ 5 byte trong 1 lần gọi.
    // async fn read_msg_header(reader: &mut BufReader<OwnedReadHalf>) -> io::Result<(u8, u32)> {
    //     let mut header = [0u8; 5];
    //     reader.read_exact(&mut header).await?;
    //     println!("------------");
    //     Ok((
    //         header[0],
    //         u32::from_be_bytes(header[1..5].try_into().unwrap()),
    //     ))
    // }
    // /// Payload của INFO: [name_len: u16][name bytes][total_size: u64].
    // async fn read_info_payload(
    //     reader: &mut BufReader<OwnedReadHalf>,
    //     payload_len: u32,
    // ) -> io::Result<(String, u64)> {
    //     let mut payload = vec![0u8; payload_len as usize];
    //     reader.read_exact(&mut payload).await?;
    //     let name_len = u16::from_be_bytes(payload[0..2].try_into().unwrap()) as usize;
    //     let name = String::from_utf8_lossy(&payload[2..2 + name_len]).to_string();
    //     let size_off = 2 + name_len;
    //     let total_size = u64::from_be_bytes(payload[size_off..size_off + 8].try_into().unwrap());
    //     Ok((name, total_size))
    // }

    /// Gửi ACK 1 byte (0/1) sau MSG_ACK header, gộp thành 1 lần `write_all` để giảm syscall.
    // async fn write_ack(writer: &mut OwnedWriteHalf, ok: bool) -> io::Result<()> {
    //     let mut frame = [0u8; 6];
    //     frame[0] = MSG_ACK;
    //     frame[1..5].copy_from_slice(&1u32.to_be_bytes()); // payload_len = 1
    //     frame[5] = ok as u8;
    //     writer.write_all(&frame).await
    // }
    /// Helper dựng io::Error cho lỗi giao thức (msg sai thứ tự, thiếu byte, timeout...).
    /// Nhận cả `&str` lẫn `String` nên dùng trực tiếp được cho cả gọi tay (`protocol_err("...")`)
    /// lẫn `.map_err(protocol_err)` (Err là String).
    fn protocol_err(msg: impl Into<String>) -> io::Error {
        io::Error::new(io::ErrorKind::InvalidData, msg.into())
    }
}
