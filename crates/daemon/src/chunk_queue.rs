use tokio::sync::{mpsc, Mutex};
use std::sync::Arc;
use model::Chunk;

pub struct ChunkQueue {
    sender: mpsc::Sender<Chunk>,
    receiver: Arc<Mutex<mpsc::Receiver<Chunk>>>,
}

impl ChunkQueue {
    pub fn new(capacity: usize) -> Self {
        let (tx, rx) = mpsc::channel(capacity); // bounded -> tự backpressure
        Self {
            sender: tx,
            receiver: Arc::new(Mutex::new(rx)),
        }
    }

    // Producer: đẩy chunk vào queue, chờ nếu đầy (backpressure)
    pub async fn push(&self, item: Chunk) -> Result<(), mpsc::error::SendError<Chunk>> {
        self.sender.send(item).await
    }

    // Consumer: nhiều luồng HDD gọi hàm này, ai rảnh lấy được item đó
    pub async fn pop(&self) -> Option<Chunk> {
        let mut rx = self.receiver.lock().await;
        rx.recv().await
    }

    pub fn sender(&self) -> mpsc::Sender<Chunk> {
        self.sender.clone()
    }
}