use rusqlite::Connection;
pub enum ObjectStatus {
    Pending,
    Committed,
    Deleted,
}

impl ObjectStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ObjectStatus::Pending => "pending",
            ObjectStatus::Committed => "committed",
            ObjectStatus::Deleted => "deleted",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(ObjectStatus::Pending),
            "committed" => Some(ObjectStatus::Committed),
            "deleted" => Some(ObjectStatus::Deleted),
            _ => None,
        }
    }
}
pub struct Objects {
    pub object_id: i64,
    pub external_id: Option<String>,   // id do tầng app đặt, nullable
    pub total_size: i64,
    pub chunk_count: i64,
    pub status: ObjectStatus,
    pub created_at: i64,               // unix timestamp
    pub updated_at: i64,               // unix timestamp
}

impl Objects {
    pub fn sql_create(conn: &Connection) -> rusqlite::Result<()> {
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS objects (
                object_id       INTEGER PRIMARY KEY,
                external_id     TEXT UNIQUE,                -- id do tầng app đặt (nullable)
                total_size      INTEGER NOT NULL DEFAULT 0,
                chunk_count     INTEGER NOT NULL DEFAULT 0,
                status          TEXT NOT NULL DEFAULT 'pending'
                                    CHECK (status IN ('pending','committed','deleted')),
                created_at      INTEGER NOT NULL,
                updated_at      INTEGER NOT NULL
            );
            "#,
        )?;

        Ok(())
    }
}
