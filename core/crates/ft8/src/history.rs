//! La cronologia tra periodi che serve alle sottopassate 3-11 di `ft8b` (JTDX 2.2.159, `lib/ft8b.f90` e `lib/ft8_decode.f90`,
//! sorgenti pubblici letti alla lettera; costanti incrociate con l'eseguibile in `docs/analisi-jtdx-reverse-rea.md`).
//!
//! FT8 alterna periodi di parita' diversa (quello che comincia a un multiplo pari o dispari di 15 s dall'inizio dei tempi);
//! un segnale CQ, una chiamata diretta a noi o il segnale del canale QSO visti in un periodo restano **per il prossimo
//! periodo della stessa parita'** (quello due periodi dopo, 30 s via, non quello subito dopo): se lo stesso segnale debole
//! si ripete la' con la stessa frequenza (a meno di 2 Hz) e lo stesso ritardo (a meno di 0,05 s), `ft8b` prova a sommarne i
//! simboli con quelli nuovi, in potenza o in ampiezza (`isubp1` 4/5, 7/8, 10/11 di [`crate::decode`]), oltre alla somma con
//! la lettura a tempo invertito dello stesso periodo (`isubp1` 3/6/9, che non serve la cronologia).
//!
//! JTDX tiene queste liste per thread (banda divisa in fette); Aurora ha un solo thread, quindi una lista sola per parita'.

use crate::sync::Complex;

/// Quanti CQ deboli si tengono per parita' (JTDX: `numcqsig`, 20 per thread).
const MAX_CQ: usize = 20;
/// Quante chiamate dirette a noi si tengono per parita' (JTDX: `nummycsig`, 5 per thread).
const MAX_MY_CALL: usize = 5;
/// Entro questa distanza in frequenza (Hz) due segnali di periodi diversi sono lo stesso (`evencq(ik,nthr)%freq`...).
const MATCH_FREQ_HZ: f32 = 2.0;
/// Entro questa distanza nel ritardo (s) due segnali di periodi diversi sono lo stesso.
const MATCH_DT_S: f32 = 0.05;

/// Un segnale tenuto da un periodo all'altro: dove si trova e i suoi simboli complessi (8 toni x 79 posizioni), gia' letti
/// nella direzione giusta per quel tipo di segnale (`tmpcqsig`/`tmpmycsig`/`tmpqsosig` di `ft8_decode.f90`).
#[derive(Debug, Clone)]
pub struct StoredSignal {
    pub freq_hz: f32,
    pub dt_s: f32,
    pub cs: Vec<[Complex; 8]>,
}

impl StoredSignal {
    pub(crate) fn new(freq_hz: f32, dt_s: f32, cs: &[[Complex; 8]]) -> StoredSignal {
        StoredSignal { freq_hz, dt_s, cs: cs.to_vec() }
    }

    fn matches(&self, freq_hz: f32, dt_s: f32) -> bool {
        (self.freq_hz - freq_hz).abs() < MATCH_FREQ_HZ && (self.dt_s - dt_s).abs() < MATCH_DT_S
    }
}

#[derive(Debug, Clone, Default)]
struct Parity {
    cq: Vec<StoredSignal>,
    my_call: Vec<StoredSignal>,
    qso: Option<StoredSignal>,
}

/// La cronologia dei periodi dispari e pari: un'istanza vive per tutta la sessione di ricezione (come [`crate::message::CallHashes`]),
/// aggiornata a ogni periodo con [`PeriodHistory::store_cq`], [`PeriodHistory::store_my_call`] e [`PeriodHistory::store_qso`].
#[derive(Debug, Clone, Default)]
pub struct PeriodHistory {
    even: Parity,
    odd: Parity,
}

impl PeriodHistory {
    fn side(&self, even_period: bool) -> &Parity {
        if even_period { &self.even } else { &self.odd }
    }

    fn side_mut(&mut self, even_period: bool) -> &mut Parity {
        if even_period { &mut self.even } else { &mut self.odd }
    }

