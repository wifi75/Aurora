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

    pub fn u16(&mut self) -> Result<u16, Error> {
        Ok(u16::from_be_bytes(self.array()?))
    }

    /// `QColor` come lo scrive `QDataStream` (versione 7 e seguenti): specifica (`qint8`), poi cinque `quint16` (alfa, tre componenti,
    /// riempitivo). Restituisce il colore in RGB a 8 bit; `None` per un colore non valido (`QColor()`), che nel protocollo vuol dire «togli».
    /// Le specifiche HSV, HSL, CMYK e RGB esteso si convertono in RGB.
    pub fn qcolor(&mut self) -> Result<Option<[u8; 3]>, Error> {
        let spec = self.u8()?;
        let _alpha = self.u16()?;
        let c1 = self.u16()?;
        let c2 = self.u16()?;
        let c3 = self.u16()?;
        let pad = self.u16()?;
        Ok(color_to_rgb(spec, [c1, c2, c3, pad]))
    }

    /// `n` byte grezzi.
    pub fn bytes(&mut self, n: usize) -> Result<&'a [u8], Error> {
        self.take(n)
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

/// Specifiche di `QColor::Spec`.
const SPEC_RGB: u8 = 1;
const SPEC_HSV: u8 = 2;
const SPEC_CMYK: u8 = 3;
const SPEC_HSL: u8 = 4;
const SPEC_EXTENDED_RGB: u8 = 5;

fn to_u8(unit: f64) -> u8 {
    (unit.clamp(0.0, 1.0) * 255.0).round() as u8
}

/// Un `qfloat16` (mezza precisione IEEE 754) in `f64`: serve alla specifica RGB estesa.
fn half_to_f64(bits: u16) -> f64 {
    let sign = if bits & 0x8000 != 0 { -1.0 } else { 1.0 };
    let exponent = i32::from((bits >> 10) & 0x1f);
    let mantissa = f64::from(bits & 0x3ff);
    match exponent {
        0 => sign * mantissa * 2f64.powi(-24),
        31 => f64::NAN,
        e => sign * (1.0 + mantissa / 1024.0) * 2f64.powi(e - 15),
    }
}

/// Da HSV o HSL (tinta in gradi, saturazione e valore/luminosita' tra 0 e 1) a RGB.
fn hue_to_rgb(hue_deg: f64, chroma: f64, offset: f64) -> [u8; 3] {
    let h = (hue_deg.rem_euclid(360.0)) / 60.0;
    let x = chroma * (1.0 - ((h % 2.0) - 1.0).abs());
    let (r, g, b) = match h as u32 {
        0 => (chroma, x, 0.0),
        1 => (x, chroma, 0.0),
        2 => (0.0, chroma, x),
        3 => (0.0, x, chroma),
        4 => (x, 0.0, chroma),
        _ => (chroma, 0.0, x),
    };
    [to_u8(r + offset), to_u8(g + offset), to_u8(b + offset)]
}

