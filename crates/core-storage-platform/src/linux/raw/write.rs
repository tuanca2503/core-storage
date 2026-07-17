pub fn write_u8(buf: &mut Vec<u8>, value: u8) {
    buf.push(value);
}

pub fn write_u16(buf: &mut Vec<u8>, value: u16) {
    buf.extend_from_slice(&value.to_le_bytes());
}

pub fn write_u32(buf: &mut Vec<u8>, value: u32) {
    buf.extend_from_slice(&value.to_le_bytes());
}

pub fn write_u64(buf: &mut Vec<u8>, value: u64) {
    buf.extend_from_slice(&value.to_le_bytes());
}

pub fn write_bytes<const N: usize>(buf: &mut Vec<u8>, value: &[u8; N]) {
    buf.extend_from_slice(value);
}