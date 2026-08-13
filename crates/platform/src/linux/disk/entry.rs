use std::{
    fs::{self, File, OpenOptions},
    io::{Error, Result},
    path::PathBuf,
    str::FromStr,
};

pub struct DiskEntry {
    pub name: String,
    pub sysfs_path: PathBuf,
    pub device_path: PathBuf,
}
impl DiskEntry {
    pub fn new(name: String) -> Self {
        Self {
            sysfs_path: PathBuf::from("/sys/block").join(&name),
            device_path: PathBuf::from("/dev").join(&name),
            name,
        }
    }
    pub fn verify(name: String) -> Result<Self> {
        let s = Self::new(name);
        if !s.device_path.exists() {
            return Err(Error::other(format!(
                "Device not found: {:?}",
                s.device_path
            )));
        }
        Ok(s)
    }
    /// Opens the device.
    ///
    /// # Arguments
    ///
    /// * `mode` - Device access mode:
    ///   - `0` - Read-only.
    ///   - `1` - Write-only.
    ///   - `2` - Read-write.
    ///
    /// # Returns
    ///
    /// An opened device handle as [`File`].
    pub fn open_device(&self, mode: u8) -> Result<File> {
        let file = match mode {
            2 => OpenOptions::new()
                .read(true)
                .write(true)
                .open(&self.device_path)?,
            1 => OpenOptions::new().write(true).open(&self.device_path)?,
            _ => OpenOptions::new().read(true).open(&self.device_path)?,
        };
        Ok(file)
    }
    pub fn serial(&self) -> String {
        self.read_sysfs::<String>("device/serial")
            .unwrap_or_default()
    }
    pub fn model(&self) -> String {
        let vendor = self
            .read_sysfs::<String>("device/vendor")
            .unwrap_or_default();
        let device_model = self
            .read_sysfs::<String>("device/model")
            .unwrap_or_default();
        format!("{}{}", vendor, device_model)
    }
    pub fn sectors(&self) -> u64 {
        self.read_sysfs::<u64>("size").unwrap_or(0)
    }
    pub fn logical_sector_size(&self) -> u32 {
        self.read_sysfs::<u32>("queue/logical_block_size")
            .unwrap_or(512)
    }
    pub fn physical_sector_size(&self, defailt: u32) -> u32 {
        self.read_sysfs::<u32>("queue/physical_block_size")
            .unwrap_or(defailt)
    }
    pub fn removable(&self) -> bool {
        self.read_sysfs::<u32>("removable").unwrap_or(0) != 0
    }
    pub fn read_only(&self) -> bool {
        self.read_sysfs::<u32>("ro").unwrap_or(0) != 0
    }
    pub fn rotational(&self) -> bool {
        self.read_sysfs::<u32>("queue/rotational").unwrap_or(0) != 0
    }
    pub fn capacity_bytes(&self, logical_sector_size: u32) -> u64 {
        self.sectors().saturating_mul(logical_sector_size as u64)
    }
    pub fn for_each_disk<F>(mut f: F) -> Result<()>
    where
        F: FnMut(&Self) -> Result<()>,
    {
        for entry in fs::read_dir("/sys/block")? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            let sysfs_path = entry.path();

            if name.starts_with("loop")
                || name.starts_with("ram")
                || name.starts_with("dm-")
                || name.starts_with("zram")
                || name.starts_with("md")
                || name.starts_with("sr")
                || !sysfs_path.join("device").exists()
            {
                continue;
            }

            f(&&Self::new(name))?;
        }

        Ok(())
    }
    pub fn has_volumes(&self) -> Result<bool> {
        for entry in fs::read_dir(&self.sysfs_path)? {
            let entry = entry?;
            if entry.file_type()?.is_dir()
                && entry.file_name().to_string_lossy().starts_with(&self.name)
            {
                return Ok(true);
            }
        }
        Ok(false)
    }
    pub fn volume_paths(&self) -> Result<Vec<PathBuf>> {
        let mut volumes = Vec::new();
        for entry in fs::read_dir(&self.sysfs_path)? {
            let entry = entry?;

            if !entry.file_type()?.is_dir() {
                continue;
            }
            let file_name = entry.file_name();
            let file_name = file_name.to_string_lossy();
            if file_name.starts_with(&self.name) {
                volumes.push(PathBuf::from("/dev").join(file_name.as_ref()));
            }
        }

        Ok(volumes)
    }
    //
    fn read_sysfs<T>(&self, file_path: &str) -> Option<T>
    where
        T: FromStr,
    {
        fs::read_to_string(&self.sysfs_path.join(file_path))
            .ok()
            .and_then(|s| s.trim().parse::<T>().ok())
    }
}
