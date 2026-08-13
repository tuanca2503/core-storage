use platform::paths;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{Connection, Error, OpenFlags, Result, Row, params_from_iter, types::Value};
use tokio::{
    sync::{mpsc, oneshot},
    task,
};

const MAX_QUEUE: usize = 64;
const MAX_POOL: u32 = 8;

struct WriteJob {
    sql: String,
    rows: Vec<Vec<Value>>,
    reply: Option<oneshot::Sender<Result<()>>>,
}

pub struct Server {
    tx: mpsc::Sender<WriteJob>,
    handle: task::JoinHandle<()>,
    pool: r2d2::Pool<SqliteConnectionManager>,
}
impl Server {
    /// Initializes the writer, setting up a connection pool, an mpsc channel
    /// for `WriteJob`s, and a background blocking task to process them.
    pub fn start() -> Self {
        let (tx, mut rx) = mpsc::channel::<WriteJob>(MAX_QUEUE);
        let handle = tokio::task::spawn_blocking(move || {
            let mut conn = Connection::open(paths::get_database_path()).expect("open db failed");
            Self::apply_pragmas(&conn).expect("Failed apply pragma");

            while let Some(job) = rx.blocking_recv() {
                let result = (|| -> Result<()> {
                    let tx = conn.transaction()?;
                    if job.rows.is_empty() {
                        // No param to bind -> (DDL, migration, script)
                        tx.execute_batch(&job.sql)?;
                    } else {
                        let mut stmt = tx.prepare(&job.sql)?;
                        for row in &job.rows {
                            stmt.execute(params_from_iter(row.iter()))?;
                        }
                    }
                    tx.commit()?;
                    Ok(())
                })();
                if let Err(ref e) = result {
                    eprintln!("write job failed: sql={}, err={:?}", job.sql, e);
                }
                if let Some(reply) = job.reply {
                    let _ = reply.send(result);
                }
            }
        });
        //
        let manager = SqliteConnectionManager::file(paths::get_database_path())
            .with_flags(OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI)
            .with_init(|conn| {
                conn.pragma_update(None, "busy_timeout", 5000)?;
                Ok(())
            });
        let pool = r2d2::Pool::builder()
            .max_size(MAX_POOL) // tùy chỉnh theo tải thực tế
            .build(manager)
            .expect("Failed create pool");

        Self { tx, handle, pool }
    }

    /// Shuts down the writer gracefully: dropping `tx` closes the channel so
    /// the blocking task can finish processing remaining jobs and exit, then
    /// awaits its completion.
    pub async fn shutdown(self) {
        drop(self.tx);
        let _ = self.handle.await;
    }

    /// Enqueues a write job and waits for it to complete.
    ///
    /// Sends `sql`/`rows` to the writer task along with a oneshot reply channel,
    /// then awaits the result once the job has been executed.
    ///
    /// # Errors
    /// Returns an error if the write queue is closed or the writer task was
    /// dropped before replying, as well as any error from executing the query.
    pub async fn wait_archive(&self, sql: String, rows: Vec<Vec<Value>>) -> Result<()> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(WriteJob {
                sql,
                rows,
                reply: Some(reply_tx),
            })
            .await
            .map_err(|_| Error::ToSqlConversionFailure("write queue closed".into()))?;

        reply_rx
            .await
            .map_err(|_| Error::ToSqlConversionFailure("writer thread dropped".into()))?
    }

    /// Enqueues a write job without waiting for it to complete (fire-and-forget).
    ///
    /// Sends `sql`/`rows` to the writer task with no reply channel; returns as
    /// soon as the job is queued, not when it's executed.
    ///
    /// # Errors
    /// Returns an error if the write queue is closed.
    pub async fn archive(&self, sql: String, rows: Vec<Vec<Value>>) -> Result<()> {
        self.tx
            .send(WriteJob {
                sql,
                rows,
                reply: None,
            })
            .await
            .map_err(|_| Error::ToSqlConversionFailure("write queue closed".into()))?;
        Ok(())
    }

    /// Executes a query on a pooled connection and maps each row into `T`.
    ///
    /// Runs on `spawn_blocking` (via the pool) so it doesn't block the async
    /// runtime. `mapper` is called once per row, following the same signature
    /// as `rusqlite::Row` mapping closures.
    ///
    /// # Errors
    /// Returns an error if the connection cannot be obtained from the pool,
    /// the query fails to execute, or `mapper` returns an error for any row.
    ///
    /// # Example
    /// ```
    /// let users: Vec<User> = write_queue
    ///     .many(
    ///         "SELECT id, name FROM users WHERE age > ?1".to_string(),
    ///         vec![Value::from(18)],
    ///         |row| Ok(User { id: row.get(0)?, name: row.get(1)? }),
    ///     )
    ///     .await?;
    /// ```
    pub async fn many<T, F>(&self, sql: String, params: Vec<Value>, mapper: F) -> Result<Vec<T>>
    where
        T: Send + 'static,
        F: Fn(&Row) -> Result<T> + Send + 'static,
    {
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> Result<Vec<T>> {
            let conn = pool.get().map_err(|_| {
                Error::ToSqlConversionFailure("pool exhausted or connection failed".into())
            })?;

            let mut stmt = conn.prepare(&sql)?;
            let rows = stmt.query_map(params_from_iter(params.iter()), |row| mapper(row))?;

            rows.collect() // Iterator<Item = Result<T>> → Result<Vec<T>>
        })
        .await
        .map_err(|_| Error::ToSqlConversionFailure("select task panicked".into()))?
    }

    /// Executes a query on a pooled connection and maps the first row into `T`.
    ///
    /// Runs on `spawn_blocking` (via the pool) so it doesn't block the async
    /// runtime. `mapper` is applied to the first returned row only.
    ///
    /// # Errors
    /// Returns an error if the connection cannot be obtained from the pool,
    /// the query fails to execute, no rows are returned
    /// (`rusqlite::Error::QueryReturnedNoRows`), or `mapper` fails.
    pub async fn one<T, F>(&self, sql: String, params: Vec<Value>, mapper: F) -> Result<T>
    where
        T: Send + 'static,
        F: Fn(&Row) -> Result<T> + Send + 'static,
    {
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> Result<T> {
            let conn = pool.get().map_err(|_| {
                Error::ToSqlConversionFailure("pool exhausted or connection failed".into())
            })?;

            conn.query_row(&sql, params_from_iter(params.iter()), |row| mapper(row))
        })
        .await
        .map_err(|_| Error::ToSqlConversionFailure("select_one task panicked".into()))?
    }

    /// Applies SQLite pragmas for durability and concurrency: enables WAL mode,
    /// sets a busy timeout so writers wait on locks instead of failing
    /// immediately, and sets `synchronous = FULL` to balance durability and
    /// throughput under WAL.
    fn apply_pragmas(conn: &Connection) -> Result<()> {
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "busy_timeout", 5000)?; // ms, chờ thay vì fail ngay khi gặp lock
        conn.pragma_update(None, "synchronous", "FULL") // cân bằng durability/throughput với WAL
    }
}
