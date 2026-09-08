use std::sync::Arc;

use model::Object;
use tokio::sync::{Mutex, mpsc};

pub struct ObjectQueue {
    sender: mpsc::Sender<Object>,
    receiver: Arc<Mutex<mpsc::Receiver<Object>>>,
}

impl ObjectQueue {
    pub fn new(capacity: usize) -> Self {
        let (tx, rx) = mpsc::channel(capacity);
        Self {
            sender: tx,
            receiver: Arc::new(Mutex::new(rx)),
        }
    }

    pub async fn push(&self, item: Object) -> Result<(), mpsc::error::SendError<Object>> {
        self.sender.send(item).await
    }

    pub async fn pop(&self) -> Option<Object> {
        let mut rx = self.receiver.lock().await;
        rx.recv().await
    }

    pub fn sender(&self) -> mpsc::Sender<Object> {
        self.sender.clone()
    }
}