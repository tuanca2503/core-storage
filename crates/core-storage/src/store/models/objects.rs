use rusqlite::Connection;

pub struct Objects {}

impl Objects {
    pub fn create_table(conn: &Connection) -> rusqlite::Result<()> {
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
