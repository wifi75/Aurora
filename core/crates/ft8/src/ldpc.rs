//! Codice LDPC(174,91) di FT8: codifica e decodifica (propagazione di credenze e OSD), come `encode174_91`, `bpdecode174_91`
//! e `osd174_91` di WSJT-X/JTDX.

/// Un codice decodificato.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decoded {
    /// I 174 bit corretti.
    pub codeword: [u8; 174],
    /// I 91 bit di messaggio e CRC (il CRC torna).
    pub message: [u8; 91],
    /// Iterazioni usate (0 per OSD).
    pub iterations: usize,
}

/// Da 91 bit (messaggio e CRC) ai 174 bit trasmessi.
pub fn encode(_message: &[u8; 91]) -> [u8; 174] {
    [0; 174]
}

/// Propagazione di credenze sui rapporti di verosimiglianza (LLR, positivo = bit 0 piu' probabile? come in WSJT-X: positivo = 1).
pub fn bp_decode(_llr: &[f32; 174], _max_iterations: usize) -> Option<Decoded> {
    None
}

/// Decodifica a statistica d'ordine (OSD) per i casi che la propagazione non risolve.
pub fn osd_decode(_llr: &[f32; 174], _order: usize) -> Option<Decoded> {
    None
}
