
pub struct Writer(Vec<u8>);

impl Writer {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn with_capacity(cap: usize) -> Self {
        Self(Vec::with_capacity(cap))
    }

    pub fn position(&self) -> usize {
        self.0.len()
    }

    pub fn seek(&mut self, pos: usize) {
        if pos > self.0.len() {
            self.0.resize(pos, 0);
        }
    }

    /// Ghi thêm `len` byte 0 (padding), tương ứng với `Reader::skip`.
    pub fn skip(&mut self, len: usize) {
        self.0.extend(std::iter::repeat(0u8).take(len));
    }

    pub fn write_u8(&mut self, v: u8) {
        self.0.push(v);
    }

    pub fn write_u16(&mut self, v: u16) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }

    pub fn write_u32(&mut self, v: u32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }

    pub fn write_u64(&mut self, v: u64) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }

    /// Ghi đúng N byte, tương ứng với `Reader::read_bytes<N>`.
    pub fn write_bytes<const N: usize>(&mut self, data: &[u8; N]) {
        self.0.extend_from_slice(data);
    }
    pub fn write_slice(&mut self, data: &[u8]) {
        self.0.extend_from_slice(data);
    }

    /// Ghi đè N byte tại vị trí `pos` mà KHÔNG làm thay đổi độ dài buffer hiện tại,
    /// tương ứng với `Reader::read_bytes_from`. Dùng để backpatch (VD ghi CRC sau
    /// khi đã biết toàn bộ nội dung phía trước).
    pub fn write_bytes_at<const N: usize>(&mut self, pos: usize, data: &[u8; N]) {
        debug_assert!(pos + N <= self.0.len(), "write_bytes_at out of bounds");
        self.0[pos..pos + N].copy_from_slice(data);
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.0
    }
}
