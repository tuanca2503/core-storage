use std::time::{SystemTime, UNIX_EPOCH};
use crate::segment::SEGMENT_SIZE;
pub mod storage_bin;
pub const HEADER_SIZE: u64 = 4 * 1024; //4096
pub const VERSION: u32 = 1;
pub const MAGIC: [u8; 12] = *b"CORE STORAGE";

#[derive(Debug, Clone, Copy,PartialEq)]
pub enum StorageState {
    Uninitialized = 0, // Un init cant run
    Initialized = 1,   // Is init but un active
    Active = 2,        // Ready for using
    Corrupt = 3,       // Need maintain
    Full = 4,          // Full disk cant write
}
impl StorageState {
    pub fn from_u32(v: u32) -> Self {
        match v {
            1 => Self::Initialized,
            2 => Self::Active,
            3 => Self::Corrupt,
            4 => Self::Full,
            _ => Self::Uninitialized,
        }
    }
    pub fn as_str(&self) -> &'static str {
        match self {
            StorageState::Initialized => "Initialized",
            StorageState::Active => "Active",
            StorageState::Corrupt => "Corrupt",
            StorageState::Full => "Full",
            StorageState::Uninitialized => "Uninitialized",
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
            Self::Full => "Full",
        };
        write!(f, "{s}")
    }
}

#[derive(Debug, Clone)]
pub struct Storage {
    // magic 12
    pub uuid: [u8; 16],               // 16
    pub version: u32,                 // 4
    pub state: StorageState,          // 4
    pub logical_sector_size: u32,     // 4
    pub physical_sector_size: u32,    // 4
    pub capacity_bytes: u64,          // 8
    pub last_segment_size_bytes: u64, // 8
    pub segment_count: u64,           // 8
    pub active_segment_index: u64,    // 8
    pub mirror_offset: u64,           // 8
    pub created_at_ms: u64,           // 8
}

impl Storage {
    pub fn new(capacity_bytes: u64, physical_sector_size: u32, logical_sector_size: u32) -> Self {
        if physical_sector_size < 1 || capacity_bytes <= 2 * HEADER_SIZE + SEGMENT_SIZE {
            return Self {
                state: StorageState::Full,
                capacity_bytes,
                ..Self::default()
            };
        }

        let mirror_offset = capacity_bytes - HEADER_SIZE;
        let created_at_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("[Storage]> System clock is before UNIX_EPOCH")
            .as_millis() as u64;
        let available_bytes = mirror_offset - HEADER_SIZE;
        let segment_count = (available_bytes + SEGMENT_SIZE - 1) / SEGMENT_SIZE;
        let last_segment_size_bytes = available_bytes - (segment_count - 1) * SEGMENT_SIZE;
        Self {
            uuid: *uuid::Uuid::now_v7().as_bytes(),
            version: VERSION,
            state: StorageState::Active,
            logical_sector_size,
            physical_sector_size,

            capacity_bytes,
            last_segment_size_bytes,
            segment_count,
            active_segment_index: 0,
            mirror_offset,
            created_at_ms,
        }
    }
}

impl Default for Storage {
    fn default() -> Self {
        Self {
            uuid: Default::default(),
            version: VERSION,
            state: StorageState::Uninitialized,
            logical_sector_size: 0,
            physical_sector_size: 0,
            capacity_bytes: 0,
            last_segment_size_bytes: 0,
            segment_count: 0,
            active_segment_index: 0,
            mirror_offset: 0,
            created_at_ms: 0,
        }
    }
}
