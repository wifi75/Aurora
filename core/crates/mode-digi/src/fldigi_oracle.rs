//! Il ricevitore RTTY di fldigi 4.2.13 compilato da `terze-parti/fldigi` (vedi `oracle/fldigi_rtty_rx.cpp` e `build.rs`), come arbitro
//! nelle prove del nostro ricevitore. Esiste solo con la feature `fldigi-oracle`, che non entra mai nell'installatore.

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
