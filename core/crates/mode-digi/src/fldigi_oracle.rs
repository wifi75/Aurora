//! I ricevitori RTTY, PSK e CW di fldigi 4.2.13 compilati da `terze-parti/fldigi` (vedi `oracle/fldigi_*_rx.cpp` e `build.rs`), come arbitri
//! nelle prove dei nostri ricevitori. Esiste solo con la feature `fldigi-oracle`, che non entra mai nell'installatore.

use std::ffi::c_void;

unsafe extern "C" {
    fn aurora_fldigi_rtty_new(baud: f64, shift: f64, centre: f64, samplerate: f64) -> *mut c_void;
    fn aurora_fldigi_rtty_process(rx: *mut c_void, samples: *const f32, len: usize);
    fn aurora_fldigi_rtty_take(rx: *mut c_void, buf: *mut u8, cap: usize) -> usize;
    fn aurora_fldigi_rtty_free(rx: *mut c_void);
}

/// Il ricevitore di fldigi in USB senza Rev (mark = tono alto), squelch e AFC spenti, UOS acceso.
pub struct FldigiRtty {
    rx: *mut c_void,
}

impl FldigiRtty {
    /// `centre_hz`: il centro fra i due toni; `samplerate`: fldigi lavora a 8000 Hz.
    pub fn new(baud: f64, shift_hz: f64, centre_hz: f64, samplerate: f64) -> Self {
        // SAFETY: crea un oggetto C++ posseduto da questa struttura e liberato in `drop`.
        let rx = unsafe { aurora_fldigi_rtty_new(baud, shift_hz, centre_hz, samplerate) };
        Self { rx }
    }

    /// Elabora altro audio (alla frequenza data in `new`); restituisce i caratteri nuovi.
    pub fn process(&mut self, samples: &[f32]) -> Vec<u8> {
        // SAFETY: `rx` e' valido finche' vive la struttura; il puntatore e la lunghezza vengono dalla stessa fetta.
        unsafe { aurora_fldigi_rtty_process(self.rx, samples.as_ptr(), samples.len()) };
        let mut out = Vec::new();
        let mut buf = [0u8; 256];
        loop {
            // SAFETY: `buf` e' lungo `buf.len()` byte e la funzione ne scrive al massimo tanti.
            let n = unsafe { aurora_fldigi_rtty_take(self.rx, buf.as_mut_ptr(), buf.len()) };
            out.extend_from_slice(&buf[..n.min(buf.len())]);
            if n < buf.len() {
                return out;
            }
        }
    }
}

impl Drop for FldigiRtty {
    fn drop(&mut self) {
        // SAFETY: `rx` viene da `aurora_fldigi_rtty_new` e si libera una volta sola.
        unsafe { aurora_fldigi_rtty_free(self.rx) };
    }
}

unsafe extern "C" {
    fn aurora_fldigi_psk_new(baud: f64, freq: f64, samplerate: f64, afc: i32, squelch: i32, squelch_value: f64) -> *mut c_void;
    fn aurora_fldigi_psk_process(rx: *mut c_void, samples: *const f32, len: usize);
    fn aurora_fldigi_psk_take(rx: *mut c_void, buf: *mut u8, cap: usize) -> usize;
    fn aurora_fldigi_psk_frequency(rx: *mut c_void) -> f64;
    fn aurora_fldigi_psk_free(rx: *mut c_void);
    fn aurora_fldigi_cw_new(freq: f64, wpm: f64, bandwidth: f64, samplerate: f64, squelch: i32, squelch_value: f64) -> *mut c_void;
    fn aurora_fldigi_cw_process(rx: *mut c_void, samples: *const f32, len: usize);
    fn aurora_fldigi_cw_take(rx: *mut c_void, buf: *mut u8, cap: usize) -> usize;
    fn aurora_fldigi_cw_wpm(rx: *mut c_void) -> f64;
    fn aurora_fldigi_cw_free(rx: *mut c_void);
}

