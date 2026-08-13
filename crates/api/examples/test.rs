use std::{sync::Arc, time::Duration};

use api::tcp::{BufferPool, TransferEvents, server::Server};
use async_trait::async_trait;
use model::Object;
use uuid::Uuid;




struct TestHandler;

#[async_trait]
impl TransferEvents for TestHandler {
    async fn on_info(&self, object:Object) -> std::io::Result<()> {
        println!("[{:?}]",object);
        Ok(())
    }

    async fn on_chunk(&self, conn_id: Uuid, chunk_index: u64, _data: Vec<u8>, len: usize) -> std::io::Result<()> {
        println!("[{conn_id}] chunk #{chunk_index}: {len} bytes");
        Ok(())
    }

    async fn on_complete(&self, conn_id: Uuid, hash: Vec<u8>) -> std::io::Result<()> {
        println!("[{conn_id}] END, hash dài {} byte", hash.len());
        Ok(())
    }
}

//cargo run -p core --example test
#[tokio::main]
async fn main() -> std::io::Result<()> {
    let buffer_pool = Arc::new(BufferPool::new(128, model::CHUNK_SIZE as usize));
    let handler: Arc<dyn TransferEvents> = Arc::new(TestHandler);

    let server = Server::start(
        7878,
        120,
        Duration::from_secs(30),
        handler,
        buffer_pool,
    );

    println!("Server đang lắng nghe tại 0.0.0.0:7878");

    tokio::signal::ctrl_c().await?;
    println!("Nhận Ctrl+C, dừng nhận kết nối mới...");
    server.shutdown().await;
    println!("Server đã dừng.");
    Ok(())
}
