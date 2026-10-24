//! Ricerca e allineamento dei segnali: `sync8`, `ft8_downsample`, `sync8d` e il calcolo delle probabilita' dei bit di `ft8b`.

/// Un segnale probabile nello spettro del periodo.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Candidate {
    pub freq_hz: f32,
    /// Ritardo rispetto all'inizio del periodo (secondi, puo' essere negativo).
    pub dt_s: f32,
    pub sync: f32,
}

/// Un numero complesso a 32 bit (parte reale e immaginaria).
pub type Complex = (f32, f32);

/// Cerca i candidati tra `f_low_hz` e `f_high_hz` nei 15 secondi di `samples` (12 kHz).
pub fn find_candidates(_samples: &[f32], _f_low_hz: f32, _f_high_hz: f32, _min_sync: f32, _max: usize) -> Vec<Candidate> {
    Vec::new()
}

/// Porta il segnale a 200 Hz di campionamento centrato su `f0_hz` (3200 campioni complessi).
pub fn downsample(_samples: &[f32], _f0_hz: f32) -> Vec<Complex> {
    Vec::new()
}

/// Corregge frequenza e tempo di un candidato sul segnale a 200 Hz.
pub fn refine(_cd: &[Complex], _candidate: Candidate) -> Candidate {
    _candidate
}

/// Le tre serie di LLR (simboli letti da 1, 2 e 3 alla volta) dei 174 bit del segnale allineato; `None` se il sincronismo non regge.
pub fn bit_metrics(_cd: &[Complex], _candidate: Candidate) -> Option<[[f32; 174]; 3]> {
    None
}
