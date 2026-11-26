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
    fn aurora_fldigi_qpsk_new(freq: f64, samplerate: f64, reverse: i32) -> *mut c_void;
    fn aurora_fldigi_cw_new(freq: f64, wpm: f64, bandwidth: f64, samplerate: f64, squelch: i32, squelch_value: f64) -> *mut c_void;
    fn aurora_fldigi_cw_process(rx: *mut c_void, samples: *const f32, len: usize);
    fn aurora_fldigi_cw_take(rx: *mut c_void, buf: *mut u8, cap: usize) -> usize;
    fn aurora_fldigi_cw_wpm(rx: *mut c_void) -> f64;
    fn aurora_fldigi_cw_free(rx: *mut c_void);
    fn aurora_fldigi_olivia_encode_block(bits_per_symbol: i32, contestia: i32, chars: *const u8, out: *mut u8, out_cap: usize) -> usize;
    fn aurora_fldigi_olivia_rx_new(tones: i32, bandwidth: i32, center_hz: f64, sync_margin: i32, sync_integ: i32, threshold: f64, contestia: i32) -> *mut c_void;
    fn aurora_fldigi_olivia_rx_process(rx: *mut c_void, samples: *const f32, len: usize);
    fn aurora_fldigi_olivia_rx_flush(rx: *mut c_void);
    fn aurora_fldigi_olivia_rx_take(rx: *mut c_void, buf: *mut u8, cap: usize) -> usize;
    fn aurora_fldigi_olivia_rx_snr(rx: *mut c_void) -> f64;
    fn aurora_fldigi_olivia_rx_freq(rx: *mut c_void) -> f64;
    fn aurora_fldigi_olivia_rx_free(rx: *mut c_void);
}

/// Il ricevitore di Olivia (o Contestia) di fldigi: audio a 8000 Hz, `center_hz` e' il centro della banda.
pub struct FldigiOlivia {
    rx: *mut c_void,
}

impl FldigiOlivia {
    /// `sync_margin` e `sync_integ` come nelle impostazioni di fldigi (di base 8 e 4); `threshold` e' la soglia del rapporto segnale/rumore (3,0 la minima).
    pub fn new(tones: usize, bandwidth: usize, center_hz: f64, contestia: bool) -> Self {
        // SAFETY: crea un oggetto C++ posseduto da questa struttura e liberato in `drop`.
        let rx = unsafe { aurora_fldigi_olivia_rx_new(tones as i32, bandwidth as i32, center_hz, 8, 4, 3.0, i32::from(contestia)) };
        Self { rx }
    }

    pub fn process(&mut self, samples: &[f32]) {
        // SAFETY: `rx` e' valido finche' vive la struttura; il puntatore e la lunghezza vengono dalla stessa fetta.
        unsafe { aurora_fldigi_olivia_rx_process(self.rx, samples.as_ptr(), samples.len()) };
    }

    /// Finisce di elaborare quello che e' in coda (come quando si passa alla trasmissione).
    pub fn flush(&mut self) {
        // SAFETY: come in `process`.
        unsafe { aurora_fldigi_olivia_rx_flush(self.rx) };
    }

    /// I caratteri decodificati finora (poi si svuota la coda).
    pub fn take(&mut self) -> Vec<u8> {
        let mut out = Vec::new();
        let mut buf = [0u8; 256];
        loop {
            // SAFETY: `buf` ha 256 byte e la funzione scrive al piu' `cap`.
            let n = unsafe { aurora_fldigi_olivia_rx_take(self.rx, buf.as_mut_ptr(), buf.len()) };
            if n == 0 {
                break;
            }
            out.extend_from_slice(&buf[..n]);
        }
        out
    }

    pub fn snr(&self) -> f64 {
        // SAFETY: `rx` e' valido finche' vive la struttura.
        unsafe { aurora_fldigi_olivia_rx_snr(self.rx) }
    }

    /// Lo spostamento di frequenza trovato dal sincronismo, in Hz.
    pub fn frequency_offset(&self) -> f64 {
        // SAFETY: `rx` e' valido finche' vive la struttura.
        unsafe { aurora_fldigi_olivia_rx_freq(self.rx) }
    }
}

