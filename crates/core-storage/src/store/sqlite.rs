use rusqlite::Connection;
use crate::store::models::*;

pub struct Sqlite {
    conn: Connection,
}

impl Sqlite {
    pub fn new(path: impl AsRef<std::path::Path>) -> rusqlite::Result<Self> {
        let conn = Connection::open(path)?;
        Self::configure(&conn)?;
        Self::create(&conn)?;
        Ok(Self { conn })
    }

    fn configure(conn: &Connection) -> rusqlite::Result<()> {
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "FULL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        Ok(())
    }
    pub fn create(conn: &Connection) -> rusqlite::Result<()> {
        disks::Disks::create_table(conn);
        segments::Segments::create_table(conn);

        Ok(())
    }
}
