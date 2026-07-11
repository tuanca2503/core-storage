use rusqlite::Connection;

pub struct Segments {}

impl Segments {
    pub fn create_table(conn: &Connection) -> rusqlite::Result<()> {
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
