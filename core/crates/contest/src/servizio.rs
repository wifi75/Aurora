//! Il servizio della pagina Contest: contest attivo, impostazioni salvate per contest, log del contest su file, ascolto dei QSO registrati e
//! delle decodifiche (per i QSO incerti), punteggio in diretta.
//!
//! File nella cartella dei dati: `contest/stato.json` (contest attivo, impostazioni, cosa ripristinare a fine gara) e
//! `contest/log-<id>-<aaaa-mm-gg>.json` (il log del contest dell'edizione che comincia quel giorno; `log-<id>-<anno>.json` per i contest
//! senza date e per i log delle versioni precedenti, che si leggono ancora). Ogni QSO va anche nel log normale di Aurora: il log del contest e'
//! una copia in piu' con gli scambi, i punti e lo stato.

use crate::calendario::{self, Finestra};
use crate::export::{self, Intestazione};
use crate::punteggio::{self, ContestQso, Contesto, Stato, Valutato};
use crate::regole::{self, Regole};
use aurora_core::{Bus, ContestSetup, DecodedMessage, Event, QsoRecord};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use tokio::sync::broadcast::error::RecvError;

/// Le impostazioni di un contest (scheda «Impostazioni del contest»): precompilate dalle regole, modificabili, salvate per contest.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Impostazioni {
    /// Attivita' dei messaggi (vuota = quella delle regole).
    pub attivita: String,
    /// Parola del CQ (vuota = quella delle regole).
    pub parola_cq: String,
    /// Scambio fisso (Field Day `1D DX`, Roundup stato o `#`).
    pub scambio: String,
    /// Primo numero progressivo.
    pub seriale_partenza: u32,
    /// Categoria scelta (id delle regole).
    pub categoria: String,
    /// Potenza in watt (Field Day: moltiplicatore di potenza).
    pub potenza_w: f64,
    /// Punti bonus del Field Day spuntati a mano.
    pub bonus: u32,
    /// Modo consigliato all'avvio (`FT8`, `FT4`...).
    pub modo: String,
    /// «Hold Tx Freq»: la frequenza di trasmissione resta ferma.
    pub hold_tx: bool,
    /// Chiusura con RRR.
    pub rrr: bool,
    /// Locatore a 6 caratteri (EU VHF, EDI).
    pub grid6: String,
    pub operatori: String,
    pub club: String,
    pub nome: String,
    /// `LOCATION` del Cabrillo (sezione ARRL/RAC o `DX`).
    pub location: String,
    pub soapbox: String,
    /// Antenna (EDI: `SAnte`), per i contest VHF.
    pub antenna: String,
    /// `CATEGORY-ASSISTED` (vuoto = quello della categoria).
    pub assistenza: String,
    /// Nome del contest nel Cabrillo (vuoto = quello delle regole; serve al «Personalizzato»).
    pub cabrillo_contest: String,
    /// Potenza dichiarata (`HIGH`, `LOW`, `QRP`; vuota = quella della categoria), se il regolamento la lascia scegliere.
    pub potenza: String,
    /// Field Day: numero di trasmettitori della classe (0 = 1).
    pub trasmettitori_n: u32,
    /// Nominativo usato in gara se diverso da quello della stazione (`CALLSIGN` del Cabrillo).
    pub call_contest: String,
    /// `ADDRESS` del Cabrillo (piu' righe).
    pub indirizzo: String,
    /// `EMAIL` del Cabrillo.
    pub email: String,
    /// `CATEGORY-TIME` e `CATEGORY-OVERLAY` del Cabrillo (vuoti = non si scrivono).
    pub categoria_tempo: String,
    pub categoria_overlay: String,
    /// Inizio e fine scelti a mano (contest «sempre» o date diverse), ms UTC.
    pub inizio_ms: Option<i64>,
    pub fine_ms: Option<i64>,
}

/// Cio' che c'era prima di attivare il contest, per tornare alle impostazioni normali a fine gara.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Ripristino {
    /// Interruttori della finestra principale (`fox`, `hound`, `hold_tx`, `rrr`, `autoseq`...).
    pub controlli: Vec<(String, bool)>,
    /// Il modo in uso (`FT8`...).
    pub modo: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct StatoFile {
    attivo: Option<String>,
    /// Nome, indirizzo, email e club dell'intestazione: scritti una volta, riusati in tutti i contest.
    anagrafica: Anagrafica,
    impostazioni: BTreeMap<String, Impostazioni>,
    ripristino: Option<Ripristino>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct Anagrafica {
    nome: String,
    indirizzo: String,
    email: String,
    club: String,
}

/// Le impostazioni di un contest: quelle salvate (o le proposte), con l'anagrafica comune dove mancano e la classe del Field Day
/// scritta alla vecchia maniera (`1D`) portata a lettera + numero di trasmettitori.
fn imp_di(stato: &StatoFile, r: &Regole) -> Impostazioni {
    let mut imp = stato.impostazioni.get(r.id()).cloned().unwrap_or_else(|| predefinite(r));
    let a = &stato.anagrafica;
    for (campo, comune) in [(&mut imp.nome, &a.nome), (&mut imp.indirizzo, &a.indirizzo), (&mut imp.email, &a.email), (&mut imp.club, &a.club)] {
        if campo.trim().is_empty() {
            campo.clone_from(comune);
        }
    }
    if r.opzioni_categoria.classe_fd && r.categoria(&imp.categoria).is_none() {
        let n: String = imp.categoria.chars().take_while(char::is_ascii_digit).collect();
        let lettera: String = imp.categoria.chars().skip(n.len()).collect();
        if r.categoria(&lettera).is_some() {
            imp.trasmettitori_n = n.parse().unwrap_or(1);
            imp.categoria = lettera;
        }
    }
    imp
}

/// La classe del Field Day (`1D`, `2A`): numero di trasmettitori + lettera della categoria.
fn classe_fd(r: &Regole, imp: &Impostazioni) -> String {
    let lettera = r.categoria(&imp.categoria).map_or("D", |c| c.sigla());
    format!("{}{lettera}", imp.trasmettitori_n.max(1))
}

/// La stazione: nominativo e locatore correnti.
pub type StazioneFn = Arc<dyn Fn() -> (String, String) + Send + Sync>;

struct Dentro {
    regole: Vec<Regole>,
    errori_regole: Vec<String>,
    stato: StatoFile,
    /// Il log del contest attivo (e l'edizione a cui appartiene).
    log: Vec<ContestQso>,
    log_di: Option<(String, String)>,
    /// QSO appena registrati: nominativo e numero, per riconoscere chi ripete lo scambio (QSO incerto).
    recenti: Vec<(String, u32, i64)>,
    /// Stazioni che hanno risposto con un rapporto in dB invece dello scambio del contest.
    non_contest: Vec<(String, i64)>,
    /// Stazioni sentite di recente su ogni banda (solo a contest acceso), per il consiglio di banda e la mappa dei moltiplicatori.
    sentiti: Vec<Sentito>,
    /// Banda e modo della radio adesso (dallo stato della stazione).
    radio: (String, String),
    /// L'ultimo consiglio calcolato e quando: si ricalcola al piu' ogni 10 s anche con piu' pagine aperte.
    consiglio: Option<(i64, serde_json::Value)>,
}

/// Una stazione sentita: chi trasmette, il suo locatore (se l'ha mandato), banda e modo, segnale, quando e se chiamava CQ.
#[derive(Clone, Debug)]
struct Sentito {
    call: String,
    grid: String,
    banda: String,
    modo: String,
    snr: i32,
    t: i64,
    cq: bool,
}

/// Quanto resta in memoria una stazione sentita.
const SENTITI_MS: i64 = 20 * 60_000;
/// La finestra «in ascolto adesso» del consiglio e della mappa.
const ASCOLTO_MS: i64 = 15 * 60_000;
/// La finestra per l'affollamento (pile-up): gli ultimi 3 minuti.
const FOLLA_MS: i64 = 3 * 60_000;
/// Stazioni diverse in 3 minuti oltre le quali la banda e' fitta e conviene FT4.
const FOLLA_FT4: usize = 25;
/// Tetto delle stazioni ricordate.
const SENTITI_MAX: usize = 3000;

/// Un nominativo plausibile: lettere e cifre (e `/`), almeno una cifra e una lettera.
fn nominativo(c: &str) -> bool {
    (3..=12).contains(&c.len())
        && c.chars().all(|x| x.is_ascii_alphanumeric() || x == '/')
        && c.chars().any(|x| x.is_ascii_digit())
        && c.chars().any(|x| x.is_ascii_alphabetic())
}

/// Chi trasmette un messaggio decodificato, il suo locatore (vuoto se non c'e') e se chiama CQ.
fn mittente(testo: &str) -> Option<(String, String, bool)> {
    let w: Vec<String> = testo.replace(['<', '>'], "").split_whitespace().map(str::to_ascii_uppercase).collect();
    if w.len() < 2 {
        return None;
    }
    let griglia = |g: &str| crate::geo::grid_valido(g) && g.len() == 4 && g != "RR73";
    if w[0] == "CQ" || w[0] == "QRZ" {
        // «CQ TEST K1ABC FN42» o «CQ K1ABC FN42»: la parola dopo CQ, se non e' un nominativo, e' la parola del CQ
        let resto = &w[1..];
        let call = if resto.len() >= 2 && !nominativo(&resto[0]) { &resto[1] } else { &resto[0] };
        if !nominativo(call) {
            return None;
        }
        let ultimo = resto.last().map(String::as_str).unwrap_or_default();
        let grid = if ultimo != call && griglia(ultimo) { ultimo.to_string() } else { String::new() };
        return Some((call.clone(), grid, true));
    }
    let call = &w[1];
    if !nominativo(call) {
        return None;
    }
    let grid = w.get(2).filter(|g| griglia(g)).cloned().or_else(|| w.get(3).filter(|g| w[2] == "R" && griglia(g)).cloned()).unwrap_or_default();
    Some((call.clone(), grid, false))
}

/// Le frequenze consuete (kHz) di FT8 e FT4 per banda, quando il regolamento non ne indica.
fn frequenza_standard(banda: &str, modo: &str) -> Option<f64> {
    let ft4 = modo.eq_ignore_ascii_case("FT4");
    let (ft8, f4) = match banda {
        "160m" => (1840.0, None),
        "80m" => (3573.0, Some(3575.5)),
        "40m" => (7074.0, Some(7047.5)),
        "30m" => (10136.0, Some(10140.0)),
        "20m" => (14074.0, Some(14080.0)),
        "17m" => (18100.0, Some(18104.0)),
        "15m" => (21074.0, Some(21140.0)),
        "12m" => (24915.0, Some(24919.0)),
        "10m" => (28074.0, Some(28180.0)),
        "6m" => (50313.0, Some(50318.0)),
        "2m" => (144174.0, Some(144170.0)),
        _ => return None,
    };
    if ft4 { f4 } else { Some(ft8) }
}

pub struct ContestService {
    dir: PathBuf,
    stazione: StazioneFn,
    versione: String,
    dentro: Mutex<Dentro>,
    /// Ricerca del paese DXCC (cty.dat) per i moltiplicatori `dxcc`: la imposta chi apre il servizio.
    paese: Mutex<Option<punteggio::PaeseFn>>,
    /// Ricerca del continente (cty.dat) per i punti `per-continente`.
    continente: Mutex<Option<punteggio::PaeseFn>>,
    /// Il QSO in corso e' partito dalla pagina Contest (doppio clic o CQ contest): solo questi entrano nel log del contest.
    da_contest: std::sync::atomic::AtomicBool,
}

fn ora_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as i64)
}

