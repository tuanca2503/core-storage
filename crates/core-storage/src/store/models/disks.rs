use chrono::Utc;
use core_storage_platform::error::BaseResult;
use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};
use uuid::Uuid;

use crate::store::file::{read_string, write_string};

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
const UUID_FILE: &str = ".uuid";

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
            active: false,
            created_at: 0,
            updated_at: 0,
            uuid: "".to_string(),
            name,
            mount_path: mount_path.to_path_buf(),
            capacity_bytes,
            available_bytes,
            kind,
            fs,
        }
    }
    pub fn synchronization(&mut self, conn: &Connection) -> BaseResult<()> {
        let uuid_path = self.mount_path.join(UUID_FILE);
        self.uuid = read_string(&uuid_path).unwrap_or_default();
        if self.uuid.is_empty() {
            self.ensure_uuid(conn)?;
            write_string(&uuid_path, &self.uuid)?;
            self.sql_insert(conn)?;
        } else {
            match self.sql_select(conn) {
                Ok(_) => {}
                Err(_) => {
                    self.sql_insert(conn)?;
                }
            }
            self.sql_update(conn)?;
        }
        Ok(())
    }
    pub fn active(&mut self, conn: &Connection) -> rusqlite::Result<()> {
        if self.active {
            return Ok(());
        }
        //
        self.active = true;
        let now = Utc::now().timestamp();
        conn.execute(
            "UPDATE disks SET active = 1, updated_at = ?1 WHERE uuid = ?2",
            (now, &self.uuid),
        )?;
        Ok(())
    }
    pub fn deactive(&mut self, conn: &Connection) -> rusqlite::Result<()> {
        if !self.active {
            return Ok(());
        }
        //
        self.active = false;
        let now = Utc::now().timestamp();
        conn.execute(
            "UPDATE disks SET active = 0, updated_at = ?1 WHERE uuid = ?2",
            (now, &self.uuid),
        )?;
        Ok(())
    }
    //
    fn ensure_uuid(&mut self, conn: &Connection) -> rusqlite::Result<()> {
        loop {
            let uuid = Uuid::now_v7().to_string();
            let exists: bool = conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM disks WHERE uuid = ?1)",
                params![&uuid],
                |row| row.get(0),
            )?;
            if !exists {
                self.uuid = uuid;
                return Ok(());
            }
        }
    }
    //SQLITE
    pub fn sql_create(conn: &Connection) -> rusqlite::Result<()> {
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS disks (
                disk_id         INTEGER PRIMARY KEY,
                uuid            TEXT NOT NULL UNIQUE,
                note            TEXT ,
                name            TEXT NOT NULL,
                mount_path      TEXT NOT NULL,
                kind            TEXT NOT NULL,
                fs              TEXT NOT NULL,
                active          INTEGER NOT NULL DEFAULT 0,
                capacity_bytes  INTEGER NOT NULL,
                available_bytes INTEGER NOT NULL,
                created_at      INTEGER NOT NULL,
                updated_at      INTEGER NOT NULL
            );
            "#,
        )?;
        Ok(())
    }
    pub fn sql_insert(&self, conn: &Connection) -> rusqlite::Result<()> {
        let now = Utc::now().timestamp();
        conn.execute(
            r#"
            INSERT INTO disks (
                name,
                uuid,
                mount_path,
                active,
                capacity_bytes,
                available_bytes,
                kind,
                fs,
                created_at,
                updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            "#,
            params![
                &self.name,
                &self.uuid,
                self.mount_path.to_string_lossy(),
                self.active,
                self.capacity_bytes as i64,
                self.available_bytes as i64,
                &self.kind,
                &self.fs,
                now,
                now,
            ],
        )?;

        Ok(())
    }
    pub fn sql_update(&self, conn: &Connection) -> rusqlite::Result<()> {
        let now = Utc::now().timestamp();
        conn.execute(
            r#"
            UPDATE disks
            SET
                name = ?1,
                mount_path = ?2,
                capacity_bytes = ?3,
                available_bytes = ?4,
                kind = ?5,
                fs = ?6,
                updated_at = ?7
            WHERE uuid = ?8
            "#,
            params![
                &self.name,
                self.mount_path.to_string_lossy(),
                self.capacity_bytes as i64,
                self.available_bytes as i64,
                &self.kind,
                &self.fs,
                now,
                &self.uuid,
            ],
        )?;
        Ok(())
    }
    pub fn sql_select(&mut self, conn: &Connection) -> rusqlite::Result<()> {
        let (disk_id, active): (i64, bool) = conn.query_row(
            "SELECT disk_id, active FROM disks WHERE uuid = ?1",
            params![&self.uuid],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        self.disk_id = disk_id;
        self.active = active;
        Ok(())
    }
}
