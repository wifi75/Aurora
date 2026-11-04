//! I rapporti degli errori, facoltativi e spenti di fabbrica, che vanno al servizio di chi sviluppa Aurora (`servizi/rapporti`).
//!
//! - Livello 1, dopo un crash: al riavvio, se l'operatore lo ha chiesto, la pagina propone di mandare un pacchetto **senza dati
//!   personali** (log ripulito, rapporti di testo del crash, `info.json` con versione, sistema, motore, radio, PTT e audio). I minidump
//!   restano sul PC: contengono la memoria del programma, quindi anche nominativi e messaggi.
//! - Livello 2, errori automatici anonimi: ogni errore grave (`tracing::error!`, panic, avvisi di CAT, PTT e audio) diventa un rapporto
//!   breve. Errori uguali si raggruppano con un conteggio: al massimo un invio per errore ogni ora e 20 al giorno; senza rete si
//!   accodano su disco e partono dopo.
//!
//! Tutto passa da [`pulisci`]: via nominativi, locatori, percorsi con il nome dell'utente di Windows, indirizzi IP, email, codici e
//! password. L'identificativo dell'installazione e' un numero casuale, non legato alla persona, che si puo' rifare con un pulsante.
//! Nessuna rete passa dal runtime asincrono: l'invio sta in un thread suo e i lucchetti si lasciano prima di ogni chiamata.

