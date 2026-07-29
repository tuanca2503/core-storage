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
    pub external_id: Option<String>,   // id do tầng app đặt, nullable
    pub total_size: i64,
    pub chunk_count: i64,
    pub status: ObjectState,
    pub created_at: i64,               // unix timestamp
    pub updated_at: i64,               // unix timestamp
}