use std::io::{Error, ErrorKind, Result};

pub struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }
    //
    pub fn position(&self) -> usize {
        self.pos
    }
    pub fn seek(&mut self, pos: usize) {
        self.pos = pos;
    }
    pub fn skip(&mut self, len: usize) {
        self.pos += len;
    }
    pub fn read_u8(&mut self) -> Result<u8> {
        Ok(self.read_array::<1>()?[0])
    }
    pub fn read_u16(&mut self) -> Result<u16> {
        Ok(u16::from_le_bytes(self.read_array::<2>()?))
    }
    pub fn read_u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.read_array::<4>()?))
    }
    pub fn read_u64(&mut self) -> Result<u64> {
        Ok(u64::from_le_bytes(self.read_array::<8>()?))
    }
    pub fn read_array<const N: usize>(&mut self) -> Result<[u8; N]> {
        let end = self
            .pos
            .checked_add(N)
            .ok_or_else(|| Error::new(ErrorKind::UnexpectedEof, "read out of bounds"))?;

        let value = self
            .buf
            .get(self.pos..end)
            .ok_or_else(|| Error::new(ErrorKind::UnexpectedEof, "read out of bounds"))?
            .try_into()
            .map_err(|_| Error::new(ErrorKind::InvalidData, "slice conversion failed"))?;

        self.pos = end;

        Ok(value)
    }
    pub fn read_bytes(&mut self, len: usize) -> Result<&[u8]> {
        let end = self
            .pos
            .checked_add(len)
            .ok_or_else(|| Error::new(ErrorKind::UnexpectedEof, "read out of bounds"))?;

        let value = self
            .buf
            .get(self.pos..end)
            .ok_or_else(|| Error::new(ErrorKind::UnexpectedEof, "read out of bounds"))?;

        self.pos = end;
        Ok(value)
    }
    pub fn read_string(&mut self) -> Result<String> {
        let start = self.pos;

        while self.pos < self.buf.len() {
            if self.buf[self.pos] == 0 {
                let bytes = &self.buf[start..self.pos];
                self.pos += 1;

                return String::from_utf8(bytes.to_vec())
                    .map_err(|_| Error::new(ErrorKind::InvalidData, "invalid utf8 string"));
            }

            self.pos += 1;
        }

        Err(Error::new(ErrorKind::UnexpectedEof, "unterminated string"))
    }
    pub fn read_bytes_from<const N: usize>(&self, pos: usize) -> Result<[u8; N]> {
        self.buf
            .get(pos..pos + N)
            .ok_or_else(|| Error::new(ErrorKind::UnexpectedEof, "read out of bounds"))?
            .try_into()
            .map_err(|_| Error::new(ErrorKind::InvalidData, "slice conversion failed"))
    }
}
