//! I contest a cartella (`contest/<nome>/`, `docs/contest-formato.md`): una cartella per contest con `contest.toml`, `punteggio.regole`,
//! il regolamento, gli elenchi `.dom`, le macro e le prove. Qui si leggono e si controllano; la conversione in [`ContestTesto`] li fa
//! funzionare come i vecchi `.contest` (motore del punteggio, riga di inserimento, Cabrillo e API senza modifiche).
//!
//! Due radici (in ordine di precedenza):
//! - la cartella dell'utente (`<dati>/contest/<nome>/`);
//! - le cartelle incluse nell'installatore (sola lettura; in sviluppo la cartella `contest/` del repository, `AURORA_CONTEST_FOLDERS`).
//!
//! Una cartella di pari nome nella radice dell'utente sostituisce quella inclusa (tutta, non file per file). Una cartella non valida si
//! salta con un avviso nel registro e non rompe le altre.
//!
//! Il nome di un contest in cartella sostituisce nell'elenco un `.contest` con lo stesso nome (senza badare alle maiuscole), salvo un
//! `.contest` personalizzato dall'utente quando la cartella e' quella inclusa (vedi [`ElencoContest::sostituito_da`]).

use crate::calendario::{civil_from_days, days_from_civil, parse_iso, weekday};
use crate::elenco::{ContestTesto, Dominio, ElencoContest, MacroContest, leggi_dominio, leggi_macro, percorso_relativo_ok, testo_da_byte};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Dimensione massima di ogni file e numero massimo di file di una cartella (come per i pacchetti `.zip`).
pub const MAX_BYTE: u64 = crate::pacchetto::MAX_BYTE;
pub const MAX_FILE: usize = 64;
/// Sottocartelle dei dati dell'utente che non sono contest.
const RISERVATE: [&str; 4] = ["macros", "dom", "regole", CARTELLA_SOVRASCRITTURE];

/// La sottocartella dei dati dell'utente con le modifiche personali ai contest in cartella: `sovrascritture/<id>.json`.
pub const CARTELLA_SOVRASCRITTURE: &str = "sovrascritture";
/// Ogni quanto si rilegge il disco (le cartelle cambiano di rado, la riga di inserimento chiede il contest a ogni tasto).
const VALIDITA_CACHE: Duration = Duration::from_secs(3);
const GIORNO_MS: i64 = 86_400_000;

/// Dove sta una cartella: nell'installatore (sola lettura) o nei dati dell'utente.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Origine {
    Inclusa,
    Utente,
}

// ---------------------------------------------------------------- contest.toml

/// `[contest]`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Meta {
    pub id: String,
    pub nome: String,
    pub organizzatore: String,
    pub sito: String,
    pub regolamento_url: String,
    pub cabrillo_contest: String,
    pub note: String,
    /// Altri nomi con cui il contest compare nei vecchi file `.contest` (per sostituirli: `ElencoContest::sostituito_da`).
    pub alias: Vec<String>,
    /// I QTC del WAE (`docs/contest-qtc.md`): 0 nessuno, 1 dalle stazioni non europee alle europee (CW, SSB), 2 in entrambe le
    /// direzioni fra continenti diversi (RTTY). Diventa `ContestTesto.qtc`.
    pub qtc: u8,
}

/// Una data di edizione scritta nel file (`[[edizione.date]]`): vale al posto del calcolo.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DataEdizione {
    pub anno: i64,
    pub inizio: String,
    pub fine: String,
    pub fonte: String,
    pub confermata: bool,
    pub nota: String,
}

/// `[edizione]`: la regola delle date come dati.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Edizione {
    pub mese: u32,
    /// L'ennesimo fine settimana pieno del mese (sabato e domenica nello stesso mese).
    pub fine_settimana_pieno: u32,
    /// `venerdi`, `sabato` o `domenica`: il giorno d'inizio rispetto al sabato di quel fine settimana.
    pub giorno_inizio: String,
    /// `hh:mm` UTC.
    pub ora_inizio: String,
    pub durata_ore: u32,
    pub ore_massime: Option<u32>,
    pub pausa_minima_min: Option<u32>,
    pub invio_log_giorni: Option<u32>,
    pub date: Vec<DataEdizione>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Bande {
    pub ammesse: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LimiteModo {
    pub modo: String,
    pub frequenza_massima_khz: Option<u64>,
    pub frequenza_minima_khz: Option<u64>,
    pub nota: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Modi {
    pub ammessi: Vec<String>,
    pub limite: Vec<LimiteModo>,
}

/// Un campo dello scambio (`[[scambio.ricevuto]]`, `[[scambio.inviato]]`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CampoScambio {
    pub nome: String,
    pub etichetta_it: String,
    pub etichetta_en: String,
    /// `rst`, `numero`, `testo`, `zona_cq` (proposta nello scambio) o `zona_itu` (solo suggerita).
    pub tipo: String,
    /// Condizione come in `punteggio.regole` (`any` = sempre).
    pub quando: String,
    pub predefinito: BTreeMap<String, String>,
    /// File `.dom` dell'elenco dei valori (`dati/stati.dom`).
    pub elenco: String,
    pub valori: Vec<String>,
    /// Il numero avanza a ogni QSO.
    pub progressivo: bool,
    /// `false` = campo non ancora gestito dal motore.
    pub motore: bool,
    pub nota: String,
}

impl Default for CampoScambio {
    fn default() -> Self {
        CampoScambio {
            nome: String::new(),
            etichetta_it: String::new(),
            etichetta_en: String::new(),
            tipo: String::new(),
            quando: String::new(),
            predefinito: BTreeMap::new(),
            elenco: String::new(),
            valori: Vec::new(),
            progressivo: false,
            motore: true,
            nota: String::new(),
        }
    }
}

/// `[scambio]`: i campi dichiarati (letti e tenuti; la riga di inserimento oggi li ricava dalle regole).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Scambio {
    pub ricevuto: Vec<CampoScambio>,
    pub inviato: Vec<CampoScambio>,
}

/// `[doppi]`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Doppi {
    /// `modo`, `banda`, `banda+modo` o `sempre`.
    pub per: String,
    pub nota: String,
}

/// Una scheda dichiarata (`[[scheda]]`): `tipo`, `titolo` per lingua, `ordine` e i parametri propri del tipo, tenuti cosi' come sono.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Scheda {
    pub tipo: String,
    #[serde(default)]
    pub titolo: BTreeMap<String, String>,
    #[serde(default)]
    pub ordine: i64,
    #[serde(flatten)]
    pub parametri: toml::Table,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileToml {
    contest: Meta,
    #[serde(default)]
    edizione: Option<Edizione>,
    #[serde(default)]
    bande: Bande,
    #[serde(default)]
    modi: Modi,
    #[serde(default)]
    scambio: Scambio,
    #[serde(default)]
    doppi: Doppi,
    #[serde(default)]
    categorie: toml::Table,
    #[serde(default)]
    cabrillo: toml::Table,
    #[serde(default)]
    scheda: Vec<Scheda>,
}

// ---------------------------------------------------------------- date

/// L'inizio e la fine (ms UTC, fine = ultimo secondo) di un'edizione.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct DateEdizione {
    pub anno: i64,
    pub inizio_ms: i64,
    pub fine_ms: i64,
    /// Date lette dal calendario (`confermata = true`) e non solo calcolate dalla regola.
    pub confermata: bool,
}

fn giorni_del_mese(anno: i64, mese: u32) -> i64 {
    let dopo = if mese == 12 { days_from_civil(anno + 1, 1, 1) } else { days_from_civil(anno, mese + 1, 1) };
    dopo - days_from_civil(anno, mese, 1)
}

/// Il giorno d'inizio rispetto al sabato del fine settimana: `venerdi` -1, `sabato` 0, `domenica` +1.
fn scarto_giorno(g: &str) -> Result<i64, String> {
    match g.trim().to_lowercase().replace('\u{ec}', "i").as_str() {
        "venerdi" | "friday" => Ok(-1),
        "sabato" | "saturday" => Ok(0),
        "domenica" | "sunday" => Ok(1),
        x => Err(format!("giorno_inizio «{x}» non ammesso (venerdi, sabato o domenica)")),
    }
}

/// `hh:mm` in minuti del giorno.
fn minuti_del_giorno(s: &str) -> Result<i64, String> {
    let (h, m) = s.trim().split_once(':').ok_or_else(|| format!("ora_inizio «{s}» non valida (hh:mm)"))?;
    let (h, m): (i64, i64) = (h.parse().map_err(|_| format!("ora_inizio «{s}» non valida"))?, m.parse().map_err(|_| format!("ora_inizio «{s}» non valida"))?);
    if !(0..24).contains(&h) || !(0..60).contains(&m) {
        return Err(format!("ora_inizio «{s}» fuori scala"));
    }
    Ok(h * 60 + m)
}

impl Edizione {
    /// Controlla la regola e le date scritte.
    pub fn controlla(&self) -> Result<(), String> {
        if !(1..=12).contains(&self.mese) {
            return Err("edizione.mese deve essere fra 1 e 12".into());
        }
        if !(1..=5).contains(&self.fine_settimana_pieno) {
            return Err("edizione.fine_settimana_pieno deve essere fra 1 e 5".into());
        }
        scarto_giorno(&self.giorno_inizio)?;
        minuti_del_giorno(&self.ora_inizio)?;
        if !(1..=24 * 14).contains(&self.durata_ore) {
            return Err("edizione.durata_ore fuori scala".into());
        }
        for d in &self.date {
            let (Some(a), Some(b)) = (parse_iso(&d.inizio), parse_iso(&d.fine)) else {
                return Err(format!("edizione.date {}: servono inizio e fine come «2026-12-12T00:00:00Z»", d.anno));
            };
            if b <= a {
                return Err(format!("edizione.date {}: la fine viene prima dell'inizio", d.anno));
            }
        }
        // la regola deve produrre una data in un anno qualsiasi (2026), cosi' un errore si vede subito
        self.calcola(2026).map(|_| ())
    }

