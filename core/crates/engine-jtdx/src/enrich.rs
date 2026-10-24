use std::sync::Arc;

use aurora_core::DecodeTag;
use dxcc::DxccDb;

use crate::{
    text::parse_message,
    worked::{Novelty, WorkedLog},
};

/// Esito dell'arricchimento di un messaggio.
pub struct Annotation {
    pub country: Option<String>,
    pub continent: Option<String>,
    pub tag: Option<DecodeTag>,
}

/// Paese e novita' di un decode, ricavati da `cty.dat` e dai QSO gia' fatti.
pub struct Enrichment {
    db: Arc<DxccDb>,
    worked: WorkedLog,
}

impl Enrichment {
    pub fn new(db: Arc<DxccDb>, worked: WorkedLog) -> Self {
        Enrichment { db, worked }
    }

    /// Continente di un nominativo (`AF`, `EU`, `NA`...), se `cty.dat` lo riconosce.
    pub fn continent_of(&self, call: &str) -> Option<String> {
        self.db.lookup(call).map(|found| self.db.entity(found.entity).continent.clone())
    }

    /// Paese ed etichetta di un messaggio sulla banda corrente.
    ///
    /// Il paese si mostra sempre che sia noto. Le novita' si segnalano solo se il registro dei lavorati
    /// non e' vuoto (altrimenti tutto sarebbe "nuovo") e se la banda e' nota. La nuova griglia conta solo
    /// per i CQ: un messaggio tra altre due stazioni segnala al piu' un paese o una zona nuovi.
    pub fn annotate(&self, text: &str, band: Option<&str>) -> Annotation {
        let parsed = parse_message(text);
        let country = parsed
            .sender
            .as_deref()
            .and_then(|call| self.db.lookup(call))
            .map(|found| self.db.entity(found.entity).name.clone());
        let continent = parsed.sender.as_deref().and_then(|call| self.continent_of(call));
        let base = parsed.is_cq.then_some(DecodeTag::Cq);
        let (Some(sender), Some(band)) = (parsed.sender.as_deref(), band) else {
            return Annotation { country, continent, tag: base };
        };
        if self.worked.is_empty() {
            return Annotation { country, continent, tag: base };
        }
        let novelty = self.worked.novelty(sender, parsed.grid.as_deref(), band, &self.db);
        Annotation { country, continent, tag: tag_for(novelty, parsed.is_cq).or(base) }
    }

    /// Un QSO appena chiuso: da subito la stazione non e' piu' una novita'.
    pub fn record_qso(&mut self, call: &str, grid: &str, band: &str) {
        self.worked.add_qso(call, grid, band, &self.db);
    }
}

/// Etichetta di una novita'. In un messaggio tra altre due stazioni contano solo paese, zona e ITU;
/// griglia, prefisso, stazione e "gia' lavorata" si segnalano per i CQ.
fn tag_for(novelty: Novelty, is_cq: bool) -> Option<DecodeTag> {
    let broad = match novelty {
        Novelty::NewDxcc => Some(DecodeTag::NewDxcc),
        Novelty::NewDxccBand => Some(DecodeTag::NewDxccBand),
        Novelty::NewZone => Some(DecodeTag::NewZone),
        Novelty::NewZoneBand => Some(DecodeTag::NewZoneBand),
        Novelty::NewItu => Some(DecodeTag::NewItu),
        Novelty::NewItuBand => Some(DecodeTag::NewItuBand),
        _ => None,
    };
    if broad.is_some() || !is_cq {
        return broad;
    }
    match novelty {
        Novelty::NewGrid => Some(DecodeTag::NewGrid),
        Novelty::NewGridBand => Some(DecodeTag::NewGridBand),
        Novelty::NewPx => Some(DecodeTag::NewPx),
        Novelty::NewPxBand => Some(DecodeTag::NewPxBand),
        Novelty::NewCall => Some(DecodeTag::NewCall),
        Novelty::NewCallBand => Some(DecodeTag::NewCallBand),
        Novelty::Worked => Some(DecodeTag::WorkedCall),
        _ => None,
    }
}
