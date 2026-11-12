//! Il ricevitore RTTY di MMTTY 1.70L (Makoto Mori JE3HHT, Nobuyuki Oba, AA6YQ, JA7UDE; LGPL 3) compilato da `oracle/mmtty_rtty_rx.cpp`,
//! come secondo arbitro nelle prove del nostro ricevitore RTTY. Esiste solo con la feature `mmtty-oracle`, che non entra mai
//! nell'installatore.

use std::ffi::c_void;

unsafe extern "C" {
    fn aurora_mmtty_rtty_new(profile: i32, baud: f64, shift: f64, centre: f64, squelch: i32) -> *mut c_void;
    fn aurora_mmtty_rtty_process(rx: *mut c_void, samples: *const f32, len: usize);
    fn aurora_mmtty_rtty_take(rx: *mut c_void, buf: *mut u8, cap: usize) -> usize;
    fn aurora_mmtty_rtty_free(rx: *mut c_void);
}

/// I profili di ricezione di `UserPara.ini` di MMTTY.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MmttyProfile {
    /// «Standard RTTY»: filtri IIR a risonanza larghi 60 Hz, limitatore, voto di maggioranza.
    Standard,
    /// «Fluttered signals»: IIR a 90 Hz con il passa-banda d'ingresso.
    Fluttered,
    /// «Fluttered signals (FIR)»: filtri FIR a 72 prese con il passa-banda d'ingresso.
    FlutteredFir,
    /// «Multi-path»: IIR a 90 Hz con l'ATC, stop di un bit.
    MultiPath,
    /// «AA6YQ FIR512»: filtri FIR a 512 prese e i filtri a due picchi di AA6YQ.
    Aa6yqFir512,
    /// «FFT demodulator»: FFT scorrevole su 0,8 bit.
    Fft,
    /// «23Hz RTTY (MTTY)»: demodulatore a PLL per gli spostamenti stretti.
    Pll23,
}

impl MmttyProfile {
    pub const ALL: [MmttyProfile; 7] = [Self::Standard, Self::Fluttered, Self::FlutteredFir, Self::MultiPath, Self::Aa6yqFir512, Self::Fft, Self::Pll23];

    pub fn name(self) -> &'static str {
        match self {
            Self::Standard => "Standard",
            Self::Fluttered => "Fluttered",
            Self::FlutteredFir => "Flutt.FIR",
            Self::MultiPath => "Multipath",
            Self::Aa6yqFir512 => "AA6YQ512",
            Self::Fft => "FFT",
            Self::Pll23 => "PLL23",
        }
    }

    fn index(self) -> i32 {
        Self::ALL.iter().position(|&p| p == self).unwrap_or(0) as i32
    }
}

/// Il ricevitore di MMTTY a 12 kHz in USB (mark = tono alto, con Rev), AFC spento, UOS acceso.
pub struct MmttyRtty {
    rx: *mut c_void,
}

impl MmttyRtty {
    /// `centre_hz`: il centro fra i due toni; `squelch`: lo squelch del profilo (soglia 600 di MMTTY).
    pub fn new(profile: MmttyProfile, baud: f64, shift_hz: f64, centre_hz: f64, squelch: bool) -> Self {
        // SAFETY: crea un oggetto C++ posseduto da questa struttura e liberato in `drop`.
        let rx = unsafe { aurora_mmtty_rtty_new(profile.index(), baud, shift_hz, centre_hz, i32::from(squelch)) };
        Self { rx }
    }

    /// Elabora altro audio a 12 kHz (ampiezza ±1); restituisce i caratteri nuovi.
    pub fn process(&mut self, samples: &[f32]) -> Vec<u8> {
        // SAFETY: `rx` e' valido finche' vive la struttura; il puntatore e la lunghezza vengono dalla stessa fetta.
        unsafe { aurora_mmtty_rtty_process(self.rx, samples.as_ptr(), samples.len()) };
        let mut out = Vec::new();
        let mut buf = [0u8; 256];
        loop {
            // SAFETY: `buf` e' lungo `buf.len()` byte e la funzione ne scrive al massimo tanti.
            let n = unsafe { aurora_mmtty_rtty_take(self.rx, buf.as_mut_ptr(), buf.len()) };
            out.extend_from_slice(&buf[..n.min(buf.len())]);
            if n < buf.len() {
                return out;
            }
        }
    }
}

impl Drop for MmttyRtty {
    fn drop(&mut self) {
        // SAFETY: `rx` viene da `aurora_mmtty_rtty_new` e si libera una volta sola.
        unsafe { aurora_mmtty_rtty_free(self.rx) };
    }
}
