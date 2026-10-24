use std::sync::Arc;

use aurora_core::DecodeTag;
use dxcc::DxccDb;

use crate::{
    text::parse_message,
    worked::{Novelty, WorkedLog},
};

/// Paese e novita' di un decode, ricavati da `cty.dat` e dai QSO gia' fatti.
pub struct Enrichment {
    db: Arc<DxccDb>,
    worked: WorkedLog,
}

impl Enrichment {
    pub fn new(db: Arc<DxccDb>, worked: WorkedLog) -> Self {
        Enrichment { db, worked }
    }

    /// Paese ed etichetta di un messaggio sulla banda corrente.
    ///
    /// Il paese si mostra sempre che sia noto. Le novita' si segnalano solo se il registro dei lavorati
    /// non e' vuoto (altrimenti tutto sarebbe "nuovo") e se la banda e' nota. La nuova griglia conta solo
    /// per i CQ: un messaggio tra altre due stazioni segnala al piu' un paese o una zona nuovi.
    pub fn annotate(&self, text: &str, band: Option<&str>) -> (Option<String>, Option<DecodeTag>) {
        let parsed = parse_message(text);
        let country = parsed
            .sender
            .as_deref()
            .and_then(|call| self.db.lookup(call))
            .map(|found| self.db.entity(found.entity).name.clone());
        let base = parsed.is_cq.then_some(DecodeTag::Cq);
        let (Some(sender), Some(band)) = (parsed.sender.as_deref(), band) else {
            return (country, base);
        };
        if self.worked.is_empty() {
            return (country, base);
        }
        let tag = match self.worked.novelty(sender, parsed.grid.as_deref(), band, &self.db) {
            Novelty::NewDxcc => Some(DecodeTag::NewDxcc),
            Novelty::NewDxccBand => Some(DecodeTag::NewDxccBand),
            Novelty::NewZone => Some(DecodeTag::NewZone),
            Novelty::NewGrid if parsed.is_cq => Some(DecodeTag::NewGrid),
            _ => base,
        };
        (country, tag)
    }

    /// Un QSO appena chiuso: da subito la stazione non e' piu' una novita'.
    pub fn record_qso(&mut self, call: &str, grid: &str, band: &str) {
        self.worked.add_qso(call, grid, band, &self.db);
    }
}
