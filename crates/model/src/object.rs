use std::time::{SystemTime, UNIX_EPOCH};

use uuid::Uuid;

use crate::CHUNK_SIZE;

pub mod object_db;
#[derive(Debug, Clone)]
pub enum ObjectState {
    Committed = 1,
    Deleted = 2,
    Pending = 0,
}

impl ObjectState {
    pub fn from_u32(v: u32) -> Self {
        match v {
            1 => Self::Committed,
            2 => Self::Deleted,
            _ => Self::Pending,
        }
    }
    pub fn as_str(&self) -> &'static str {
        match self {
            ObjectState::Committed => "Committed",
            ObjectState::Deleted => "Deleted",
            ObjectState::Pending => "Pending",
        }
    }
}
impl std::fmt::Display for ObjectState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Committed => "Committed",
            Self::Deleted => "Deleted",
            Self::Pending => "Pending",
        };
        write!(f, "{s}")
    }
}
#[derive(Debug, Clone)]

pub struct Object {
    pub object_id: i64,
    pub external_id: Uuid, // id do tầng app đặt

    pub original_filename: String,
    pub extension: Option<String>,
    pub mime_type: Option<String>,

    pub checksum: [u8; 32],
    pub total_size: u64,
    pub chunk_count: u64,
    pub chunk_size: u64,

    pub state: ObjectState,
    pub created_at: u64, // unix timestamp
    pub updated_at: u64, // unix timestamp
}

impl Object {
    pub fn new(
        original_filename: String,
        extension: Option<String>,
        mime_type: Option<String>,
        checksum: [u8; 32],
        total_size: u64,
    ) -> Self {
        let created_at: u64 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Self {
            object_id: 0,
            external_id: Uuid::now_v7(),
            original_filename,
            extension,
            mime_type,
            checksum,
            total_size,
            chunk_count: (total_size + CHUNK_SIZE - 1) / CHUNK_SIZE,
            chunk_size: CHUNK_SIZE,
            state: ObjectState::Pending,
            created_at,
            updated_at: 0,
        }
    }
}
