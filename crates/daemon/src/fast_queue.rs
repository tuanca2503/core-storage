use model::Chunk;
use tokio::sync::broadcast;

pub struct FastQueue {
    sender: broadcast::Sender<Chunk>,
}

impl FastQueue {
    pub fn new(capacity: usize) -> Self {
        let (tx, _rx) = broadcast::channel(capacity);
        Self { sender: tx }
    }

    pub fn push(&self, item: Chunk) -> Result<usize, broadcast::error::SendError<Chunk>> {
        self.sender.send(item)
    }

    // Mỗi luồng SSD gọi hàm này để có 1 receiver riêng, tự nhận toàn bộ broadcast
    pub fn subscribe(&self) -> broadcast::Receiver<Chunk> {
        self.sender.subscribe()
    }
}