    /// Il CQ del periodo precedente della stessa parita' che corrisponde a (freq, dt), se c'e'.
    pub fn find_cq(&self, even_period: bool, freq_hz: f32, dt_s: f32) -> Option<&StoredSignal> {
        self.side(even_period).cq.iter().find(|s| s.matches(freq_hz, dt_s))
    }

    /// La chiamata diretta a noi del periodo precedente della stessa parita' che corrisponde a (freq, dt), se c'e'.
    pub fn find_my_call(&self, even_period: bool, freq_hz: f32, dt_s: f32) -> Option<&StoredSignal> {
        self.side(even_period).my_call.iter().find(|s| s.matches(freq_hz, dt_s))
    }

    /// Il segnale del canale QSO del periodo precedente della stessa parita', se corrisponde a (freq, dt).
    pub fn find_qso(&self, even_period: bool, freq_hz: f32, dt_s: f32) -> Option<&StoredSignal> {
        self.side(even_period).qso.as_ref().filter(|s| s.matches(freq_hz, dt_s))
    }

    /// Sostituisce l'elenco dei CQ visti in questo periodo (quelli di questa parita' diventano la cronologia per il
    /// prossimo periodo della stessa parita'); tiene solo i primi [`MAX_CQ`].
    pub fn store_cq(&mut self, even_period: bool, signals: Vec<StoredSignal>) {
        let side = self.side_mut(even_period);
        side.cq = signals;
        side.cq.truncate(MAX_CQ);
    }

    /// Come [`PeriodHistory::store_cq`], per le chiamate dirette a noi.
    pub fn store_my_call(&mut self, even_period: bool, signals: Vec<StoredSignal>) {
        let side = self.side_mut(even_period);
        side.my_call = signals;
        side.my_call.truncate(MAX_MY_CALL);
    }

    /// Sostituisce il segnale del canale QSO di questo periodo (uno solo, come in JTDX).
    pub fn store_qso(&mut self, even_period: bool, signal: Option<StoredSignal>) {
        self.side_mut(even_period).qso = signal;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NN;

    fn cs(value: f32) -> Vec<[Complex; 8]> {
        vec![[Complex::new(value, 0.0); 8]; NN]
    }

    #[test]
    fn un_segnale_si_ritrova_solo_nella_stessa_parita_entro_la_tolleranza() {
        let mut h = PeriodHistory::default();
        h.store_cq(true, vec![StoredSignal::new(1500.0, 0.1, &cs(1.0))]);
        assert!(h.find_cq(true, 1501.0, 0.12).is_some());
        assert!(h.find_cq(true, 1503.0, 0.12).is_none(), "oltre 2 Hz non e' lo stesso segnale");
        assert!(h.find_cq(true, 1501.0, 0.2).is_none(), "oltre 0,05 s non e' lo stesso segnale");
        assert!(h.find_cq(false, 1501.0, 0.12).is_none(), "la parita' dispari non vede quella pari");
    }

    #[test]
    fn store_sostituisce_non_accumula() {
        let mut h = PeriodHistory::default();
        h.store_cq(true, vec![StoredSignal::new(1500.0, 0.0, &cs(1.0))]);
        h.store_cq(true, vec![StoredSignal::new(2000.0, 0.0, &cs(1.0))]);
        assert!(h.find_cq(true, 1500.0, 0.0).is_none());
        assert!(h.find_cq(true, 2000.0, 0.0).is_some());
    }

    #[test]
    fn il_canale_qso_tiene_un_solo_segnale_per_parita() {
        let mut h = PeriodHistory::default();
        h.store_qso(false, Some(StoredSignal::new(1000.0, 0.0, &cs(1.0))));
        assert!(h.find_qso(false, 1000.0, 0.0).is_some());
        assert!(h.find_qso(true, 1000.0, 0.0).is_none());
        h.store_qso(false, None);
        assert!(h.find_qso(false, 1000.0, 0.0).is_none());
    }
}