impl Drop for FldigiOlivia {
    fn drop(&mut self) {
        // SAFETY: `rx` viene da `aurora_fldigi_olivia_rx_new` e si libera una volta sola.
        unsafe { aurora_fldigi_olivia_rx_free(self.rx) };
    }
}

/// Un blocco FEC di Olivia (o Contestia) codificato dalla libreria di fldigi: `chars` ha un carattere per ogni bit del simbolo.
pub fn olivia_encode_block(bits_per_symbol: usize, contestia: bool, chars: &[u8]) -> Vec<u8> {
    let mut input = [0u8; 8];
    for (slot, ch) in input.iter_mut().zip(chars) {
        *slot = *ch;
    }
    let mut out = vec![0u8; 256];
    // SAFETY: `input` ha 8 byte (come il blocco della libreria) e `out` 256: la funzione scrive al piu' `out_cap` byte.
    let n = unsafe { aurora_fldigi_olivia_encode_block(bits_per_symbol as i32, i32::from(contestia), input.as_ptr(), out.as_mut_ptr(), out.len()) };
    out.truncate(n);
    out
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

    /// Il ricevitore QPSK31 di fldigi (squelch e AFC spenti); `reverse`: inversione di banda laterale come l'opzione di fldigi.
    pub fn qpsk31(freq_hz: f64, samplerate: f64, reverse: bool) -> Self {
        // SAFETY: crea un oggetto C++ posseduto da questa struttura e liberato in `drop` (con `aurora_fldigi_psk_free`).
        let rx = unsafe { aurora_fldigi_qpsk_new(freq_hz, samplerate, i32::from(reverse)) };
        Self { rx }
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

unsafe extern "C" {
    fn aurora_fldigi_throb_new(variant: i32, freq: f64, reverse: i32) -> *mut c_void;
    fn aurora_fldigi_throb_process(rx: *mut c_void, samples: *const f32, len: usize);
    fn aurora_fldigi_throb_take(rx: *mut c_void, buf: *mut u8, cap: usize) -> usize;
    fn aurora_fldigi_throb_free(rx: *mut c_void);
    fn aurora_fldigi_throb_tx(variant: i32, freq: f64, reverse: i32, text: *const std::ffi::c_char, out: *mut f32, cap: usize) -> usize;
}

/// L'indice della variante come lo vuole `oracle/fldigi_throb.cpp` (l'ordine di [`ThrobVariant::ALL`]).
fn throb_index(v: crate::throb::ThrobVariant) -> i32 {
    crate::throb::ThrobVariant::ALL.iter().position(|&x| x == v).map_or(0, |i| i as i32)
}

/// Il ricevitore Throb/ThrobX di fldigi (squelch e AFC spenti), audio a 8000 Hz; `freq_hz` e' il centro.
pub struct FldigiThrob {
    rx: *mut c_void,
}

impl FldigiThrob {
    pub fn new(v: crate::throb::ThrobVariant, freq_hz: f64, reverse: bool) -> Self {
        // SAFETY: crea un oggetto C++ posseduto da questa struttura e liberato in `drop`.
        let rx = unsafe { aurora_fldigi_throb_new(throb_index(v), freq_hz, i32::from(reverse)) };
        Self { rx }
    }

    pub fn process(&mut self, samples: &[f32]) -> Vec<u8> {
        // SAFETY: `rx` e' valido finche' vive la struttura; il puntatore e la lunghezza vengono dalla stessa fetta.
        unsafe { aurora_fldigi_throb_process(self.rx, samples.as_ptr(), samples.len()) };
        take_all(self.rx, aurora_fldigi_throb_take)
    }

    /// Il trasmettitore di fldigi: il testo in audio a 8 kHz (quattro simboli di attesa, i caratteri, un'attesa alla fine).
    pub fn transmit(v: crate::throb::ThrobVariant, freq_hz: f64, reverse: bool, text: &str) -> Vec<f32> {
        let Ok(c) = std::ffi::CString::new(text) else { return Vec::new() };
        // SAFETY: con `out` nullo la funzione dice solo quanti campioni servono.
        let n = unsafe { aurora_fldigi_throb_tx(throb_index(v), freq_hz, i32::from(reverse), c.as_ptr(), std::ptr::null_mut(), 0) };
        let mut out = vec![0.0_f32; n];
        // SAFETY: `out` e' lungo `n` campioni e la funzione ne scrive al massimo tanti.
        unsafe { aurora_fldigi_throb_tx(throb_index(v), freq_hz, i32::from(reverse), c.as_ptr(), out.as_mut_ptr(), n) };
        out
    }
}

impl Drop for FldigiThrob {
    fn drop(&mut self) {
        // SAFETY: `rx` viene da `aurora_fldigi_throb_new` e si libera una volta sola.
        unsafe { aurora_fldigi_throb_free(self.rx) };
    }
}

unsafe extern "C" {
    fn aurora_fldigi_sitorb_new(samplerate: f64, centre: f64) -> *mut c_void;
    fn aurora_fldigi_sitorb_process(rx: *mut c_void, samples: *const f32, len: usize);
    fn aurora_fldigi_sitorb_take(rx: *mut c_void, buf: *mut u8, cap: usize) -> usize;
    fn aurora_fldigi_sitorb_free(rx: *mut c_void);
    fn aurora_fldigi_sitorb_tx(samplerate: f64, centre: f64, text: *const std::ffi::c_char, out: *mut f32, cap: usize) -> usize;
}

/// Il ricevitore SITOR-B (Amtor FEC) di fldigi, in USB senza Rev e con l'AFC spento; `centre_hz` e' il centro fra i due toni.
pub struct FldigiSitorb {
    rx: *mut c_void,
}

impl FldigiSitorb {
    pub fn new(samplerate: f64, centre_hz: f64) -> Self {
        // SAFETY: crea un oggetto C++ posseduto da questa struttura e liberato in `drop`.
        let rx = unsafe { aurora_fldigi_sitorb_new(samplerate, centre_hz) };
        Self { rx }
    }

    pub fn process(&mut self, samples: &[f32]) -> Vec<u8> {
        // SAFETY: `rx` e' valido finche' vive la struttura; il puntatore e la lunghezza vengono dalla stessa fetta.
        unsafe { aurora_fldigi_sitorb_process(self.rx, samples.as_ptr(), samples.len()) };
        take_all(self.rx, aurora_fldigi_sitorb_take)
    }

    /// Il trasmettitore SITOR-B di fldigi: il testo con le due coppie di fasatura e le due di coda.
    pub fn transmit(samplerate: f64, centre_hz: f64, text: &str) -> Vec<f32> {
        let Ok(c) = std::ffi::CString::new(text) else { return Vec::new() };
        // SAFETY: con `out` nullo la funzione dice solo quanti campioni servono.
        let n = unsafe { aurora_fldigi_sitorb_tx(samplerate, centre_hz, c.as_ptr(), std::ptr::null_mut(), 0) };
        let mut out = vec![0.0_f32; n];
        // SAFETY: `out` e' lungo `n` campioni e la funzione ne scrive al massimo tanti.
        unsafe { aurora_fldigi_sitorb_tx(samplerate, centre_hz, c.as_ptr(), out.as_mut_ptr(), n) };
        out
    }
}

impl Drop for FldigiSitorb {
    fn drop(&mut self) {
        // SAFETY: `rx` viene da `aurora_fldigi_sitorb_new` e si libera una volta sola.
        unsafe { aurora_fldigi_sitorb_free(self.rx) };
    }
}

unsafe extern "C" {
    fn aurora_fldigi_mt63_new(bandwidth: i32, long_intlv: i32, freq: f64, integ: i32) -> *mut c_void;
    fn aurora_fldigi_mt63_process(rx: *mut c_void, samples: *const f32, len: usize);
    fn aurora_fldigi_mt63_take(rx: *mut c_void, buf: *mut u8, cap: usize) -> usize;
    fn aurora_fldigi_mt63_snr(rx: *mut c_void) -> f64;
    fn aurora_fldigi_mt63_free(rx: *mut c_void);
    fn aurora_fldigi_mt63_tx(
        bandwidth: i32,
        long_intlv: i32,
        freq: f64,
        text: *const std::ffi::c_char,
        out: *mut f32,
        cap: usize,
        fill: *mut u8,
        jam: *mut u8,
    ) -> usize;
}

/// Il ricevitore MT63 di fldigi (7 bit, senza squelch), audio a 8000 Hz; `freq_hz` e' il centro; `integ`: 16 o 32 simboli.
pub struct FldigiMt63 {
    rx: *mut c_void,
}

/// Quello che il trasmettitore di fldigi ha prodotto: l'audio grezzo e i numeri a caso usati (riempimento e disturbo).
pub struct FldigiMt63Tx {
    pub wave: Vec<f32>,
    pub fill: Vec<u8>,
    pub jam: Vec<bool>,
}

impl FldigiMt63 {
    pub fn new(v: crate::mt63::Mt63Variant, freq_hz: f64, integ: i32) -> Self {
        // SAFETY: crea un oggetto C++ posseduto da questa struttura e liberato in `drop`.
        let rx = unsafe { aurora_fldigi_mt63_new(v.bandwidth_hz() as i32, i32::from(v.long()), freq_hz, integ) };
        Self { rx }
    }

    pub fn process(&mut self, samples: &[f32]) -> Vec<u8> {
        if self.rx.is_null() {
            return Vec::new();
        }
        // SAFETY: `rx` e' valido finche' vive la struttura; il puntatore e la lunghezza vengono dalla stessa fetta.
        unsafe { aurora_fldigi_mt63_process(self.rx, samples.as_ptr(), samples.len()) };
        take_all(self.rx, aurora_fldigi_mt63_take)
    }

    /// Il rapporto segnale/rumore del decodificatore (lineare, come lo vede fldigi).
    pub fn snr(&self) -> f64 {
        // SAFETY: `rx` e' valido finche' vive la struttura (la funzione accetta anche il nullo).
        unsafe { aurora_fldigi_mt63_snr(self.rx) }
    }

    /// Il trasmettitore di fldigi (sequenza di `tx_process`, uscita grezza).
    pub fn transmit(v: crate::mt63::Mt63Variant, freq_hz: f64, text: &str) -> FldigiMt63Tx {
        let Ok(c) = std::ffi::CString::new(text) else { return FldigiMt63Tx { wave: Vec::new(), fill: Vec::new(), jam: Vec::new() } };
        let (bw, long) = (v.bandwidth_hz() as i32, i32::from(v.long()));
        let mut fill = vec![0u8; v.interleave() * 64];
        let mut jam = vec![0u8; 64];
        let null = std::ptr::null_mut();
        // SAFETY: con `out` nullo la funzione dice solo quanti campioni servono; `fill` e `jam` sono lunghi quanto chiede.
        let n = unsafe { aurora_fldigi_mt63_tx(bw, long, freq_hz, c.as_ptr(), null, 0, fill.as_mut_ptr(), jam.as_mut_ptr()) };
        let mut wave = vec![0.0_f32; n];
        // SAFETY: `wave` e' lungo `n` campioni e la funzione ne scrive al massimo tanti.
        unsafe { aurora_fldigi_mt63_tx(bw, long, freq_hz, c.as_ptr(), wave.as_mut_ptr(), n, fill.as_mut_ptr(), jam.as_mut_ptr()) };
        FldigiMt63Tx { wave, fill, jam: jam.into_iter().map(|b| b != 0).collect() }
    }
}

impl Drop for FldigiMt63 {
    fn drop(&mut self) {
        // SAFETY: `rx` viene da `aurora_fldigi_mt63_new` e si libera una volta sola (il nullo e' ammesso).
        unsafe { aurora_fldigi_mt63_free(self.rx) };
    }
}