    /// Inizio e fine dell'anno dalla regola (ignora le date scritte).
    pub fn calcola(&self, anno: i64) -> Result<(i64, i64), String> {
        let primo = days_from_civil(anno, self.mese, 1);
        let ultimo = primo + giorni_del_mese(anno, self.mese) - 1;
        // un fine settimana e' pieno se la domenica sta nello stesso mese del sabato
        let sabati: Vec<i64> = (primo..=ultimo).filter(|d| weekday(*d) == 6 && *d < ultimo).collect();
        let sabato = sabati.get(self.fine_settimana_pieno as usize - 1).copied().ok_or_else(|| format!("il mese {} del {anno} non ha {} fine settimana pieni", self.mese, self.fine_settimana_pieno))?;
        let inizio = (sabato + scarto_giorno(&self.giorno_inizio)?) * GIORNO_MS + minuti_del_giorno(&self.ora_inizio)? * 60_000;
        Ok((inizio, inizio + i64::from(self.durata_ore) * 3_600_000 - 1000))
    }

    /// Le date dell'edizione di un anno: quelle scritte nel file se ci sono, altrimenti calcolate dalla regola.
    pub fn date_dell_anno(&self, anno: i64) -> Result<DateEdizione, String> {
        if let Some(d) = self.date.iter().find(|d| d.anno == anno)
            && let (Some(a), Some(b)) = (parse_iso(&d.inizio), parse_iso(&d.fine))
        {
            return Ok(DateEdizione { anno, inizio_ms: a, fine_ms: b, confermata: d.confermata });
        }
        let (a, b) = self.calcola(anno)?;
        Ok(DateEdizione { anno, inizio_ms: a, fine_ms: b, confermata: false })
    }

    /// L'edizione da mostrare a `ora_ms`: quella in corso; se non ce n'e', quella finita da meno di `invio_log_giorni` (7 se non detto:
    /// il log si esporta ancora); altrimenti la prossima.
    pub fn edizione_per(&self, ora_ms: i64) -> Option<DateEdizione> {
        let anno = civil_from_days(ora_ms.div_euclid(GIORNO_MS)).0;
        let mut v: Vec<DateEdizione> = (anno - 1..=anno + 1).filter_map(|a| self.date_dell_anno(a).ok()).collect();
        v.sort_by_key(|d| d.inizio_ms);
        if let Some(d) = v.iter().find(|d| d.inizio_ms <= ora_ms && ora_ms <= d.fine_ms) {
            return Some(*d);
        }
        let dopo = i64::from(self.invio_log_giorni.unwrap_or(7)) * GIORNO_MS;
        if let Some(d) = v.iter().rev().find(|d| d.fine_ms < ora_ms).filter(|d| ora_ms - d.fine_ms < dopo) {
            return Some(*d);
        }
        v.into_iter().find(|d| d.inizio_ms > ora_ms)
    }
}

/// Un istante come `aaaammgghhmmss` (il formato di `m_dtStart` e `m_dtEnd`).
pub fn data_compatta(ms: i64) -> String {
    let (y, m, d) = civil_from_days(ms.div_euclid(GIORNO_MS));
    let s = ms.div_euclid(1000).rem_euclid(86_400);
    format!("{y:04}{m:02}{d:02}{:02}{:02}{:02}", s / 3600, s / 60 % 60, s % 60)
}

// ---------------------------------------------------------------- la cartella

/// Una cartella contest letta e controllata.
#[derive(Debug, Clone)]
pub struct ContestCartella {
    /// Il nome della cartella: e' l'identificativo del contest.
    pub id: String,
    pub percorso: PathBuf,
    pub origine: Origine,
    pub meta: Meta,
    pub edizione: Option<Edizione>,
    pub bande: Bande,
    pub modi: Modi,
    pub scambio: Scambio,
    pub doppi: Doppi,
    pub categorie: toml::Table,
    pub cabrillo: toml::Table,
    pub schede: Vec<Scheda>,
    /// Le regole del punteggio senza commenti ne' righe vuote (come la sezione `[Score]`).
    pub punteggio_regole: String,
    /// Lo script Rhai dei casi speciali (vuoto = nessuno).
    pub punteggio_rhai: String,
    /// Il regolamento (markdown), solo per mostrarlo.
    pub regolamento: String,
    /// Gli elenchi `.dom`: `dati/<file>` -> testo.
    pub domini: BTreeMap<String, String>,
    /// Le macro: `macros/<file>` -> testo.
    pub macros: BTreeMap<String, String>,
    /// Il testo di `prove.toml`, se c'e'.
    pub prove: Option<String>,
}

fn leggi_file(p: &Path) -> Result<String, String> {
    let nome = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let meta = std::fs::symlink_metadata(p).map_err(|e| format!("{nome}: {e}"))?;
    if !meta.is_file() {
        return Err(format!("{nome}: non e' un file"));
    }
    if meta.len() > MAX_BYTE {
        return Err(format!("{nome}: troppo grande"));
    }
    std::fs::read(p).map(|b| testo_da_byte(&b)).map_err(|e| format!("{nome}: {e}"))
}

fn leggi_facoltativo(p: &Path) -> Result<Option<String>, String> {
    if std::fs::symlink_metadata(p).is_err() {
        return Ok(None);
    }
    leggi_file(p).map(Some)
}

/// I file di una sottocartella con una certa estensione (`dati` + `.dom`), chiave `dati/<nome>`.
fn leggi_sottocartella(base: &Path, sotto: &str, estensione: &str, totale: &mut usize) -> Result<BTreeMap<String, String>, String> {
    let mut out = BTreeMap::new();
    let Ok(rd) = std::fs::read_dir(base.join(sotto)) else { return Ok(out) };
    let mut nomi: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
    nomi.sort();
    for p in nomi {
        let Some(nome) = p.file_name().map(|n| n.to_string_lossy().to_string()) else { continue };
        if !nome.to_lowercase().ends_with(estensione) {
            continue;
        }
        *totale += 1;
        if *totale > MAX_FILE {
            return Err("troppi file nella cartella".into());
        }
        out.insert(format!("{sotto}/{nome}"), leggi_file(&p)?);
    }
    Ok(out)
}

/// Le righe `chiave=valore` del punteggio senza commenti (`;`) ne' righe vuote.
fn regole_pulite(testo: &str) -> String {
    testo.lines().map(str::trim).filter(|r| !r.is_empty() && !r.starts_with(';')).map(|r| format!("{r}\n")).collect()
}

fn id_corto_ok(id: &str) -> bool {
    !id.is_empty() && id.len() <= 40 && id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-') && !id.starts_with('-') && !id.ends_with('-')
}

/// Un rimando a un file `.dom` dichiarato (`dati/x.dom`): percorso della cartella, estensione giusta e file presente.
fn rimando_dom_ok(rif: &str, domini: &BTreeMap<String, String>) -> Result<(), String> {
    let r = rif.trim().replace('\\', "/");
    if !percorso_relativo_ok(&r) || !r.to_lowercase().starts_with("dati/") || !r.to_lowercase().ends_with(".dom") {
        return Err(format!("rimando non ammesso: {rif} (serve dati/<nome>.dom)"));
    }
    if !domini.keys().any(|k| k.eq_ignore_ascii_case(&r)) {
        return Err(format!("file mancante: {rif}"));
    }
    Ok(())
}

/// Legge e controlla la cartella di un contest.
pub fn carica(percorso: &Path, origine: Origine) -> Result<ContestCartella, String> {
    let id = percorso.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    if id.trim().is_empty() || id.contains(['/', '\\', ':']) || id.contains("..") {
        return Err("nome della cartella non valido".into());
    }
    let toml_testo = leggi_file(&percorso.join("contest.toml")).map_err(|e| format!("contest.toml: {e}"))?;
    let t: FileToml = toml::from_str(&toml_testo).map_err(|e| format!("contest.toml: {e}"))?;
    if t.contest.nome.trim().is_empty() {
        return Err("contest.toml: [contest] nome mancante".into());
    }
    if !id_corto_ok(&t.contest.id) {
        return Err("contest.toml: [contest] id mancante o non valido (minuscole, cifre e trattini)".into());
    }
    if t.contest.qtc > 2 {
        return Err("contest.toml: [contest] qtc deve essere 0, 1 o 2".into());
    }
    if let Some(e) = &t.edizione {
        e.controlla().map_err(|e| format!("contest.toml: {e}"))?;
    }
    let regole = leggi_file(&percorso.join("punteggio.regole")).map_err(|e| format!("punteggio.regole: {e}"))?;
    let punteggio_regole = regole_pulite(&regole);
    crate::regole_punti::RegoleScore::da_testo(&punteggio_regole).map_err(|e| format!("punteggio.regole: {e}"))?;
    let punteggio_rhai = leggi_facoltativo(&percorso.join("punteggio.rhai")).map_err(|e| format!("punteggio.rhai: {e}"))?.unwrap_or_default();
    if !punteggio_rhai.trim().is_empty() {
        crate::script::ScriptContest::compila(&punteggio_rhai).map_err(|e| format!("punteggio.rhai: {e}"))?;
    }
    let regolamento = leggi_facoltativo(&percorso.join("regolamento.md")).map_err(|e| format!("regolamento.md: {e}"))?.unwrap_or_default();
    let prove = leggi_facoltativo(&percorso.join("prove.toml")).map_err(|e| format!("prove.toml: {e}"))?;
    if let Some(p) = &prove {
        toml::from_str::<toml::Table>(p).map_err(|e| format!("prove.toml: {e}"))?;
    }
    let mut totale = 0;
    let domini = leggi_sottocartella(percorso, "dati", ".dom", &mut totale)?;
    let macros = leggi_sottocartella(percorso, "macros", ".mc", &mut totale)?;
    for c in t.scambio.ricevuto.iter().chain(&t.scambio.inviato) {
        if c.nome.trim().is_empty() {
            return Err("contest.toml: un campo dello scambio non ha il nome".into());
        }
        if !c.elenco.trim().is_empty() {
            rimando_dom_ok(&c.elenco, &domini).map_err(|e| format!("contest.toml: scambio {}: {e}", c.nome))?;
        }
    }
    for (i, s) in t.scheda.iter().enumerate() {
        if s.tipo.trim().is_empty() {
            return Err(format!("contest.toml: la scheda {} non ha il tipo", i + 1));
        }
        if let Some(d) = s.parametri.get("dominio").and_then(toml::Value::as_str) {
            rimando_dom_ok(d, &domini).map_err(|e| format!("contest.toml: scheda {}: {e}", s.tipo))?;
        }
    }
    if !t.doppi.per.is_empty() && !["modo", "banda", "banda+modo", "sempre"].contains(&t.doppi.per.as_str()) {
        return Err(format!("contest.toml: [doppi] per «{}» non ammesso (modo, banda, banda+modo, sempre)", t.doppi.per));
    }
    Ok(ContestCartella {
        id,
        percorso: percorso.to_path_buf(),
        origine,
        meta: t.contest,
        edizione: t.edizione,
        bande: t.bande,
        modi: t.modi,
        scambio: t.scambio,
        doppi: t.doppi,
        categorie: t.categorie,
        cabrillo: t.cabrillo,
        schede: t.scheda,
        punteggio_regole,
        punteggio_rhai,
        regolamento,
        domini,
        macros,
        prove,
    })
}

