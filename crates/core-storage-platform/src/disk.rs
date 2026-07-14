#[cfg(target_os = "windows")]
pub use crate::windows::disk::*;

#[cfg(target_os = "linux")]
pub use crate::linux::disk::*;

#[cfg(target_os = "macos")]
pub use crate::macos::disk::*;

pub struct PhysicalDisk {
    pub disk_number: u32,
    pub serial: String,
    pub model: String,
    pub device_path: String,
    pub removable: bool,
    pub read_only: bool,
    pub bus_type: i32,
    pub capacity_bytes: u64,
    pub logical_sector_size: u32,
    pub physical_sector_size: u32,
}