use std::{
    collections::HashMap,
    io::Write as _,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Condvar, Mutex, OnceLock,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// L'indirizzo del servizio che raccoglie i rapporti (il Worker in `servizi/rapporti`). Finche' contiene il segnaposto non parte nulla:
/// dopo `npx wrangler deploy` si mette qui l'indirizzo vero (`https://aurora-rapporti.<sottodominio>.workers.dev`).
pub const INDIRIZZO: &str = "https://aurora-rapporti.tizianocassone.workers.dev";
const SEGNAPOSTO: &str = "SOTTODOMINIO";
/// La chiave di ingresso: pubblica, non e' un segreto. Serve solo al servizio per scartare il rumore; deve essere uguale a
/// `CHIAVE_INGRESSO` in `servizi/rapporti/wrangler.toml`.
pub const CHIAVE_INGRESSO: &str = "aurora-rapporti-v1";

/// Al massimo tanti rapporti automatici al giorno.
const MAX_GIORNO: u32 = 20;
/// Lo stesso errore parte al massimo una volta ogni tanti secondi (nel frattempo si contano le ripetizioni).
const OGNI_ERRORE: u64 = 3600;
/// I rapporti che aspettano la rete, al massimo.
const MAX_CODA: usize = 100;
/// Gli errori raggruppati che aspettano il loro turno, al massimo.
const MAX_GRUPPI: usize = 200;
/// Gli eventi raccolti tra un giro e l'altro del thread, al massimo.
const MAX_IN_ATTESA: usize = 200;
const MAX_MESSAGGIO: usize = 600;
const TIMEOUT: Duration = Duration::from_secs(10);
const TIMEOUT_PACCHETTO: Duration = Duration::from_secs(60);
/// Il pacchetto del crash resta sotto i 10 MB che il servizio accetta.
const MAX_PACCHETTO: u64 = 8 * 1024 * 1024;
/// Di `aurora.log` il pacchetto del crash tiene le ultime parti.
const MAX_LOG: u64 = 2 * 1024 * 1024;
const MAX_TESTO_CRASH: u64 = 512 * 1024;

const FILE_IMPOSTAZIONI: &str = "rapporti.json";
const FILE_ID: &str = "rapporti-installazione.txt";
const FILE_CODA: &str = "rapporti-coda.json";
const FILE_LIMITI: &str = "rapporti-limiti.json";
const FILE_CRASH_VISTO: &str = "rapporti-crash-visto.json";

// ---------------------------------------------------------------------------------------------------------------------------------
// La pulizia dei dati personali
// ---------------------------------------------------------------------------------------------------------------------------------

/// Toglie da un testo i dati personali: i valori `noti` (nominativo, locatore, nome utente e nome del PC, in qualsiasi maiuscola), i
/// percorsi `C:\Users\<nome>` e `/home/<nome>`, le email, gli indirizzi IP, i nominativi e i locatori riconosciuti dalla forma, i
/// valori dopo `password=`, `token:` e simili e i codici lunghi (chiavi). Quello che resta e' il testo dell'errore.
pub fn pulisci(testo: &str, noti: &[String]) -> String {
    let mut s = testo.to_string();
    // i valori noti per primi: il nome utente potrebbe non stare in un percorso
    let mut noti: Vec<&str> = noti.iter().map(|n| n.trim()).filter(|n| n.chars().count() >= 3).collect();
    noti.sort_by_key(|n| std::cmp::Reverse(n.len()));
    for n in noti {
        s = sostituisci_senza_maiuscole(&s, n, "<personale>");
    }
    s = pulisci_percorsi(&s);
    pulisci_parole(&s)
}

/// Sostituisce `cosa` ovunque compaia, senza guardare le maiuscole (solo per testo ASCII; altrimenti la forma esatta).
fn sostituisci_senza_maiuscole(testo: &str, cosa: &str, con: &str) -> String {
    if !cosa.is_ascii() {
        return testo.replace(cosa, con);
    }
    let basso = testo.to_ascii_lowercase();
    let cosa = cosa.to_ascii_lowercase();
    let mut out = String::with_capacity(testo.len());
    let mut da = 0;
    while let Some(i) = basso[da..].find(&cosa) {
        let i = da + i;
        out.push_str(&testo[da..i]);
        out.push_str(con);
        da = i + cosa.len();
    }
    out.push_str(&testo[da..]);
    out
}

/// `C:\Users\mario\…` → `C:\Users\<utente>\…` (anche con `/`, `\\` raddoppiati come nei JSON, `Documents and Settings` e `/home/`).
fn pulisci_percorsi(testo: &str) -> String {
    let mut s = testo.to_string();
    for radice in ["users", "documents and settings", "home"] {
        let mut out = String::with_capacity(s.len());
        let basso = s.to_ascii_lowercase();
        let mut da = 0;
        let mut cerca = 0;
        while let Some(i) = basso[cerca..].find(radice) {
            let i = cerca + i;
            let fine = i + radice.len();
            let prima_ok = i > 0 && matches!(s.as_bytes()[i - 1], b'\\' | b'/');
            // dopo la radice uno o piu' separatori, poi il nome fino al separatore successivo
            let resto = &s[fine..];
            let sep = resto.len() - resto.trim_start_matches(['\\', '/']).len();
            if !prima_ok || sep == 0 {
                cerca = fine;
                continue;
            }
            let nome_da = fine + sep;
            let nome_len = s[nome_da..].find(['\\', '/', '"', '\'', '\n', '\r', '\t', ':', ';', ',', ')', ']', '>']).unwrap_or(s.len() - nome_da);
            if nome_len == 0 || s[nome_da..nome_da + nome_len].starts_with('<') {
                cerca = fine;
                continue;
            }
            out.push_str(&s[da..nome_da]);
            out.push_str("<utente>");
            da = nome_da + nome_len;
            cerca = da;
        }
        out.push_str(&s[da..]);
        s = out;
    }
    s
}

/// Parola per parola: email, IP, nominativi, locatori, segreti dopo `nome=` e codici lunghi.
fn pulisci_parole(testo: &str) -> String {
    // un «pezzo» e' una sequenza senza spazi e senza i segni che chiudono un valore
    let separa = |c: char| c.is_whitespace() || "\"'(),;[]{}<>=".contains(c);
    let mut out = String::with_capacity(testo.len());
    let mut segreto = false;
    let mut resto = testo;
    while !resto.is_empty() {
        let n = resto.find(|c: char| !separa(c)).unwrap_or(resto.len());
        let sep = &resto[..n];
        out.push_str(sep);
        if sep.contains('\n') {
            segreto = false;
        }
        resto = &resto[n..];
        if resto.is_empty() {
            break;
        }
        let n = resto.find(separa).unwrap_or(resto.len());
        let pezzo = &resto[..n];
        resto = &resto[n..];
        if segreto {
            // nei JSON tra il nome e il valore c'e' un «:» da solo
            if pezzo == ":" {
                out.push_str(pezzo);
            } else {
                out.push_str("***");
                segreto = false;
            }
            continue;
        }
        // «password=», «api_key:», «"token": …»: il valore dopo e' segreto
        if nome_segreto(pezzo.trim_end_matches(':')) && (pezzo.ends_with(':') || resto.trim_start_matches([' ', '"', '\'']).starts_with(['=', ':'])) {
            out.push_str(pezzo);
            segreto = true;
            continue;
        }
        out.push_str(&pulisci_pezzo(pezzo));
    }
    out
}

/// Un nome che finisce con una parola da segreto (`password`, `api_key`, `lotw-token`, `pin`): piu' stretto di
/// `remote_api::is_secret_name`, perche' nel log «decoder:» o «ping:» non annunciano un segreto.
fn nome_segreto(nome: &str) -> bool {
    const PAROLE: [&str; 9] = ["password", "passwd", "pwd", "key", "token", "secret", "code", "pin", "credential"];
    let basso = nome.to_ascii_lowercase();
    !basso.is_empty() && PAROLE.iter().any(|w| basso.ends_with(w))
}

/// Un pezzo senza spazi: intero se e' un'email o un IP, altrimenti parola per parola (lettere, cifre e `_`).
fn pulisci_pezzo(pezzo: &str) -> String {
    if let Some(at) = pezzo.find('@') {
        let (utente, dominio) = (&pezzo[..at], &pezzo[at + 1..]);
        if !utente.is_empty() && dominio.contains('.') && !dominio.starts_with('.') {
            return "<email>".to_string();
        }
    }
    let nudo = pezzo.trim_end_matches(['.', ':']);
    let coda = &pezzo[nudo.len()..];
    if e_ipv4(nudo) || e_ipv4_con_porta(nudo) || e_ipv6(nudo) {
        return format!("<ip>{coda}");
    }
    let mut out = String::with_capacity(pezzo.len());
    let mut parola = String::new();
    for c in pezzo.chars() {
        if c.is_ascii_alphanumeric() || c == '_' {
            parola.push(c);
        } else {
            out.push_str(&pulisci_parola(&parola));
            parola.clear();
            out.push(c);
        }
    }
    out.push_str(&pulisci_parola(&parola));
    out
}

fn pulisci_parola(parola: &str) -> String {
    if parola.is_empty() || parola.contains('_') {
        return parola.to_string();
    }
    if e_locatore(parola) {
        return "<locatore>".to_string();
    }
    if e_nominativo(parola) {
        return "<nominativo>".to_string();
    }
    // chiavi e codici: lunghi, con lettere e cifre mescolate
    if parola.len() >= 24 && parola.chars().any(|c| c.is_ascii_digit()) && parola.chars().any(|c| c.is_ascii_alphabetic()) && !parola.starts_with("0x") {
        return "<codice>".to_string();
    }
    parola.to_string()
}

/// Locatore Maidenhead: `JN65`, `JN65ab`, `JN65ab12` (campo A–R, quadrato 0–9, sottoquadrato A–X).
pub fn e_locatore(p: &str) -> bool {
    let b = p.as_bytes();
    if !matches!(b.len(), 4 | 6 | 8) {
        return false;
    }
    let campo = |c: u8| matches!(c.to_ascii_uppercase(), b'A'..=b'R');
    let sotto = |c: u8| matches!(c.to_ascii_uppercase(), b'A'..=b'X');
    // il campo in maiuscolo: «ab12» in un testo qualsiasi non e' un locatore
    if !(b[0].is_ascii_uppercase() && b[1].is_ascii_uppercase() && campo(b[0]) && campo(b[1]) && b[2].is_ascii_digit() && b[3].is_ascii_digit()) {
        return false;
    }
    (b.len() < 6 || (sotto(b[4]) && sotto(b[5]))) && (b.len() < 8 || (b[6].is_ascii_digit() && b[7].is_ascii_digit()))
}

/// Nominativo radioamatoriale in maiuscolo: prefisso di 1–3 caratteri con almeno una lettera, una cifra, poi 1–4 lettere
/// (`IU3ABC`, `K1A`, `3DA0RU`, `9A1AA`). Le parti dopo `/` (`/P`, `EA8/`) restano: da sole non dicono chi e'.
pub fn e_nominativo(p: &str) -> bool {
    let b = p.as_bytes();
    if !(3..=10).contains(&b.len()) || b.iter().any(|c| c.is_ascii_lowercase()) {
        return false;
    }
    // l'ultima cifra separa prefisso+numero dal suffisso
    let Some(ultima_cifra) = b.iter().rposition(u8::is_ascii_digit) else { return false };
    let suffisso = &b[ultima_cifra + 1..];
    let prefisso = &b[..ultima_cifra];
    (1..=4).contains(&suffisso.len())
        && suffisso.iter().all(u8::is_ascii_uppercase)
        && (1..=3).contains(&prefisso.len())
        && prefisso.iter().all(u8::is_ascii_alphanumeric)
        && prefisso.iter().any(u8::is_ascii_uppercase)
}

fn e_ipv4(p: &str) -> bool {
    let parti: Vec<&str> = p.split('.').collect();
    parti.len() == 4 && parti.iter().all(|x| !x.is_empty() && x.len() <= 3 && x.bytes().all(|c| c.is_ascii_digit()) && x.parse::<u16>().is_ok_and(|n| n <= 255))
}

fn e_ipv4_con_porta(p: &str) -> bool {
    p.rsplit_once(':').is_some_and(|(ip, porta)| e_ipv4(ip) && !porta.is_empty() && porta.bytes().all(|c| c.is_ascii_digit()))
}

/// IPv6 (anche tra `[ ]` con la porta): almeno due `:`, solo cifre esadecimali, e una lettera esadecimale o `::` (cosi' un'ora
/// «12:34:56» resta com'e').
fn e_ipv6(p: &str) -> bool {
    let p = p.trim_start_matches('[');
    let p = p.split(']').next().unwrap_or(p);
    let p = p.split('%').next().unwrap_or(p);
    p.matches(':').count() >= 2
        && p.chars().all(|c| c.is_ascii_hexdigit() || c == ':' || c == '.')
        && (p.contains("::") || p.chars().any(|c| c.is_ascii_alphabetic()))
        && p.split(':').all(|g| g.len() <= 4 || e_ipv4(g))
}

// ---------------------------------------------------------------------------------------------------------------------------------
// La raccolta degli errori dal log
// ---------------------------------------------------------------------------------------------------------------------------------

/// Un errore visto nel log, prima della pulizia.
struct Evento {
    livello: &'static str,
    origine: String,
    messaggio: String,
}

/// Acceso solo con «Manda in automatico i rapporti degli errori»: spento, il log non tiene niente da parte.
static RACCOLTA: AtomicBool = AtomicBool::new(false);
static IN_ATTESA: Mutex<Vec<Evento>> = Mutex::new(Vec::new());
static SVEGLIA: Condvar = Condvar::new();

/// Lo strato del log che raccoglie gli errori gravi per i rapporti automatici. Si aggiunge al registratore del programma:
/// `tracing_subscriber::registry().with(…).with(aurora::rapporti::Strato)`.
pub struct Strato;

impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for Strato {
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        if !RACCOLTA.load(Ordering::Relaxed) {
            return;
        }
        let meta = event.metadata();
        if !da_segnalare(*meta.level(), meta.target()) {
            return;
        }
        let mut testo = Testo(String::new());
        event.record(&mut testo);
        let livello = if *meta.level() == tracing::Level::ERROR { "error" } else { "warn" };
        registra(Evento { livello, origine: meta.target().to_string(), messaggio: testo.0 });
    }
}

