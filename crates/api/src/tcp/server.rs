//server.rs
use std::io;
use std::sync::Arc;

use tokio::io::BufReader;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Semaphore, watch};

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
                                println!("[DEBUG|ERROR] Lỗi xử lý client {addr}: {e}");
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
                    Message::buffer_from_reader(&mut reader, &mut buf[filled..filled + read_len])
                        .await?;
                    filled += read_len;
                    bytes_received += read_len as u64;
                    //
                    if bytes_received == total_size {
                        // last chunk > send buf
                        // Phần buf sau vị trí `filled` có thể là rác từ lần dùng trước của pool,
                        // nhưng không sao vì on_chunk luôn nhận kèm `filled` để biết đọc tới đâu là đủ.
                        // events
                        //     .on_chunk(chunk_index, buf, filled)
                        //     .await?;
                        println!(
                            "DEBUG> send on_chunk: chunk_index={}, filled={}, data={:?}",
                            chunk_index,
                            filled,
                            &buf[..filled]
                        );
                        break;
                    }
                    if filled == buf.len() {
                        // filled full accquire new buffer
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
                        println!(
                            "DEBUG>(new buff) send on_chunk: chunk_index={}, filled={}, data={:?}",
                            chunk_index,
                            filled,
                            &full[..filled]
                        );
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
                let uuid = msg.as_string()?;
                let (mut chunk_index, mut bytes_received, total_size) =
                    events.on_resume(&uuid).await?;
                Message::stream(bytes_received.to_string())
                    .send(&mut writer)
                    .await?;
                // TODO resume obj here
                let mut filled: usize = 0;
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
                    Message::buffer_from_reader(&mut reader, &mut buf[filled..filled + read_len])
                        .await?;
                    filled += read_len;
                    bytes_received += read_len as u64;
                    //
                    if bytes_received == total_size {
                        // last chunk > send buf
                        // Phần buf sau vị trí `filled` có thể là rác từ lần dùng trước của pool,
                        // nhưng không sao vì on_chunk luôn nhận kèm `filled` để biết đọc tới đâu là đủ.
                        // events
                        //     .on_chunk(chunk_index, buf, filled)
                        //     .await?;
                        println!(
                            "DEBUG> send on_chunk: chunk_index={}, filled={}, data={:?}",
                            chunk_index,
                            filled,
                            &buf[..filled]
                        );
                        break;
                    }
                    if filled == buf.len() {
                        // filled full accquire new buffer
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
                        println!(
                            "DEBUG>(new buff) send on_chunk: chunk_index={}, filled={}, data={:?}",
                            chunk_index,
                            filled,
                            &full[..filled]
                        );
                        // events
                        //     .on_chunk(chunk_index, full, filled)
                        //     .await?;
                        chunk_index += 1;
                        filled = 0;
                    }
                }
                //
                events.on_complete(&uuid).await?;
            }
            MessageType::Close => {
                // TODO: when user proactively close the connection or call cancle
                let uuid = msg.as_string()?;
                events.on_close(&uuid).await?;
            }
            _ => {
                //Message::error("unsupport").send(&mut writer).await?;
                return Ok(());
            }
        }
        Message::success().send(&mut writer).await
    }

}