/// Svuota la coda dei caratteri di un ricevitore di fldigi con la funzione `take` data.
fn take_all(rx: *mut c_void, take: unsafe extern "C" fn(*mut c_void, *mut u8, usize) -> usize) -> Vec<u8> {
    let mut out = Vec::new();
    let mut buf = [0u8; 256];
    loop {
        // SAFETY: `rx` e' valido (lo garantisce chi chiama); `buf` e' lungo `buf.len()` byte e la funzione ne scrive al massimo tanti.
        let n = unsafe { take(rx, buf.as_mut_ptr(), buf.len()) };
        out.extend_from_slice(&buf[..n.min(buf.len())]);
        if n < buf.len() {
            return out;
        }
    }
}

/// Il ricevitore BPSK di fldigi (PSK31 o PSK63, a 8000 Hz come in fldigi).
pub struct FldigiPsk {
    rx: *mut c_void,
}

impl FldigiPsk {
    /// `baud`: 31,25 o 62,5; `squelch`: `None` spento, altrimenti la soglia della metrica (0-100).
    pub fn new(baud: f64, freq_hz: f64, samplerate: f64, afc: bool, squelch: Option<f64>) -> Self {
        // SAFETY: crea un oggetto C++ posseduto da questa struttura e liberato in `drop`.
        let rx = unsafe { aurora_fldigi_psk_new(baud, freq_hz, samplerate, i32::from(afc), i32::from(squelch.is_some()), squelch.unwrap_or(0.0)) };
        Self { rx }
    }

    pub fn process(&mut self, samples: &[f32]) -> Vec<u8> {
        // SAFETY: `rx` e' valido finche' vive la struttura; il puntatore e la lunghezza vengono dalla stessa fetta.
        unsafe { aurora_fldigi_psk_process(self.rx, samples.as_ptr(), samples.len()) };
        take_all(self.rx, aurora_fldigi_psk_take)
    }

    /// La frequenza raggiunta dall'AFC.
    pub fn frequency(&self) -> f64 {
        // SAFETY: `rx` e' valido finche' vive la struttura.
        unsafe { aurora_fldigi_psk_frequency(self.rx) }
    }
}

impl Drop for FldigiPsk {
    fn drop(&mut self) {
        // SAFETY: `rx` viene da `aurora_fldigi_psk_new` e si libera una volta sola.
        unsafe { aurora_fldigi_psk_free(self.rx) };
    }
}

/// Il ricevitore CW di fldigi (a 8000 Hz come in fldigi), con l'inseguimento della velocita' acceso.
pub struct FldigiCw {
    rx: *mut c_void,
}

impl FldigiCw {
    /// `wpm`: la velocita' impostata (fldigi insegue entro ±10 WPM); `bandwidth_hz`: la banda del filtro (150 di base).
    pub fn new(tone_hz: f64, wpm: f64, bandwidth_hz: f64, samplerate: f64, squelch: Option<f64>) -> Self {
        // SAFETY: crea un oggetto C++ posseduto da questa struttura e liberato in `drop`.
        let rx = unsafe { aurora_fldigi_cw_new(tone_hz, wpm, bandwidth_hz, samplerate, i32::from(squelch.is_some()), squelch.unwrap_or(0.0)) };
        Self { rx }
    }

    pub fn process(&mut self, samples: &[f32]) -> Vec<u8> {
        // SAFETY: `rx` e' valido finche' vive la struttura; il puntatore e la lunghezza vengono dalla stessa fetta.
        unsafe { aurora_fldigi_cw_process(self.rx, samples.as_ptr(), samples.len()) };
        take_all(self.rx, aurora_fldigi_cw_take)
    }

    /// La velocita' capita.
    pub fn wpm(&self) -> f64 {
        // SAFETY: `rx` e' valido finche' vive la struttura.
        unsafe { aurora_fldigi_cw_wpm(self.rx) }
    }
}

impl Drop for FldigiCw {
    fn drop(&mut self) {
        // SAFETY: `rx` viene da `aurora_fldigi_cw_new` e si libera una volta sola.
        unsafe { aurora_fldigi_cw_free(self.rx) };
    }
}