/// La categoria del motore di un modo del `contest.toml`: `cw`, `phone` o `digital`.
fn categoria_modo(m: &str) -> &'static str {
    match m.trim().to_ascii_lowercase().as_str() {
        "cw" => "cw",
        "ssb" | "usb" | "lsb" | "am" | "fm" | "phone" | "fonia" => "phone",
        _ => "digital",
    }
}

impl ContestCartella {
    /// Il file di macro che parte con il contest: quello col nome della cartella, altrimenti il primo.
    pub fn macro_principale(&self) -> Option<(&str, &str)> {
        let per_nome = self.macros.iter().find(|(k, _)| k.rsplit('/').next().and_then(|f| f.rsplit_once('.')).is_some_and(|(stem, _)| stem.eq_ignore_ascii_case(&self.id)));
        per_nome.or_else(|| self.macros.iter().next()).map(|(k, v)| (k.as_str(), v.as_str()))
    }

    /// L'elenco dei domini del contest: quello del primo campo dello scambio ricevuto che ne ha uno, poi quello della prima scheda
    /// che ne nomina uno, poi l'unico presente.
    pub fn dominio_principale(&self) -> Option<(&str, &str)> {
        let da_scambio = self.scambio.ricevuto.iter().map(|c| c.elenco.as_str());
        let da_schede = self.schede.iter().filter_map(|s| s.parametri.get("dominio").and_then(toml::Value::as_str));
        let nome = da_scambio.chain(da_schede).map(|r| r.trim().replace('\\', "/")).find(|r| !r.is_empty());
        if let Some(n) = nome
            && let Some((k, v)) = self.domini.iter().find(|(k, _)| k.eq_ignore_ascii_case(&n))
        {
            return Some((k.as_str(), v.as_str()));
        }
        if self.domini.len() == 1 { self.domini.iter().next().map(|(k, v)| (k.as_str(), v.as_str())) } else { None }
    }

    /// Le date dell'edizione da mostrare a `ora_ms` (`None` senza `[edizione]`).
    pub fn edizione_per(&self, ora_ms: i64) -> Option<DateEdizione> {
        self.edizione.as_ref()?.edizione_per(ora_ms)
    }

    /// Lo stesso contest come lo e' un `.contest`: il motore del punteggio, la riga di inserimento, il Cabrillo e le API lo usano senza
    /// sapere che viene da una cartella. `ora_ms` sceglie l'edizione per `start` e `end`.
    pub fn in_contesto(&self, ora_ms: i64) -> ContestTesto {
        let mut c = ContestTesto { id: self.id.clone(), name: self.meta.nome.clone(), notes: self.meta.note.clone(), score: self.punteggio_regole.clone(), script: self.punteggio_rhai.clone(), cabrillo_contest: self.meta.cabrillo_contest.trim().to_string(), ..ContestTesto::default() };
        if let Some(d) = self.edizione_per(ora_ms) {
            c.start = data_compatta(d.inizio_ms);
            c.end = data_compatta(d.fine_ms);
        }
        if let Some(e) = &self.edizione {
            c.work_time = e.ore_massime.map_or(0, |o| i32::try_from(o).unwrap_or(0));
        }
        // modi ammessi: nessuno scritto = tutti; digitale spento se ci sono modi dichiarati (i vecchi file lo accendevano per base)
        c.valid_cw = false;
        c.valid_phone = false;
        c.valid_digital = false;
        for m in &self.modi.ammessi {
            match categoria_modo(m) {
                "cw" => c.valid_cw = true,
                "phone" => c.valid_phone = true,
                _ => c.valid_digital = true,
            }
        }
        // doppi: su cosa si fa la chiave (senza `[doppi]`: banda e modo)
        let (senza_banda, senza_modo) = match self.doppi.per.as_str() {
            "modo" => (true, false),
            "banda" => (false, true),
            "sempre" => (true, true),
            _ => (false, false),
        };
        c.dupe_no_band = senza_banda;
        c.dupe_no_mode = senza_modo;
        // scambio inviato: un numero che avanza a ogni QSO
        c.auto_inc = self.scambio.inviato.iter().any(|x| x.progressivo);
        // numero ricevuto: un progressivo (`numero`) o la zona CQ (`zona_cq`, proposta dal nominativo come nei vecchi file del CQ WW)
        let numero_ricevuto = self.scambio.ricevuto.iter().any(|x| x.tipo == "numero");
        let zona_cq = self.scambio.ricevuto.iter().any(|x| x.tipo == "zona_cq");
        c.grab_number = numero_ricevuto || zona_cq;
        c.grab_number_type = if zona_cq { 2 } else if numero_ricevuto { 0 } else { -1 };
        c.suggest_cq_zone = zona_cq;
        // zona ITU (`zona_itu`): solo suggerita sotto il nominativo, non messa nello scambio (il motore non guarda `quando`: nell'EUDX
        // le stazioni UE mandano la regione e una zona proposta d'ufficio prenderebbe il suo posto)
        c.suggest_itu_zone = self.scambio.ricevuto.iter().any(|x| x.tipo == "zona_itu");
        c.log_bar_mode = 1;
        c.qtc = i32::from(self.meta.qtc.min(2));
        if let Some((nome, _)) = self.macro_principale() {
            c.macros = nome.to_string();
        }
        if let Some((nome, _)) = self.dominio_principale() {
            c.domain_file = nome.replacen("dati/", "dom/", 1);
            c.grab_state = true;
            c.suggest_domain = true;
        }
        c
    }

    /// Le macro del contest (`macros/*.mc`, il file principale).
    pub fn macro_del_contest(&self) -> Vec<MacroContest> {
        self.macro_principale().map(|(_, t)| leggi_macro(t)).unwrap_or_default()
    }

    /// L'elenco dei domini del contest.
    pub fn dominio_del_contest(&self) -> Option<Dominio> {
        self.dominio_principale().map(|(_, t)| leggi_dominio(t))
    }

    /// La parte della dichiarazione che serve all'app per sapere quali voci delle impostazioni valgono per questo contest: bande, modi
    /// (con i limiti), categorie, doppi, l'edizione di `ora_ms` (calcolata dalla regola o dal calendario) e l'intestazione del Cabrillo.
    pub fn dichiarazione(&self, ora_ms: i64) -> serde_json::Value {
        use serde_json::json;
        let edizione = self.edizione.as_ref().map(|e| {
            let date = e.edizione_per(ora_ms).map(|d| json!({ "anno": d.anno, "inizio": data_compatta(d.inizio_ms), "fine": data_compatta(d.fine_ms), "confermata": d.confermata }));
            json!({ "mese": e.mese, "durata_ore": e.durata_ore, "ore_massime": e.ore_massime, "pausa_minima_min": e.pausa_minima_min, "invio_log_giorni": e.invio_log_giorni, "date": date })
        });
        json!({
            "bande": self.bande.ammesse,
            "modi": { "ammessi": self.modi.ammessi, "limite": self.modi.limite },
            "categorie": self.categorie,
            "doppi": self.doppi,
            "edizione": edizione,
            "cabrillo": self.cabrillo,
        })
    }

    /// Il messaggio quando si prova a modificare o cancellare il contest dall'elenco.
    pub fn rifiuto_modifica(&self) -> &'static str {
        match self.origine {
            Origine::Inclusa => "contest incluso: copialo nella tua cartella per modificarlo",
            Origine::Utente => "contest in cartella: si modifica nei file della sua cartella",
        }
    }
}

// ---------------------------------------------------------------- l'elenco

/// Le cartelle trovate sul disco e gli errori delle cartelle saltate.
#[derive(Debug, Default)]
pub struct Scansione {
    pub cartelle: Vec<Arc<ContestCartella>>,
    pub errori: Vec<String>,
}

/// Una voce dell'elenco: il contest come `.contest` e, se viene da una cartella, la cartella.
#[derive(Debug, Clone)]
pub struct Voce {
    pub contest: ContestTesto,
    pub cartella: Option<Arc<ContestCartella>>,
    /// Le modifiche personali dell'utente gia' applicate a `contest` (solo contest in cartella che ne hanno).
    pub sovrascritta: Option<Sovrascrittura>,
}

impl Voce {
    /// `cartella` o `file`.
    pub fn origine(&self) -> &'static str {
        if self.cartella.is_some() { "cartella" } else { "file" }
    }

    /// I campi in piu' delle API: `origine`, `schede`, `scambio` (e `regolamento` se richiesto: testo per le cartelle, `null` per i file).
    pub fn campi_api(&self, con_regolamento: bool) -> serde_json::Map<String, serde_json::Value> {
        use serde_json::json;
        let mut m = serde_json::Map::new();
        m.insert("origine".into(), json!(self.origine()));
        m.insert("sovrascritta".into(), json!(self.sovrascritta));
        m.insert("schede".into(), json!(self.cartella.as_ref().map(|c| c.schede.clone()).unwrap_or_default()));
        m.insert("scambio".into(), json!(self.cartella.as_ref().map(|c| c.scambio.clone()).unwrap_or_default()));
        if con_regolamento {
            m.insert("regolamento".into(), self.cartella.as_ref().map_or(serde_json::Value::Null, |c| json!(c.regolamento)));
            m.insert("dichiarazione".into(), self.cartella.as_ref().map_or(serde_json::Value::Null, |c| c.dichiarazione(ora_ms())));
        }
        m
    }
}

