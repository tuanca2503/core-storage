use async_trait::async_trait;
use model::Object;
use uuid::Uuid;

#[async_trait]
pub trait TransferEvents: Send + Sync {
    async fn on_info(&self, object:Object) -> std::io::Result<()>;
    async fn on_chunk(&self, conn_id: Uuid, chunk_index: u64, data: Vec<u8>, len: usize) -> std::io::Result<()>;
    async fn on_complete(&self, conn_id: Uuid, hash: Vec<u8>) -> std::io::Result<()>;
}