struct Testo(String);

impl tracing::field::Visit for Testo {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.0.push_str(value);
        } else {
            self.0.push_str(&format!(" {}={value}", field.name()));
        }
    }
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.0.push_str(&format!("{value:?}"));
        } else {
            self.0.push_str(&format!(" {}={value:?}", field.name()));
        }
    }
}

/// Gli errori sempre; gli avvisi solo di radio, PTT e audio. Mai quelli dei rapporti stessi e delle librerie di rete che usano.
fn da_segnalare(livello: tracing::Level, origine: &str) -> bool {
    const ESCLUSI: [&str; 6] = ["aurora::rapporti", "ureq", "rustls", "hyper", "h2", "tungstenite"];
    if ESCLUSI.iter().any(|e| origine.starts_with(e)) {
        return false;
    }
    match livello {
        tracing::Level::ERROR => true,
        tracing::Level::WARN => ["rig_control", "tx_native", "input_level", "audio_spectrum"].iter().any(|p| origine.starts_with(p)) || origine.contains("audio"),
        _ => false,
    }
}

fn registra(evento: Evento) {
    let Ok(mut attesa) = IN_ATTESA.lock() else { return };
    if attesa.len() < MAX_IN_ATTESA {
        attesa.push(evento);
    }
    drop(attesa);
    SVEGLIA.notify_one();
}

