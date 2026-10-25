use std::sync::{Arc, RwLock};

use aurora_core::DecodeTag;
use dxcc::DxccDb;

use crate::{
    text::parse_message,
    worked::{HighlightFlags, Novelty, WorkedLog},
};

/// Esito dell'arricchimento di un messaggio.
pub struct Annotation {
    pub country: Option<String>,
    pub continent: Option<String>,
    pub tag: Option<DecodeTag>,
}

/// I QSO gia' fatti, condivisi: chi registra un QSO o importa un log li aggiorna e le etichette delle decodifiche, la scheda DXCC e le
/// statistiche guardano gli stessi insiemi.
pub type SharedWorked = Arc<RwLock<WorkedLog>>;

/// Le evidenziazioni di «nuovo» (scheda Notifiche), condivise: chi le cambia le aggiorna e il prossimo decode le usa.
pub type SharedFlags = Arc<RwLock<HighlightFlags>>;

/// Paese e novita' di un decode, ricavati da `cty.dat` e dai QSO gia' fatti.
pub struct Enrichment {
    db: Arc<DxccDb>,
    worked: SharedWorked,
    flags: SharedFlags,
}

impl Enrichment {
    pub fn new(db: Arc<DxccDb>, worked: WorkedLog) -> Self {
        Enrichment { db, worked: Arc::new(RwLock::new(worked)), flags: Arc::default() }
    }

    /// Come `new`, ma con i QSO gia' fatti condivisi con altri (scheda DXCC, importazione del log).
    pub fn with_shared(db: Arc<DxccDb>, worked: SharedWorked) -> Self {
        Enrichment { db, worked, flags: Arc::default() }
    }

    /// Le evidenziazioni di «nuovo» condivise con chi le cambia (di base tutte accese, per banda e per modo).
    pub fn with_flags(mut self, flags: SharedFlags) -> Self {
        self.flags = flags;
        self
    }

    /// I QSO gia' fatti, da aggiornare quando il log cambia.
    pub fn shared_worked(&self) -> SharedWorked {
        Arc::clone(&self.worked)
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
        self.annotate_in(text, band, None)
    }

    /// Come `annotate`, con il modo dei decode (`FT8`, `FT4`): serve alle evidenziazioni «per modo». Il «gia' lavorato» e' quello di JTDX: la
    /// stazione che non e' nuova per nessuna evidenziazione accesa (`WorkedLog::novelty_flags`).
    pub fn annotate_in(&self, text: &str, band: Option<&str>, mode: Option<&str>) -> Annotation {
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
        let Ok(worked) = self.worked.read() else {
            return Annotation { country, continent, tag: base };
        };
        if worked.is_empty() {
            return Annotation { country, continent, tag: base };
        }
        let flags = self.flags.read().map(|f| *f).unwrap_or_default();
        let tag = worked.novelty_flags(sender, parsed.grid.as_deref(), band, mode, &flags, &self.db).and_then(|n| tag_for(n, parsed.is_cq || flags.potential));
        Annotation { country, continent, tag: tag.or(base) }
    }

    /// Un QSO appena chiuso: da subito la stazione non e' piu' una novita'.
    pub fn record_qso(&self, call: &str, grid: &str, band: &str, mode: &str) {
        if let Ok(mut worked) = self.worked.write() {
            worked.add_qso_mode(call, grid, band, mode, &self.db);
        }
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
