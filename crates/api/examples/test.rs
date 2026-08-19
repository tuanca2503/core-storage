use std::{sync::Arc, time::Duration};

use api::tcp::{BufferPool, TransferEvents, server::Server};
use async_trait::async_trait;
use model::Object;

struct TestHandler;

#[async_trait]
impl TransferEvents for TestHandler {
    async fn on_new(&self, object: Object) -> std::io::Result<()> {
        println!("start streammode");
        Ok(())
    }
    async fn on_resume(&self, uuid: &str) -> std::io::Result<(u64, u64, u64)> {
        println!("[{:?}] 12", uuid);
        //TODO: 
        let chunk_index=0; // get form queue
        let bytes_received=0; // chunk_index * obj.chunk_size
        let total_size =0; // obj.total_size

        Ok((chunk_index, bytes_received, total_size))
    }

    async fn on_complete(&self, uuid: &str) -> std::io::Result<()>{
        println!("done [{:?}]", uuid);
        Ok(())
    }

    //TODO check exits on queue
    // async fn on_exists(&self, conn_id: Uuid, hash: Vec<u8>) -> std::io::Result<()> {
    //     println!("[{conn_id}] END, hash dài {} byte", hash.len());
    //     Ok(())
    // }
}

//cargo run -p core --example test
#[tokio::main]
async fn main() -> std::io::Result<()> {
    let buffer_pool = Arc::new(BufferPool::new(128, 8 as usize));
    let handler: Arc<dyn TransferEvents> = Arc::new(TestHandler);

    let server = Server::start(7878, 120,  handler, buffer_pool);

    println!("Server đang lắng nghe tại 0.0.0.0:7878");

    tokio::signal::ctrl_c().await?;
    println!("Nhận Ctrl+C, dừng nhận kết nối mới...");
    server.shutdown().await;
    println!("Server đã dừng.");
    Ok(())
}