/// L'area del problema, per le classifiche del servizio.
fn area(origine: &str, messaggio: &str) -> &'static str {
    let m = messaggio.to_ascii_lowercase();
    if m.starts_with("panic") {
        "panic"
    } else if origine.starts_with("tx_native") || m.contains("ptt") {
        "ptt"
    } else if origine.starts_with("rig_control") || m.contains("omnirig") || m.contains("hamlib") || m.contains("rigctld") {
        "cat"
    } else if origine.contains("audio") || origine.starts_with("input_level") || m.contains("audio") {
        "audio"
    } else if origine.starts_with("engine_") || origine.contains("decod") {
        "decoder"
    } else if origine.starts_with("udp_") {
        "udp"
    } else if ["reporting", "online_info", "wishlist", "updater", "dx_cluster", "qso_export", "clock_sync", "dxcc"].iter().any(|p| origine.starts_with(p)) {
        "servizi"
    } else if origine.starts_with("logbook") {
        "log"
    } else if origine.starts_with("remote_api") || origine.starts_with("auth") {
        "api"
    } else {
        "altro"
    }
}

/// Lo stesso errore con numeri diversi (frequenze, durate, porte) e' lo stesso errore.
fn chiave(area: &str, origine: &str, messaggio: &str) -> String {
    let forma: String = messaggio.chars().map(|c| if c.is_ascii_digit() { '#' } else { c }).take(200).collect();
    format!("{area}|{origine}|{forma}")
}

// ---------------------------------------------------------------------------------------------------------------------------------
// Lo stato e i file
// ---------------------------------------------------------------------------------------------------------------------------------

/// Quello che il rapporto dice della stazione, senza dati personali; `noti` sono i valori da togliere (non si mandano).
#[derive(Debug, Clone, Default, Serialize)]
pub struct Contesto {
    pub versione: String,
    pub sistema: String,
    pub motore: String,
    /// Il modello della radio, non il suo nome personalizzato.
    pub radio: String,
    /// Il PTT davvero in uso.
    pub ptt: String,
    /// Il backend del CAT (hamlib, omnirig).
    pub cat: String,
    pub audio_in: String,
    pub audio_out: String,
    #[serde(skip)]
    pub noti: Vec<String>,
}

pub type FonteContesto = Arc<dyn Fn() -> Contesto + Send + Sync>;

/// Accesi di fabbrica (scelta del proprietario): la domanda dopo un crash e gli errori anonimi; al primo avvio l'interfaccia lo dice
/// una volta e si spengono in Impostazioni > Assistenza.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct Impostazioni {
    #[serde(default = "acceso")]
    ask_after_crash: bool,
    #[serde(default = "acceso")]
    auto_errors: bool,
    /// Una volta al giorno «questa installazione e' aperta» (codice casuale, versione, sistema): serve solo a contare chi usa Aurora.
    #[serde(default = "acceso")]
    count_active: bool,
}

fn acceso() -> bool {
    true
}

