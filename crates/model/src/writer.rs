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

    pub fn seek(mut self, pos: usize) -> Self {
        if pos > self.0.len() {
            self.0.resize(pos, 0);
        }
        self
    }

    /// Ghi thêm `len` byte 0 (padding), tương ứng với `Reader::skip`.
    pub fn skip(mut self, len: usize) -> Self {
        self.0.extend(std::iter::repeat(0u8).take(len));
        self
    }

    pub fn write_u8(mut self, v: u8) -> Self {
        self.0.push(v);
        self
    }

    pub fn write_u16(mut self, v: u16) -> Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }

    pub fn write_u32(mut self, v: u32) -> Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }

    pub fn write_u64(mut self, v: u64) -> Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }

    /// Ghi đúng N byte, tương ứng với `Reader::read_bytes<N>`.
    pub fn write_bytes<const N: usize>(mut self, data: &[u8; N]) -> Self {
        self.0.extend_from_slice(data);
        self
    }
    pub fn write_string(mut self, data: Option<String>) -> Self {
        if let Some(s) = data {
            self.0.extend_from_slice(s.as_bytes());
        }
        self.0.push(0);// \x00 delimiter
        self
    }

    pub fn write_slice(mut self, data: &[u8]) -> Self {
        self.0.extend_from_slice(data);
        self
    }

    /// Ghi đè N byte tại vị trí `pos` mà KHÔNG làm thay đổi độ dài buffer hiện tại,
    /// tương ứng với `Reader::read_bytes_from`. Dùng để backpatch (VD ghi CRC sau
    /// khi đã biết toàn bộ nội dung phía trước).
    pub fn write_bytes_at<const N: usize>(mut self, pos: usize, data: &[u8; N]) -> Self {
        debug_assert!(pos + N <= self.0.len(), "write_bytes_at out of bounds");
        self.0[pos..pos + N].copy_from_slice(data);
        self
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.0
    }
}
