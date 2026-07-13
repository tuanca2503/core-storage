use rusqlite::Connection;
pub enum SegmentStatus {
    Open,
    Sealed,
    Compacting,
    Deleted,
}

impl SegmentStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            SegmentStatus::Open => "open",
            SegmentStatus::Sealed => "sealed",
            SegmentStatus::Compacting => "compacting",
            SegmentStatus::Deleted => "deleted",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "open" => Some(SegmentStatus::Open),
            "sealed" => Some(SegmentStatus::Sealed),
            "compacting" => Some(SegmentStatus::Compacting),
            "deleted" => Some(SegmentStatus::Deleted),
            _ => None,
        }
    }
}
pub struct Segments {
    pub segment_id: i64,
    pub disk_id: i64,
    pub file_name: String, // segment000001.dat
    pub size_bytes: i64,
    pub status: SegmentStatus,
    pub created_at: i64,        // unix timestamp
    pub sealed_at: Option<i64>, // NULL cho tới khi seal
}

impl Segments {
    pub fn sql_create(conn: &Connection) -> rusqlite::Result<()> {
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS segments (
                segment_id      INTEGER PRIMARY KEY,
                disk_id         INTEGER NOT NULL REFERENCES disks(disk_id),
                file_name       TEXT NOT NULL,              -- segment000001.dat
                size_bytes      INTEGER NOT NULL DEFAULT 0,
                status          TEXT NOT NULL DEFAULT 'open'
                                    CHECK (status IN ('open','sealed','compacting','deleted')),
                created_at      INTEGER NOT NULL,
                sealed_at       INTEGER,
                UNIQUE (disk_id, file_name)
            );
            "#,
        )?;

        Ok(())
    }
}
