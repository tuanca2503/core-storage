use crate::store::file::read_string;
use rusqlite::Connection;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Disks {
    pub disk_id: i64,
    pub name: String,
    pub uuid: String,
    pub mount_path: PathBuf,
    pub active: bool,
    pub capacity_bytes: u64,
    pub available_bytes: u64,
    pub kind: String,
    pub fs: String,
    pub created_at: i64,
    pub updated_at: i64,
}

impl Disks {
    pub fn new(
        name: String,
        mount_path: &Path,
        capacity_bytes: u64,
        available_bytes: u64,
        kind: String,
        fs: String,
    ) -> Self {
        Self {
            disk_id: 0,
            uuid: "".to_string(),
            active: false,
            created_at: 0,
            updated_at: 0,
            name,
            mount_path: mount_path.to_path_buf(),
            capacity_bytes,
            available_bytes,
            kind,
            fs,
        }
    }

    //SQLITE
    fn sql_insert(conn: &Connection) -> rusqlite::Result<()> {
        // read_string(path);
        
        Ok(())
    }

    pub fn sql_create(conn: &Connection) -> rusqlite::Result<()> {
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS disks (
                disk_id         INTEGER PRIMARY KEY,
                uuid            TEXT NOT NULL UNIQUE,
                mount_path      TEXT NOT NULL,
                active          INTEGER NOT NULL DEFAULT 0,
                capacity_bytes  INTEGER,
                available_bytes INTEGER NOT NULL DEFAULT 0,
                created_at      INTEGER NOT NULL,
                updated_at      INTEGER NOT NULL
            );
            "#,
        )?;
        Ok(())
    }
    pub fn sql_select(conn: &Connection) -> rusqlite::Result<()> {
        Ok(())
    }
    pub fn sql_update_status(conn: &Connection) -> rusqlite::Result<()> {
        Ok(())
    }
}
