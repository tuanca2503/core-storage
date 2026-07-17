use std::time::{SystemTime, UNIX_EPOCH};
use crate::{error::{BaseError, BaseResult, Codes}, raw::read::Reader};

pub const MAGIC: [u8; 12] = *b"CORE STORAGE";
pub const HEADER_SIZE: u64 = 4096;
pub const VERSION: u32 = 1;

#[derive(Debug, Clone)]
pub enum StorageState {
    Uninitialized = 0,
    Active = 1,
    Formatting = 2,
    Corrupt = 3,
}
impl StorageState {
    fn from_u32(v: u32) -> Self {
        match v {
            1 => Self::Active,
            2 => Self::Formatting,
            3 => Self::Corrupt,
            _ => Self::Uninitialized,
        }
    }
}

pub struct Header {
    //magic 12
    pub version: u32,               //4
    pub uuid: [u8; 16],             //16
    pub created_at_ms: u64,         //8
    pub state: StorageState,        //4
    pub total_bytes: u64,           //8
    pub segment_size_bytes: u64,    //8
    pub segment_region_offset: u64, //8
    pub segment_count: u32,         //4
    pub bitmap_offset: u64,         //8
    pub bitmap_size_bytes: u64,     //8
}

impl Header {
    pub fn new(total_bytes: u64, layout: Layout, segment_size_bytes: u64) -> Self {
        let created_at_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        Header {
            version: VERSION,
            uuid: *uuid::Uuid::now_v7().as_bytes(),
            created_at_ms,
            state: StorageState::Active,
            total_bytes,
            segment_size_bytes,
            segment_count: layout.segment_count,
            bitmap_offset: layout.bitmap_offset,
            bitmap_size_bytes: layout.bitmap_size_bytes,
            segment_region_offset: layout.segment_region_offset,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut body = Vec::with_capacity(HEADER_SIZE as usize);
        body.extend_from_slice(&MAGIC);
        body.extend_from_slice(&self.version.to_le_bytes());
        body.extend_from_slice(&self.uuid);
        body.extend_from_slice(&self.created_at_ms.to_le_bytes());
        body.extend_from_slice(&(self.state.clone() as u32).to_le_bytes());
        body.extend_from_slice(&self.total_bytes.to_le_bytes());
        body.extend_from_slice(&self.segment_size_bytes.to_le_bytes());
        body.extend_from_slice(&self.segment_count.to_le_bytes());
        body.extend_from_slice(&self.bitmap_offset.to_le_bytes());
        body.extend_from_slice(&self.bitmap_size_bytes.to_le_bytes());
        body.extend_from_slice(&self.segment_region_offset.to_le_bytes());
        let crc = crc32c::crc32c(&body);
        body.extend_from_slice(&crc.to_le_bytes());
        // padding
        body.resize(4096, 0);
        body
    }

    pub fn from_bytes(buf: &[u8]) -> BaseResult<Self> {
        if buf.len() < HEADER_SIZE as usize {
            return Err(BaseError::system_error(
                "Buffer shorter HEADER_SIZE",
                Codes::Corrupt,
            ));
        }
        let mut reader = Reader::new(buf);
        let magic = reader.read_bytes::<12>();
        if magic != MAGIC {
            return Err(BaseError::system_error("Wrong magic", Codes::Corrupt));
        }

        let version = reader.read_u32();
        let uuid = reader.read_bytes::<16>();
        let created_at_ms = reader.read_u64();
        let state = StorageState::from_u32(reader.read_u32());
        let total_bytes = reader.read_u64();
        let segment_size_bytes = reader.read_u64();
        let segment_region_offset = reader.read_u64();
        let segment_count = reader.read_u32();
        let bitmap_offset = reader.read_u64();
        let bitmap_size_bytes = reader.read_u64();
        let body_len_before_crc = reader.position();//88
        let header_crc32 = reader.read_u32();

        let stored_crc = reader.read_bytes_from::<body_len_before_crc>(0);
        
        let stored_crc = u32::from_le_bytes(buf[pos..pos + 4].try_into().unwrap());
        let computed_crc = crc32c::crc32c(&buf[..body_len_before_crc]);
        if stored_crc != computed_crc {
            return Err(FormatError::Corrupt("crc32c không khớp"));
        }

        let state =
            StorageState::from_u32(state_raw).ok_or(FormatError::Corrupt("state không hợp lệ"))?;

        Ok(Header {
            version,
            uuid,
            created_at_ms,
            state,
            total_bytes,
            segment_size_bytes,
            segment_count,
            bitmap_offset,
            bitmap_size_bytes,
            segment_region_offset,
        })
    }
}