fn anno_di(ms: i64) -> i64 {
    calendario::civil_from_days(ms.div_euclid(86_400_000)).0
}

impl ContestService {
    /// Apre il servizio nella cartella dei dati (`<dati>/contest`). Le regole dell'utente stanno in `<dati>/contest/regole`.
    pub fn open(data_dir: &Path, stazione: StazioneFn, versione: &str) -> Arc<ContestService> {
        let dir = data_dir.join("contest");
        let _ = std::fs::create_dir_all(dir.join("regole"));
        let (regole, errori_regole) = regole::carica(Some(&dir.join("regole")));
        for e in &errori_regole {
            tracing::warn!("regole dei contest: {e}");
        }
        let stato: StatoFile = std::fs::read_to_string(dir.join("stato.json")).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
        let s = Arc::new(ContestService {
            dir,
            stazione,
            versione: versione.to_string(),
            paese: Mutex::new(None),
            continente: Mutex::new(None),
            da_contest: std::sync::atomic::AtomicBool::new(false),
            dentro: Mutex::new(Dentro { regole, errori_regole, stato, log: Vec::new(), log_di: None, recenti: Vec::new(), non_contest: Vec::new(), sentiti: Vec::new(), radio: (String::new(), String::new()), consiglio: None }),
        });
        if let Ok(mut d) = s.dentro.lock() {
            s.carica_log(&mut d);
        }
        s
    }

    /// Da usare per i moltiplicatori DXCC: il paese di un nominativo (prefisso principale di cty.dat).
    pub fn imposta_paese(&self, f: punteggio::PaeseFn) {
        *self.paese.lock().unwrap_or_else(|e| e.into_inner()) = Some(f);
    }

    /// Da usare per i punti `per-continente`: il continente di un nominativo (cty.dat).
    pub fn imposta_continente(&self, f: punteggio::PaeseFn) {
        *self.continente.lock().unwrap_or_else(|e| e.into_inner()) = Some(f);
    }

    fn lock(&self) -> MutexGuard<'_, Dentro> {
        self.dentro.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn salva_stato(&self, d: &Dentro) {
        if let Ok(t) = serde_json::to_string_pretty(&d.stato)
            && let Err(e) = std::fs::write(self.dir.join("stato.json"), t)
        {
            tracing::warn!("contest: stato non salvato: {e}");
        }
    }

    /// L'edizione del contest attivo: il giorno d'inizio (`aaaa-mm-gg`) della finestra attuale (in corso, appena finita o prossima) o di
    /// quella scelta a mano; per i contest senza date l'anno di adesso. Cosi' i contest settimanali e mensili hanno un log per edizione.
    fn edizione(d: &Dentro, r: &Regole) -> String {
        match Self::finestra(d, r, ora_ms()) {
            Some(f) => calendario::data(f.inizio_ms),
            None => anno_di(ora_ms()).to_string(),
        }
    }

    fn file_log(&self, id: &str, edizione: &str) -> PathBuf {
        self.dir.join(format!("log-{id}-{edizione}.json"))
    }

    fn leggi_log(&self, id: &str, edizione: &str) -> Option<Vec<ContestQso>> {
        std::fs::read_to_string(self.file_log(id, edizione)).ok().and_then(|t| serde_json::from_str(&t).ok())
    }

    /// Rilegge il log del contest attivo se l'edizione e' cambiata. Non toglie mai QSO: il log di un'edizione finita resta com'e'.
    fn carica_log(&self, d: &mut Dentro) {
        let Some(id) = d.stato.attivo.clone() else {
            d.log.clear();
            d.log_di = None;
            return;
        };
        let Some(r) = d.regole.iter().find(|r| r.id() == id).cloned() else { return };
        let ed = Self::edizione(d, &r);
        if d.log_di.as_ref() == Some(&(id.clone(), ed.clone())) {
            return;
        }
        d.log = match self.leggi_log(&id, &ed) {
            Some(l) => l,
            None => {
                // il log per anno delle versioni precedenti: se ne copiano i QSO di questa edizione (il vecchio file resta com'e')
                let f = Self::finestra(d, &r, ora_ms());
                match (f, ed.get(..4)) {
                    (Some(f), Some(anno)) if ed.len() == 10 => {
                        self.leggi_log(&id, anno).unwrap_or_default().into_iter().filter(|q| q.time_ms >= f.inizio_ms && q.time_ms < f.fine_ms).collect()
                    }
                    _ => Vec::new(),
                }
            }
        };
        d.log_di = Some((id, ed));
    }

    fn salva_log(&self, d: &Dentro) {
        let Some((id, ed)) = &d.log_di else { return };
        match serde_json::to_string(&d.log) {
            Ok(t) => {
                let tmp = self.file_log(id, ed).with_extension("json.tmp");
                if std::fs::write(&tmp, t).and_then(|()| std::fs::rename(&tmp, self.file_log(id, ed))).is_err() {
                    tracing::warn!("contest: log del contest non salvato");
                }
            }
            Err(e) => tracing::warn!("contest: log del contest non salvato: {e}"),
        }
    }

    /// Tutte le regole (le predefinite e quelle dell'utente) e gli errori dei file dell'utente.
    pub fn regole(&self) -> (Vec<Regole>, Vec<String>) {
        let d = self.lock();
        (d.regole.clone(), d.errori_regole.clone())
    }

    pub fn regola(&self, id: &str) -> Option<Regole> {
        self.lock().regole.iter().find(|r| r.id() == id).cloned()
    }

    /// Il contest attivo.
    pub fn attivo(&self) -> Option<Regole> {
        let d = self.lock();
        let id = d.stato.attivo.clone()?;
        d.regole.iter().find(|r| r.id() == id).cloned()
    }

    /// Le impostazioni di un contest: quelle salvate o, la prima volta, quelle proposte dalle regole.
    pub fn impostazioni(&self, id: &str) -> Option<Impostazioni> {
        let d = self.lock();
        let r = d.regole.iter().find(|r| r.id() == id)?;
        Some(imp_di(&d.stato, r))
    }

