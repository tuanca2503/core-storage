use std::path::PathBuf;

#[cfg(target_os = "windows")]
pub use crate::windows::disk::*;

#[cfg(target_os = "linux")]
pub use crate::linux::disk::*;

#[cfg(target_os = "macos")]
pub use crate::macos::disk::*;
#[derive(Debug)]
pub struct PhysicalDisk {
    // pub disk_number: u32,
    pub name: String,              // sda
    pub serial: String,
    pub model: String,
    pub sysfs_path: PathBuf,       // /sys/block/sda
    pub device_path: PathBuf,
    pub volume_paths: Vec<PathBuf>,
    pub removable: bool,
    pub read_only: bool,
    // pub bus_type: i32,
    pub capacity_bytes: u64,
    pub logical_sector_size: u32,
    pub physical_sector_size: u32,
}
