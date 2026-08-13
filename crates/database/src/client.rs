use platform::paths;
use rusqlite::{Connection, OpenFlags, Result, Row, params_from_iter, types::Value};

fn get_connection() -> Result<Connection> {
    let conn = Connection::open_with_flags(
        &paths::get_database_path(),
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
    )?;
    conn.pragma_update(None, "busy_timeout", 5000)?;
    Ok(conn)
}

pub fn one<T, F>(sql: String, params: Vec<Value>, mapper: F) -> Result<T>
where
    T: Send + 'static,
    F: Fn(&Row) -> Result<T> + Send + 'static,
{
    Ok(get_connection()?.query_row(&sql, params_from_iter(params.iter()), |row| mapper(row))?)
}

pub fn many<T, F>(sql: String, params: Vec<Value>, mapper: F) -> Result<Vec<T>>
where
    T: Send + 'static,
    F: Fn(&Row) -> Result<T> + Send + 'static,
{
    let conn = get_connection()?;
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(params.iter()), |row| mapper(row))?;
    rows.collect()
}
