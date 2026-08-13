use tokio::sync::{Mutex, mpsc};


pub struct BufferPool {
    tx: mpsc::Sender<Vec<u8>>,
    rx: Mutex<mpsc::Receiver<Vec<u8>>>,
}

impl BufferPool {
    pub fn new(pool_size: usize, chunk_size: usize) -> Self {
        let (tx, rx) = mpsc::channel(pool_size);
        for _ in 0..pool_size {
            tx.try_send(vec![0u8; chunk_size]).unwrap();
        }
        Self { tx, rx: Mutex::new(rx) }
    }

    // pool cạn -> .await ở đây tự chặn lại, KHÔNG fallback cấp phát mới
    // -> đây chính là chỗ bound RAM tổng của cả server, không phụ thuộc N client
    pub async fn acquire(&self) -> Vec<u8> {
        self.rx.lock().await.recv().await.expect("pool sender không bao giờ đóng")
    }

    pub async fn release(&self, mut buf: Vec<u8>) {
        buf.fill(0);
        let _ = self.tx.send(buf).await;
    }
}