/// Le modifiche personali a un contest in cartella: solo dati dell'utente, mai regole. Stanno in `<dati>/contest/sovrascritture/<id>.json`
/// e si applicano sopra i valori della cartella (che resta com'e', aggiornabile con le versioni). Un campo assente vale quello della cartella.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Sovrascrittura {
    /// Inizio e fine dell'edizione, `aaaammgghhmmss` UTC (vanno in coppia).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<String>,
    /// Ore di lavoro (0-48, 0 = nessun limite).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work_time: Option<i32>,
    /// Il numero di partenza e l'incremento automatico.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nrs: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_inc: Option<bool>,
    /// Il file di macro scelto (`macros/<nome>.mc`; vuoto = nessuna macro).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub macros: Option<String>,
}

impl Sovrascrittura {
    /// Nessun campo: non c'e' niente da tenere.
    pub fn vuota(&self) -> bool {
        *self == Sovrascrittura::default()
    }

    /// Mette le modifiche sopra i valori della cartella.
    pub fn applica(&self, c: &mut ContestTesto) {
        if let Some(v) = &self.start {
            c.start = v.clone();
        }
        if let Some(v) = &self.end {
            c.end = v.clone();
        }
        if let Some(v) = self.work_time {
            c.work_time = v;
        }
        if let Some(v) = &self.nrs {
            c.nrs = v.clone();
        }
        if let Some(v) = self.auto_inc {
            c.auto_inc = v;
        }
        if let Some(v) = &self.macros {
            c.macros = v.clone();
        }
    }
}

/// Un contest con i file (percorso nel pacchetto e contenuto) che lo accompagnano.
pub(crate) type ContestEFile = (ContestTesto, Vec<(String, Vec<u8>)>);

/// Un contest cercato per identificativo: un file `.contest` o una cartella.
pub enum Risolto {
    File(Box<ContestTesto>),
    Cartella(Arc<ContestCartella>),
}

#[derive(Debug)]
pub(crate) struct CacheCartelle {
    quando: Instant,
    scansione: Arc<Scansione>,
}

fn normalizza(s: &str) -> String {
    s.trim().to_lowercase()
}

fn ora_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as i64)
}

impl ElencoContest {
    /// La cartella dei contest inclusi nell'installatore (sola lettura): la imposta chi apre il servizio.
    pub fn imposta_cartelle_incluse(&self, da: Option<PathBuf>) {
        if let Ok(mut g) = self.incluse.lock() {
            *g = da;
        }
        self.dimentica_cartelle();
    }

    /// Dimentica la scansione: la prossima richiesta rilegge il disco.
    pub fn dimentica_cartelle(&self) {
        if let Ok(mut g) = self.cache.lock() {
            *g = None;
        }
    }

    fn scansiona(&self) -> Scansione {
        let incluse = self.incluse.lock().ok().and_then(|g| g.clone());
        let mut per_nome: BTreeMap<String, Arc<ContestCartella>> = BTreeMap::new();
        let mut errori = Vec::new();
        // prima le incluse, poi quelle dell'utente: a pari nome vince l'utente
        for (radice, origine) in incluse.into_iter().map(|p| (p, Origine::Inclusa)).chain(std::iter::once((self.dir().to_path_buf(), Origine::Utente))) {
            let Ok(rd) = std::fs::read_dir(&radice) else { continue };
            let mut dirs: Vec<PathBuf> = rd.flatten().map(|e| e.path()).filter(|p| std::fs::symlink_metadata(p).is_ok_and(|m| m.is_dir())).collect();
            dirs.sort();
            for p in dirs {
                let nome = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                if RISERVATE.contains(&nome.to_lowercase().as_str()) || !p.join("contest.toml").is_file() {
                    continue;
                }
                match carica(&p, origine) {
                    Ok(c) => {
                        per_nome.insert(normalizza(&c.id), Arc::new(c));
                    }
                    Err(e) => errori.push(format!("{}: {e}", p.display())),
                }
            }
        }
        // due cartelle con lo stesso nome di contest: resta la prima, l'altra si segnala
        let mut visti: BTreeMap<String, String> = BTreeMap::new();
        let mut cartelle = Vec::new();
        for c in per_nome.into_values() {
            if let Some(altra) = visti.get(&normalizza(&c.meta.nome)) {
                errori.push(format!("{}: stesso nome di contest della cartella {altra}", c.percorso.display()));
                continue;
            }
            visti.insert(normalizza(&c.meta.nome), c.id.clone());
            cartelle.push(c);
        }
        Scansione { cartelle, errori }
    }

    /// Le cartelle valide (con una breve memoria: la riga di inserimento chiede il contest a ogni tasto).
    pub fn scansione(&self) -> Arc<Scansione> {
        if let Ok(g) = self.cache.lock()
            && let Some(c) = g.as_ref().filter(|c| c.quando.elapsed() < VALIDITA_CACHE)
        {
            return Arc::clone(&c.scansione);
        }
        let nuova = Arc::new(self.scansiona());
        if let Ok(mut g) = self.cache.lock() {
            let prima: Vec<String> = g.as_ref().map(|c| c.scansione.errori.clone()).unwrap_or_default();
            for e in nuova.errori.iter().filter(|e| !prima.contains(e)) {
                tracing::warn!("contest in cartella saltato: {e}");
            }
            *g = Some(CacheCartelle { quando: Instant::now(), scansione: Arc::clone(&nuova) });
        }
        nuova
    }

    /// La cartella con quell'identificativo (il nome della cartella, senza badare alle maiuscole).
    pub fn cartella(&self, id: &str) -> Option<Arc<ContestCartella>> {
        let id = normalizza(id);
        self.scansione().cartelle.iter().find(|c| normalizza(&c.id) == id).cloned()
    }

    /// Il `.contest` con lo stesso identificativo nella cartella dei predefiniti (quello distribuito con Aurora), se c'e'.
    fn predefinito_di(&self, c: &ContestTesto) -> Option<ContestTesto> {
        let da = self.predefiniti.lock().ok().and_then(|g| g.clone())?;
        let b = std::fs::read(da.join(format!("{}.{}", c.id, crate::elenco::ESTENSIONE))).ok()?;
        Some(ContestTesto::da_testo(&c.id, &testo_da_byte(&b)))
    }

    /// Il `.contest` e' la copia non modificata di quello distribuito con Aurora (cartella dei predefiniti)?
    ///
    /// Il segno `beta` e il numero da mandare non contano: le versioni nuove li cambiano nei file distribuiti (il segno Beta, i valori fissi
    /// dell'altro operatore tolti dallo scambio) e le copie fatte prima non devono per questo diventare «personalizzate», altrimenti la cartella
    /// verificata non le sostituirebbe piu'.
    fn e_predefinito_intatto(&self, c: &ContestTesto) -> bool {
        let Some(distribuito) = self.predefinito_di(c) else { return false };
        let senza_segni = |x: &ContestTesto| ContestTesto { beta: false, nrs: String::new(), ..x.clone() };
        senza_segni(&distribuito) == senza_segni(c)
    }

    /// La cartella che sostituisce questo `.contest` (stesso nome di contest, senza badare alle maiuscole). La regola:
    /// - una cartella nei dati dell'utente vale sempre piu' del `.contest`: l'ha messa lui;
    /// - una cartella inclusa vale al posto del `.contest` solo se questo e' la copia intatta di quello dei predefiniti;
    ///   se l'utente l'ha personalizzato (o non si puo' dire), il suo `.contest` resta.
    ///
    /// Il `.contest` sostituito resta nel suo file ma non compare piu' nell'elenco.
    pub fn sostituito_da(&self, c: &ContestTesto) -> Option<Arc<ContestCartella>> {
        let nome = normalizza(&c.name);
        let f = self.scansione().cartelle.iter().find(|f| normalizza(&f.meta.nome) == nome || f.meta.alias.iter().any(|a| normalizza(a) == nome)).cloned()?;
        match f.origine {
            Origine::Utente => Some(f),
            Origine::Inclusa => self.e_predefinito_intatto(c).then_some(f),
        }
    }

    /// Cerca un contest per identificativo: il file `.contest` (se non e' sostituito da una cartella), altrimenti la cartella.
    pub fn risolvi(&self, id: &str) -> Option<Risolto> {
        if let Ok(p) = self.file(id)
            && let Ok(b) = std::fs::read(&p)
        {
            let c = ContestTesto::da_testo(id, &testo_da_byte(&b));
            return Some(match self.sostituito_da(&c) {
                Some(f) => Risolto::Cartella(f),
                None => Risolto::File(Box::new(c)),
            });
        }
        if id.trim().is_empty() || id.contains(['/', '\\', ':']) || id.contains("..") {
            return None;
        }
        self.cartella(id).map(Risolto::Cartella)
    }

    /// Il file delle modifiche personali di un contest in cartella.
    fn file_sovrascrittura(&self, id: &str) -> Result<PathBuf, String> {
        if id.trim().is_empty() || id.contains(['/', '\\', ':']) || id.contains("..") {
            return Err("identificativo non valido".into());
        }
        Ok(self.dir().join(CARTELLA_SOVRASCRITTURE).join(format!("{id}.json")))
    }

    /// Le modifiche personali salvate per un contest in cartella (`None` se non ce ne sono o il file non si legge).
    pub fn sovrascrittura(&self, id: &str) -> Option<Sovrascrittura> {
        let b = std::fs::read(self.file_sovrascrittura(id).ok()?).ok()?;
        serde_json::from_slice::<Sovrascrittura>(&b).ok().filter(|s| !s.vuota())
    }

