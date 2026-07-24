use crate::{
    HEADER_SIZE, MAGIC, SEGMENT_SIZE, StorageState, VERSION,
    error::{BaseError, BaseResult, Codes},
    raw::{Reader, Writer},
};
use std::{
    fs::OpenOptions,
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Clone, Copy)]
pub struct Header {
    //magic 12
    pub uuid: [u8; 16],      //16
    pub version: u32,        //4
    pub state: StorageState, //4
    pub segment_count: u64,  //8
    //
    pub total_bytes: u64,             //8
    pub bitmap_size_bytes: u64,       //8
    pub segment_size_bytes: u64,      //8
    pub last_segment_size_bytes: u64, //8
    //
    pub mirror_offset: u64,  //8
    pub bitmap_offset: u64,  //8
    pub segment_offset: u64, //8
    //
    pub created_at_ms: u64, //8
}

/*
+----------+---------+-----------+-----------+-----------+------------------+-----------------+
| Header   | Bitmap  | Segment 1 | Segment 2 | Segment 3 |      Còn lại     | Header mirror   |
+----------+---------+-----------+-----------+-----------+------------------+-----------------+
*/
impl Header {
    pub fn new() -> Self {
        Header {
            version: VERSION,
            uuid: Default::default(),
            created_at_ms: 0,
            state: StorageState::Uninitialized,
            total_bytes: 0,
            segment_size_bytes: 0,
            last_segment_size_bytes: 0,
            segment_count: 0,
            bitmap_offset: 0,
            bitmap_size_bytes: 0,
            segment_offset: 0,
            mirror_offset: 0,
        }
    }

