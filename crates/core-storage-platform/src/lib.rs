use crate::{
    disk::DiskInfo,
    error::{BaseError, BaseResult, Codes},
    header::Header,
};
use chrono::DateTime;

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "macos")]
mod macos;

pub mod disk;
pub mod format;
pub mod raw;
//
pub mod error;
pub mod header;

pub const KB: u64 = 1024;
pub const MB: u64 = 1024 * KB;
pub const GB: u64 = 1024 * MB;
pub const TB: u64 = 1024 * GB;
//
pub const HEADER_SIZE: u64 = 4 * KB;
pub const SEGMENT_SIZE: u64 = 64 * GB;
pub const MAGIC: [u8; 12] = *b"CORE STORAGE";
pub const VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StorageState {
    Uninitialized = 0,
    Initialized = 1,
    Active = 2,
    Corrupt = 3,
}
impl StorageState {
    pub fn from_u32(v: u32) -> Self {
        match v {
            1 => Self::Initialized,
            2 => Self::Active,
            3 => Self::Corrupt,
            _ => Self::Uninitialized,
        }
    }
}
impl std::fmt::Display for StorageState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Uninitialized => "Uninitialized",
            Self::Initialized => "Initialized",
            Self::Active => "Active",
            Self::Corrupt => "Corrupt",
        };
        write!(f, "{s}")
    }
}

#[derive(Debug)]
pub struct StorageInfo {
    pub disk: DiskInfo,
    pub header: Header,
}

impl StorageInfo {
    pub fn new(disk: DiskInfo, header: Header) -> Self {
        Self { disk, header }
    }
}

pub fn ensure_root() -> BaseResult<()> {
    if unsafe { libc::geteuid() } != 0 {
        return Err(BaseError::system_warning(
            "This operation requires root privileges.".to_string(),
            Codes::PermissionDenied,
        ));
    }
    Ok(())
}

pub fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.2} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}
pub fn format_date(ms: u64) -> String {
    DateTime::from_timestamp_millis(ms as i64)
        .expect("invalid timestamp")
        .to_string()
}
