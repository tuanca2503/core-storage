use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::{Semaphore, OwnedSemaphorePermit};


const ACCQUIRE_TIMEOUT: Duration = Duration::from_secs(180);

pub struct BufferPool {
    semaphore: Arc<Semaphore>,
    slots: Arc<Mutex<Vec<Vec<u8>>>>,
}

pub struct PooledBuffer {
    buf: Option<Vec<u8>>,
    slots: Arc<Mutex<Vec<Vec<u8>>>>,
    _permit: OwnedSemaphorePermit, // giữ tới khi buffer trả về pool
}

fn lock_slots(m: &Mutex<Vec<Vec<u8>>>) -> std::sync::MutexGuard<'_, Vec<Vec<u8>>> {
    // tránh panic-lồng-panic (abort) nếu mutex từng bị poison do 1 panic hiếm gặp
    m.lock().unwrap_or_else(|e| e.into_inner())
}

impl BufferPool {
    pub fn new(pool_size: usize, chunk_size: usize) -> Self {
        assert!(pool_size > 0, "pool_size phải > 0");
        assert!(chunk_size > 0, "chunk_size phải > 0");
        let slots = (0..pool_size).map(|_| vec![0u8; chunk_size]).collect();
        Self {
            semaphore: Arc::new(Semaphore::new(pool_size)),
            slots: Arc::new(Mutex::new(slots)),
        }
    }

    pub async fn acquire(&self) -> PooledBuffer {
        let permit = self.semaphore.clone().acquire_owned().await
            .expect("semaphore không bao giờ close");
        let buf = lock_slots(&self.slots).pop()
            .expect("permit đảm bảo luôn còn slot tương ứng");
        PooledBuffer { buf: Some(buf), slots: self.slots.clone(), _permit: permit }
    }

    /// Bound thời gian chờ tối đa khi pool cạn — chặn slow-drip DoS thay vì treo vô hạn.
    pub async fn acquire_timeout(&self) -> Option<PooledBuffer> {
        tokio::time::timeout(ACCQUIRE_TIMEOUT, self.acquire()).await.ok()
    }

    /// Số buffer đang rảnh — log/metric để biết pool sắp cạn trước khi nó cạn thật.
    pub fn available(&self) -> usize {
        self.semaphore.available_permits()
    }
}

impl std::ops::Deref for PooledBuffer {
    type Target = Vec<u8>;
    fn deref(&self) -> &Vec<u8> { self.buf.as_ref().expect("buf chỉ None sau drop") }
}
impl std::ops::DerefMut for PooledBuffer {
    fn deref_mut(&mut self) -> &mut Vec<u8> { self.buf.as_mut().expect("buf chỉ None sau drop") }
}

impl Drop for PooledBuffer {
    fn drop(&mut self) {
        if let Some(mut buf) = self.buf.take() {
            buf.fill(0);
            lock_slots(&self.slots).push(buf);
        }
        // _permit drop ngay sau (thứ tự field khai báo) -> slot đã nằm trong `slots`
        // TRƯỚC KHI semaphore báo permit rảnh, nên task đang chờ acquire() không
        // bao giờ pop trúng lúc rỗng.
    }
}