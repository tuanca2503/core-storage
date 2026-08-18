use std::{
    fs::File,
    io::{Read, Result, Seek, SeekFrom},
};

use crate::{
    Reader, Writer,
    storage::{HEADER_SIZE, MAGIC, Storage, StorageState},
};

pub fn to_bytes(storage: &Storage) -> Vec<u8> {
    let mut w = Writer::with_capacity(HEADER_SIZE as usize);
    w.write_bytes(&MAGIC);
    w.write_bytes(&storage.uuid);
    w.write_u32(storage.version);
    w.write_u32(storage.state as u32);
    w.write_u32(storage.logical_sector_size);
    w.write_u32(storage.physical_sector_size);

    w.write_u64(storage.capacity_bytes);
    w.write_u64(storage.last_segment_size_bytes);
    w.write_u64(storage.segment_count);
    w.write_u64(storage.active_segment_index);
    w.write_u64(storage.mirror_offset);
    w.write_u64(storage.created_at);
    let crc = crc32c::crc32c(w.as_slice());
    w.write_u32(crc);
    // padding
    w.seek(HEADER_SIZE as usize);
    w.into_bytes()
}

pub fn from_bytes(buf: &[u8]) -> Result<Storage> {
    let dfl = Storage::default();
    let mut r = Reader::new(buf);
    let magic = r.read_array::<12>()?;
    if magic != MAGIC {
        return Ok(dfl);
    }
    let uuid = r.read_array::<16>()?;
    let version = r.read_u32()?;
    let state = StorageState::from_u32(r.read_u32()?);
    let logical_sector_size = r.read_u32()?;
    let physical_sector_size = r.read_u32()?;
    //
    let capacity_bytes = r.read_u64()?;
    let last_segment_size_bytes = r.read_u64()?;
    let segment_count = r.read_u64()?;
    let active_segment_index = r.read_u64()?;
    let mirror_offset = r.read_u64()?;
    let created_at = r.read_u64()?;
    //
    let computed_crc = crc32c::crc32c(&buf[..r.position()]);
    let header_crc32 = r.read_u32()?;
    if header_crc32 != computed_crc {
        return Ok(Storage {
            state: StorageState::Corrupt,
            capacity_bytes,
            ..dfl
        });
    }
    Ok(Storage {
        uuid,
        version,
        state,
        logical_sector_size,
        physical_sector_size,
        capacity_bytes,
        last_segment_size_bytes,
        segment_count,
        active_segment_index,
        mirror_offset,
        created_at,
    })
}

pub fn from_device(device: &mut File) -> Result<Storage> {
    let mut buf = vec![0u8; HEADER_SIZE as usize];

    device.seek(SeekFrom::Start(0))?;
    device.read_exact(&mut buf)?;
    let mut s = from_bytes(&buf)?;

    if s.state == StorageState::Corrupt {
        eprintln!("[Storage]> Primary header corrupt, try recovered from mirror");
        device.seek(SeekFrom::End(-(HEADER_SIZE as i64)))?;
        device.read_exact(&mut buf)?;
        s = from_bytes(&buf)?;
    }
    Ok(s)
}

pub fn has_valid_magic(device: &mut File) -> Result<bool> {
    let mut buf = [0u8; MAGIC.len()];
    device.seek(SeekFrom::Start(0))?;
    device.read_exact(&mut buf)?;
    Ok(buf == MAGIC)
}
