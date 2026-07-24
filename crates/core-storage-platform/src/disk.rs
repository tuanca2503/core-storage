#[cfg(target_os = "windows")]
pub use crate::windows::disk::*;

#[cfg(target_os = "linux")]
pub use crate::linux::disk::*;

#[cfg(target_os = "macos")]
pub use crate::macos::disk::*;

#[derive(Debug)]
pub struct DiskInfo {
    pub serial: String,
    pub model: String,
    pub sysfs_path: std::path::PathBuf, // /sys/block/sda
    pub device_path: std::path::PathBuf,
    pub volume_paths: Vec<std::path::PathBuf>,
    pub removable: bool,
    pub read_only: bool,
    pub capacity_bytes: u64,
    pub logical_sector_size: u32,
    pub physical_sector_size: u32,
}

impl DiskInfo {
    pub fn new(
        serial: String,
        model: String,
        sysfs_path: std::path::PathBuf,
        device_path: std::path::PathBuf,
        volume_paths: Vec<std::path::PathBuf>,
        removable: bool,
        read_only: bool,
        capacity_bytes: u64,
        logical_sector_size: u32,
        physical_sector_size: u32,
    ) -> Self {
        Self {
            serial,
            model,
            sysfs_path,
            device_path,
            volume_paths,
            removable,
            read_only,
            capacity_bytes,
            logical_sector_size,
            physical_sector_size,
        }
    }

    pub fn to_vec(&self) -> Vec<String> {
        vec![
            self.serial.clone(),
            self.model.clone(),
            self.sysfs_path.display().to_string(),
            self.device_path.display().to_string(),
            self.volume_paths
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(", "),
            self.removable.to_string(),
            self.read_only.to_string(),
            self.capacity_bytes.to_string(),
            self.logical_sector_size.to_string(),
            self.physical_sector_size.to_string(),
        ]
    }

    pub fn to_table(disks: Vec<Self>) -> Vec<Vec<String>> {
        let mut rows = Vec::new();
        rows.push(vec![
            "SERIAL".into(),
            "MODULE".into(),
            "SP".into(),
            "DP".into(),
            "VOLS".into(),
            "REMOVE".into(),
            "RO".into(),
            "TOTAL".into(),
            "LS".into(),
            "PS".into(),
        ]);
        rows.extend(disks.into_iter().map(|d| d.to_vec()));

        rows
    }
}
