use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
};

use crate::{
    Reader, Writer,
    segment::{HEADER_SIZE, SEGMENT_SIZE, Segment},
};

pub fn to_bytes(segment: &Segment) -> Vec<u8> {
    let mut w = Writer::with_capacity(HEADER_SIZE as usize);
    w.write_u64(segment.chunk_count);
    w.write_u64(segment.chunk_capacity);
    // padding
    w.seek(HEADER_SIZE as usize);
    w.into_bytes()
}

pub fn from_bytes(buf: &[u8]) -> Segment {
    let mut r = Reader::new(buf);
    Segment {
        chunk_count: r.read_u64(),
        chunk_capacity: r.read_u64(),
    }
}

pub fn from_device(index: u64, device: &mut File) -> Segment {
    let mut buf = vec![0u8; HEADER_SIZE as usize];
    device
        .seek(SeekFrom::Start(offset(index)))
        .expect("[Segment]> Failed to seek index of device");
    device
        .read_exact(&mut buf)
        .expect("[Segment]> Failed to read header of device");
    from_bytes(&buf)
}

pub fn offset(index: u64) -> u64 {
    SEGMENT_SIZE * index + HEADER_SIZE
}
