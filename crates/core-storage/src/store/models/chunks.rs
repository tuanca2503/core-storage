use rusqlite::Connection;

#[derive(Debug, Clone)]
pub enum ChunkStatus {
    Temp,
    Committed,
    Trash,
    Deleted,
}
impl ChunkStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ChunkStatus::Temp => "temp",
            ChunkStatus::Committed => "committed",
            ChunkStatus::Trash => "trash",
            ChunkStatus::Deleted => "deleted",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "temp" => Some(ChunkStatus::Temp),
            "committed" => Some(ChunkStatus::Committed),
            "trash" => Some(ChunkStatus::Trash),
            "deleted" => Some(ChunkStatus::Deleted),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Chunks {
    pub chunk_id: i64,
    pub object_id: i64,
    pub chunk_index: i64, // thứ tự trong object: 0,1,2...
    pub disk_id: i64,
    pub segment_id: Option<i64>, // NULL ở Giai đoạn 1
    pub offset: Option<i64>,     // NULL ở Giai đoạn 1, bắt buộc ở Giai đoạn 2
    pub length: i64,
    pub checksum: Vec<u8>, // CRC32/SHA-256 của chunk
    pub status: ChunkStatus,
    pub created_at: i64,
}

impl Chunks {
    pub fn sql_create(conn: &Connection) -> rusqlite::Result<()> {
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS chunks (
                chunk_id        INTEGER PRIMARY KEY,
                object_id       INTEGER NOT NULL REFERENCES objects(object_id),
                chunk_index     INTEGER NOT NULL,           -- thứ tự trong object: 0,1,2...
                disk_id         INTEGER NOT NULL REFERENCES disks(disk_id),
                segment_id      INTEGER REFERENCES segments(segment_id), -- NULL ở Giai đoạn 1
                "offset"        INTEGER,                    -- NULL ở Giai đoạn 1, bắt buộc ở Giai đoạn 2
                length          INTEGER NOT NULL,
                checksum        BLOB NOT NULL,               -- CRC32/SHA-256 của riêng chunk này
                status          TEXT NOT NULL DEFAULT 'temp'
                                    CHECK (status IN ('temp','committed','trash','deleted')),
                created_at      INTEGER NOT NULL,
                UNIQUE (object_id, chunk_index)
            );
            CREATE INDEX IF NOT EXISTS idx_chunks_object   ON chunks(object_id, chunk_index);
            CREATE INDEX IF NOT EXISTS idx_chunks_segment  ON chunks(segment_id);
            CREATE INDEX IF NOT EXISTS idx_chunks_disk     ON chunks(disk_id);
            CREATE INDEX IF NOT EXISTS idx_chunks_status   ON chunks(status);
            "#,
        )?;

        Ok(())
    }
}
