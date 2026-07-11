use rusqlite::Connection;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiskStatus {
    Active,
    Readonly,
    Offline,
    Retired,
}
#[derive(Debug, Clone)]
pub struct Disks {
    pub disk_id: i64,
    pub uuid: String,
    pub mount_path: String,
    pub status: DiskStatus,
    pub capacity_bytes: Option<i64>,
    pub used_bytes: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

impl Disks {
    pub fn create_table(conn: &Connection) -> rusqlite::Result<()> {
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS disks (
                disk_id         INTEGER PRIMARY KEY,
                uuid            TEXT NOT NULL UNIQUE,
                mount_path      TEXT NOT NULL,
                status          TEXT NOT NULL DEFAULT 'active'
                                    CHECK (status IN ('active','readonly','offline','retired')),
                capacity_bytes  INTEGER,
                used_bytes      INTEGER NOT NULL DEFAULT 0,
                created_at      INTEGER NOT NULL,
                updated_at      INTEGER NOT NULL
            );
            "#,
        )?;

        Ok(())
    }
}
