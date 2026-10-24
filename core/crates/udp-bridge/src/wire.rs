use crate::Error;

/// Lettore sequenziale big-endian; non indicizza mai fuori dai limiti.
pub struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    pub fn is_empty(&self) -> bool {
        self.pos >= self.buf.len()
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], Error> {
        let end = self.pos.checked_add(n).ok_or(Error::Truncated)?;
        let slice = self.buf.get(self.pos..end).ok_or(Error::Truncated)?;
        self.pos = end;
        Ok(slice)
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], Error> {
        self.take(N)?.try_into().map_err(|_| Error::Truncated)
    }

    pub fn u8(&mut self) -> Result<u8, Error> {
        Ok(self.array::<1>()?[0])
    }

    pub fn bool(&mut self) -> Result<bool, Error> {
        Ok(self.u8()? != 0)
    }

    pub fn u32(&mut self) -> Result<u32, Error> {
        Ok(u32::from_be_bytes(self.array()?))
    }

    pub fn i32(&mut self) -> Result<i32, Error> {
        Ok(i32::from_be_bytes(self.array()?))
    }

    pub fn u64(&mut self) -> Result<u64, Error> {
        Ok(u64::from_be_bytes(self.array()?))
    }

    pub fn i64(&mut self) -> Result<i64, Error> {
        Ok(i64::from_be_bytes(self.array()?))
    }

    pub fn f64(&mut self) -> Result<f64, Error> {
        Ok(f64::from_be_bytes(self.array()?))
    }

    /// Stringa Qt (`QByteArray` UTF-8): lunghezza u32, `0xFFFFFFFF` = nulla.
    pub fn utf8(&mut self) -> Result<Option<String>, Error> {
        let len = self.u32()?;
        if len == u32::MAX {
            return Ok(None);
        }
        let remaining = self.buf.len() - self.pos;
        if len as usize > remaining {
            return Err(Error::TooLong(len));
        }
        let bytes = self.take(len as usize)?;
        let text = std::str::from_utf8(bytes).map_err(|_| Error::BadUtf8)?;
        Ok(Some(text.to_string()))
    }
}

/// Scrittore speculare al lettore.
#[derive(Default)]
pub struct Writer {
    buf: Vec<u8>,
}

impl Writer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn u8(&mut self, v: u8) {
        self.buf.push(v);
    }

    pub fn bool(&mut self, v: bool) {
        self.buf.push(u8::from(v));
    }

    pub fn u32(&mut self, v: u32) {
        self.buf.extend_from_slice(&v.to_be_bytes());
    }

    pub fn i32(&mut self, v: i32) {
        self.buf.extend_from_slice(&v.to_be_bytes());
    }

    pub fn u64(&mut self, v: u64) {
        self.buf.extend_from_slice(&v.to_be_bytes());
    }

    pub fn i64(&mut self, v: i64) {
        self.buf.extend_from_slice(&v.to_be_bytes());
    }

    pub fn f64(&mut self, v: f64) {
        self.buf.extend_from_slice(&v.to_be_bytes());
    }

    pub fn utf8(&mut self, v: Option<&str>) {
        match v {
            None => self.u32(u32::MAX),
            Some(s) => {
                self.u32(s.len() as u32);
                self.buf.extend_from_slice(s.as_bytes());
            }
        }
    }

    /// Byte grezzi, per costruire pacchetti volutamente malformati nei test.
    pub fn raw(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reader_reads_big_endian_u32() {
        let mut r = Reader::new(&[0, 0, 1, 2]);
        assert_eq!(r.u32(), Ok(258));
    }

    #[test]
    fn reader_utf8_null_is_none() {
        let mut r = Reader::new(&[0xFF, 0xFF, 0xFF, 0xFF]);
        assert_eq!(r.utf8(), Ok(None));
    }

    #[test]
    fn reader_utf8_empty_is_some_empty() {
        let mut r = Reader::new(&[0, 0, 0, 0]);
        assert_eq!(r.utf8(), Ok(Some(String::new())));
    }

    #[test]
    fn reader_truncated_is_error() {
        let mut r = Reader::new(&[0, 0, 1]);
        assert_eq!(r.u32(), Err(Error::Truncated));
    }

    #[test]
    fn reader_huge_length_is_too_long_without_allocating() {
        let mut data = vec![0xFF, 0xFF, 0xFF, 0xF0];
        data.extend_from_slice(&[0u8; 16]);
        let mut r = Reader::new(&data);
        assert_eq!(r.utf8(), Err(Error::TooLong(0xFFFF_FFF0)));
    }

    #[test]
    fn writer_and_reader_roundtrip_every_type() {
        let mut w = Writer::new();
        w.u8(7);
        w.u32(70_000);
        w.u64(14_074_000);
        w.i32(-12);
        w.f64(0.3);
        w.bool(true);
        w.utf8(Some("CQ DX"));
        w.utf8(None);
        let bytes = w.into_bytes();
        let mut r = Reader::new(&bytes);
        assert_eq!(r.u8(), Ok(7));
        assert_eq!(r.u32(), Ok(70_000));
        assert_eq!(r.u64(), Ok(14_074_000));
        assert_eq!(r.i32(), Ok(-12));
        assert_eq!(r.f64(), Ok(0.3));
        assert_eq!(r.bool(), Ok(true));
        assert_eq!(r.utf8(), Ok(Some("CQ DX".to_string())));
        assert_eq!(r.utf8(), Ok(None));
        assert!(r.is_empty());
    }
}