    /// La voce di una cartella con le modifiche personali applicate (le date calcolate dalla regola valgono solo senza le sue).
    fn voce_di_cartella(&self, f: Arc<ContestCartella>, ora: i64) -> Voce {
        let mut contest = f.in_contesto(ora);
        let sovrascritta = self.sovrascrittura(&f.id);
        if let Some(s) = &sovrascritta {
            s.applica(&mut contest);
        }
        Voce { contest, cartella: Some(f), sovrascritta }
    }

    /// Salva le modifiche personali di un contest in cartella: dal corpo si leggono SOLO `start`, `end`, `work_time`, `nrs`, `auto_inc` e
    /// `macros`; il resto (regole, nome, bande, modi...) si ignora senza errore. Si tengono solo i valori diversi da quelli della cartella,
    /// cosi' un aggiornamento della cartella arriva ancora. Non scrive mai nella cartella. Restituisce la voce aggiornata.
    pub fn salva_sovrascrittura(&self, id: &str, corpo: &serde_json::Value) -> Result<Voce, String> {
        let Some(Risolto::Cartella(f)) = self.risolvi(id) else { return Err("non e' un contest a cartella".into()) };
        let chiesta: Sovrascrittura = serde_json::from_value(corpo.clone()).map_err(|e| format!("valori non validi: {e}"))?;
        let base = f.in_contesto(ora_ms());
        let mut s = Sovrascrittura::default();
        // date: in coppia; una data vuota vale «come la cartella»
        let inizio = chiesta.start.as_deref().map_or("", str::trim);
        let fine = chiesta.end.as_deref().map_or("", str::trim);
        if !crate::elenco::data_valida(inizio) || !crate::elenco::data_valida(fine) {
            return Err("data non valida (aaaammgghhmmss)".into());
        }
        if let (Some(a), Some(b)) = (crate::elenco::data_ms(inizio), crate::elenco::data_ms(fine))
            && b < a
        {
            return Err("la fine viene prima dell'inizio".into());
        }
        if !inizio.is_empty() && !fine.is_empty() && (inizio != base.start || fine != base.end) {
            s.start = Some(inizio.to_string());
            s.end = Some(fine.to_string());
        }
        if let Some(h) = chiesta.work_time {
            if !(0..=48).contains(&h) {
                return Err("ore di lavoro: da 0 a 48".into());
            }
            if h != base.work_time {
                s.work_time = Some(h);
            }
        }
        if let Some(n) = chiesta.nrs.as_deref().map(str::trim) {
            if n.chars().count() > 64 || n.chars().any(char::is_control) {
                return Err("numero di partenza non valido".into());
            }
            if n != base.nrs {
                s.nrs = Some(n.to_string());
            }
        }
        if let Some(a) = chiesta.auto_inc
            && a != base.auto_inc
        {
            s.auto_inc = Some(a);
        }
        if let Some(m) = chiesta.macros.as_deref().map(|m| m.trim().replace('\\', "/")) {
            if !m.is_empty() {
                let nota = f.macros.keys().any(|k| k.eq_ignore_ascii_case(&m)) || (m.starts_with("macros/") && percorso_relativo_ok(&m) && self.dir().join(&m).is_file());
                if !nota {
                    return Err(format!("file di macro sconosciuto: {m}"));
                }
            }
            if m != base.macros {
                s.macros = Some(m);
            }
        }
        let file = self.file_sovrascrittura(&f.id)?;
        if s.vuota() {
            let _ = std::fs::remove_file(&file);
        } else {
            if let Some(d) = file.parent() {
                std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
            }
            let testo = serde_json::to_string_pretty(&s).map_err(|e| e.to_string())?;
            std::fs::write(&file, testo).map_err(|e| e.to_string())?;
        }
        self.voce(&f.id).ok_or_else(|| "contest sconosciuto".to_string())
    }

    /// «Ripristina i valori del contest»: toglie le modifiche personali. Restituisce la voce com'e' nella cartella.
    pub fn ripristina_sovrascrittura(&self, id: &str) -> Result<Voce, String> {
        let Some(Risolto::Cartella(f)) = self.risolvi(id) else { return Err("non e' un contest a cartella".into()) };
        let file = self.file_sovrascrittura(&f.id)?;
        match std::fs::remove_file(&file) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.to_string()),
        }
        self.voce(&f.id).ok_or_else(|| "contest sconosciuto".to_string())
    }

    /// Una voce dell'elenco per identificativo (anche quello di un `.contest` sostituito da una cartella).
    pub fn voce(&self, id: &str) -> Option<Voce> {
        Some(match self.risolvi(id)? {
            Risolto::File(contest) => Voce { contest: *contest, cartella: None, sovrascritta: None },
            Risolto::Cartella(f) => self.voce_di_cartella(f, ora_ms()),
        })
    }

    /// Tutti i contest, in ordine di nome: i `.contest` non sostituiti e le cartelle.
    pub fn voci(&self) -> Vec<Voce> {
        let ora = ora_ms();
        // un `.contest` copiato prima del segno Beta lo riceve da quello distribuito: lo era anche allora (non e' verificato sul regolamento)
        let mut out: Vec<Voce> = self
            .elenco_file()
            .into_iter()
            .filter(|c| self.sostituito_da(c).is_none())
            .map(|mut contest| {
                contest.beta = contest.beta || self.predefinito_di(&contest).is_some_and(|d| d.beta);
                Voce { contest, cartella: None, sovrascritta: None }
            })
            .collect();
        // un `.contest` personalizzato vale piu' della cartella inclusa con lo stesso nome (non di quella dell'utente)
        let personalizzati: Vec<String> = out.iter().map(|v| normalizza(&v.contest.name)).collect();
        // (anche per alias: il `.contest` personalizzato con il vecchio nome della cartella vale piu' della cartella inclusa, non due righe)
        let visibili = self
            .scansione()
            .cartelle
            .iter()
            .filter(|f| f.origine == Origine::Utente || !(personalizzati.contains(&normalizza(&f.meta.nome)) || f.meta.alias.iter().any(|a| personalizzati.contains(&normalizza(a)))))
            .cloned()
            .collect::<Vec<_>>();
        out.extend(visibili.into_iter().map(|f| self.voce_di_cartella(f, ora)));
        out.sort_by_key(|v| v.contest.name.to_lowercase());
        out
    }

    /// Il contest di una cartella come pacchetto `.zip` (`<id>.contest`, `macros/`, `dom/`): il file e i testi da mettere nel pacchetto.
    pub(crate) fn file_del_pacchetto(&self, id: &str) -> Option<ContestEFile> {
        let Some(Risolto::Cartella(f)) = self.risolvi(id) else { return None };
        let c = f.in_contesto(ora_ms());
        let mut file = Vec::new();
        if let Some((nome, t)) = f.macro_principale() {
            file.push((nome.to_string(), t.as_bytes().to_vec()));
        }
        if let Some((nome, t)) = f.dominio_principale() {
            file.push((nome.replacen("dati/", "dom/", 1), t.as_bytes().to_vec()));
        }
        Some((c, file))
    }

    /// «Copia nella mia cartella»: copia la cartella di un contest incluso in `<dati>/contest/<nome>/`, dove si puo' modificare e dove
    /// sostituisce quella inclusa. Restituisce l'identificativo.
    pub fn copia_nella_mia_cartella(&self, id: &str) -> Result<String, String> {
        let f = self.cartella(id).ok_or_else(|| "contest sconosciuto".to_string())?;
        if f.origine == Origine::Utente {
            return Err("il contest e' gia' nella tua cartella".into());
        }
        let dst = self.dir().join(&f.id);
        if std::fs::symlink_metadata(&dst).is_ok() {
            return Err("nella tua cartella c'e' gia' un contest con questo nome".into());
        }
        let fatto = copia_cartella(&f.percorso, &dst);
        if let Err(e) = &fatto {
            let _ = std::fs::remove_dir_all(&dst);
            return Err(e.clone());
        }
        self.dimentica_cartelle();
        // la copia deve essere valida (altrimenti non sostituirebbe niente): se non lo e', si toglie
        if let Err(e) = carica(&dst, Origine::Utente) {
            let _ = std::fs::remove_dir_all(&dst);
            return Err(format!("copia non valida: {e}"));
        }
        Ok(f.id.clone())
    }
}

