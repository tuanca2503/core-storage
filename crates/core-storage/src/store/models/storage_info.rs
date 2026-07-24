// use chrono::Utc;
// use core_storage_platform::{StorageInfo, StorageState, error::BaseResult};
// use rusqlite::{Connection, params};

// pub trait StorageInfoExt {
//     fn synchronization(&mut self, _conn: &Connection) -> BaseResult<()>;
//     fn active(&mut self, conn: &Connection) -> rusqlite::Result<()>;
//     fn deactive(&mut self, conn: &Connection) -> rusqlite::Result<()>;
//     fn sql_create(conn: &Connection) -> rusqlite::Result<()>;
//     fn sql_insert(&self, conn: &Connection) -> rusqlite::Result<()>;
//     fn sql_update(&self, conn: &Connection) -> rusqlite::Result<()>;
//     fn sql_select(&mut self, conn: &Connection) -> rusqlite::Result<()>;
//     fn load_header(&mut self);
//     fn is_formatted(&self) -> bool;
// }


// impl StorageInfoExt for StorageInfo {
    
//     fn synchronization(&mut self, _conn: &Connection) -> BaseResult<()> {
//         // let uuid_path = self.mount_path.join(UUID_FILE);
//         // self.uuid = read_string(&uuid_path).unwrap_or_default();
//         // if self.uuid.is_empty() {
//         //     self.ensure_uuid(conn)?;
//         //     write_string(&uuid_path, &self.uuid)?;
//         //     self.sql_insert(conn)?;
//         // } else {
//         //     match self.sql_select(conn) {
//         //         Ok(_) => {}
//         //         Err(_) => {
//         //             self.sql_insert(conn)?;
//         //         }
//         //     }
//         //     self.sql_update(conn)?;
//         // }
//         Ok(())
//     }
//     fn active(&mut self, conn: &Connection) -> rusqlite::Result<()> {
//         if self.header.state == StorageState::Active {
//             return Ok(());
//         }
//         //
//         self.header.state = StorageState::Active;
//         let now = Utc::now().timestamp();
//         conn.execute(
//             "UPDATE storage_info SET active = 1, updated_at = ?1 WHERE uuid = ?2",
//             (now, &self.uuid),
//         )?;
//         Ok(())
//     }
//     fn deactive(&mut self, conn: &Connection) -> rusqlite::Result<()> {
//         if !self.active {
//             return Ok(());
//         }
//         //
//         self.active = false;
//         let now = Utc::now().timestamp();
//         conn.execute(
//             "UPDATE disks SET active = 0, updated_at = ?1 WHERE uuid = ?2",
//             (now, &self.uuid),
//         )?;
//         Ok(())
//     }

//     //SQLITE
//     fn sql_create(conn: &Connection) -> rusqlite::Result<()> {
//         conn.execute_batch(
//             r#"
//             CREATE TABLE IF NOT EXISTS disks (
//                 disk_id         INTEGER PRIMARY KEY,
//                 uuid            TEXT NOT NULL UNIQUE,
//                 note            TEXT ,
//                 name            TEXT NOT NULL,
//                 mount_path      TEXT NOT NULL,
//                 kind            TEXT NOT NULL,
//                 fs              TEXT NOT NULL,
//                 active          INTEGER NOT NULL DEFAULT 0,
//                 capacity_bytes  INTEGER NOT NULL,
//                 available_bytes INTEGER NOT NULL,
//                 created_at      INTEGER NOT NULL,
//                 updated_at      INTEGER NOT NULL
//             );
//             "#,
//         )?;
//         Ok(())
//     }
//     fn sql_insert(&self, conn: &Connection) -> rusqlite::Result<()> {
//         let now = Utc::now().timestamp();
//         conn.execute(
//             r#"
//             INSERT INTO disks (
//                 name,
//                 uuid,
//                 mount_path,
//                 active,
//                 capacity_bytes,
//                 available_bytes,
//                 kind,
//                 fs,
//                 created_at,
//                 updated_at
//             )
//             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
//             "#,
//             params![
//                 &self.name,
//                 &self.uuid,
//                 self.mount_path.to_string_lossy(),
//                 self.active,
//                 self.capacity_bytes as i64,
//                 self.available_bytes as i64,
//                 &self.kind,
//                 &self.fs,
//                 now,
//                 now,
//             ],
//         )?;

//         Ok(())
//     }
//     fn sql_update(&self, conn: &Connection) -> rusqlite::Result<()> {
//         let now = Utc::now().timestamp();
//         conn.execute(
//             r#"
//             UPDATE disks
//             SET
//                 name = ?1,
//                 mount_path = ?2,
//                 capacity_bytes = ?3,
//                 available_bytes = ?4,
//                 kind = ?5,
//                 fs = ?6,
//                 updated_at = ?7
//             WHERE uuid = ?8
//             "#,
//             params![
//                 &self.name,
//                 self.mount_path.to_string_lossy(),
//                 self.capacity_bytes as i64,
//                 self.available_bytes as i64,
//                 &self.kind,
//                 &self.fs,
//                 now,
//                 &self.uuid,
//             ],
//         )?;
//         Ok(())
//     }
//     fn sql_select(&mut self, conn: &Connection) -> rusqlite::Result<()> {
//         let (disk_id, active): (i64, bool) = conn.query_row(
//             "SELECT disk_id, active FROM disks WHERE uuid = ?1",
//             params![&self.uuid],
//             |row| Ok((row.get(0)?, row.get(1)?)),
//         )?;

//         self.disk_id = disk_id;
//         self.active = active;
//         Ok(())
//     }
    
//     fn load_header(&mut self) {
//         todo!()
//     }
    
//     fn is_formatted(&self) -> bool {
//         todo!()
//     }
// }
