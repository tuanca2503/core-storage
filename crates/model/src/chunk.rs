pub mod chunk_db;
#[derive(Debug, Clone)]
pub struct Chunk {
    pub disk_uuid: [u8; 16],
    pub object_id: i64,
    pub sequence: i64, // Position in object: 0,1,2...
    pub segment_index: i64, // Segment index: 0,1,2...
    pub chunk_index: i64, // Chunk index: 0,1,2...
    pub checksum: Vec<u8>, // CRC32/SHA-256 của chunk
}