    pub fn create(capacity_bytes: u64, physical_sector_size: u32) -> BaseResult<Self> {
        if capacity_bytes <= HEADER_SIZE * 2 + SEGMENT_SIZE {
            return Err(BaseError::system_error(
                "Capacity too small",
                Codes::Corrupt,
            ));
        }
        let mirror_offset = capacity_bytes - HEADER_SIZE;
        let sector_size = physical_sector_size.max(1) as u64;
        let mut bitmap_size_bytes = 0;
        let mut segment_offset = 0;
        let mut converged = false;

        // ceil thay vì floor ngay từ ước lượng ban đầu
        let mut segment_count = (mirror_offset - HEADER_SIZE + SEGMENT_SIZE - 1) / SEGMENT_SIZE;
        let created_at_ms = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64;
        //
        for _ in 0..8 {
            let bitmap_bytes_needed = (segment_count + 7) / 8;
            bitmap_size_bytes = Self::align_up(bitmap_bytes_needed, sector_size);
            segment_offset = Self::align_up(HEADER_SIZE + bitmap_size_bytes, sector_size);

            if segment_offset >= mirror_offset {
                return Err(BaseError::system_error(
                    "Capacity too small",
                    Codes::Corrupt,
                ));
            }
            let available = mirror_offset - segment_offset;

            // ceil: đảm bảo mọi byte thừa đều được gom vào segment cuối, không bỏ phí
            let new_count = (available + SEGMENT_SIZE - 1) / SEGMENT_SIZE;

            if new_count == segment_count {
                converged = true;
                break;
            }
            if new_count == 0 {
                return Err(BaseError::system_error(
                    "Capacity too small",
                    Codes::Corrupt,
                ));
            }
            segment_count = new_count;
        }

        if !converged {
            return Err(BaseError::system_error(
                "Capacity too small",
                Codes::Corrupt,
            ));
        }
        //
        let available = mirror_offset - segment_offset;
        let last_segment_size_bytes = available - (segment_count - 1) * SEGMENT_SIZE;
        Ok(Header {
            version: VERSION,
            uuid: *uuid::Uuid::now_v7().as_bytes(),
            created_at_ms,
            state: StorageState::Active,
            total_bytes: capacity_bytes,
            segment_size_bytes: SEGMENT_SIZE,
            last_segment_size_bytes,
            segment_count,
            bitmap_offset: HEADER_SIZE,
            bitmap_size_bytes,
            segment_offset,
            mirror_offset,
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        // let mut body = Vec::with_capacity(HEADER_SIZE as usize);
        let mut w = Writer::with_capacity(HEADER_SIZE as usize);
        w.write_bytes(&MAGIC);
        w.write_bytes(&self.uuid);
        w.write_u32(self.version);
        w.write_u32(self.state as u32);
        w.write_u64(self.segment_count);
        //
        w.write_u64(self.total_bytes);
        w.write_u64(self.bitmap_size_bytes);
        w.write_u64(self.segment_size_bytes);
        w.write_u64(self.last_segment_size_bytes);
        //
        w.write_u64(self.mirror_offset);
        w.write_u64(self.bitmap_offset);
        w.write_u64(self.segment_offset);
        //
        w.write_u64(self.created_at_ms);
        let crc = crc32c::crc32c(w.as_slice());
        w.write_u32(crc);
        // padding
        w.seek(HEADER_SIZE as usize);
        w.into_bytes()
    }

    pub fn load(device: &mut (impl Read + Write + Seek)) -> BaseResult<Self> {
        match Self::read_at(device, SeekFrom::Start(0)) {
            Ok(header) => Ok(header),
            Err(primary_err) => {
                // Seek đến vị trí cách cuối device đúng HEADER_SIZE byte
                match Self::read_at(device, SeekFrom::End(-(HEADER_SIZE as i64))) {
                    Ok(header) => {
                        eprintln!("Primary header corrupt, recovered from mirror");
                        device.seek(SeekFrom::Start(0))?;
                        device.write_all(&header.to_bytes())?;
                        device.flush()?;
                        Ok(header)
                    }
                    Err(mirror_err) => Err(BaseError::system_error(
                        format!(
                            "Both primary and mirror headers corrupt. \n primary: {} \n mirror: {}",
                            primary_err, mirror_err
                        ),
                        Codes::Corrupt,
                    )),
                }
            }
        }
    }

    pub fn try_load(volume_paths: &[PathBuf], device_path: &Path) -> Self {
        if volume_paths.len() > 0 {
            return Header::new();
        }
        match OpenOptions::new().read(true).write(true).open(device_path) {
            Ok(mut device) => match Header::load(&mut device) {
                Ok(header) => return header,
                Err(e) => {
                    eprintln!("Failed to load header on {}: {}", device_path.display(), e);
                }
            },
            Err(e) => {
                eprintln!("Failed to load header on {}: {}", device_path.display(), e);
            }
        };
        Header::new()
    }

    pub fn is_magic(device_path: &Path) -> BaseResult<bool> {
        let mut device = OpenOptions::new()
            .read(true)
            .write(false)
            .open(&device_path)?;

        device.seek(SeekFrom::Start(0))?;
        let mut buf = [0u8; MAGIC.len()];
        device.read_exact(&mut buf)?;
        Ok(buf == MAGIC)
    }

    
    
    fn from_bytes(buf: &[u8]) -> BaseResult<Self> {
        let mut r = Reader::new(buf);
        let magic = r.read_bytes::<12>();
        if magic != MAGIC {
            return Err(BaseError::system_error("Wrong magic", Codes::Corrupt));
        }
        let uuid = r.read_bytes::<16>();
        let version = r.read_u32();
        let state = StorageState::from_u32(r.read_u32());
        let segment_count = r.read_u64();
        //
        let total_bytes = r.read_u64();
        let bitmap_size_bytes = r.read_u64();
        let segment_size_bytes = r.read_u64();
        let last_segment_size_bytes = r.read_u64();
        //
        let mirror_offset = r.read_u64();
        let bitmap_offset = r.read_u64();
        let segment_offset = r.read_u64();
        let created_at_ms = r.read_u64();

        let body_len_before_crc = r.position();
        let header_crc32 = r.read_u32();
        let computed_crc = crc32c::crc32c(&buf[..body_len_before_crc]);

        if header_crc32 != computed_crc {
            return Err(BaseError::system_error("Crc32c not match", Codes::Corrupt));
        }

        Ok(Header {
            version,
            uuid,
            created_at_ms,
            state,
            total_bytes,
            segment_size_bytes,
            last_segment_size_bytes,
            segment_count,
            bitmap_offset,
            bitmap_size_bytes,
            segment_offset,
            mirror_offset,
        })
    }

    fn read_at(device: &mut (impl Read + Seek), pos: SeekFrom) -> BaseResult<Self> {
        device.seek(pos)?;
        let mut buf = vec![0u8; HEADER_SIZE as usize];
        device.read_exact(&mut buf)?;
        Self::from_bytes(&buf)
    }

    fn align_up(value: u64, align: u64) -> u64 {
        debug_assert!(align.is_power_of_two() || align > 0);
        (value + align - 1) / align * align
    }
}