fn color_to_rgb(spec: u8, c: [u16; 4]) -> Option<[u8; 3]> {
    let unit = |v: u16| f64::from(v) / 65535.0;
    // la tinta vale 0-35999 (centesimi di grado); 65535 = acromatico (grigio)
    let hue = |v: u16| if v == u16::MAX { None } else { Some(f64::from(v) / 100.0) };
    match spec {
        SPEC_RGB => Some([(c[0] >> 8) as u8, (c[1] >> 8) as u8, (c[2] >> 8) as u8]),
        SPEC_HSV => {
            let (s, v) = (unit(c[1]), unit(c[2]));
            let chroma = v * s;
            Some(match hue(c[0]) {
                Some(h) => hue_to_rgb(h, chroma, v - chroma),
                None => [to_u8(v); 3],
            })
        }
        SPEC_HSL => {
            let (s, l) = (unit(c[1]), unit(c[2]));
            let chroma = (1.0 - (2.0 * l - 1.0).abs()) * s;
            Some(match hue(c[0]) {
                Some(h) => hue_to_rgb(h, chroma, l - chroma / 2.0),
                None => [to_u8(l); 3],
            })
        }
        SPEC_CMYK => {
            let k = unit(c[3]);
            Some([to_u8((1.0 - unit(c[0])) * (1.0 - k)), to_u8((1.0 - unit(c[1])) * (1.0 - k)), to_u8((1.0 - unit(c[2])) * (1.0 - k))])
        }
        SPEC_EXTENDED_RGB => Some([to_u8(half_to_f64(c[0])), to_u8(half_to_f64(c[1])), to_u8(half_to_f64(c[2]))]),
        // `Invalid` (0) o una specifica sconosciuta: nessun colore
        _ => None,
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

    pub fn u16(&mut self, v: u16) {
        self.buf.extend_from_slice(&v.to_be_bytes());
    }

    pub fn u32(&mut self, v: u32) {
        self.buf.extend_from_slice(&v.to_be_bytes());
    }

    /// `QColor` RGB come lo scrive Qt (componenti a 16 bit, `v * 0x101`); `None` e' il colore non valido `QColor()`.
    pub fn qcolor(&mut self, rgb: Option<[u8; 3]>) {
        match rgb {
            Some([r, g, b]) => {
                self.u8(SPEC_RGB);
                self.u16(u16::MAX);
                for c in [r, g, b] {
                    self.u16(u16::from(c) * 0x101);
                }
                self.u16(0);
            }
            None => {
                self.u8(0);
                self.u16(u16::MAX);
                for _ in 0..4 {
                    self.u16(0);
                }
            }
        }
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

    /// `QDateTime` in UTC (come lo scrive JTDX): giorno giuliano (i64), millisecondi del giorno (u32), tipo di fuso 1 (u8).
    /// `None` e' un `QDateTime` nullo (giorno 0, ora non valida).
    pub fn qdatetime_utc(&mut self, unix_ms: Option<i64>) {
        match unix_ms {
            Some(ms) => {
                self.i64(ms.div_euclid(86_400_000) + 2_440_588);
                self.u32(ms.rem_euclid(86_400_000) as u32);
            }
            None => {
                self.i64(0);
                self.u32(u32::MAX);
            }
        }
        self.u8(1);
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
    fn qcolor_rgb_and_invalid_roundtrip() {
        let mut w = Writer::new();
        w.qcolor(Some([255, 128, 0]));
        w.qcolor(None);
        let bytes = w.into_bytes();
        assert_eq!(bytes.len(), 22, "11 byte per colore");
        let mut r = Reader::new(&bytes);
        assert_eq!(r.qcolor(), Ok(Some([255, 128, 0])));
        assert_eq!(r.qcolor(), Ok(None));
        assert!(r.is_empty());
    }

    #[test]
    fn qcolor_other_specs_become_rgb() {
        // HSV: rosso pieno (tinta 0, saturazione e valore massimi)
        assert_eq!(color_to_rgb(SPEC_HSV, [0, u16::MAX, u16::MAX, 0]), Some([255, 0, 0]));
        // HSV acromatico: grigio al valore
        assert_eq!(color_to_rgb(SPEC_HSV, [u16::MAX, 0, u16::MAX, 0]), Some([255, 255, 255]));
        // HSL: verde (120 gradi), luminosita' a meta'
        assert_eq!(color_to_rgb(SPEC_HSL, [12_000, u16::MAX, 32_768, 0]), Some([0, 255, 0]));
        // CMYK: solo nero = nero
        assert_eq!(color_to_rgb(SPEC_CMYK, [0, 0, 0, u16::MAX]), Some([0, 0, 0]));
        // RGB esteso: 1.0 in mezza precisione = 0x3c00
        assert_eq!(color_to_rgb(SPEC_EXTENDED_RGB, [0x3c00, 0, 0x3c00, 0]), Some([255, 0, 255]));
        assert_eq!(color_to_rgb(9, [1, 2, 3, 4]), None);
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
