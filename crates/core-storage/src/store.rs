mod file;
mod models;
mod sqlite;

use crate::{
    error::{BaseError, BaseResult},
    store::{models::disks::Disks, sqlite::Sqlite},
};

pub struct Store {
    disks: Vec<Disks>,
    sqlite: Sqlite,
}
impl Store {
    pub fn new(path: &str) -> BaseResult<Self> {
        let mut s = Self {
            disks: Default::default(),
            sqlite: Sqlite::new(path)?, //test
        };
        s.scan_disks()?;
        Ok(s)
    }

    pub fn scan_disks(&mut self) -> BaseResult<()> {
        self.disks.clear();
        for item in sysinfo::Disks::new_with_refreshed_list().list() {
            let mount_path = item.mount_point();
            if item.is_read_only() || item.is_removable() || mount_path.as_os_str().is_empty() {
                continue;
            }
            //
            let mut disk = Disks::new(
                item.name().to_string_lossy().into_owned(),
                mount_path,
                item.total_space(),
                item.available_space(),
                item.kind().to_string(),
                item.file_system().to_string_lossy().into_owned(),
            );
            disk.synchronization(self.sqlite.get_conn())?;
            self.disks.push(disk);
        }
        Ok(())
    }
    pub fn get_disks(&self) -> Vec<&Disks> {
        self.disks.iter().collect()
    }
    pub fn get_active_disks(&self) -> Vec<&Disks> {
        self.disks.iter().filter(|disk| disk.active).collect()
    }

    pub fn active_disks(&mut self, index: usize) -> BaseResult<()> {
        //
        let disk = self
            .disks
            .get_mut(index)
            .ok_or_else(|| BaseError::not_found(format!("not found disk from index {}", index)))?;
        disk.active(self.sqlite.get_conn())?;
        Ok(())
    }
}