/// Copia una cartella (file e sottocartelle, niente collegamenti simbolici) con gli stessi limiti della lettura.
fn copia_cartella(src: &Path, dst: &Path) -> Result<(), String> {
    fn passo(src: &Path, dst: &Path, n: &mut usize, livello: u32) -> Result<(), String> {
        if livello > 3 {
            return Err("cartella troppo profonda".into());
        }
        std::fs::create_dir_all(dst).map_err(|e| e.to_string())?;
        for e in std::fs::read_dir(src).map_err(|e| e.to_string())?.flatten() {
            let (p, nome) = (e.path(), e.file_name());
            let meta = std::fs::symlink_metadata(&p).map_err(|e| e.to_string())?;
            if meta.file_type().is_symlink() {
                continue;
            }
            if meta.is_dir() {
                passo(&p, &dst.join(&nome), n, livello + 1)?;
            } else if meta.is_file() {
                *n += 1;
                if *n > MAX_FILE {
                    return Err("troppi file nella cartella".into());
                }
                if meta.len() > MAX_BYTE {
                    return Err(format!("{}: troppo grande", nome.to_string_lossy()));
                }
                std::fs::copy(&p, dst.join(&nome)).map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }
    passo(src, dst, &mut 0, 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ARRL: &str = "mese = 12\nfine_settimana_pieno = 2\ngiorno_inizio = \"sabato\"\nora_inizio = \"00:00\"\ndurata_ore = 48\n";

    fn edizione() -> Edizione {
        let e: Edizione = toml::from_str(ARRL).unwrap_or_default();
        e
    }

    fn utc(y: i64, m: u32, d: u32, h: i64, mi: i64) -> i64 {
        days_from_civil(y, m, d) * GIORNO_MS + (h * 60 + mi) * 60_000
    }

    #[test]
    fn date_dalla_regola() {
        let e = edizione();
        assert_eq!(e.controlla(), Ok(()));
        let d = e.date_dell_anno(2026);
        assert_eq!(d.as_ref().map(|d| (data_compatta(d.inizio_ms), data_compatta(d.fine_ms))), Ok(("20261212000000".to_string(), "20261213235959".to_string())));
        let d = e.date_dell_anno(2027);
        assert_eq!(d.as_ref().map(|d| (data_compatta(d.inizio_ms), data_compatta(d.fine_ms))), Ok(("20271211000000".to_string(), "20271212235959".to_string())));
        // un mese che comincia di sabato (ottobre 2022): il primo fine settimana pieno e' il 1-2
        let mut m = e.clone();
        m.mese = 10;
        m.fine_settimana_pieno = 1;
        assert_eq!(data_compatta(m.calcola(2022).map(|x| x.0).unwrap_or(0)), "20221001000000");
        // il sabato che e' l'ultimo giorno del mese non da' un fine settimana pieno (30 aprile 2022)
        m.mese = 4;
        m.fine_settimana_pieno = 5;
        assert!(m.calcola(2022).is_err());
        // inizio di venerdi' e di domenica
        m.mese = 12;
        m.fine_settimana_pieno = 2;
        m.giorno_inizio = "venerdi".into();
        assert_eq!(data_compatta(m.calcola(2026).map(|x| x.0).unwrap_or(0)), "20261211000000");
        m.giorno_inizio = "domenica".into();
        assert_eq!(data_compatta(m.calcola(2026).map(|x| x.0).unwrap_or(0)), "20261213000000");
        m.giorno_inizio = "martedi".into();
        assert!(m.controlla().is_err());
    }

    #[test]
    fn date_scritte_valgono_piu_del_calcolo() {
        let mut e = edizione();
        e.date.push(DataEdizione { anno: 2026, inizio: "2026-12-19T00:00:00Z".into(), fine: "2026-12-20T23:59:59Z".into(), confermata: true, ..DataEdizione::default() });
        assert_eq!(e.controlla(), Ok(()));
        let d = e.date_dell_anno(2026).unwrap_or(DateEdizione { anno: 0, inizio_ms: 0, fine_ms: 0, confermata: false });
        assert_eq!(data_compatta(d.inizio_ms), "20261219000000");
        assert!(d.confermata);
        e.date[0].fine = "2026-12-18T00:00:00Z".into();
        assert!(e.controlla().is_err());
    }

    #[test]
    fn edizione_in_corso_prossima_o_appena_finita() {
        let e = edizione();
        let anno = |d: Option<DateEdizione>| d.map(|d| (d.anno, data_compatta(d.inizio_ms)));
        // 17 novembre 2026: la prossima e' quella del 2026
        assert_eq!(anno(e.edizione_per(utc(2026, 11, 17, 0, 0))), Some((2026, "20261212000000".to_string())));
        // in corso
        assert_eq!(anno(e.edizione_per(utc(2026, 12, 12, 12, 0))), Some((2026, "20261212000000".to_string())));
        // finita da tre giorni: si puo' ancora esportare il log
        assert_eq!(anno(e.edizione_per(utc(2026, 12, 16, 0, 0))), Some((2026, "20261212000000".to_string())));
        // finita da piu' di sette giorni: la prossima
        assert_eq!(anno(e.edizione_per(utc(2026, 12, 25, 0, 0))), Some((2027, "20271211000000".to_string())));
        // l'invio del log dura quanto dice il file
        let mut corto = e.clone();
        corto.invio_log_giorni = Some(1);
        assert_eq!(anno(corto.edizione_per(utc(2026, 12, 16, 0, 0))), Some((2027, "20271211000000".to_string())));
    }

    fn cartella_di_prova(nome: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("aurora-cartelle-{nome}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    fn scrivi(base: &Path, rel: &str, testo: &str) {
        let p = base.join(rel);
        if let Some(d) = p.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        let _ = std::fs::write(p, testo);
    }

    const TOML_MINIMO: &str = "[contest]\nid = \"prova\"\nnome = \"Contest di prova\"\nnote = \"CW\"\n\n[edizione]\nmese = 3\nfine_settimana_pieno = 1\ngiorno_inizio = \"sabato\"\nora_inizio = \"12:00\"\ndurata_ore = 24\nore_massime = 12\n\n[modi]\nammessi = [\"cw\"]\n\n[doppi]\nper = \"banda+modo\"\n\n[[scambio.ricevuto]]\nnome = \"numero\"\ntipo = \"numero\"\nelenco = \"dati/x.dom\"\n\n[[scambio.inviato]]\nnome = \"numero\"\ntipo = \"numero\"\nprogressivo = true\n\n[[scheda]]\ntipo = \"moltiplicatori\"\ntitolo = { it = \"Stati\", en = \"States\" }\nordine = 1\ndominio = \"dati/x.dom\"\nper_modo = true\n";

    fn prova_valida(d: &Path) {
        scrivi(d, "contest.toml", TOML_MINIMO);
        scrivi(d, "punteggio.regole", "; commento = non una regola\nbands=20\npoints=any=3\n\nmult=exch\n");
        scrivi(d, "dati/x.dom", "AA = AA\nBB = BB\n");
        scrivi(d, "macros/prova.mc", "[Macros]\r\nnItems=1\r\nName0=F1\r\nText0=CQ\r\n");
        scrivi(d, "regolamento.md", "# Regolamento\n");
    }

    #[test]
    fn legge_e_converte_una_cartella() {
        let base = cartella_di_prova("legge");
        let d = base.join("Contest di prova");
        prova_valida(&d);
        let f = carica(&d, Origine::Utente).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(f.id, "Contest di prova");
        assert_eq!(f.punteggio_regole, "bands=20\npoints=any=3\nmult=exch\n", "senza commenti");
        assert_eq!(f.schede.len(), 1);
        assert_eq!(f.schede[0].titolo.get("it").map(String::as_str), Some("Stati"));
        assert_eq!(f.schede[0].parametri.get("per_modo").and_then(toml::Value::as_bool), Some(true));
        assert_eq!(f.scambio.ricevuto.len(), 1);
        assert!(f.scambio.inviato[0].progressivo && f.scambio.ricevuto[0].motore);
        let c = f.in_contesto(utc(2026, 2, 1, 0, 0));
        assert_eq!((c.name.as_str(), c.notes.as_str(), c.id.as_str()), ("Contest di prova", "CW", "Contest di prova"));
        assert_eq!((c.start.as_str(), c.end.as_str()), ("20260307120000", "20260308115959"));
        assert!(c.valid_cw && !c.valid_phone && !c.valid_digital);
        assert!(!c.dupe_no_band && !c.dupe_no_mode && c.auto_inc && c.grab_number);
        assert_eq!((c.macros.as_str(), c.domain_file.as_str(), c.work_time), ("macros/prova.mc", "dom/x.dom", 12));
        assert_eq!(c.controlla(), Ok(()));
        assert_eq!(f.macro_del_contest().len(), 1);
        assert_eq!(f.dominio_del_contest().map(|d| d.entries.len()), Some(2));
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn zone_dello_scambio_e_qtc() {
        let base = cartella_di_prova("zone");
        let carica_con = |nome: &str, tipo: &str, qtc: &str| {
            let d = base.join(nome);
            prova_valida(&d);
            let t = TOML_MINIMO.replace("tipo = \"numero\"\nelenco", &format!("tipo = \"{tipo}\"\nelenco")).replace("note = \"CW\"", &format!("note = \"CW\"{qtc}"));
            scrivi(&d, "contest.toml", &t);
            carica(&d, Origine::Utente).map(|f| f.in_contesto(utc(2026, 2, 1, 0, 0)))
        };
        // zona CQ: proposta nello scambio e suggerita; zona ITU: solo suggerita
        let c = carica_con("cq", "zona_cq", "").unwrap_or_else(|e| panic!("{e}"));
        assert!(c.grab_number && c.grab_number_type == 2 && c.suggest_cq_zone && !c.suggest_itu_zone && c.qtc == 0);
        let c = carica_con("itu", "zona_itu", "\nqtc = 2").unwrap_or_else(|e| panic!("{e}"));
        assert!(!c.grab_number && c.suggest_itu_zone && !c.suggest_cq_zone);
        assert_eq!(c.qtc, 2);
        assert!(carica_con("qtc-brutto", "numero", "\nqtc = 3").is_err_and(|e| e.contains("qtc")));
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn rifiuta_le_cartelle_sbagliate() {
        let base = cartella_di_prova("rifiuta");
        let prova = |nome: &str, f: &dyn Fn(&Path)| {
            let d = base.join(nome);
            prova_valida(&d);
            f(&d);
            carica(&d, Origine::Utente).err()
        };
        assert!(prova("ok", &|_| {}).is_none());
        assert!(prova("senza-regole", &|d| drop(std::fs::remove_file(d.join("punteggio.regole")))).is_some_and(|e| e.contains("punteggio.regole")));
        assert!(prova("regole-sbagliate", &|d| scrivi(d, "punteggio.regole", "mult=pippo\n")).is_some_and(|e| e.contains("punteggio.regole")));
        assert!(prova("toml-rotto", &|d| scrivi(d, "contest.toml", "[contest\n")).is_some());
        assert!(prova("chiave-sconosciuta", &|d| scrivi(d, "contest.toml", &TOML_MINIMO.replace("note = \"CW\"", "nota = \"CW\""))).is_some());
        assert!(prova("senza-nome", &|d| scrivi(d, "contest.toml", &TOML_MINIMO.replace("nome = \"Contest di prova\"", "nome = \"\""))).is_some_and(|e| e.contains("nome")));
        assert!(prova("id-brutto", &|d| scrivi(d, "contest.toml", &TOML_MINIMO.replace("\"prova\"", "\"Prova X\""))).is_some_and(|e| e.contains("id")));
        assert!(prova("dom-mancante", &|d| drop(std::fs::remove_file(d.join("dati/x.dom")))).is_some_and(|e| e.contains("dati/x.dom")));
        assert!(prova("dom-fuori", &|d| scrivi(d, "contest.toml", &TOML_MINIMO.replace("elenco = \"dati/x.dom\"", "elenco = \"../x.dom\""))).is_some_and(|e| e.contains("non ammesso")));
        assert!(prova("dom-assoluto", &|d| scrivi(d, "contest.toml", &TOML_MINIMO.replace("elenco = \"dati/x.dom\"", "elenco = \"C:/x.dom\""))).is_some());
        assert!(prova("mese", &|d| scrivi(d, "contest.toml", &TOML_MINIMO.replace("mese = 3", "mese = 13"))).is_some_and(|e| e.contains("mese")));
        assert!(prova("doppi", &|d| scrivi(d, "contest.toml", &TOML_MINIMO.replace("banda+modo", "boh"))).is_some_and(|e| e.contains("doppi")));
        assert!(prova("scheda-senza-tipo", &|d| scrivi(d, "contest.toml", &TOML_MINIMO.replace("tipo = \"moltiplicatori\"", "tipo = \"\""))).is_some_and(|e| e.contains("scheda")));
        assert!(prova("grande", &|d| scrivi(d, "regolamento.md", &"x".repeat(MAX_BYTE as usize + 1))).is_some_and(|e| e.contains("troppo grande")));
        assert!(prova("script", &|d| scrivi(d, "punteggio.rhai", "fn punti(q) { 1 + }")).is_some_and(|e| e.contains("punteggio.rhai")));
        assert!(prova("prove-rotte", &|d| scrivi(d, "prove.toml", "[prova\n")).is_some_and(|e| e.contains("prove.toml")));
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn nessun_riferimento_esce_dalla_cartella() {
        let mut dom = BTreeMap::new();
        dom.insert("dati/a.dom".to_string(), String::new());
        assert!(rimando_dom_ok("dati/a.dom", &dom).is_ok());
        assert!(rimando_dom_ok("DATI/A.DOM", &dom).is_ok());
        for brutto in ["../dati/a.dom", "/dati/a.dom", "dati/../a.dom", "C:\\dati\\a.dom", "macros/a.dom", "dati/a.txt", "dati/b.dom"] {
            assert!(rimando_dom_ok(brutto, &dom).is_err(), "{brutto}");
        }
    }

    #[test]
    fn un_contest_personalizzato_con_il_vecchio_nome_di_un_alias_non_compare_due_volte() {
        let base = cartella_di_prova("alias_personalizzato");
        let inclusa = base.join("incluse");
        let dati = base.join("dati");
        // la cartella si chiama «Contest di prova» e porta il vecchio nome come alias
        prova_valida(&inclusa.join("Contest di prova"));
        let con_alias = TOML_MINIMO.replace("note = \"CW\"", "note = \"CW\"
alias = [\"Vecchio nome\"]");
        scrivi(&inclusa, "Contest di prova/contest.toml", &con_alias);
        // il `.contest` col vecchio nome e' stato cambiato dall'utente (e nessun predefinito lo dice intatto)
        let mio = ContestTesto { id: "Mio".into(), name: "Vecchio nome".into(), notes: "mio".into(), ..ContestTesto::default() };
        scrivi(&dati, "Mio.contest", &mio.in_testo());
        let e = ElencoContest::new(&dati);
        e.imposta_cartelle_incluse(Some(inclusa));
        let v = e.voci();
        assert_eq!(v.iter().map(|x| (x.origine(), x.contest.name.as_str())).collect::<Vec<_>>(), vec![("file", "Vecchio nome")], "vale il suo, una riga sola");
    }

    #[test]
    fn una_copia_vecchia_resta_intatta_con_il_segno_beta_e_lo_scambio_nuovi_dei_predefiniti() {
        let base = cartella_di_prova("segno_beta");
        let inclusa = base.join("incluse");
        let dati = base.join("dati");
        let predef = base.join("predefiniti");
        prova_valida(&inclusa.join("Contest di prova"));
        // la copia dell'utente e' quella di una versione vecchia (niente segno Beta, con lo scambio fisso di allora)
        let vecchia = ContestTesto { id: "Vecchio".into(), name: "Contest di prova".into(), nrs: "14".into(), ..ContestTesto::default() };
        scrivi(&dati, "Vecchio.contest", &vecchia.in_testo());
        let senza_cartella = ContestTesto { id: "Solo vecchio".into(), name: "Solo vecchio".into(), nrs: "JO21".into(), ..ContestTesto::default() };
        scrivi(&dati, "Solo vecchio.contest", &senza_cartella.in_testo());
        // quelli distribuiti oggi: segno Beta e scambio fisso tolto
        scrivi(&predef, "Vecchio.contest", &ContestTesto { beta: true, nrs: String::new(), ..vecchia.clone() }.in_testo());
        scrivi(&predef, "Solo vecchio.contest", &ContestTesto { beta: true, nrs: String::new(), ..senza_cartella.clone() }.in_testo());
        let e = ElencoContest::new(&dati);
        e.imposta_cartelle_incluse(Some(inclusa));
        assert_eq!(e.copia_predefiniti(&predef), 0, "i file dell'utente non si toccano");
        let v = e.voci();
        // la cartella verificata sostituisce la copia vecchia (non e' «personalizzata»)
        assert_eq!(v.iter().filter(|x| x.contest.name == "Contest di prova").map(|x| x.origine()).collect::<Vec<_>>(), vec!["cartella"]);
        // la copia senza cartella resta, ed e' Beta anche se il suo file e' di prima del segno
        let solo = v.iter().find(|x| x.contest.name == "Solo vecchio").map(|x| (x.origine(), x.contest.beta));
        assert_eq!(solo, Some(("file", true)));
        // una copia che l'utente ha cambiato davvero resta sua
        scrivi(&dati, "Vecchio.contest", &ContestTesto { notes: "mio".into(), ..vecchia.clone() }.in_testo());
        let v = e.voci();
        assert!(v.iter().any(|x| x.contest.name == "Contest di prova" && x.origine() == "file" && x.contest.notes == "mio"));
    }

    #[test]
    fn elenco_con_due_radici_e_regola_dei_nomi() {
        let base = cartella_di_prova("radici");
        let inclusa = base.join("incluse");
        let dati = base.join("dati");
        let predef = base.join("predefiniti");
        prova_valida(&inclusa.join("Contest di prova"));
        // una cartella non valida accanto a una buona: si salta
        scrivi(&inclusa, "Rotta/contest.toml", "[contest\n");
        let e = ElencoContest::new(&dati);
        e.imposta_cartelle_incluse(Some(inclusa.clone()));
        let voci = e.voci();
        assert_eq!(voci.len(), 1);
        assert_eq!(voci[0].origine(), "cartella");
        assert!(e.scansione().errori.iter().any(|x| x.contains("Rotta")));
        assert_eq!(e.leggi("Contest di prova").map(|c| c.name), Some("Contest di prova".to_string()));
        assert_eq!(e.leggi("contest di prova").map(|c| c.id), Some("Contest di prova".to_string()), "maiuscole indifferenti");

        // un `.contest` con lo stesso nome, personalizzato (o predefiniti sconosciuti): vale il suo
        let vecchio = ContestTesto { name: "Contest di prova".into(), notes: "vecchio".into(), ..ContestTesto::default() };
        // l'identificativo del nuovo file non coincide con quello della cartella
        assert_eq!(e.salva(vecchio.clone()).as_deref(), Ok("Contest di prova (2)"));
        e.dimentica_cartelle();
        let v = e.voci();
        assert_eq!(v.len(), 1);
        assert_eq!((v[0].origine(), v[0].contest.notes.as_str()), ("file", "vecchio"));
        assert_eq!(e.leggi("Contest di prova (2)").map(|c| c.notes), Some("vecchio".to_string()));

        // lo stesso file e' la copia intatta di quello dei predefiniti: la cartella vale al posto suo, il file resta sul disco
        scrivi(&predef, "Contest di prova (2).contest", &ContestTesto { id: "Contest di prova (2)".into(), ..vecchio.clone() }.in_testo());
        assert_eq!(e.copia_predefiniti(&predef), 0, "il file dell'utente non si tocca");
        let v = e.voci();
        assert_eq!((v.len(), v[0].origine()), (1, "cartella"));
        assert!(dati.join("Contest di prova (2).contest").is_file());
        assert_eq!(e.leggi("Contest di prova (2)").map(|c| c.id), Some("Contest di prova".to_string()), "l'id vecchio porta alla cartella");

        // l'utente lo personalizza: il suo vale di nuovo
        scrivi(&dati, "Contest di prova (2).contest", &ContestTesto { id: "Contest di prova (2)".into(), notes: "mio".into(), ..vecchio.clone() }.in_testo());
        let v = e.voci();
        assert_eq!((v.len(), v[0].origine(), v[0].contest.notes.as_str()), (1, "file", "mio"));

        // il nome con altre maiuscole (id e nome del vecchio file diversi da quelli della cartella) e' sostituito lo stesso
        let altro = TOML_MINIMO.replace("\"prova\"", "\"altro\"").replace("Contest di prova", "Altro Contest");
        prova_valida(&inclusa.join("Altro Contest"));
        scrivi(&inclusa, "Altro Contest/contest.toml", &altro);
        let vecchio_altro = ContestTesto { id: "ALTRO".into(), name: "altro contest".into(), ..ContestTesto::default() };
        scrivi(&dati, "ALTRO.contest", &vecchio_altro.in_testo());
        scrivi(&predef, "ALTRO.contest", &vecchio_altro.in_testo());
        e.dimentica_cartelle();
        let nomi: Vec<String> = e.voci().iter().map(|v| format!("{}:{}", v.origine(), v.contest.name)).collect();
        assert_eq!(nomi, vec!["cartella:Altro Contest".to_string(), "file:Contest di prova".to_string()], "il vecchio file e' nascosto");
        assert_eq!(e.leggi("ALTRO").map(|c| c.id), Some("Altro Contest".to_string()), "l'id del vecchio file porta alla cartella");
        assert!(e.seleziona(Some("ALTRO")).is_ok());
        assert_eq!(e.attivo_id().as_deref(), Some("Altro Contest"));
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn la_cartella_dell_utente_sostituisce_quella_inclusa_e_il_vecchio_file() {
        let base = cartella_di_prova("utente");
        let inclusa = base.join("incluse");
        let dati = base.join("dati");
        prova_valida(&inclusa.join("Contest di prova"));
        let e = ElencoContest::new(&dati);
        e.imposta_cartelle_incluse(Some(inclusa));
        // un `.contest` vecchio dello stesso nome (personalizzato: la cartella inclusa non lo sostituisce)
        e.salva(ContestTesto { name: "Contest di prova".into(), notes: "vecchio".into(), ..ContestTesto::default() }).unwrap_or_default();
        e.dimentica_cartelle();
        assert_eq!(e.voci()[0].origine(), "file");
        // la copia nella cartella dell'utente: sostituisce la inclusa e, perche' sta nei suoi dati, anche il vecchio file
        assert_eq!(e.copia_nella_mia_cartella("Contest di prova"), Ok("Contest di prova".to_string()));
        assert!(dati.join("Contest di prova/contest.toml").is_file());
        assert!(e.copia_nella_mia_cartella("Contest di prova").is_err(), "gia' nella cartella dell'utente");
        let v = e.voci();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].origine(), "cartella");
        assert_eq!(v[0].cartella.as_ref().map(|c| c.origine), Some(Origine::Utente));
        assert!(e.leggi("nessuno").is_none());
        // modifica e cancellazione dall'elenco rifiutate
        assert!(e.salva(v[0].contest.clone()).is_err());
        assert!(e.elimina("Contest di prova").is_err());
        // il contest attivo con macro e domini
        assert!(e.seleziona(Some("Contest di prova")).is_ok());
        let a = e.attivo().map(|a| (a.macros.len(), a.domain.map(|d| d.entries.len()), a.start_ms.is_some()));
        assert_eq!(a, Some((1, Some(2), true)));
        // il pacchetto
        let zip = e.esporta("Contest di prova").unwrap_or_default();
        assert!(zip.starts_with(b"PK"));
        // una cartella dell'utente rotta: si salta e la inclusa e' di nuovo la candidata (qui il `.contest` personalizzato resta davanti)
        scrivi(&dati, "Contest di prova/contest.toml", "[contest\n");
        e.dimentica_cartelle();
        assert!(e.scansione().errori.iter().any(|x| x.contains("Contest di prova")));
        assert_eq!(e.scansione().cartelle.iter().map(|c| c.origine).collect::<Vec<_>>(), vec![Origine::Inclusa]);
        assert_eq!(e.voci()[0].origine(), "file");
        let _ = std::fs::remove_dir_all(&base);
    }
    /// Elenco ricorsivo di file e contenuti: per verificare che il core non scriva nella cartella inclusa.
    fn istantanea(d: &Path) -> Vec<(String, Vec<u8>)> {
        let mut out = Vec::new();
        let mut da_fare = vec![d.to_path_buf()];
        while let Some(x) = da_fare.pop() {
            for e in std::fs::read_dir(&x).into_iter().flatten().flatten() {
                let p = e.path();
                if p.is_dir() {
                    da_fare.push(p);
                } else {
                    out.push((p.display().to_string(), std::fs::read(&p).unwrap_or_default()));
                }
            }
        }
        out.sort();
        out
    }

    #[test]
    fn le_modifiche_personali_di_un_contest_incluso() {
        let base = cartella_di_prova("sovrascritture");
        let inclusa = base.join("incluse");
        let dati = base.join("dati");
        prova_valida(&inclusa.join("Contest di prova"));
        // un altro contest in cartella, che non deve essere toccato
        let mut altro = TOML_MINIMO.replace("\"prova\"", "\"altro\"").replace("Contest di prova", "Altro contest");
        altro = altro.replace("note = \"CW\"", "note = \"SSB\"");
        scrivi(&inclusa.join("Altro contest"), "contest.toml", &altro);
        scrivi(&inclusa.join("Altro contest"), "punteggio.regole", "bands=20
points=any=3
mult=exch
");
        scrivi(&inclusa.join("Altro contest"), "dati/x.dom", "AA = AA
");
        let e = ElencoContest::new(&dati);
        e.imposta_cartelle_incluse(Some(inclusa.clone()));
        let prima = istantanea(&inclusa);
        let originale = e.voce("Contest di prova").map(|v| v.contest).unwrap_or_default();
        let altro_prima = e.voce("Altro contest").map(|v| v.contest).unwrap_or_default();
        assert!(!originale.start.is_empty());

        // le date, le ore e il numero si salvano; le regole e il nome nel corpo si ignorano senza errore
        let corpo = serde_json::json!({ "name": "Rubato", "score": "mult=pippo", "valid_cw": false, "start": "20300101000000", "end": "20300102235959", "work_time": 8, "nrs": "7", "auto_inc": false });
        let v = e.salva_sovrascrittura("Contest di prova", &corpo).unwrap_or_else(|x| panic!("{x}"));
        assert_eq!((v.contest.start.as_str(), v.contest.end.as_str(), v.contest.work_time, v.contest.nrs.as_str(), v.contest.auto_inc), ("20300101000000", "20300102235959", 8, "7", false));
        assert_eq!(v.contest.name, originale.name, "il nome non cambia");
        assert_eq!(v.contest.score, originale.score, "le regole non cambiano");
        assert!(v.contest.valid_cw);
        assert!(dati.join("sovrascritture/Contest di prova.json").is_file());
        // riletto da zero (altra istanza sullo stesso disco) e nell'elenco
        let e2 = ElencoContest::new(&dati);
        e2.imposta_cartelle_incluse(Some(inclusa.clone()));
        let riletto = e2.voce("Contest di prova").map(|v| v.contest).unwrap_or_default();
        assert_eq!((riletto.start.as_str(), riletto.work_time), ("20300101000000", 8));
        assert_eq!(e2.voci().iter().find(|v| v.contest.id == "Contest di prova").map(|v| v.contest.end.clone()), Some("20300102235959".to_string()));
        assert_eq!(e2.attivo().map(|a| a.start_ms), None, "nessun contest attivo scelto");
        // la sottocartella delle modifiche non e' un contest
        assert!(e2.scansione().errori.is_empty());
        assert_eq!(e2.voci().len(), 2);
        // un altro contest in cartella non e' toccato; la cartella inclusa nemmeno
        assert_eq!(e.voce("Altro contest").map(|v| v.contest), Some(altro_prima));
        assert_eq!(istantanea(&inclusa), prima, "il core non scrive nella cartella inclusa");

        // valori non validi
        assert!(e.salva_sovrascrittura("Contest di prova", &serde_json::json!({ "work_time": 49 })).is_err());
        assert!(e.salva_sovrascrittura("Contest di prova", &serde_json::json!({ "work_time": -1 })).is_err());
        assert!(e.salva_sovrascrittura("Contest di prova", &serde_json::json!({ "start": "2030", "end": "20300102235959" })).is_err());
        assert!(e.salva_sovrascrittura("Contest di prova", &serde_json::json!({ "start": "20300102000000", "end": "20300101000000" })).is_err());
        assert!(e.salva_sovrascrittura("Contest di prova", &serde_json::json!({ "macros": "macros/non-esiste.mc" })).is_err());
        assert!(e.salva_sovrascrittura("Contest di prova", &serde_json::json!({ "macros": "../x.mc" })).is_err());
        assert!(e.salva_sovrascrittura("Sconosciuto", &serde_json::json!({})).is_err());
        // dopo gli errori il salvato di prima e' ancora li'
        assert_eq!(e.voce("Contest di prova").map(|v| v.contest.work_time), Some(8));

        // una macro dell'utente: si sceglie e il contest attivo la usa; vuota = nessuna macro
        scrivi(&dati, "macros/mia.mc", "[Macros]
nItems=2
Name0=F1
Text0=CQ
Name1=F2
Text1=73
");
        let v = e.salva_sovrascrittura("Contest di prova", &serde_json::json!({ "start": "20300101000000", "end": "20300102235959", "work_time": 8, "nrs": "7", "auto_inc": false, "macros": "macros/mia.mc" })).unwrap_or_else(|x| panic!("{x}"));
        assert_eq!(v.contest.macros, "macros/mia.mc");
        assert!(e.seleziona(Some("Contest di prova")).is_ok());
        assert_eq!(e.attivo().map(|a| a.macros.len()), Some(2));
        let v = e.salva_sovrascrittura("Contest di prova", &serde_json::json!({ "start": "20300101000000", "end": "20300102235959", "work_time": 8, "nrs": "7", "auto_inc": false, "macros": "" })).unwrap_or_else(|x| panic!("{x}"));
        assert_eq!(v.contest.macros, "");
        assert_eq!(e.attivo().map(|a| a.macros.len()), Some(0));

        // un corpo uguale ai valori della cartella non lascia niente; ripristina toglie tutto
        let v = e.salva_sovrascrittura("Contest di prova", &serde_json::json!({ "start": originale.start, "end": originale.end, "work_time": originale.work_time, "nrs": originale.nrs, "auto_inc": originale.auto_inc, "macros": originale.macros })).unwrap_or_else(|x| panic!("{x}"));
        assert!(v.sovrascritta.is_none() && !dati.join("sovrascritture/Contest di prova.json").exists());
        assert_eq!(v.contest, originale);
        e.salva_sovrascrittura("Contest di prova", &serde_json::json!({ "work_time": 3 })).unwrap_or_else(|x| panic!("{x}"));
        assert_eq!(e.voce("Contest di prova").map(|v| v.contest.work_time), Some(3));
        let v = e.ripristina_sovrascrittura("Contest di prova").unwrap_or_else(|x| panic!("{x}"));
        assert_eq!(v.contest, originale);
        assert!(e.ripristina_sovrascrittura("Contest di prova").is_ok(), "ripristinare due volte non e' un errore");
        assert_eq!(istantanea(&inclusa), prima);
        let _ = std::fs::remove_dir_all(&base);
    }
}