    /// Salva le impostazioni di un contest. Se e' quello attivo, la sequenza del motore cambia subito (`Some` da mandare).
    pub fn salva_impostazioni(&self, id: &str, imp: Impostazioni) -> Result<Option<ContestSetup>, String> {
        let mut d = self.lock();
        if !d.regole.iter().any(|r| r.id() == id) {
            return Err("contest sconosciuto".into());
        }
        if !imp.attivita.is_empty() && qso_attivita_sconosciuta(&imp.attivita) {
            return Err(format!("attivita' sconosciuta: {}", imp.attivita));
        }
        // nome, indirizzo, email e club valgono per tutti i contest
        d.stato.anagrafica = Anagrafica { nome: imp.nome.clone(), indirizzo: imp.indirizzo.clone(), email: imp.email.clone(), club: imp.club.clone() };
        d.stato.impostazioni.insert(id.to_string(), imp);
        self.salva_stato(&d);
        if d.stato.attivo.as_deref() == Some(id) {
            // date cambiate a mano: il log in corso segue la nuova edizione se quella non ha ancora un log (nessun QSO si perde)
            let (prima_di, prima) = (d.log_di.take(), std::mem::take(&mut d.log));
            self.carica_log(&mut d);
            if d.log_di != prima_di && d.log.is_empty() && !prima.is_empty() {
                d.log = prima;
                self.salva_log(&d);
            } else if d.log_di == prima_di {
                d.log = prima;
            }
            return Ok(self.setup_di(&d));
        }
        Ok(None)
    }

    /// Attiva un contest. `ripristino` e' cio' che c'era prima (solo se non c'era gia' un contest attivo: a fine gara si torna li').
    /// Restituisce la sequenza da dare al motore.
    pub fn attiva(&self, id: &str, ripristino: Option<Ripristino>) -> Result<ContestSetup, String> {
        let mut d = self.lock();
        if !d.regole.iter().any(|r| r.id() == id) {
            return Err("contest sconosciuto".into());
        }
        if d.stato.attivo.is_none() {
            d.stato.ripristino = ripristino;
        }
        d.stato.attivo = Some(id.to_string());
        if !d.stato.impostazioni.contains_key(id)
            && let Some(r) = d.regole.iter().find(|r| r.id() == id)
        {
            let p = predefinite(r);
            d.stato.impostazioni.insert(id.to_string(), p);
        }
        d.log_di = None;
        self.carica_log(&mut d);
        self.salva_stato(&d);
        self.setup_di(&d).ok_or_else(|| "contest sconosciuto".into())
    }

    /// Spegne il contest: restituisce cio' che va ripristinato.
    pub fn disattiva(&self) -> Option<Ripristino> {
        let mut d = self.lock();
        d.stato.attivo = None;
        let r = d.stato.ripristino.take();
        d.log.clear();
        d.log_di = None;
        self.salva_stato(&d);
        r
    }

    /// La sequenza per il motore del contest attivo (con il prossimo seriale).
    pub fn setup(&self) -> Option<ContestSetup> {
        let d = self.lock();
        self.setup_di(&d)
    }

    fn setup_di(&self, d: &Dentro) -> Option<ContestSetup> {
        let id = d.stato.attivo.as_ref()?;
        let r = d.regole.iter().find(|r| r.id() == *id)?;
        let imp = imp_di(&d.stato, r);
        // il prossimo seriale: QSO nel log del contest + 1 (dal primo scelto), mai uno gia' mandato a un QSO del log
        let usati = d.log.iter().map(|q| seriale_di(&q.exch_sent)).max().unwrap_or(0);
        let contati = imp.seriale_partenza.max(1).saturating_add(d.log.len() as u32);
        let attivita = if imp.attivita.is_empty() { r.messaggi.attivita.clone() } else { imp.attivita.clone() };
        Some(ContestSetup {
            activity: attivita,
            cq_word: if imp.parola_cq.is_empty() { r.messaggi.parola_cq.clone() } else { imp.parola_cq.clone() },
            exchange: scambio_fisso(r, &imp),
            next_serial: contati.max(usati + 1),
            grid6: imp.grid6.clone(),
            rrr: imp.rrr || r.messaggi.chiusura == "RRR",
            bands: r.bande.ammesse.iter().map(|b| b.to_ascii_lowercase()).collect(),
            modes: r.modi.ammessi.iter().map(|m| m.to_ascii_uppercase()).collect(),
        })
    }

