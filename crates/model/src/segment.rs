
pub mod segment_bin;
pub const SEGMENT_SIZE: u64 = 64 * 1024 * 1024 * 1024; //64GB
pub const HEADER_SIZE: u64 = 4 * 1024; //4096
pub const CHUNK_SIZE: u64 = 4 * 1024; //4096
pub struct Segment {
    pub chunk_count: u64,
    pub chunk_capacity: u64,
}
impl Segment {
    pub fn new(capacity_bytes: u64) -> Self {
        Self {
            chunk_count: 0,
            chunk_capacity: capacity_bytes / CHUNK_SIZE,
        }
    }
}

impl Default for Segment {
    fn default() -> Self {
        Self {
            chunk_count: 0,
            chunk_capacity: SEGMENT_SIZE / CHUNK_SIZE,
        }
    }
}