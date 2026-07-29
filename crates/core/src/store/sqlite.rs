// use crate::store::models::{chunks::Chunks,  objects::Objects, segments::Segments};
use rusqlite::Connection;
use std::path::Path;

pub struct Sqlite {
    conn: Connection,
}

impl Sqlite {
    pub fn new<P: AsRef<Path>>(path: P) -> rusqlite::Result<Self> {
        let conn = Connection::open(path)?;
        Self::configure(&conn)?;
        Self::create(&conn)?;
        Ok(Self { conn })
    }
    pub fn get_conn(&self) -> &Connection {
        &self.conn
    }
    //
    fn create(conn: &Connection) -> rusqlite::Result<()> {
        // _ = Segments::sql_create(conn);
        // _ = Chunks::sql_create(conn);
        // _ = Objects::sql_create(conn);
        Ok(())
    }
    fn configure(conn: &Connection) -> rusqlite::Result<()> {
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "FULL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        Ok(())
    }
}