    /// Un QSO registrato nel log di Aurora: se c'e' un contest attivo entra anche nel suo log.
    /// Da quale pagina e' partita l'ultima chiamata o l'ultimo CQ: `true` = pagina Contest.
    pub fn imposta_origine(&self, contest: bool) {
        self.da_contest.store(contest, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn registra(&self, q: &QsoRecord) -> Option<u32> {
        // un QSO fatto dalla pagina Stazione resta solo nel log normale, anche con il contest acceso
        if !self.da_contest.load(std::sync::atomic::Ordering::Relaxed) {
            return None;
        }
        let mut d = self.lock();
        let id = d.stato.attivo.clone()?;
        // fuori dalla finestra della gara (prima dell'inizio o dopo la fine) il QSO resta solo nel log normale: non conta per il contest
        let time_ms = if q.time_on_unix_ms > 0 { q.time_on_unix_ms as i64 } else { ora_ms() };
        if let Some(r) = d.regole.iter().find(|r| r.id() == id).cloned()
            && let Some(f) = Self::finestra(&d, &r, time_ms)
            && (time_ms < f.inizio_ms || time_ms >= f.fine_ms)
        {
            return None;
        }
        self.carica_log(&mut d);
        // chi ha risposto con un rapporto invece dello scambio del contest (ultimi 30 minuti) non fa la gara: resta nel log, non valido e 0 punti
        let call_su = q.call.to_ascii_uppercase();
        let non_contest = d.non_contest.iter().any(|(c, t)| *c == call_su && ora_ms() - *t < 30 * 60_000);
        let n = d.log.iter().map(|x| x.n).max().unwrap_or(0) + 1;
        let time_ms = if q.time_on_unix_ms > 0 { q.time_on_unix_ms as i64 } else { ora_ms() };
        d.log.push(ContestQso {
            n,
            time_ms,
            call: q.call.to_ascii_uppercase(),
            band: q.band.to_ascii_lowercase(),
            mode: q.mode.to_ascii_uppercase(),
            freq_hz: q.freq_hz,
            grid: q.grid.clone(),
            rst_sent: q.rst_sent.clone(),
            rst_rcvd: q.rst_rcvd.clone(),
            exch_sent: q.exch_sent.clone(),
            exch_rcvd: q.exch_rcvd.clone(),
            stato: if non_contest { Stato::Errore } else { Stato::Valido },
            modificato: false,
        });
        d.recenti.push((q.call.to_ascii_uppercase(), n, ora_ms()));
        d.recenti.retain(|(_, _, t)| ora_ms() - *t < 5 * 60_000);
        self.salva_log(&d);
        Some(n)
    }

    /// Una decodifica: chi abbiamo appena messo a log e ripete lo scambio (non ha ricevuto il nostro RR73) diventa «incerto»; il suo
    /// RR73/73 lo rimette valido. Chi risponde con un rapporto in dB a un contest con scambio diverso si segna come «non fa il contest».
    pub fn nota_decodifica(&self, m: &DecodedMessage) {
        // a contest spento non si fa niente (niente lavoro a ogni decodifica)
        if self.lock().stato.attivo.is_none() {
            return;
        }
        if let Some((call, grid, cq)) = mittente(&m.text) {
            self.nota_sentito(call, grid, cq, m.snr);
        }
        let (mio, _) = (self.stazione)();
        if mio.is_empty() {
            return;
        }
        let w: Vec<String> = m.text.replace(['<', '>'], "").split_whitespace().map(str::to_ascii_uppercase).collect();
        if w.len() < 3 || !w[0].eq_ignore_ascii_case(&mio) {
            return;
        }
        let da = w[1].clone();
        let mut d = self.lock();
        let Some(id) = d.stato.attivo.clone() else { return };
        let attivita = d.regole.iter().find(|r| r.id() == id).map(|r| r.messaggi.attivita.clone()).unwrap_or_default();
        let fine = w[2..].join(" ");
        let chiude = fine == "RR73" || fine == "73" || fine == "RRR";
        let ripete = !chiude && ripete_scambio(&w[2]);
        let mut cambiato = false;
        let recenti = d.recenti.clone();
        if let Some((_, n, _)) = recenti.iter().rev().find(|(c, _, _)| *c == da)
            && let Some(q) = d.log.iter_mut().find(|q| q.n == *n)
        {
            if chiude && q.stato == Stato::Incerto {
                q.stato = Stato::Valido;
                cambiato = true;
            } else if ripete && q.stato == Stato::Valido {
                q.stato = Stato::Incerto;
                cambiato = true;
            }
        }
        if attivita != "standard" && w.len() == 3 && is_rapporto(&w[2]) {
            d.non_contest.retain(|(c, _)| *c != da);
            d.non_contest.push((da, ora_ms()));
            if d.non_contest.len() > 50 {
                d.non_contest.remove(0);
            }
        }
        if cambiato {
            self.salva_log(&d);
        }
    }

    /// Ricorda una stazione sentita sulla banda e nel modo di adesso (una riga per nominativo e banda, la piu' recente).
    fn nota_sentito(&self, call: String, grid: String, cq: bool, snr: i32) {
        let t = ora_ms();
        let mut d = self.lock();
        let (banda, modo) = d.radio.clone();
        if banda.is_empty() {
            return;
        }
        d.sentiti.retain(|x| t - x.t < SENTITI_MS);
        if let Some(x) = d.sentiti.iter_mut().find(|x| x.call == call && x.banda == banda) {
            // il locatore resta quello gia' visto se questo messaggio non lo porta
            if !grid.is_empty() {
                x.grid = grid;
            }
            x.modo = modo;
            x.snr = snr;
            x.t = t;
            x.cq = cq;
        } else {
            d.sentiti.push(Sentito { call, grid, banda, modo, snr, t, cq });
            if d.sentiti.len() > SENTITI_MAX {
                d.sentiti.remove(0);
            }
        }
    }

    /// La banda e il modo della radio (dallo stato della stazione). A contest spento non si tiene niente.
    pub fn nota_radio(&self, dial_hz: u64, modo: &str) {
        let mut d = self.lock();
        if d.stato.attivo.is_none() {
            return;
        }
        let banda = rig_control::bands::amateur_band(dial_hz).unwrap_or_default();
        if d.radio.0 != banda || !d.radio.1.eq_ignore_ascii_case(modo) {
            d.radio = (banda.to_string(), modo.to_ascii_uppercase());
        }
    }

    /// Consiglio di banda e modo: per ogni banda ammessa stazioni in ascolto, moltiplicatori nuovi, rate stimato e affollamento; la banda
    /// migliore e il modo (FT4 se il pile-up e' fitto). Calcolato al piu' ogni 10 s. `None` a contest spento.
    ///
    /// Rate stimato: il piu' alto fra i QSO validi fatti su quella banda nell'ultima ora e la meta' delle stazioni non doppie sentite negli
    /// ultimi 15 minuti riportata all'ora (x4 /2), con un tetto di 120. Punteggio della banda: rate stimato + 15 per ogni moltiplicatore
    /// nuovo in ascolto. Una banda senza decodifiche recenti non si consiglia (non sappiamo com'e').
    pub fn consiglio(&self) -> Option<serde_json::Value> {
        let (_, grid) = (self.stazione)();
        let ora = ora_ms();
        let mut d = self.lock();
        d.stato.attivo.as_ref()?;
        if let Some((t, v)) = &d.consiglio
            && ora - t < 10_000
        {
            return Some(v.clone());
        }
        self.carica_log(&mut d);
        let id = d.stato.attivo.clone()?;
        let r = d.regole.iter().find(|r| r.id() == id)?.clone();
        let ctx = self.contesto(&d, &r, &grid);
        let (banda_ora, modo_ora) = d.radio.clone();
        let validi: Vec<&ContestQso> = d.log.iter().filter(|q| q.stato != Stato::Errore).collect();
        let modo_di = |m: &str| if m.is_empty() { "FT8".to_string() } else { m.to_string() };
        let mut bande = Vec::new();
        for b in &r.bande.ammesse {
            let qui: Vec<&Sentito> = d.sentiti.iter().filter(|x| x.banda.eq_ignore_ascii_case(b) && ora - x.t < ASCOLTO_MS).collect();
            let lista: Vec<(String, String, String, String)> = qui.iter().map(|x| (x.call.clone(), x.grid.clone(), b.clone(), modo_di(&x.modo))).collect();
            let val = punteggio::valori(&r, &ctx, &d.log, &lista);
            let non_doppi = val.iter().filter(|v| !v.doppio && !v.fuori_regola).count();
            let mut molt: Vec<String> = val.iter().flat_map(|v| v.molt_nuovi.iter().cloned()).collect();
            molt.sort();
            molt.dedup();
            let qso_ora = validi.iter().filter(|q| q.band.eq_ignore_ascii_case(b) && ora - q.time_ms < 3_600_000 && q.time_ms <= ora).count() as u32;
            let stimato = qso_ora.max(((non_doppi * 2) as u32).min(120));
            let folla = qui.iter().filter(|x| ora - x.t < FOLLA_MS).count();
            let modo = if r.modo_ammesso("FT4") && folla >= FOLLA_FT4 {
                "FT4".to_string()
            } else if r.modo_ammesso("FT8") {
                "FT8".to_string()
            } else {
                r.modi.ammessi.first().cloned().unwrap_or_default()
            };
            let khz = r.bande.frequenze.get(b).and_then(|f| f.iter().find(|(m, _)| m.eq_ignore_ascii_case(&modo)).map(|(_, k)| *k)).or_else(|| frequenza_standard(b, &modo));
            let punteggio = stimato + 15 * molt.len() as u32;
            bande.push(serde_json::json!({
                "banda": b, "sentiti": qui.len(), "non_doppi": non_doppi, "molt_nuovi": molt, "qso_ora": qso_ora, "rate_stimato": stimato,
                "affollamento": folla, "modo": modo, "khz": khz, "punteggio": punteggio, "dati": !qui.is_empty(),
            }));
        }
        let migliore = bande
            .iter()
            .filter(|v| v["dati"].as_bool() == Some(true))
            .max_by_key(|v| v["punteggio"].as_u64().unwrap_or(0))
            .map(|v| (v["banda"].as_str().unwrap_or_default().to_string(), v["modo"].as_str().unwrap_or_default().to_string()));
        // cambi di banda nell'ora UTC in corso, contati dal log del contest (solo se il regolamento li limita)
        let limite = r.restrizioni.cambi_banda_ora;
        let cambi = limite.map(|_| {
            let inizio_ora = ora - ora.rem_euclid(3_600_000);
            let mut nell_ora: Vec<&&ContestQso> = validi.iter().filter(|q| q.time_ms >= inizio_ora && q.time_ms <= ora).collect();
            nell_ora.sort_by_key(|q| q.time_ms);
            let mut ultima = validi.iter().filter(|q| q.time_ms < inizio_ora).max_by_key(|q| q.time_ms).map(|q| q.band.to_ascii_lowercase());
            let mut n = 0u32;
            for q in nell_ora {
                let b = q.band.to_ascii_lowercase();
                if ultima.as_ref().is_some_and(|u| *u != b) {
                    n += 1;
                }
                ultima = Some(b);
            }
            n
        });
        let v = serde_json::json!({
            "banda": banda_ora, "modo": modo_ora, "bande": bande,
            "migliore": migliore.as_ref().map(|m| m.0.clone()), "modo_migliore": migliore.map(|m| m.1),
            "cambi_banda_ora": cambi, "cambi_banda_limite": limite, "ora_ms": ora,
        });
        d.consiglio = Some((ora, v.clone()));
        Some(v)
    }

    /// La mappa dei moltiplicatori di una banda: tipo del primo moltiplicatore, quelli gia' presi e le stazioni in ascolto negli ultimi
    /// 15 minuti con il loro valore (moltiplicatori nuovi, doppio). `None` a contest spento.
    pub fn mappa(&self, banda: &str) -> Option<serde_json::Value> {
        let (_, grid) = (self.stazione)();
        let ora = ora_ms();
        let mut d = self.lock();
        self.carica_log(&mut d);
        let id = d.stato.attivo.clone()?;
        let r = d.regole.iter().find(|r| r.id() == id)?.clone();
        let ctx = self.contesto(&d, &r, &grid);
        let tipo = r.moltiplicatori.first().map(|m| m.tipo.clone()).unwrap_or_default();
        let lavorati = punteggio::molt_lavorati(&r, &ctx, &d.log, banda).into_iter().next().map(|(_, v)| v).unwrap_or_default();
        let qui: Vec<Sentito> = d.sentiti.iter().filter(|x| x.banda.eq_ignore_ascii_case(banda) && ora - x.t < ASCOLTO_MS).cloned().collect();
        let lista: Vec<(String, String, String, String)> = qui.iter().map(|x| (x.call.clone(), x.grid.clone(), banda.to_string(), if x.modo.is_empty() { "FT8".into() } else { x.modo.clone() })).collect();
        let val = punteggio::valori(&r, &ctx, &d.log, &lista);
        let sentiti: Vec<serde_json::Value> = qui
            .iter()
            .zip(val)
            .map(|(x, v)| serde_json::json!({ "call": x.call, "grid": x.grid, "snr": x.snr, "t": x.t, "cq": x.cq, "molt_nuovi": v.molt_nuovi, "doppio": v.doppio, "punti": v.punti }))
            .collect();
        Some(serde_json::json!({ "banda": banda, "tipo": tipo, "griglia": tipo == "grid4" || tipo == "campo2", "mio_grid": ctx.mio_grid, "lavorati": lavorati, "sentiti": sentiti }))
    }

    /// Le edizioni di cui c'e' un log del contest su disco: `(id, edizione)` (`aaaa-mm-gg` o, per i vecchi log, `aaaa`), la piu' recente prima.
    pub fn edizioni(&self) -> Vec<(String, String)> {
        let mut out: Vec<(String, String)> = std::fs::read_dir(&self.dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| nome_edizione(&e.file_name().to_string_lossy()))
            .collect();
        out.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        out
    }

    /// L'analisi dopo la gara di un'edizione (o del contest attivo): totali, QSO ora per ora, per banda il rate migliore e il tempo perso,
    /// e i dati per un consiglio. Si calcola solo quando la pagina la chiede (linguetta «Analisi» aperta).
    pub fn analisi(&self, scelta: Option<(String, String)>) -> Option<serde_json::Value> {
        let (_, grid) = (self.stazione)();
        let mut d = self.lock();
        self.carica_log(&mut d);
        let (id, anno) = match scelta {
            Some(x) => x,
            None => d.log_di.clone()?,
        };
        // l'edizione arriva dalla rete e finisce nel nome di un file: solo cifre e trattini
        if anno.is_empty() || anno.len() > 10 || !anno.chars().all(|c| c.is_ascii_digit() || c == '-') {
            return None;
        }
        let r = d.regole.iter().find(|r| r.id() == id)?.clone();
        // il log in memoria se e' quello attivo (puo' non essere ancora sul disco), altrimenti il file dell'edizione
        let log: Vec<ContestQso> = if d.log_di.as_ref() == Some(&(id.clone(), anno.clone())) {
            d.log.clone()
        } else {
            self.leggi_log(&id, &anno).unwrap_or_default()
        };
        let ctx = self.contesto(&d, &r, &grid);
        drop(d);
        let (valutati, tot) = punteggio::valuta_log(&r, &ctx, &log);
        let pausa_ms = i64::from(r.calendario.pausa_minima_min.unwrap_or(60)) * 60_000;
        let mut buoni: Vec<&Valutato> = valutati.iter().filter(|v| !v.doppio && v.qso.stato != Stato::Errore).collect();
        buoni.sort_by_key(|v| v.qso.time_ms);
        const ORA: i64 = 3_600_000;
        // QSO ora per ora (ore UTC intere), con quelli di ogni banda
        let mut ore: Vec<(i64, BTreeMap<String, u32>)> = Vec::new();
        if let (Some(a), Some(b)) = (buoni.first(), buoni.last()) {
            let (h0, h1) = (a.qso.time_ms.div_euclid(ORA), b.qso.time_ms.div_euclid(ORA));
            for h in h0..=h1.min(h0 + 240) {
                ore.push((h * ORA, BTreeMap::new()));
            }
            for v in &buoni {
                let i = (v.qso.time_ms.div_euclid(ORA) - h0) as usize;
                if let Some((_, m)) = ore.get_mut(i) {
                    *m.entry(v.qso.band.clone()).or_default() += 1;
                }
            }
        }
        let prevalente = |m: &BTreeMap<String, u32>| m.iter().max_by_key(|(_, n)| **n).map(|(b, n)| (b.clone(), *n));
        // tempo perso: i buchi fra due QSO piu' lunghi di 15 minuti ma piu' corti di una pausa valida, dati alla banda su cui si era
        let mut perso: BTreeMap<String, (i64, u32)> = BTreeMap::new();
        let mut buco_max: Option<(i64, i64, String)> = None;
        for w in buoni.windows(2) {
            let gap = w[1].qso.time_ms - w[0].qso.time_ms;
            if gap >= 15 * 60_000 && gap < pausa_ms {
                let e = perso.entry(w[0].qso.band.clone()).or_default();
                e.0 += gap;
                e.1 += 1;
                if buco_max.as_ref().is_none_or(|(_, g, _)| gap > *g) {
                    buco_max = Some((w[0].qso.time_ms, gap, w[0].qso.band.clone()));
                }
            }
        }
        let bande: Vec<serde_json::Value> = tot
            .per_banda
            .iter()
            .map(|b| {
                let migliore = ore.iter().filter_map(|(h, m)| m.get(&b.banda).map(|n| (*n, *h))).max_by_key(|(n, h)| (*n, -*h));
                let (perso_ms, buchi) = perso.get(&b.banda).copied().unwrap_or_default();
                serde_json::json!({
                    "banda": b.banda, "qso": b.qso, "molt": b.molt, "punti": b.punti,
                    "rate_migliore": migliore.map(|x| x.0).unwrap_or(0), "ora_migliore_ms": migliore.map(|x| x.1),
                    "perso_min": perso_ms / 60_000, "buchi": buchi,
                })
            })
            .collect();
        // consiglio: l'ora in cui una banda rendeva molto di piu' (nel suo momento migliore) di quella su cui si era; vale solo con
        // almeno il doppio dei QSO e almeno 10 all'ora, cosi' non si consiglia per differenze piccole
        let mut consiglio: Option<serde_json::Value> = None;
        let mut scarto = 0i64;
        for (h, m) in &ore {
            let Some((su, n)) = prevalente(m) else { continue };
            for b in &tot.per_banda {
                if b.banda == su {
                    continue;
                }
                let Some((rb, hb)) = ore.iter().filter_map(|(h2, m2)| m2.get(&b.banda).map(|x| (*x, *h2))).max_by_key(|(x, _)| *x) else { continue };
                if rb >= 10 && rb >= 2 * n && i64::from(rb - n) > scarto {
                    scarto = i64::from(rb - n);
                    consiglio = Some(serde_json::json!({ "tipo": "banda", "ora_ms": h, "banda_su": su, "qso_su": n, "banda_meglio": b.banda, "rate_meglio": rb, "ora_meglio_ms": hb }));
                }
            }
        }
        if consiglio.is_none()
            && let Some((t, gap, banda)) = buco_max
        {
            consiglio = Some(serde_json::json!({ "tipo": "buco", "ora_ms": t, "minuti": gap / 60_000, "banda": banda }));
        }
        let righe_ore: Vec<serde_json::Value> = ore
            .iter()
            .map(|(h, m)| serde_json::json!({ "ora_ms": h, "qso": m.values().sum::<u32>(), "banda": prevalente(m).map(|x| x.0).unwrap_or_default() }))
            .collect();
        Some(serde_json::json!({
            "id": id, "anno": anno, "nome": r.contest.nome,
            "totali": tot,
            "ore_attive": punteggio::ore_attive(&valutati, r.calendario.pausa_minima_min.unwrap_or(60)),
            "ore": righe_ore,
            "bande": bande,
            "consiglio": consiglio,
        }))
    }

    fn contesto(&self, d: &Dentro, r: &Regole, grid: &str) -> Contesto {
        let imp = imp_di(&d.stato, r);
        let g = if imp.grid6.len() == 6 && r.punti.tipo == "km" { imp.grid6.clone() } else { grid.to_string() };
        let paese = self.paese.lock().unwrap_or_else(|e| e.into_inner()).clone();
        let continente = self.continente.lock().unwrap_or_else(|e| e.into_inner()).clone();
        // il nominativo di gara se c'e', altrimenti quello della stazione
        let mio_call = if imp.call_contest.trim().is_empty() { (self.stazione)().0 } else { imp.call_contest.trim().to_ascii_uppercase() };
        Contesto { mio_grid: g, potenza_w: imp.potenza_w, bonus: imp.bonus, paese, continente, mio_call }
    }

    /// Il log del contest attivo con i numeri di ogni QSO e i totali.
    pub fn log(&self) -> Option<(Vec<Valutato>, punteggio::Totali)> {
        let (_, grid) = (self.stazione)();
        let mut d = self.lock();
        self.carica_log(&mut d);
        let id = d.stato.attivo.clone()?;
        let r = d.regole.iter().find(|r| r.id() == id)?.clone();
        let ctx = self.contesto(&d, &r, &grid);
        Some(punteggio::valuta_log(&r, &ctx, &d.log))
    }

    /// La finestra del contest attivo: quella scelta a mano o quella del calendario.
    fn finestra(d: &Dentro, r: &Regole, ora: i64) -> Option<Finestra> {
        let imp = d.stato.impostazioni.get(r.id());
        if let (Some(a), Some(b)) = (imp.and_then(|i| i.inizio_ms), imp.and_then(|i| i.fine_ms))
            && b > a
        {
            return Some(Finestra { inizio_ms: a, fine_ms: b });
        }
        calendario::attuale(&r.calendario, ora)
    }

    /// Tutto quello che la barra del punteggio e la pagina mostrano.
    pub fn stato(&self) -> serde_json::Value {
        let (call, grid) = (self.stazione)();
        let ora = ora_ms();
        let mut d = self.lock();
        self.carica_log(&mut d);
        // per ogni contest anche le date dell'edizione in corso o prossima, calcolate ogni anno dalla regola del calendario
        let elenco: Vec<serde_json::Value> = d
            .regole
            .iter()
            .map(|r| {
                serde_json::json!({
                    "id": r.id(), "nome": r.contest.nome, "famiglia": r.contest.famiglia, "edizione": r.contest.edizione,
                    "finestra": calendario::prossima(&r.calendario, ora), "da_verificare": !r.contest.da_verificare.is_empty(),
                    "url_regole": r.contest.url_regole, "ore_massime": r.calendario.ore_massime, "pausa_minima_min": r.calendario.pausa_minima_min,
                    // i modi ammessi: le Impostazioni mostrano ogni contest nel ramo del suo modo (FT8 · FT4, VHF · EME)
                    "modi": r.modi.ammessi,
                })
            })
            .collect();
        let Some(id) = d.stato.attivo.clone() else {
            return serde_json::json!({ "attivo": null, "contest": elenco, "errori_regole": d.errori_regole });
        };
        let Some(r) = d.regole.iter().find(|r| r.id() == id).cloned() else {
            return serde_json::json!({ "attivo": null, "contest": elenco, "errori_regole": d.errori_regole });
        };
        let ctx = self.contesto(&d, &r, &grid);
        let (valutati, tot) = punteggio::valuta_log(&r, &ctx, &d.log);
        let rt = punteggio::ritmo(&valutati, ora, 12);
        let fin = Self::finestra(&d, &r, ora);
        let (fase, restanti_h) = match fin {
            Some(f) if ora < f.inizio_ms => ("prima", (f.fine_ms - f.inizio_ms) as f64 / 3_600_000.0),
            Some(f) if ora < f.fine_ms => ("in-corso", (f.fine_ms - ora) as f64 / 3_600_000.0),
            Some(_) => ("finito", 0.0),
            None => ("sempre", 0.0),
        };
        let ore_usate = punteggio::ore_attive(&valutati, r.calendario.pausa_minima_min.unwrap_or(60));
        let restanti_attive = r.calendario.ore_massime.map_or(restanti_h, |m| (m - ore_usate).max(0.0).min(restanti_h));
        let previsto = punteggio::previsto(&r, &ctx, &tot, rt.rate60, restanti_attive);
        let imp = imp_di(&d.stato, &r);
        let setup = self.setup_di(&d);
        let inviato = setup.as_ref().map(|s| scambio_inviato(&r, s, &grid)).unwrap_or_default();
        let cat = r.categoria(&imp.categoria);
        let categoria = if r.opzioni_categoria.classe_fd { classe_fd(&r, &imp) } else { cat.map(|c| c.sigla().to_string()).unwrap_or_default() };
        let categoria_nome = cat.map(|c| c.nome.clone()).unwrap_or_default();
        let non_contest: Vec<String> = d.non_contest.iter().filter(|(_, t)| ora - *t < 30 * 60_000).map(|(c, _)| c.clone()).collect();
        serde_json::json!({
            "attivo": id,
            "contest": elenco,
            "errori_regole": d.errori_regole,
            "regole": r,
            "impostazioni": imp,
            "sequenza": setup.map(|s| serde_json::json!({ "attivita": s.activity, "parola_cq": s.cq_word, "scambio": s.exchange, "seriale": s.next_serial })),
            "stazione": { "call": call, "grid": grid },
            "scambio_inviato": inviato,
            "categoria": categoria,
            "categoria_nome": categoria_nome,
            "finestra": fin,
            "fase": fase,
            "ore_restanti": restanti_h,
            "ore_usate": ore_usate,
            "ore_massime": r.calendario.ore_massime,
            "totali": tot,
            "rate10": rt.rate10,
            "rate60": rt.rate60,
            "per_ora": rt.per_ora,
            "previsto": previsto,
            "fox_vietato": r.vieta_fox(),
            "non_contest": non_contest,
            "ora_ms": ora,
        })
    }

    /// Il valore di una lista di stazioni (decodifiche): `(nominativo, locatore, banda, modo)`.
    pub fn valuta(&self, stazioni: &[(String, String, String, String)]) -> Vec<punteggio::Valore> {
        let (_, grid) = (self.stazione)();
        let mut d = self.lock();
        self.carica_log(&mut d);
        let Some(id) = d.stato.attivo.clone() else { return Vec::new() };
        let Some(r) = d.regole.iter().find(|r| r.id() == id).cloned() else { return Vec::new() };
        let ctx = self.contesto(&d, &r, &grid);
        punteggio::valori(&r, &ctx, &d.log, stazioni)
    }

    /// I moltiplicatori gia' presi su una banda.
    pub fn lavorati(&self, banda: &str) -> Vec<(String, Vec<String>)> {
        let (_, grid) = (self.stazione)();
        let mut d = self.lock();
        self.carica_log(&mut d);
        let Some(id) = d.stato.attivo.clone() else { return Vec::new() };
        let Some(r) = d.regole.iter().find(|r| r.id() == id).cloned() else { return Vec::new() };
        let ctx = self.contesto(&d, &r, &grid);
        punteggio::molt_lavorati(&r, &ctx, &d.log, banda)
    }

    /// Corregge a mano un QSO del log (scambio ricevuto, locatore, stato). Resta segnato come modificato.
    pub fn modifica(&self, n: u32, exch_rcvd: Option<String>, grid: Option<String>, stato: Option<Stato>) -> Result<(), String> {
        let mut d = self.lock();
        self.carica_log(&mut d);
        let q = d.log.iter_mut().find(|q| q.n == n).ok_or("QSO non trovato")?;
        if let Some(x) = exch_rcvd {
            q.exch_rcvd = x.trim().to_ascii_uppercase();
            q.modificato = true;
        }
        if let Some(g) = grid {
            q.grid = g.trim().to_string();
            q.modificato = true;
        }
        if let Some(s) = stato {
            q.stato = s;
        }
        self.salva_log(&d);
        Ok(())
    }

    /// Ricomincia il log del contest attivo: il vecchio resta come copia (`.bak`). Restituisce la sequenza con il seriale ripartito.
    pub fn azzera_log(&self) -> Option<ContestSetup> {
        let mut d = self.lock();
        self.carica_log(&mut d);
        if let Some((id, ed)) = d.log_di.clone() {
            let f = self.file_log(&id, &ed);
            let _ = std::fs::rename(&f, f.with_extension(format!("{}.bak", ora_ms())));
        }
        d.log.clear();
        self.salva_log(&d);
        self.setup_di(&d)
    }

    fn intestazione(&self, r: &Regole, imp: &Impostazioni, call: &str, grid: &str) -> Intestazione {
        let cat = r.categoria(&imp.categoria).cloned().or_else(|| r.categorie.first().cloned());
        let grid = if imp.grid6.len() == 6 { imp.grid6.clone() } else { grid.to_string() };
        let call = if imp.call_contest.trim().is_empty() { call.to_string() } else { imp.call_contest.trim().to_ascii_uppercase() };
        let operatore = cat.as_ref().map(|c| c.operatore.clone()).unwrap_or_else(|| "SINGLE-OP".into());
        // chi gareggia da solo: l'operatore e' la stazione stessa
        let operatori = if imp.operatori.trim().is_empty() && operatore != "MULTI-OP" { call.clone() } else { normalizza_operatori(&imp.operatori) };
        let trasmettitori = if r.opzioni_categoria.classe_fd {
            match imp.trasmettitori_n.max(1) {
                1 => "ONE".to_string(),
                2 => "TWO".to_string(),
                _ => "UNLIMITED".to_string(),
            }
        } else {
            cat.as_ref().map(|c| c.trasmettitori.clone()).unwrap_or_else(|| "ONE".into())
        };
        let stazione = match cat.as_ref() {
            Some(c) if !c.stazione.is_empty() => c.stazione.clone(),
            Some(c) if c.operatore == "ROVER" => "ROVER".into(),
            _ => "FIXED".into(),
        };
        Intestazione {
            call,
            stazione,
            indirizzo: imp.indirizzo.clone(),
            email: imp.email.clone(),
            grid,
            operatore: if operatore == "ROVER" { "SINGLE-OP".into() } else { operatore },
            potenza: if !imp.potenza.is_empty() && r.opzioni_categoria.potenze.contains(&imp.potenza) {
                imp.potenza.clone()
            } else {
                cat.as_ref().map(|c| c.potenza.clone()).unwrap_or_else(|| "LOW".into())
            },
            trasmettitori,
            bande: cat.as_ref().map(|c| c.bande.clone()).unwrap_or_else(|| "ALL".into()),
            assistenza: if imp.assistenza.is_empty() { cat.as_ref().map(|c| c.assistenza.clone()).unwrap_or_default() } else { imp.assistenza.clone() },
            operatori,
            club: imp.club.clone(),
            nome: imp.nome.clone(),
            location: imp.location.clone(),
            soapbox: imp.soapbox.clone(),
            antenna: imp.antenna.clone(),
            cabrillo_contest: imp.cabrillo_contest.clone(),
            tempo: imp.categoria_tempo.trim().to_string(),
            overlay: imp.categoria_overlay.trim().to_string(),
            versione: self.versione.clone(),
        }
    }

    /// Il file da mandare allo sponsor nel formato delle regole (o in quello chiesto: `cabrillo`, `edi`, `adif`), il nome proposto e
    /// l'elenco dei problemi trovati dal controllo.
    pub fn esporta(&self, formato: Option<&str>, banda: Option<&str>) -> Option<(String, String, Vec<String>)> {
        let (call, grid) = (self.stazione)();
        let mut d = self.lock();
        self.carica_log(&mut d);
        let id = d.stato.attivo.clone()?;
        let r = d.regole.iter().find(|r| r.id() == id)?.clone();
        let imp = imp_di(&d.stato, &r);
        let ctx = self.contesto(&d, &r, &grid);
        let (valutati, tot) = punteggio::valuta_log(&r, &ctx, &d.log);
        let h = self.intestazione(&r, &imp, &call, &grid);
        let problemi = export::controlla(&r, &h, &valutati);
        let formato = formato.unwrap_or(&r.export.formato);
        let base = format!("{}-{}", h.call.replace('/', "_"), r.id());
        Some(match formato {
            "edi" => {
                let banda = banda.map(str::to_string).or_else(|| valutati.first().map(|v| v.qso.band.clone())).unwrap_or_else(|| r.bande.ammesse[0].clone());
                (export::edi(&r, &h, &valutati, &banda), format!("{base}-{banda}.edi"), problemi)
            }
            "adif" => (export::adif(&r, &h, &valutati), format!("{base}.adi"), problemi),
            _ => (export::cabrillo(&r, &h, &valutati, &tot), format!("{base}.log"), problemi),
        })
    }

    /// I contest dei prossimi `giorni` giorni (e quelli in corso), in ordine di inizio.
    pub fn calendario(&self, giorni: i64) -> Vec<serde_json::Value> {
        let ora = ora_ms();
        let d = self.lock();
        let mut out: Vec<(i64, serde_json::Value)> = Vec::new();
        for r in &d.regole {
            let anno = anno_di(ora);
            for a in anno..=anno + 1 {
                for f in calendario::finestre(&r.calendario, a) {
                    if f.fine_ms > ora && f.inizio_ms < ora + giorni * 86_400_000 {
                        out.push((f.inizio_ms, serde_json::json!({ "id": r.id(), "nome": r.contest.nome, "inizio_ms": f.inizio_ms, "fine_ms": f.fine_ms, "da_verificare": !r.contest.da_verificare.is_empty() })));
                    }
                }
            }
        }
        out.sort_by_key(|(t, _)| *t);
        out.dedup_by(|a, b| a.1 == b.1);
        out.into_iter().map(|(_, v)| v).collect()
    }

    /// Ascolta i QSO registrati e le decodifiche sul bus. Va chiamato dentro un runtime Tokio.
    pub fn spawn(self: &Arc<Self>, bus: &Bus) {
        let me = Arc::clone(self);
        let mut events = bus.subscribe_events();
        let commands = bus.commands();
        tokio::spawn(async move {
            loop {
                match events.recv().await {
                    Ok(Event::QsoLogged(q)) => {
                        if let Some(n) = me.registra(&q) {
                            tracing::info!("contest: QSO {n} con {} nel log del contest", q.call);
                        }
                        // il seriale del motore va avanti da solo a ogni QSO, anche a uno che il log del contest scarta (fuori orario, dalla
                        // pagina Stazione): si riallinea sempre al log del contest
                        if let Some(s) = me.setup() {
                            let _ = commands.try_send(aurora_core::Command::Contest(Some(s)));
                        }
                    }
                    Ok(Event::Decode(m)) => me.nota_decodifica(&m),
                    Ok(Event::Status(s)) => me.nota_radio(s.dial_hz, &s.mode),
                    Ok(_) | Err(RecvError::Lagged(_)) => {}
                    Err(RecvError::Closed) => break,
                }
            }
        });
    }
}

/// Le impostazioni proposte dalle regole.
pub fn predefinite(r: &Regole) -> Impostazioni {
    // Field Day: dall'Italia si parte da una stazione di casa (classe D)
    let fd = r.opzioni_categoria.classe_fd.then(|| r.categoria("D")).flatten();
    let cat = fd.or_else(|| r.categorie.iter().find(|c| c.potenza == "LOW")).or_else(|| r.categorie.first());
    Impostazioni {
        scambio: r.scambio.predefinito.clone(),
        seriale_partenza: 1,
        categoria: cat.map(|c| c.id.clone()).unwrap_or_default(),
        potenza_w: 100.0,
        modo: r.modi.ammessi.first().cloned().unwrap_or_else(|| "FT8".into()),
        hold_tx: true,
        rrr: r.messaggi.chiusura == "RRR",
        trasmettitori_n: 1,
        location: if r.contest.sponsor == "ARRL" || r.contest.id == "ww-digi" { "DX".into() } else { String::new() },
        ..Default::default()
    }
}

/// Gli operatori scritti con spazi o virgole diventano l'elenco del Cabrillo (`IU3ABC IU3XYZ`).
fn normalizza_operatori(s: &str) -> String {
    s.split(|c: char| c == ',' || c == ';' || c.is_whitespace()).filter(|x| !x.is_empty()).map(str::to_ascii_uppercase).collect::<Vec<_>>().join(" ")
}

/// Lo scambio fisso che si manda: per il Field Day la classe scelta nella categoria seguita dalla sezione (`2A EMA`, `1D DX`).
fn scambio_fisso(r: &Regole, imp: &Impostazioni) -> String {
    let base = if imp.scambio.is_empty() { r.scambio.predefinito.clone() } else { imp.scambio.clone() };
    if !r.opzioni_categoria.classe_fd {
        return base;
    }
    let resto: Vec<&str> = base.split_whitespace().skip(1).collect();
    let sezione = if !resto.is_empty() {
        resto.join(" ")
    } else if !imp.location.trim().is_empty() {
        imp.location.trim().to_ascii_uppercase()
    } else {
        "DX".into()
    };
    format!("{} {sezione}", classe_fd(r, imp))
}

fn qso_attivita_sconosciuta(a: &str) -> bool {
    !["standard", "na-vhf", "arrl-digi", "ww-digi", "eu-vhf", "field-day", "roundup"].contains(&a)
}

/// Il seriale dentro uno scambio inviato (`570123 IO91WM` -> 123, `559 0013` -> 13); 0 se non c'e'.
fn seriale_di(x: &str) -> u32 {
    let t: Vec<&str> = x.split_whitespace().collect();
    match t.as_slice() {
        [a, _] if a.len() == 6 && a.chars().all(|c| c.is_ascii_digit()) => a[2..].parse().unwrap_or(0),
        [_, b] if b.len() == 4 && b.chars().all(|c| c.is_ascii_digit()) => b.parse().unwrap_or(0),
        _ => 0,
    }
}

/// `log-<id>-<edizione>.json` -> `(id, edizione)`: edizione `aaaa-mm-gg` o, per i vecchi log, `aaaa`. Le copie azzerate (`.<ora>.bak`) no.
fn nome_edizione(nome: &str) -> Option<(String, String)> {
    let base = nome.strip_prefix("log-")?.strip_suffix(".json")?;
    let cifre = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit());
    if base.len() > 11 && base.is_char_boundary(base.len() - 11) {
        let (id, ed) = base.split_at(base.len() - 11);
        let p: Vec<&str> = ed[1..].split('-').collect();
        if ed.starts_with('-') && p.len() == 3 && p[0].len() == 4 && p[1].len() == 2 && p[2].len() == 2 && p.iter().all(|x| cifre(x)) {
            return Some((id.to_string(), ed[1..].to_string()));
        }
    }
    let (id, anno) = base.rsplit_once('-')?;
    (anno.len() == 4 && cifre(anno)).then(|| (id.to_string(), anno.to_string()))
}

/// La parola dopo i nominativi ripete lo scambio: «R» da sola o «R» + rapporto (`R+05`, `R-10`, `R579`), non un locatore che comincia
/// per R (`RO51`) ne' una chiusura (`RR73`, `RRR`).
fn ripete_scambio(p: &str) -> bool {
    p == "R" || p.strip_prefix('R').and_then(|x| x.chars().next()).is_some_and(|c| c == '+' || c == '-' || c.is_ascii_digit())
}

fn is_rapporto(t: &str) -> bool {
    let t = t.strip_prefix('R').unwrap_or(t);
    t.len() == 3 && (t.starts_with('+') || t.starts_with('-')) && t[1..].chars().all(|c| c.is_ascii_digit())
}

/// Lo scambio che si manda adesso, come lo vede la barra (`IO91`, `1D DX`, `5x9 0013`, `5x0013 IO91WM`, `rapporto + IO91`).
fn scambio_inviato(r: &Regole, s: &ContestSetup, grid: &str) -> String {
    let g4: String = grid.chars().take(4).collect::<String>().to_ascii_uppercase();
    match s.activity.as_str() {
        "na-vhf" | "arrl-digi" | "ww-digi" => g4,
        "field-day" => s.exchange.clone(),
        "roundup" => {
            let x = if s.exchange.is_empty() || s.exchange == "#" || s.exchange == "DX" { format!("{:04}", s.next_serial) } else { s.exchange.clone() };
            format!("5x9 {x}")
        }
        "eu-vhf" => format!("5x{:04} {}", s.next_serial, if s.grid6.len() == 6 { s.grid6.to_ascii_uppercase() } else { grid.to_ascii_uppercase() }),
        _ => {
            if r.scambio.inviato.iter().any(|c| c.starts_with("grid")) { format!("dB {g4}") } else { "dB".into() }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn servizio(nome: &str) -> Arc<ContestService> {
        let dir = std::env::temp_dir().join(format!("aurora-contest-{nome}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        ContestService::open(&dir, Arc::new(|| ("IU3ABC".to_string(), "IO91wm".to_string())), "prova")
    }

    /// Attivare, registrare un QSO, vedere il punteggio e il Cabrillo; disattivare restituisce cio' che c'era prima.
    #[test]
    fn ciclo_completo() {
        let s = servizio("ciclo");
        let setup = s.attiva("ww-digi", Some(Ripristino { controlli: vec![("fox".into(), true)], modo: "FT8".into() })).expect("attiva");
        assert_eq!(setup.activity, "ww-digi");
        assert_eq!(setup.cq_word, "WW");
        // sabato 29 agosto 2026 alle 14:00 UTC, durante la gara (comincia alle 12:00)
        let q = QsoRecord { call: "K1ABC".into(), grid: "FN42".into(), band: "20m".into(), mode: "FT8".into(), freq_hz: 14_074_000, exch_sent: "IO91".into(), exch_rcvd: "FN42".into(), rst_rcvd: "-10".into(), time_on_unix_ms: 1_788_012_000_000, ..Default::default() };
        // un QSO avviato dalla pagina Stazione non entra nel log del contest
        assert_eq!(s.registra(&q), None);
        s.imposta_origine(true);
        // ne' uno fatto prima dell'inizio della gara (10:40 UTC)
        assert_eq!(s.registra(&QsoRecord { time_on_unix_ms: 1_788_000_000_000, ..q.clone() }), None);
        assert_eq!(s.registra(&q), Some(1));
        let (v, t) = s.log().expect("log");
        assert_eq!(v.len(), 1);
        assert_eq!(t.qso, 1);
        let (testo, nome, _) = s.esporta(None, None).expect("cabrillo");
        assert!(testo.contains("K1ABC"));
        assert!(nome.ends_with(".log"));
        let r = s.disattiva().expect("ripristino");
        assert_eq!(r.controlli, vec![("fox".to_string(), true)]);
        assert!(s.attivo().is_none());
    }

    /// Il seriale riparte dopo l'ultimo usato.
    #[test]
    fn seriale_dal_log() {
        assert_eq!(seriale_di("570123 IO91WM"), 123);
        assert_eq!(seriale_di("559 0013"), 13);
        assert_eq!(seriale_di("IO91"), 0);
    }

    /// Field Day: la classe nasce da trasmettitori + lettera e va nello scambio; la vecchia «1D» salvata si legge ancora.
    #[test]
    fn classe_del_field_day() {
        let (regole, _) = regole::carica(None);
        let r = regole.into_iter().find(|r| r.id() == "arrl-fd").expect("fd");
        let mut imp = predefinite(&r);
        imp.categoria = "A".into();
        imp.trasmettitori_n = 2;
        imp.scambio = "1D EMA".into();
        assert_eq!(scambio_fisso(&r, &imp), "2A EMA");
        let mut st = StatoFile::default();
        let mut vecchia = predefinite(&r);
        vecchia.categoria = "3E".into();
        st.impostazioni.insert("arrl-fd".into(), vecchia);
        let i = imp_di(&st, &r);
        assert_eq!((i.categoria.as_str(), i.trasmettitori_n), ("E", 3));
    }

    /// Gli operatori separati da virgole o spazi diventano un elenco pulito.
    #[test]
    fn elenco_operatori() {
        assert_eq!(normalizza_operatori("iu3abc, iu3xyz;k1abc"), "IU3ABC IU3XYZ K1ABC");
    }

    /// «R + rapporto» ripete lo scambio, un locatore che comincia per R no.
    #[test]
    fn ripete_solo_r_e_rapporto() {
        assert!(ripete_scambio("R"));
        assert!(ripete_scambio("R+05"));
        assert!(ripete_scambio("R-10"));
        assert!(ripete_scambio("R579"));
        assert!(!ripete_scambio("RO51"));
        assert!(!ripete_scambio("RR73"));
        assert!(!ripete_scambio("RRR"));
    }

    /// I nomi dei file del log: per edizione (giorno d'inizio) e, per i vecchi, per anno; le copie azzerate no.
    #[test]
    fn nomi_dei_log() {
        assert_eq!(nome_edizione("log-darc-ft4-2026-11-10.json"), Some(("darc-ft4".into(), "2026-11-10".into())));
        assert_eq!(nome_edizione("log-ww-digi-2026.json"), Some(("ww-digi".into(), "2026".into())));
        assert_eq!(nome_edizione("log-ww-digi-2026-08-29.1788.bak"), None);
        assert_eq!(nome_edizione("stato.json"), None);
    }

    /// Finita la gara il log resta (anche riaprendo il programma), la fase e' «finito», il seriale e' QSO a log + 1; cambiare le date a
    /// mano non toglie QSO.
    #[test]
    fn il_log_resta_dopo_la_fine() {
        let s = servizio("dopo-fine");
        s.attiva("ww-digi", None).expect("attiva");
        let ora = ora_ms();
        let mut imp = s.impostazioni("ww-digi").expect("imp");
        imp.inizio_ms = Some(ora - 5 * 3_600_000);
        imp.fine_ms = Some(ora - 3_600_000);
        s.salva_impostazioni("ww-digi", imp.clone()).expect("salva");
        s.imposta_origine(true);
        let q = QsoRecord { call: "K1ABC".into(), grid: "FN42".into(), band: "20m".into(), mode: "FT8".into(), exch_sent: "IO91".into(), exch_rcvd: "FN42".into(), rst_rcvd: "-10".into(), time_on_unix_ms: (ora - 2 * 3_600_000) as u64, ..Default::default() };
        assert_eq!(s.registra(&q), Some(1));
        assert_eq!(s.stato()["fase"], "finito");
        assert_eq!(s.setup().map(|x| x.next_serial), Some(2));
        let dir = s.dir.parent().map(Path::to_path_buf).expect("cartella");
        drop(s);
        let s = ContestService::open(&dir, Arc::new(|| ("IU3ABC".to_string(), "IO91wm".to_string())), "prova");
        assert_eq!(s.log().map(|(v, _)| v.len()), Some(1), "il log dell'edizione finita resta");
        imp.inizio_ms = Some(ora - 30 * 86_400_000);
        s.salva_impostazioni("ww-digi", imp).expect("salva");
        assert_eq!(s.log().map(|(v, _)| v.len()), Some(1), "cambiare le date non toglie QSO");
        assert_eq!(s.azzera_log().map(|x| x.next_serial), Some(1));
    }

    /// Nome, indirizzo, email e club salvati in un contest tornano negli altri.
    #[test]
    fn anagrafica_riusata() {
        let s = servizio("anagrafica");
        let mut imp = s.impostazioni("ww-digi").expect("imp");
        imp.nome = "Mario Rossi".into();
        imp.email = "mario@example.org".into();
        s.salva_impostazioni("ww-digi", imp).expect("salva");
        let altro = s.impostazioni("arrl-digi").expect("altro");
        assert_eq!(altro.nome, "Mario Rossi");
        assert_eq!(altro.email, "mario@example.org");
    }
}