impl Default for Impostazioni {
    fn default() -> Self {
        Impostazioni { ask_after_crash: true, auto_errors: true, count_active: true }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Limiti {
    #[serde(default)]
    giorno: String,
    #[serde(default)]
    inviati: u32,
    /// Chiave dell'errore → secondi Unix dell'ultimo rapporto messo in coda.
    #[serde(default)]
    ultimi: HashMap<String, u64>,
}

/// Fin dove si e' gia' guardato nella cartella dei crash.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
struct CrashVisto {
    /// Il file di guasto piu' recente (millisecondi Unix della modifica).
    ultimo_ms: u64,
    /// La lunghezza di `stderr.log` (vi si accoda: se cresce c'e' un errore nuovo).
    stderr: u64,
}

struct Gruppo {
    livello: &'static str,
    area: &'static str,
    origine: String,
    messaggio: String,
    conteggio: u32,
}

struct Stato {
    impostazioni: Impostazioni,
    id: String,
    limiti: Limiti,
    coda: Vec<Value>,
    gruppi: HashMap<String, Gruppo>,
    crash_pendente: Option<CrashVisto>,
}

/// I rapporti degli errori di questa installazione.
pub struct Rapporti {
    dir: PathBuf,
    stato: Mutex<Stato>,
    contesto: Mutex<Option<FonteContesto>>,
    /// Un invio alla volta (la coda e il pacchetto del crash).
    invio: Mutex<()>,
}

/// L'indirizzo del servizio e' stato scritto (non e' piu' il segnaposto).
pub fn configurato() -> bool {
    !INDIRIZZO.contains(SEGNAPOSTO)
}

impl Rapporti {
    /// Legge impostazioni, coda e limiti dalla cartella dati, guarda se c'e' un crash nuovo e avvia il thread che manda.
    pub fn avvia(data_dir: &Path) -> Arc<Rapporti> {
        let impostazioni: Impostazioni = leggi_json(&data_dir.join(FILE_IMPOSTAZIONI)).unwrap_or_default();
        let id = std::fs::read_to_string(data_dir.join(FILE_ID)).ok().map(|s| s.trim().to_string()).filter(|s| s.len() == 36).unwrap_or_else(|| {
            let id = uuid();
            let _ = std::fs::write(data_dir.join(FILE_ID), &id);
            id
        });
        let limiti: Limiti = leggi_json(&data_dir.join(FILE_LIMITI)).unwrap_or_default();
        let coda: Vec<Value> = leggi_json(&data_dir.join(FILE_CODA)).unwrap_or_default();
        // un crash nuovo si propone solo se la domanda e' accesa; spenta, lo si da' per visto (accendendola dopo non torna un crash vecchio)
        let adesso = guarda_crash(&data_dir.join("crash"));
        let visto: Option<CrashVisto> = leggi_json(&data_dir.join(FILE_CRASH_VISTO));
        let nuovo = visto.is_some_and(|v| adesso.ultimo_ms > v.ultimo_ms || adesso.stderr > v.stderr);
        let crash_pendente = (nuovo && impostazioni.ask_after_crash).then_some(adesso);
        if crash_pendente.is_none() {
            scrivi_json(&data_dir.join(FILE_CRASH_VISTO), &adesso);
        }
        RACCOLTA.store(impostazioni.auto_errors, Ordering::Relaxed);
        let rapporti = Arc::new(Rapporti {
            dir: data_dir.to_path_buf(),
            stato: Mutex::new(Stato { impostazioni, id, limiti, coda, gruppi: HashMap::new(), crash_pendente }),
            contesto: Mutex::new(None),
            invio: Mutex::new(()),
        });
        let lavoro = Arc::clone(&rapporti);
        let _ = std::thread::Builder::new().name("rapporti".into()).spawn(move || lavoro.lavora());
        rapporti
    }

    /// Da dove prendere radio, PTT, audio e i valori personali da togliere (si chiama a ogni rapporto: le scelte possono cambiare).
    pub fn imposta_contesto(&self, fonte: FonteContesto) {
        if let Ok(mut c) = self.contesto.lock() {
            *c = Some(fonte);
        }
    }

    fn contesto(&self) -> Contesto {
        // la fonte si chiama senza tenere il lucchetto: legge radio e audio, che possono rispondere lenti
        let fonte = self.contesto.lock().ok().and_then(|c| c.clone());
        let mut c = fonte.map(|f| f()).unwrap_or_default();
        if c.versione.is_empty() {
            c.versione = aurora_core::version::APP_VERSION.to_string();
        }
        c.sistema = sistema().to_string();
        c.noti.extend(["USERNAME", "COMPUTERNAME", "USERDOMAIN"].iter().filter_map(|v| std::env::var(v).ok()));
        let noti = c.noti.clone();
        for campo in [&mut c.radio, &mut c.audio_in, &mut c.audio_out] {
            *campo = pulisci(campo, &noti);
        }
        c
    }

    /// Una volta al giorno, se acceso: «questa installazione e' aperta». Il giorno dell'ultimo invio riuscito sta in `rapporti-attivo.txt`.
    fn segnala_attivo(&self) {
        let (acceso, id) = match self.stato.lock() {
            Ok(s) => (s.impostazioni.count_active, s.id.clone()),
            Err(_) => return,
        };
        if !acceso || !configurato() {
            return;
        }
        let file = self.dir.join("rapporti-attivo.txt");
        let oggi = giorno_utc();
        if std::fs::read_to_string(&file).is_ok_and(|g| g.trim() == oggi) {
            return;
        }
        let corpo = json!({ "installazione": id, "versione": aurora_core::version::APP_VERSION, "sistema": sistema() }).to_string();
        let inviato = agente(TIMEOUT)
            .post(&format!("{INDIRIZZO}/v1/attivo"))
            .header("X-Aurora-Key", CHIAVE_INGRESSO)
            .header("Content-Type", "application/json")
            .header("User-Agent", "Aurora-rapporti")
            .send(corpo.as_str())
            .is_ok_and(|r| r.status().is_success());
        if inviato {
            let _ = std::fs::write(&file, &oggi);
        }
    }

    fn lavora(self: Arc<Self>) {
        loop {
            let eventi = {
                let Ok(mut attesa) = IN_ATTESA.lock() else { return };
                if attesa.is_empty() {
                    attesa = match SVEGLIA.wait_timeout(attesa, Duration::from_secs(60)) {
                        Ok((a, _)) => a,
                        Err(_) => return,
                    };
                }
                std::mem::take(&mut *attesa)
            };
            self.segnala_attivo();
            let acceso = self.stato.lock().map(|s| s.impostazioni.auto_errors).unwrap_or(false);
            if !acceso {
                continue;
            }
            if !eventi.is_empty() {
                self.raggruppa(eventi);
            }
            self.matura();
            self.svuota_coda();
            // un errore che ne genera altri non fa girare il thread a vuoto
            std::thread::sleep(Duration::from_secs(2));
        }
    }

    fn raggruppa(&self, eventi: Vec<Evento>) {
        let noti = self.contesto().noti;
        let Ok(mut stato) = self.stato.lock() else { return };
        for e in eventi {
            let messaggio: String = pulisci(&e.messaggio, &noti).chars().take(MAX_MESSAGGIO).collect();
            let area = area(&e.origine, &messaggio);
            let k = chiave(area, &e.origine, &messaggio);
            if let Some(g) = stato.gruppi.get_mut(&k) {
                g.conteggio = g.conteggio.saturating_add(1);
            } else if stato.gruppi.len() < MAX_GRUPPI {
                stato.gruppi.insert(k, Gruppo { livello: e.livello, area, origine: e.origine, messaggio, conteggio: 1 });
            }
        }
    }

    /// I gruppi il cui turno e' arrivato diventano rapporti in coda, nei limiti dell'ora e del giorno.
    fn matura(&self) {
        let pronti = self.stato.lock().map(|s| !s.gruppi.is_empty()).unwrap_or(false);
        if !pronti {
            return;
        }
        let contesto = self.contesto();
        let ora = secondi();
        let oggi = giorno(ora);
        let Ok(mut stato) = self.stato.lock() else { return };
        let stato = &mut *stato;
        if stato.limiti.giorno != oggi {
            stato.limiti = Limiti { giorno: oggi.clone(), inviati: 0, ultimi: std::mem::take(&mut stato.limiti.ultimi) };
            stato.limiti.ultimi.retain(|_, t| ora.saturating_sub(*t) < OGNI_ERRORE);
        }
        let chiavi: Vec<String> = stato.gruppi.keys().cloned().collect();
        let mut cambiato = false;
        for k in chiavi {
            if stato.limiti.inviati >= MAX_GIORNO {
                break;
            }
            if stato.limiti.ultimi.get(&k).is_some_and(|t| ora.saturating_sub(*t) < OGNI_ERRORE) {
                continue;
            }
            let Some(g) = stato.gruppi.remove(&k) else { continue };
            let rapporto = rapporto(&stato.id, &contesto, &oggi, &g);
            stato.coda.push(rapporto);
            if stato.coda.len() > MAX_CODA {
                stato.coda.remove(0);
            }
            stato.limiti.ultimi.insert(k, ora);
            stato.limiti.inviati += 1;
            cambiato = true;
        }
        if cambiato {
            scrivi_json(&self.dir.join(FILE_CODA), &stato.coda);
            scrivi_json(&self.dir.join(FILE_LIMITI), &stato.limiti);
        }
    }

    /// Manda i rapporti in coda; alla prima mancanza di rete si ferma e riprova al giro dopo.
    fn svuota_coda(&self) {
        if !configurato() {
            return;
        }
        let Ok(_uno) = self.invio.lock() else { return };
        loop {
            let primo = self.stato.lock().ok().and_then(|s| s.coda.first().cloned());
            let Some(primo) = primo else { break };
            let esito = manda_json(&primo);
            let Ok(mut stato) = self.stato.lock() else { return };
            match esito {
                // accettato, o rifiutato per sempre (troppo grande, chiave sbagliata): via dalla coda
                Ok(()) | Err(Invio::Rifiutato) => {
                    if stato.coda.first() == Some(&primo) {
                        stato.coda.remove(0);
                    }
                    scrivi_json(&self.dir.join(FILE_CODA), &stato.coda);
                }
                Err(Invio::Riprova(e)) => {
                    tracing::debug!("rapporto degli errori non mandato, riprovo dopo: {e}");
                    break;
                }
            }
        }
    }

    fn salva_impostazioni(&self, i: Impostazioni) {
        scrivi_json(&self.dir.join(FILE_IMPOSTAZIONI), &i);
    }

    /// Il rapporto che partirebbe adesso per un errore d'esempio: lo mostra «Vedi cosa mando».
    fn esempio(&self) -> Value {
        let contesto = self.contesto();
        let id = self.stato.lock().map(|s| s.id.clone()).unwrap_or_default();
        let g = Gruppo { livello: "error", area: "cat", origine: "rig_control::hamlib".into(), messaggio: pulisci("rigctld non risponde su 127.0.0.1:4532 (radio di K1ABC a FN42ab)", &contesto.noti), conteggio: 3 };
        rapporto(&id, &contesto, &giorno(secondi()), &g)
    }

    /// I file del pacchetto del crash, gia' ripuliti: nome e contenuto.
    fn pacchetto_crash(&self) -> Vec<(String, Vec<u8>)> {
        let contesto = self.contesto();
        let id = self.stato.lock().map(|s| s.id.clone()).unwrap_or_default();
        let noti = contesto.noti.clone();
        let mut file: Vec<(String, Vec<u8>)> = Vec::new();
        let mut usati: u64 = 0;
        let crash = self.dir.join("crash");
        let mut dump_lasciati = 0;
        for (path, nome) in crate::support::newest_first(&crash) {
            if nome.to_ascii_lowercase().ends_with(".dmp") {
                dump_lasciati += 1;
                continue;
            }
            if file.len() >= 20 || usati > MAX_PACCHETTO / 2 {
                break;
            }
            let Ok(dati) = crate::support::read_tail(&path, MAX_TESTO_CRASH) else { continue };
            let testo = pulisci(&String::from_utf8_lossy(&dati), &noti);
            usati += testo.len() as u64;
            file.push((format!("crash/{nome}"), testo.into_bytes()));
        }
        if let Ok(dati) = crate::support::read_tail(&self.dir.join("logs").join("aurora.log"), MAX_LOG.min(MAX_PACCHETTO.saturating_sub(usati))) {
            let testo = String::from_utf8_lossy(&dati);
            let righe: Vec<String> = testo.lines().filter_map(|r| riga_di_log(r, &noti)).collect();
            file.push(("logs/aurora.log".into(), (righe.join("\n") + "\n").into_bytes()));
        }
        let info = json!({
            "schema": 1,
            "installazione": id,
            "giorno": giorno(secondi()),
            "contesto": contesto,
            "minidump_lasciati_sul_pc": dump_lasciati,
            "nota": "Pacchetto senza dati personali: nominativi, locatori, percorsi con il nome utente, IP, email e codici sono tolti; i minidump, la configurazione, il log dei QSO e ALL.TXT restano sul PC.",
        });
        file.insert(0, ("info.json".into(), serde_json::to_vec_pretty(&info).unwrap_or_default()));
        file
    }

    /// Manda lo zip senza dati personali; `motivo` e' «crash» o «assistenza». Restituisce il codice breve da dire a chi aiuta.
    fn manda_pacchetto(&self, motivo: &str) -> Result<Value, String> {
        if !configurato() {
            return Err("indirizzo del servizio dei rapporti non configurato".into());
        }
        let file = self.pacchetto_crash();
        let zip = comprimi(&file)?;
        let id = self.stato.lock().map(|s| s.id.clone()).unwrap_or_default();
        // il codice breve: 8 cifre esadecimali casuali, con cui il servizio ritrova il pacchetto
        let codice: String = uuid().chars().filter(char::is_ascii_hexdigit).take(8).collect::<String>().to_ascii_uppercase();
        let uno = self.invio.lock().map_err(|_| "invio occupato".to_string())?;
        let esito = manda_zip(&zip, &id, &codice, motivo);
        drop(uno);
        esito.map_err(|e| match e {
            Invio::Rifiutato => "pacchetto rifiutato dal servizio".to_string(),
            Invio::Riprova(e) => e,
        })?;
        tracing::info!("pacchetto senza dati personali mandato ({motivo}, codice {codice}, {} KB)", zip.len() / 1024);
        Ok(json!({ "ok": true, "code": codice, "bytes": zip.len() }))
    }

    fn segna_crash_visto(&self) {
        let visto = self.stato.lock().ok().and_then(|mut s| s.crash_pendente.take());
        let visto = visto.unwrap_or_else(|| guarda_crash(&self.dir.join("crash")));
        scrivi_json(&self.dir.join(FILE_CRASH_VISTO), &visto);
    }
}

impl remote_api::ErrorReports for Rapporti {
    fn state(&self) -> Value {
        let esempio = self.esempio();
        let Ok(s) = self.stato.lock() else { return json!({}) };
        json!({
            "ask_after_crash": s.impostazioni.ask_after_crash,
            "auto_errors": s.impostazioni.auto_errors,
            "count_active": s.impostazioni.count_active,
            "install_id": s.id,
            "configured": configurato(),
            "endpoint": INDIRIZZO,
            "crash_pending": s.crash_pendente.is_some() && s.impostazioni.ask_after_crash,
            "queue": s.coda,
            "waiting": s.gruppi.len(),
            "sent_today": if s.limiti.giorno == giorno(secondi()) { s.limiti.inviati } else { 0 },
            "max_per_day": MAX_GIORNO,
            "sample": esempio,
        })
    }

    fn update(&self, patch: &Value) -> Result<Value, String> {
        {
            let mut s = self.stato.lock().map_err(|_| "stato non leggibile".to_string())?;
            if let Some(v) = patch.get("ask_after_crash").and_then(Value::as_bool) {
                s.impostazioni.ask_after_crash = v;
            }
            if let Some(v) = patch.get("count_active").and_then(Value::as_bool) {
                s.impostazioni.count_active = v;
            }
            if let Some(v) = patch.get("auto_errors").and_then(Value::as_bool) {
                s.impostazioni.auto_errors = v;
                RACCOLTA.store(v, Ordering::Relaxed);
                // spenti: niente resta da parte
                if !v {
                    s.gruppi.clear();
                    s.coda.clear();
                    scrivi_json(&self.dir.join(FILE_CODA), &s.coda);
                }
            }
            let i = s.impostazioni;
            drop(s);
            self.salva_impostazioni(i);
        }
        Ok(self.state())
    }

    fn new_install_id(&self) -> Value {
        let id = uuid();
        let _ = std::fs::write(self.dir.join(FILE_ID), &id);
        if let Ok(mut s) = self.stato.lock() {
            s.id = id.clone();
            // i rapporti in coda prendono il nuovo identificativo: il vecchio non deve piu' partire
            for r in &mut s.coda {
                r["installazione"] = json!(id);
            }
            scrivi_json(&self.dir.join(FILE_CODA), &s.coda);
        }
        self.state()
    }

    fn crash_preview(&self) -> Result<Value, String> {
        let file = self.pacchetto_crash();
        let bytes: usize = file.iter().map(|(_, d)| d.len()).sum();
        let elenco: Vec<Value> = file.iter().map(|(n, d)| json!({ "name": n, "bytes": d.len(), "text": String::from_utf8_lossy(d) })).collect();
        Ok(json!({ "files": elenco, "bytes": bytes, "configured": configurato(), "endpoint": INDIRIZZO }))
    }

    fn crash_send(&self) -> Result<Value, String> {
        let esito = self.manda_pacchetto("crash")?;
        self.segna_crash_visto();
        Ok(esito)
    }

    fn bundle_send(&self) -> Result<Value, String> {
        self.manda_pacchetto("assistenza")
    }

    fn crash_dismiss(&self) {
        self.segna_crash_visto();
    }
}

/// Il rapporto breve del livello 2.
fn rapporto(id: &str, c: &Contesto, giorno: &str, g: &Gruppo) -> Value {
    json!({
        "schema": 1,
        "id": uuid(),
        "installazione": id,
        "giorno": giorno,
        "versione": c.versione,
        "sistema": c.sistema,
        "motore": c.motore,
        "radio": c.radio,
        "ptt": c.ptt,
        "cat": c.cat,
        "audio_in": c.audio_in,
        "audio_out": c.audio_out,
        "area": g.area,
        "livello": g.livello,
        "origine": g.origine,
        "messaggio": g.messaggio,
        "conteggio": g.conteggio,
    })
}

/// Una riga del log per il pacchetto del crash: ripulita; quelle che non sono avvisi o errori e parlano di nominativi o locatori
/// (decodifiche, QSO) restano fuori.
fn riga_di_log(riga: &str, noti: &[String]) -> Option<String> {
    let pulita = pulisci(riga, noti);
    let grave = riga.contains(" ERROR ") || riga.contains(" WARN ");
    if !grave && (pulita.contains("<nominativo>") || pulita.contains("<locatore>") || pulita.contains("<personale>")) {
        return None;
    }
    if riga.contains(" DEBUG ") || riga.contains(" TRACE ") {
        return None;
    }
    Some(pulita)
}

fn guarda_crash(dir: &Path) -> CrashVisto {
    let mut ultimo_ms = 0;
    let mut stderr = 0;
    if let Ok(voci) = std::fs::read_dir(dir) {
        for v in voci.flatten() {
            let nome = v.file_name().to_string_lossy().to_ascii_lowercase();
            let Ok(meta) = v.metadata() else { continue };
            if nome == "stderr.log" {
                stderr = meta.len();
            } else if ["crash-", "panic-", "hang-"].iter().any(|p| nome.starts_with(p)) {
                let ms = meta.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_millis() as u64).unwrap_or(0);
                ultimo_ms = ultimo_ms.max(ms);
            }
        }
    }
    CrashVisto { ultimo_ms, stderr }
}

enum Invio {
    /// Il servizio non lo vorra' mai (troppo grande, chiave di ingresso sbagliata): non si riprova.
    Rifiutato,
    /// Rete assente, servizio occupato: si riprova piu' tardi.
    Riprova(String),
}

fn agente(timeout: Duration) -> ureq::Agent {
    ureq::Agent::config_builder().timeout_global(Some(timeout)).http_status_as_error(false).build().into()
}

fn esito(status: u16) -> Result<(), Invio> {
    match status {
        200..=299 => Ok(()),
        429 | 500..=599 => Err(Invio::Riprova(format!("il servizio ha risposto {status}"))),
        _ => Err(Invio::Rifiutato),
    }
}

fn manda_json(rapporto: &Value) -> Result<(), Invio> {
    let corpo = serde_json::to_string(rapporto).map_err(|_| Invio::Rifiutato)?;
    let risposta = agente(TIMEOUT)
        .post(&format!("{INDIRIZZO}/v1/report"))
        .header("X-Aurora-Key", CHIAVE_INGRESSO)
        .header("Content-Type", "application/json")
        .header("User-Agent", "Aurora-rapporti")
        .send(corpo.as_str())
        .map_err(|e| Invio::Riprova(e.to_string()))?;
    esito(risposta.status().as_u16())
}

fn manda_zip(zip: &[u8], id: &str, codice: &str, motivo: &str) -> Result<(), Invio> {
    let risposta = agente(TIMEOUT_PACCHETTO)
        .post(&format!("{INDIRIZZO}/v1/bundle"))
        .header("X-Aurora-Key", CHIAVE_INGRESSO)
        .header("X-Aurora-Installazione", id)
        .header("X-Aurora-Codice", codice)
        .header("X-Aurora-Motivo", motivo)
        .header("X-Aurora-Versione", aurora_core::version::APP_VERSION)
        .header("Content-Type", "application/zip")
        .header("User-Agent", "Aurora-rapporti")
        .send(zip)
        .map_err(|e| Invio::Riprova(e.to_string()))?;
    esito(risposta.status().as_u16())
}

fn comprimi(file: &[(String, Vec<u8>)]) -> Result<Vec<u8>, String> {
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let opzioni = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for (nome, dati) in file {
        zip.start_file(nome.as_str(), opzioni).map_err(|e| e.to_string())?;
        zip.write_all(dati).map_err(|e| e.to_string())?;
    }
    Ok(zip.finish().map_err(|e| e.to_string())?.into_inner())
}

/// La versione di Windows, letta una volta.
fn sistema() -> &'static str {
    static SISTEMA: OnceLock<String> = OnceLock::new();
    SISTEMA.get_or_init(|| if cfg!(windows) { crate::support::windows_version() } else { std::env::consts::OS.to_string() })
}

