use std::sync::Arc;
use tokio::io::{BufReader, Result};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Semaphore, watch};
use crate::tcp::{BufferPool, Message, MessageType, TransferEvents};

pub struct Server {
    pub shutdown_tx: watch::Sender<bool>,
    pub handle: tokio::task::JoinHandle<Result<()>>,
}

impl Server {
    pub fn start(
        port: u64,
        max_concurrent_clients: u64,
        chunk_size: u64,
        queue_size: u64,
        events_trait: impl TransferEvents + 'static,
    ) -> Self {
        let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
        let handle = tokio::spawn(async move {
            let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
            let semaphore = Arc::new(Semaphore::new(max_concurrent_clients as usize));
            let events: Arc<dyn TransferEvents> = Arc::new(events_trait);
            let buffer_pool = Arc::new(BufferPool::new(
                (max_concurrent_clients + queue_size + 20) as usize,
                chunk_size as usize,
            )); //client + queue + margin(20)
            loop {
                tokio::select! {
                    accept_result = listener.accept() => {
                        let Ok((socket, addr)) = accept_result else { continue };
                        let _ = socket.set_nodelay(true);
                        let permit = Arc::clone(&semaphore).acquire_owned().await.expect("semaphore was closed unexpectedly — this should never happen");
                        let events = Arc::clone(&events);
                        let buffer_pool = Arc::clone(&buffer_pool);
                        tokio::spawn(async move {
                            let _permit = permit;
                            if let Err(e) = Server::handle(socket,  events, buffer_pool).await {
                                println!("[DEBUG|ERROR] Lỗi xử lý client {addr}: {e}");
                            }
                        });
                    }
                    _ = shutdown_rx.changed() => break,
                }
            }
            Ok(())
        });
        Self {
            shutdown_tx,
            handle,
        }
    }

    pub async fn shutdown(self) {
        drop(self.shutdown_tx);
        let _ = self.handle.await;
    }

    async fn handle(
        socket: TcpStream,
        events: Arc<dyn TransferEvents>,
        buffer_pool: Arc<BufferPool>,
    ) -> Result<()> {
        let (reader, mut writer) = socket.into_split();
        let mut reader = BufReader::new(reader);
        let msg = Message::from_reader(&mut reader).await?;

        match msg.message_type {
            MessageType::New => {
                // Prepare data
                let obj = msg.as_object()?;
                let total_size = obj.total_size;
                let uuid = obj.external_id.to_string();
                // TODO: trường hợp file nhỏ hơn 32 chunk(32x32Mib = 1gb) > đẩy vào fast queue(ssd)
                let need_faster = total_size < 1_000_000_000;
                events.on_new(obj).await?; // call event
                let mut sequence: u64 = 0;
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
                Message::stream(&uuid).send(&mut writer).await?;
                // END
                // Start receive chunks
                while bytes_received < total_size {
                    let remaining_total = (total_size - bytes_received) as usize;
                    let capacity = buf.len() - filled;
                    let read_len = remaining_total.min(capacity);
                    Message::buffer_from_reader(&mut reader, &mut buf[filled..filled + read_len])
                        .await?;
                    filled += read_len;
                    bytes_received += read_len as u64;
                    if bytes_received == total_size {
                        // last chunk > send buf
                        // Phần buf sau vị trí `filled` có thể là rác từ lần dùng trước của pool,
                        // nhưng không sao vì on_chunk luôn nhận kèm `filled` để biết đọc tới đâu là đủ.
                        // events
                        //     .on_chunk(chunk_index, buf, filled)
                        //     .await?;
                        println!(
                            "DEBUG> send on_chunk: chunk_index={}, filled={}, data={:?}",
                            sequence,
                            filled,
                            &buf[..filled]
                        );
                        break;
                    }
                    if filled == buf.len() {
                        // filled full accquire new buffer
                        // events
                        //     .on_chunk(chunk_index, full, filled)
                        //     .await?;
                        println!(
                            "DEBUG> send on_chunk: chunk_index={}, filled={}, data={:?}",
                            sequence,
                            filled,
                            &buf[..filled]
                        );
                        buf = match buffer_pool.acquire_timeout().await {
                            Some(buf) => buf,
                            None => {
                                Message::error("not enough PooledBuffer")
                                    .send(&mut writer)
                                    .await?;
                                return Ok(());
                            }
                        };
                        sequence += 1;
                        filled = 0;
                    }
                }
                // END
                events.on_complete(&uuid).await?; // call event
            }
            MessageType::Resume => {
                let uuid = msg.as_string()?;
                let (mut chunk_index, mut bytes_received, total_size) =
                    events.on_resume(&uuid).await?;

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
                Message::stream(bytes_received.to_string())
                    .send(&mut writer)
                    .await?;
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
                        // events
                        //     .on_chunk(chunk_index, full, filled)
                        //     .await?;
                        buf = match buffer_pool.acquire_timeout().await {
                            Some(buf) => buf,
                            None => {
                                Message::error("not enough PooledBuffer")
                                    .send(&mut writer)
                                    .await?;
                                return Ok(());
                            }
                        };
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
