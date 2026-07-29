
pub struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self {
            buf,
            pos: 0,
        }
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
    pub fn read_u8(&mut self) -> u8 {
        let value = self.buf[self.pos];
        self.pos += 1;
        value
    }
    pub fn read_u32(&mut self) -> u32 {
        let value = u32::from_le_bytes(
            self.buf[self.pos..self.pos + 4]
                .try_into()
                .unwrap(),
        );
        self.pos += 4;
        value
    }
    pub fn read_u64(&mut self) -> u64 {
        let value = u64::from_le_bytes(
            self.buf[self.pos..self.pos + 8]
                .try_into()
                .unwrap(),
        );
        self.pos += 8;
        value
    }
    pub fn read_bytes<const N: usize>(&mut self) -> [u8; N] {
        let value = self.buf[self.pos..self.pos + N]
            .try_into()
            .unwrap();

        self.pos += N;

        value
    }
    pub fn read_bytes_from<const N: usize>(&self, pos: usize) -> [u8; N] {
        self.buf[pos..pos + N]
            .try_into()
            .unwrap()
    }
}