/// Un UUID versione 4 dal generatore casuale del sistema.
fn uuid() -> String {
    let mut b = [0u8; 16];
    if getrandom::fill(&mut b).is_err() {
        // senza generatore (non dovrebbe succedere) basta l'ora in nanosecondi: l'identificativo resta anonimo
        let n = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
        b = n.to_le_bytes();
    }
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    let h: String = b.iter().map(|x| format!("{x:02x}")).collect();
    format!("{}-{}-{}-{}-{}", &h[0..8], &h[8..12], &h[12..16], &h[16..20], &h[20..32])
}

fn secondi() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn giorno(secs: u64) -> String {
    let (a, m, g, ..) = crate::support::civil(secs as i64);
    format!("{a:04}-{m:02}-{g:02}")
}

fn leggi_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Option<T> {
    std::fs::read_to_string(path).ok().and_then(|t| serde_json::from_str(&t).ok())
}

fn scrivi_json<T: Serialize>(path: &Path, valore: &T) {
    if let Ok(testo) = serde_json::to_string_pretty(valore) {
        let tmp = path.with_extension("tmp");
        if std::fs::write(&tmp, testo).is_ok() {
            let _ = std::fs::rename(&tmp, path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> String {
        pulisci(s, &[])
    }

    #[test]
    fn nominativi_e_locatori_vanno_via() {
        assert_eq!(p("CQ IU3ABC JN65"), "CQ <nominativo> <locatore>");
        assert_eq!(p("K1ABC W9XYZ FN42ab"), "<nominativo> <nominativo> <locatore>");
        assert_eq!(p("3DA0RU 9A1AA EA8/DL1XYZ/P"), "<nominativo> <nominativo> EA8/<nominativo>/P");
        // parole comuni, versioni e codici dei programmi restano
        assert_eq!(p("FT8 FT4 JT65 RR73 73 0.11.95 x86_64 utf8 IC-7300 TX1"), "FT8 FT4 JT65 <locatore> 73 0.11.95 x86_64 utf8 IC-7300 TX1");
    }

    #[test]
    fn percorsi_ip_email_e_segreti() {
        assert_eq!(p(r"C:\Users\Mario Rossi\AppData\Roaming\Aurora"), r"C:\Users\<utente>\AppData\Roaming\Aurora");
        assert_eq!(p(r"C:\Users\mario\AppData"), r"C:\Users\<utente>\AppData");
        assert_eq!(p(r#"{"dir":"C:\\Users\\mario\\Desktop"}"#), r#"{"dir":"C:\\Users\\<utente>\\Desktop"}"#);
        assert_eq!(p("collegato a 192.168.1.20:4532 e fe80::1"), "collegato a <ip> e <ip>");
        assert_eq!(p("ore 12:34:56"), "ore 12:34:56");
        assert_eq!(p("scrivi a mario.rossi@example.com."), "scrivi a <email>");
        assert_eq!(p("password=segreta token: abc123"), "password=*** token: ***");
        assert_eq!(p("chiave 9f8e7d6c5b4a39281706f5e4d3c2b1a0"), "chiave <codice>");
    }

    #[test]
    fn valori_noti_senza_maiuscole() {
        assert_eq!(pulisci("utente tiziano su PC-SALA", &["Tiziano".into(), "pc-sala".into()]), "utente <personale> su <personale>");
    }

    #[test]
    fn errori_uguali_stessa_chiave() {
        assert_eq!(chiave("cat", "x", "porta 4532 chiusa"), chiave("cat", "x", "porta 4533 chiusa"));
        assert_eq!(area("rig_control::hamlib", "rigctld fermo"), "cat");
        assert_eq!(area("aurora_desktop::health", "panic nel thread"), "panic");
    }

    #[test]
    fn uuid_versione_4() {
        let u = uuid();
        assert_eq!(u.len(), 36);
        assert_eq!(&u[14..15], "4");
    }
}

/// La data di oggi in UTC, `AAAA-MM-GG`.
fn giorno_utc() -> String {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64);
    let (y, m, d, ..) = crate::support::civil(secs);
    format!("{y:04}-{m:02}-{d:02}")
}
