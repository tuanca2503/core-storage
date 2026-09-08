use api::tcp::Server as TcpServer;
use api::tcp::TransferEvents;
use async_trait::async_trait;
use database::Server as DatabaseServer;
use model::CHUNK_SIZE;
use model::Chunk;
use model::Object;
use service::BaseResult;
use tokio::signal::unix::SignalKind;
use tokio::signal::unix::signal;
use tokio::sync::watch;

use crate::{ChunkQueue, FastQueue, ObjectQueue};

const CLIENT_SIZE: u16 = 10;
const QUEUE_SIZE: u16 = 10 * 10; //100 x 32mib = ~3.12 GiB
const PORT: u16 = 7878;

struct TestHandler;
pub struct App {
    tcp: TcpServer,
    db: DatabaseServer,
    // http: HttpServer,
    // ws: WsServer,
    //
    chunk_queue: ChunkQueue,
    fast_queue: FastQueue,
    pending_queue: ObjectQueue,
    write_queue: ObjectQueue,
    //
    shutdown_tx: watch::Sender<bool>,
}

impl App {
    pub fn new() -> Self {
        let (shutdown_tx, _shutdown_rx) = watch::channel(false);
        Self {
            db: DatabaseServer::start(),
            tcp: TcpServer::start(PORT, CLIENT_SIZE, CHUNK_SIZE, QUEUE_SIZE, TestHandler),
            //
            chunk_queue: ChunkQueue::new(QUEUE_SIZE as usize),
            fast_queue: FastQueue::new(QUEUE_SIZE as usize),
            pending_queue: ObjectQueue::new((CLIENT_SIZE * 2) as usize),
            write_queue: ObjectQueue::new((CLIENT_SIZE * 2) as usize),
            shutdown_tx,
        }
    }

    pub async fn start(self) -> BaseResult<()> {
        let mut sigterm = signal(SignalKind::terminate())?;
        let mut sigint = signal(SignalKind::interrupt())?;
        let mut shutdown_rx = self.shutdown_tx.subscribe();
        tokio::select! {
            _ = shutdown_rx.changed() => tracing::info!("Server shutdown"),
            _ = sigterm.recv() => tracing::info!("Receive SIGTERM"),
            _ = sigint.recv() => tracing::info!("Receive SIGINT (Ctrl+C)")
        }
        self.stop().await
    }

    pub async fn stop(self) -> BaseResult<()> {
        drop(self.shutdown_tx);
        tokio::join!(self.tcp.stop(), self.db.stop(),);
        Ok(())
    }
}


#[async_trait]
impl TransferEvents for TestHandler {
    async fn on_new(&self, object: Object) -> std::io::Result<()> {
        println!(
            "[on_new] uuid={} filename={:?} extension={:?} mime={:?} total_size={}",
            object.external_id,
            object.filename,
            object.extension,
            object.mime_type,
            object.total_size,
        );
        println!("[on_new] -> bắt đầu stream mode, object đã sẵn sàng nhận chunk");
        Ok(())
    }

    async fn on_resume(&self, uuid: &str) -> std::io::Result<(u64, u64, u64)> {
        println!("[on_resume] uuid={uuid} -> client yêu cầu resume upload");

        // TODO thật: tra trong queue xem uuid đã tồn tại chưa, lấy chunk_index hiện có
        let chunk_index: u64 = 0; // giả lập: chưa có chunk nào
        let bytes_received: u64 = 0; // giả lập: chunk_index * chunk_size
        let total_size: u64 = 0; // giả lập: lấy từ object đã lưu theo uuid

        println!(
            "[on_resume] uuid={uuid} -> tra được: chunk_index={chunk_index}, bytes_received={bytes_received}, total_size={total_size}"
        );

        Ok((chunk_index, bytes_received, total_size))
    }
    async fn on_chunk(&self, chunk: Chunk, faster: bool) -> std::io::Result<()> {
        // let preview_len = chunk.data.len().min(16);
        // let hex_preview: String = chunk.data[..preview_len]
        //     .iter()
        //     .map(|b| format!("{b:02x}"))
        //     .collect::<Vec<_>>()
        //     .join(" ");

        // println!(
        //     "[on_chunk] uuid={} chunk_index={} len={} preview=[{}{}]",
        //     chunk.uuid,
        //     chunk.chunk_index,
        //     chunk.len,
        //     hex_preview,
        //     if chunk.data.len() > preview_len { " ..." } else { "" },
        // );

        // TODO thật: push (chunk.uuid, chunk.chunk_index, chunk.data, chunk.len) vào processing queue
        println!("[on_chunk] -> (giả lập) đã đẩy chunk vào processing queue");

        Ok(())
    }

    async fn on_complete(&self, uuid: &str) -> std::io::Result<()> {
        println!("[on_complete] uuid={uuid} -> nhận đủ toàn bộ data, verify OK");
        println!("[on_complete] -> (giả lập) đã push object sang completed queue");
        Ok(())
    }

    async fn on_close(&self, uuid: &str) -> std::io::Result<()> {
        println!("[on_close] uuid={uuid} -> client đóng kết nối / hủy transfer giữa chừng");
        println!("[on_close] -> (giả lập) đã remove object khỏi pending queue");
        Ok(())
    }
}
