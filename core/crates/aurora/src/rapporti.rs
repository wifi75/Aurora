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
    // le parole comuni non sono dati personali: un PC che si chiama «User» o «Radio» toglieva «Users» dai percorsi e «radio» dai messaggi
    // (rapporti del 2026-11-12: «nessuna <personale> Cat4OM collegata»)
    let mut noti: Vec<&str> = noti.iter().map(|n| n.trim()).filter(|n| n.chars().count() >= 3 && !parola_comune(n)).collect();
    noti.sort_by_key(|n| std::cmp::Reverse(n.len()));
    for n in noti {
        s = sostituisci_senza_maiuscole(&s, n, "<personale>");
    }
    s = pulisci_percorsi(&s);
    pulisci_parole(&s)
}

/// Nomi generici di utenti e PC che non dicono chi e' la persona: non si tolgono dai rapporti.
fn parola_comune(n: &str) -> bool {
    const COMUNI: [&str; 22] = [
        "user", "users", "utente", "admin", "administrator", "amministratore", "owner", "guest", "default", "radio", "ham", "shack", "station",
        "stazione", "home", "desktop", "laptop", "notebook", "computer", "workgroup", "pc", "windows",
    ];
    let basso = n.to_lowercase();
    COMUNI.contains(&basso.as_str())
}

/// Sostituisce `cosa` dove compare come parola intera (non dentro un'altra parola: «User» non tocca «Users»), senza guardare le
/// maiuscole per il testo ASCII; altrimenti la forma esatta. I valori lunghi (6 caratteri o piu') si tolgono anche dentro altre parole,
/// per non lasciarne pezzi («tizianocassone» in un nome composto).
fn sostituisci_senza_maiuscole(testo: &str, cosa: &str, con: &str) -> String {
    let (basso, cosa_cmp) = if cosa.is_ascii() { (testo.to_ascii_lowercase(), cosa.to_ascii_lowercase()) } else { (testo.to_string(), cosa.to_string()) };
    let anche_dentro = cosa.chars().count() >= 6;
    let alnum_prima = |i: usize| testo[..i].chars().next_back().is_some_and(char::is_alphanumeric);
    let alnum_dopo = |j: usize| testo[j..].chars().next().is_some_and(char::is_alphanumeric);
    let mut out = String::with_capacity(testo.len());
    let mut da = 0;
    let mut cerca = 0;
    while let Some(i) = basso[cerca..].find(&cosa_cmp) {
        let i = cerca + i;
        let j = i + cosa_cmp.len();
        if anche_dentro || (!alnum_prima(i) && !alnum_dopo(j)) {
            out.push_str(&testo[da..i]);
            out.push_str(con);
            da = j;
        }
        cerca = j;
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
            // un nome gia' sostituito in parte («<personale> Rossi», dai valori noti) va tolto fino al separatore, segnaposto compreso:
            // prima restava il cognome (registro dell'installatore, 2026-11-12)
            let nome_len = if s[nome_da..].starts_with('<') {
                s[nome_da..].find(['\\', '/', '"', '\'', '\n', '\r', '\t'])
            } else {
                s[nome_da..].find(['\\', '/', '"', '\'', '\n', '\r', '\t', ':', ';', ',', ')', ']', '>'])
            }
            .unwrap_or(s.len() - nome_da);
            if nome_len == 0 || s[nome_da..].starts_with("<utente>") {
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

/// Il nominativo di base, per contare le persone e non i PC: maiuscolo, senza `/P`, `/M`, `/MM`, `/QRP`, `/A` ne' prefissi di
/// portatile (`EA/IU3ABC` -> `IU3ABC`). `None` se vuoto o non valido.
pub fn nominativo_base(nominativo: &str) -> Option<String> {
    let su = nominativo.trim().to_ascii_uppercase();
    su.split('/').filter(|p| e_nominativo(p)).max_by_key(|p| p.len()).map(str::to_string)
}

/// Fisso del progetto davanti al nominativo: lo stesso nominativo da' lo stesso codice su ogni PC, ma il codice non e' quello di altri servizi.
const SALE_PERSONA: &str = "aurora-persona-v1:";

/// Il codice della persona: SHA-256 di `SALE_PERSONA` + nominativo di base, in esadecimale (64 caratteri). Il nominativo non lascia il PC.
pub fn codice_persona(nominativo: &str) -> Option<String> {
    use sha2::{Digest, Sha256};
    let base = nominativo_base(nominativo)?;
    let hash = Sha256::digest(format!("{SALE_PERSONA}{base}").as_bytes());
    Some(hash.iter().map(|b| format!("{b:02x}")).collect())
}

/// Il corpo del segnale «attivo», o `None` se l'utente l'ha spento: codice casuale dell'installazione, versione, sistema e, se c'e' un
/// nominativo valido, il codice della persona (mai il nominativo).
pub fn corpo_attivo(acceso: bool, installazione: &str, nominativo: &str) -> Option<String> {
    if !acceso {
        return None;
    }
    let pc = pc();
    let mut corpo = json!({ "installazione": installazione, "versione": aurora_core::version::APP_VERSION, "sistema": sistema(), "cpu": pc.cpu, "core": pc.core, "ram_gb": pc.ram_gb });
    if let Some(persona) = codice_persona(nominativo) {
        corpo["persona"] = json!(persona);
    }
    Some(corpo.to_string())
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
        if scelta_dell_operatore(&testo.0) || rumore_di_libreria(&testo.0) {
            return;
        }
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

/// Situazioni che dipendono da come e' configurata la stazione, non da un errore del programma: la radio non ancora collegata o non
/// accesa (nei rapporti del 2026-11-09 erano un terzo delle righe, da 80 installazioni), o il TX premuto nei primi secondi dopo il
/// collegamento, quando il PTT e' fermo apposta (protezione voluta, rapporti del 2026-11-10). L'interfaccia le dice gia' all'operatore.
/// Lo stesso per la porta seriale tenuta da un altro programma («La porta COM8 e' occupata da OmniRig», `rig_control::busy_port`) e per
/// OmniRig che non risponde mentre si avvia o con la radio non in linea (rapporti del 2026-11-11 da 0.11.236/0.11.237).
fn scelta_dell_operatore(testo: &str) -> bool {
    let t = testo.to_lowercase();
    [
        "nessuna radio collegata",
        "radio non collegata",
        "nessuna radio tci collegata",
        "radio tci non collegata",
        "link non collegata",
        "radio non in linea",
        "scegli prima il modello della radio",
        "dopo il collegamento",
        "e' occupata da",
        "in linea in omnirig?",
    ]
    .iter()
    .any(|p| t.contains(p))
}

/// Gli errori sempre; gli avvisi solo di radio, PTT e audio. Mai quelli dei rapporti stessi e delle librerie di rete che usano.
/// Messaggi di librerie che non sono errori di Aurora. Su Linux la libreria di PulseAudio scrive «Client disconnected» come errore ogni volta
/// che un suo collegamento si chiude (a ogni elenco dei dispositivi audio, o quando il server PulseAudio non c'e' e si usa ALSA): nei rapporti
/// del 2026-11-12 erano 63 righe da un solo PC, con l'audio che funzionava.
fn rumore_di_libreria(testo: &str) -> bool {
    testo.contains("pulseaudio::client::reactor") && testo.contains("Client disconnected")
}

fn da_segnalare(livello: tracing::Level, origine: &str) -> bool {
    const ESCLUSI: [&str; 6] = ["aurora::rapporti", "ureq", "rustls", "hyper", "h2", "tungstenite"];
    if ESCLUSI.iter().any(|e| origine.starts_with(e)) {
        return false;
    }
    match livello {
        tracing::Level::ERROR => true,
        tracing::Level::WARN => ["rig_control", "tx_native", "input_level", "audio_spectrum", "remote_api::associazione", "engine_native::lento", "aurora_desktop::blocco"].iter().any(|p| origine.starts_with(p)) || origine.contains("audio"),
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
    } else if origine.starts_with("aurora_desktop::blocco") || m.starts_with("programma bloccato") {
        "blocco"
    } else if origine.starts_with("remote_api::associazione") {
        "telefono"
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

/// Situazioni della stazione, non errori del programma, che servono a sapere su che macchine Aurora fatica: partono al massimo una volta
/// per installazione e versione, qualunque numero portino. «Decodifica lenta» arrivava da ogni PC vecchio a ogni avvio (17 rapporti il
/// 2026-11-11, tutti da portatili a 4 processori); sullo schermo l'operatore ha gia' l'avviso con la profondita' da abbassare.
const UNA_PER_VERSIONE: [&str; 1] = ["engine_native::lento"];

/// L'avviso di `origine` e' gia' partito con questa `versione` da questa installazione: non si manda di nuovo.
fn gia_segnalato(limiti: &Limiti, origine: &str, versione: &str) -> bool {
    UNA_PER_VERSIONE.iter().any(|p| origine.starts_with(p)) && limiti.per_versione.get(origine).is_some_and(|v| v == versione)
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
    /// Il nominativo della stazione: serve solo a calcolare in locale il codice della persona, non si manda.
    #[serde(skip)]
    pub nominativo: String,
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
    /// Origine → versione dell'ultimo rapporto, per le situazioni che partono una volta per versione ([`UNA_PER_VERSIONE`]).
    #[serde(default)]
    per_versione: HashMap<String, String>,
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
        // stderr.log cresce anche a ogni chiusura normale (righe innocue di WebView2, «Failed to unregister class Chrome_WidgetWin_0», e
        // l'errore di PowerShell del riavvio dopo l'aggiornamento): conta come crash solo se le righe nuove hanno qualcosa di vero.
        // Prima ogni aggiornamento e ogni spegnimento del PC mandavano un pacchetto «crash» (rapporti del 2026-11-13)
        let nuovo = visto.is_some_and(|v| adesso.ultimo_ms > v.ultimo_ms || (adesso.stderr > v.stderr && stderr_vero(&data_dir.join("crash").join("stderr.log"), v.stderr)));
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
        // il nominativo arriva con la fonte del contesto, impostata dopo l'avvio del server: fino ad allora si aspetta
        let Some(fonte) = self.contesto.lock().ok().and_then(|c| c.clone()) else { return };
        let nominativo = fonte().nominativo;
        let Some(corpo) = corpo_attivo(acceso, &id, &nominativo) else { return };
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
        // l'ultima installazione, una volta all'avvio: il rapporto va in coda e parte al primo giro
        self.segnala_installazione();
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
            stato.limiti = Limiti { giorno: oggi.clone(), inviati: 0, ultimi: std::mem::take(&mut stato.limiti.ultimi), per_versione: std::mem::take(&mut stato.limiti.per_versione) };
            stato.limiti.ultimi.retain(|_, t| ora.saturating_sub(*t) < OGNI_ERRORE);
        }
        let chiavi: Vec<String> = stato.gruppi.keys().cloned().collect();
        let mut cambiato = false;
        for k in chiavi {
            if stato.limiti.inviati >= MAX_GIORNO {
                break;
            }
            // gia' mandato con questa versione: il gruppo si butta (resta solo la riga nel log locale)
            if stato.gruppi.get(&k).is_some_and(|g| gia_segnalato(&stato.limiti, &g.origine, &contesto.versione)) {
                stato.gruppi.remove(&k);
                continue;
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
            if UNA_PER_VERSIONE.iter().any(|p| g.origine.starts_with(p)) {
                stato.limiti.per_versione.insert(g.origine.clone(), contesto.versione.clone());
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
        // il registro dell'ultima installazione, ripulito: dice come e' stato installato Aurora (WebView2, file non copiati, antivirus)
        if let Some(testo) = registro_installazione(&self.dir) {
            let pulito = pulisci(&testo, &noti);
            usati += pulito.len() as u64;
            file.push((FILE_INSTALLAZIONE.into(), pulito.into_bytes()));
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
        let riassunto = riassunto_crash(&file);
        let zip = comprimi(&file)?;
        let id = self.stato.lock().map(|s| s.id.clone()).unwrap_or_default();
        // il codice breve: 8 cifre esadecimali casuali, con cui il servizio ritrova il pacchetto
        let codice: String = uuid().chars().filter(char::is_ascii_hexdigit).take(8).collect::<String>().to_ascii_uppercase();
        let uno = self.invio.lock().map_err(|_| "invio occupato".to_string())?;
        let esito = manda_zip(&zip, &id, &codice, motivo, &riassunto);
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
        "pc": pc().descrizione(),
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

/// Le righe di `stderr.log` scritte dopo il byte `da` contengono un errore vero (non solo il rumore noto di WebView2 e del riavvio)?
fn stderr_vero(path: &Path, da: u64) -> bool {
    let Ok(bytes) = std::fs::read(path) else { return false };
    let start = usize::try_from(da).unwrap_or(0).min(bytes.len());
    String::from_utf8_lossy(&bytes[start..]).lines().any(|riga| {
        let r = riga.trim();
        !r.is_empty() && !rumore_stderr(r)
    })
}

/// Righe di `stderr.log` che non indicano un crash: WebView2 alla chiusura e l'errore di PowerShell quando il riavvio dopo l'aggiornamento
/// non trova il file (con le righe che lo accompagnano).
fn rumore_stderr(riga: &str) -> bool {
    riga.contains("Failed to unregister class Chrome_WidgetWin")
        || riga.contains("window_impl.cc")
        || riga.starts_with("Start-Process")
        || riga.starts_with('+')
        || riga.contains("CategoryInfo")
        || riga.contains("FullyQualifiedErrorId")
        || !riga.is_ascii() && !riga.to_ascii_lowercase().contains("panic")
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

/// Il riassunto del crash piu' recente (gia' ripulito come il resto del pacchetto): l'inizio del suo file di testo (`panic-`, `crash-`,
/// `hang-`), cosi' il servizio lo salva nell'indice e si legge senza scaricare lo zip.
fn riassunto_crash(file: &[(String, Vec<u8>)]) -> String {
    const MAX: usize = 1500;
    let Some((nome, dati)) = file.iter().find(|(n, _)| n.starts_with("crash/") && n.ends_with(".txt") && !n.ends_with("stderr.log")) else {
        return String::new();
    };
    let testo = String::from_utf8_lossy(dati);
    let mut out = format!("{}\n{}", nome.trim_start_matches("crash/"), testo.trim());
    if out.len() > MAX {
        let mut fine = MAX;
        while !out.is_char_boundary(fine) {
            fine -= 1;
        }
        out.truncate(fine);
    }
    out
}

/// Percent-encoding per un'intestazione HTTP (solo ASCII sicuro).
fn per_intestazione(testo: &str) -> String {
    let mut out = String::with_capacity(testo.len() * 3);
    for b in testo.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~' | b' ') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

fn manda_zip(zip: &[u8], id: &str, codice: &str, motivo: &str, riassunto: &str) -> Result<(), Invio> {
    let risposta = agente(TIMEOUT_PACCHETTO)
        .post(&format!("{INDIRIZZO}/v1/bundle"))
        .header("X-Aurora-Key", CHIAVE_INGRESSO)
        .header("X-Aurora-Installazione", id)
        .header("X-Aurora-Codice", codice)
        .header("X-Aurora-Motivo", motivo)
        .header("X-Aurora-Versione", aurora_core::version::APP_VERSION)
        .header("X-Aurora-Riassunto", &per_intestazione(riassunto))
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

/// Il computer, a grandi linee, per capire su che macchine Aurora fatica (decodifica lenta, blocchi): modello del processore, processori
/// logici e memoria. Niente che identifichi la persona o il PC (nessun numero di serie, nome del computer o indirizzo).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pc {
    pub cpu: String,
    pub core: u32,
    pub ram_gb: u32,
}

impl Pc {
    /// «Intel(R) Core(TM) i5-8250U CPU @ 1.60GHz · 8 core · 8 GB»
    pub fn descrizione(&self) -> String {
        format!("{} · {} core · {} GB", self.cpu, self.core, self.ram_gb)
    }
}

/// Il computer, letto una volta.
pub fn pc() -> &'static Pc {
    static PC: OnceLock<Pc> = OnceLock::new();
    PC.get_or_init(|| {
        let cpu = crate::support::cpu_model().map(|c| c.split_whitespace().collect::<Vec<_>>().join(" ")).filter(|c| !c.is_empty()).unwrap_or_else(|| "?".into());
        let core = std::thread::available_parallelism().map_or(0, |n| u32::try_from(n.get()).unwrap_or(u32::MAX));
        let ram_gb = crate::support::ram_bytes().map_or(0, |b| u32::try_from((b + (1 << 29)) >> 30).unwrap_or(u32::MAX));
        Pc { cpu: cpu.chars().take(80).collect(), core, ram_gb }
    })
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

/// La data di oggi in UTC, `AAAA-MM-GG`.
fn giorno_utc() -> String {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64);
    let (y, m, d, ..) = crate::support::civil(secs);
    format!("{y:04}-{m:02}-{d:02}")
}

// ---------------------------------------------------------------------------------------------------------------------------------
// Il registro dell'installatore
// ---------------------------------------------------------------------------------------------------------------------------------

/// Il registro che l'installatore di Windows scrive nella cartella dati a ogni installazione o aggiornamento (sovrascrive il precedente):
/// versioni, modo, Windows, spazio, WebView2, i passi, gli avvisi e l'esito (`desktop/src-tauri/installer/hooks.nsh`).
const FILE_INSTALLAZIONE: &str = "installazione.log";
/// L'impronta dell'ultimo registro dell'installazione gia' guardato: il suo rapporto parte una volta sola.
const FILE_INSTALLAZIONE_VISTA: &str = "rapporti-installazione-vista.txt";
/// Un registro dell'installazione piu' grande di cosi' non e' dell'installatore: non si legge.
const MAX_REGISTRO: u64 = 1024 * 1024;
/// Le righe dei dettagli (l'elenco della finestra dell'installatore) stanno tra questi due segni: non sono passi di Aurora.
const INIZIO_DETTAGLI: &str = "--- dettagli dell'installazione";
const FINE_DETTAGLI: &str = "--- fine dei dettagli";

/// Il registro dell'installazione come testo, se c'e': UTF-16LE con BOM (lo scrive NSIS), oppure UTF-8.
pub(crate) fn registro_installazione(dir: &Path) -> Option<String> {
    let file = dir.join(FILE_INSTALLAZIONE);
    if std::fs::metadata(&file).ok()?.len() > MAX_REGISTRO {
        return None;
    }
    std::fs::read(&file).ok().map(|d| testo_registro(&d))
}

fn testo_registro(dati: &[u8]) -> String {
    if let Some(resto) = dati.strip_prefix(&[0xFF, 0xFE]) {
        let unita = resto.as_chunks::<2>().0.iter().map(|c| u16::from_le_bytes(*c));
        return char::decode_utf16(unita).map(|c| c.unwrap_or(char::REPLACEMENT_CHARACTER)).collect();
    }
    String::from_utf8_lossy(dati.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(dati)).into_owned()
}

/// Com'e' andata l'installazione, secondo il suo registro.
#[derive(Debug, Default, PartialEq, Eq)]
struct Installazione {
    versione_nuova: String,
    versione_prima: String,
    modo: String,
    /// Le righe «AVVISO: …» ed «ERRORE: …», senza l'ora.
    problemi: Vec<String>,
    /// «riuscita», «fallita», «annullata»; vuoto se il registro non arriva in fondo (installatore chiuso a meta').
    esito: String,
    /// L'installatore ha gia' mandato il suo rapporto (installazione non riuscita): Aurora non lo rimanda.
    gia_mandato: bool,
    /// L'ultimo passo scritto prima dei dettagli: dove si e' fermata un'installazione interrotta.
    ultimo_passo: String,
}

/// Una riga del registro senza l'ora davanti («[10:22:34] »).
fn senza_ora(riga: &str) -> &str {
    let r = riga.trim();
    match r.strip_prefix('[').and_then(|x| x.split_once(']')) {
        Some((ora, resto)) if ora.chars().all(|c| c.is_ascii_digit() || c == ':') => resto.trim_start(),
        _ => r,
    }
}

fn leggi_installazione(testo: &str) -> Installazione {
    let mut i = Installazione::default();
    let mut dettagli = false;
    for riga in testo.lines() {
        // dopo i dettagli possono venire l'ambiente (antivirus, file bloccati) e l'esito dell'invio
        if riga.starts_with(INIZIO_DETTAGLI) || riga.starts_with(FINE_DETTAGLI) {
            dettagli = riga.starts_with(INIZIO_DETTAGLI);
            continue;
        }
        let r = senza_ora(riga);
        if dettagli || r.is_empty() {
            continue;
        }
        if let Some(v) = r.strip_prefix("versione nuova:") {
            i.versione_nuova = v.split_whitespace().next().unwrap_or_default().to_string();
        } else if let Some(v) = r.strip_prefix("versione installata prima:") {
            i.versione_prima = v.trim().to_string();
        } else if let Some(v) = r.strip_prefix("modo:") {
            i.modo = v.trim().to_string();
        } else if let Some(v) = r.strip_prefix("esito:") {
            i.esito = v.split([':', ' ']).find(|p| !p.is_empty()).unwrap_or_default().to_string();
        } else if r.starts_with("rapporto dell'installatore: mandato") {
            i.gia_mandato = true;
        } else if r.starts_with("AVVISO:") || r.starts_with("ERRORE:") {
            i.problemi.push(r.to_string());
        }
        // i passi dopo l'esito (fine, invio del rapporto) non dicono dove si e' fermata
        if i.esito.is_empty() {
            i.ultimo_passo = r.to_string();
        }
    }
    i
}

/// Il rapporto per questa installazione, livello e messaggio (ancora da ripulire): `None` se e' andata bene, se l'ha annullata la
/// persona o se l'installatore ha gia' mandato il suo. Prima la situazione (il servizio raggruppa sui primi 300 caratteri), poi modo e versioni.
fn messaggio_installazione(i: &Installazione) -> Option<(&'static str, String)> {
    if i.gia_mandato {
        return None;
    }
    let (livello, mut m) = match i.esito.as_str() {
        "riuscita" if i.problemi.is_empty() => return None,
        "annullata" => return None,
        "riuscita" => ("warn", "installazione riuscita con avvisi".to_string()),
        "" => ("error", format!("installazione interrotta prima della fine (ultimo passo: {})", i.ultimo_passo)),
        altro => ("error", format!("installazione non riuscita ({altro})")),
    };
    for p in i.problemi.iter().take(6) {
        m.push_str(" | ");
        m.push_str(p);
    }
    let versioni = match (i.versione_prima.as_str(), i.versione_nuova.as_str()) {
        ("" | "nessuna", nuova) => format!("prima installazione di {nuova}"),
        (prima, nuova) => format!("da {prima} a {nuova}"),
    };
    m.push_str(&format!(" ({}; {versioni})", if i.modo.is_empty() { "modo ?" } else { i.modo.as_str() }));
    Some((livello, m))
}

/// L'impronta del registro (SHA-256, i primi 16 caratteri): lo stesso registro non fa partire due rapporti.
fn impronta(dati: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(dati).iter().take(8).map(|b| format!("{b:02x}")).collect()
}

impl Rapporti {
    /// Al primo giro dopo l'avvio: se l'ultima installazione ha avuto avvisi o errori (WebView2 mancante, file non copiati, Hamlib tolto
    /// dall'antivirus, installatore fermato a meta') parte un rapporto area «installatore», una sola volta per installazione. Con gli
    /// errori automatici spenti il registro si da' per visto e non parte nulla (riaccendendoli non torna un'installazione vecchia).
    fn segnala_installazione(&self) {
        let file = self.dir.join(FILE_INSTALLAZIONE);
        let Ok(meta) = std::fs::metadata(&file) else { return };
        if meta.len() > MAX_REGISTRO {
            return;
        }
        let Ok(dati) = std::fs::read(&file) else { return };
        let firma = impronta(&dati);
        let vista = self.dir.join(FILE_INSTALLAZIONE_VISTA);
        if std::fs::read_to_string(&vista).is_ok_and(|v| v.trim() == firma) {
            return;
        }
        let installazione = leggi_installazione(&testo_registro(&dati));
        // senza esito e appena scritto: l'installatore potrebbe essere ancora al lavoro, se ne riparla al prossimo avvio
        let recente = meta.modified().ok().and_then(|t| t.elapsed().ok()).is_some_and(|d| d < Duration::from_secs(300));
        if installazione.esito.is_empty() && recente {
            return;
        }
        let acceso = self.stato.lock().map(|s| s.impostazioni.auto_errors).unwrap_or(false);
        if let Some((livello, messaggio)) = messaggio_installazione(&installazione).filter(|_| acceso && configurato()) {
            let contesto = self.contesto();
            let messaggio: String = pulisci(&messaggio, &contesto.noti).chars().take(MAX_MESSAGGIO).collect();
            let g = Gruppo { livello, area: "installatore", origine: "installatore".into(), messaggio, conteggio: 1 };
            if let Ok(mut s) = self.stato.lock() {
                let r = rapporto(&s.id, &contesto, &giorno(secondi()), &g);
                s.coda.push(r);
                if s.coda.len() > MAX_CODA {
                    s.coda.remove(0);
                }
                scrivi_json(&self.dir.join(FILE_CODA), &s.coda);
            }
        }
        let _ = std::fs::write(&vista, &firma);
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn webview2_noise_in_stderr_is_not_a_crash() {
        let dir = std::env::temp_dir().join(format!("aurora-stderr-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let f = dir.join("stderr.log");
        let _ = std::fs::write(&f, "[1113/142415.207:ERROR:window_impl.cc:172] Failed to unregister class Chrome_WidgetWin_0. Error = 1412
");
        assert!(!stderr_vero(&f, 0));
        let _ = std::fs::write(&f, "thread 'main' panicked at src/x.rs:1:1
");
        assert!(stderr_vero(&f, 0));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn pulseaudio_disconnections_are_not_reported() {
        assert!(rumore_di_libreria("Reactor error: Client disconnected log.target=pulseaudio::client::reactor log.module_path=pulseaudio::client::reactor"));
        assert!(!rumore_di_libreria("ingresso audio non trovato"));
    }

    #[test]
    fn il_pc_si_descrive_senza_dati_personali() {
        let pc = super::pc();
        assert!(pc.core >= 1 && !pc.cpu.is_empty(), "{pc:?}");
        let d = super::Pc { cpu: "Intel(R) Celeron(R) N4020".into(), core: 2, ram_gb: 4 }.descrizione();
        assert_eq!(d, "Intel(R) Celeron(R) N4020 · 2 core · 4 GB");
        let corpo = super::corpo_attivo(true, "00000000-0000-4000-8000-000000000000", "").expect("corpo");
        let v: serde_json::Value = serde_json::from_str(&corpo).expect("json");
        assert!(v["cpu"].is_string() && v["core"].is_u64() && v["ram_gb"].is_u64(), "{v}");
    }

    #[test]
    fn la_radio_non_collegata_non_e_un_errore_da_segnalare() {
        assert!(super::scelta_dell_operatore("trasmissione fallita: PTT: nessuna radio collegata: scegli il CAT in Impostazioni"));
        assert!(super::scelta_dell_operatore("PTT: spegnimento fallito (x): PTT: radio non collegata"));
        assert!(super::scelta_dell_operatore("trasmissione fallita: PTT: PTT bloccato per altri 3 s dopo il collegamento"));
        assert!(!super::scelta_dell_operatore("errore del flusso di uscita audio: Failed to get current padding"));
        // la porta tenuta da un altro programma e OmniRig che si avvia (o con la radio spenta): avvisi a schermo, non rapporti
        assert!(super::scelta_dell_operatore("PTT sulla seriale non attivo: La porta COM8 e' occupata da OmniRig: chiudi OmniRig oppure scegli OmniRig come sistema di controllo in Impostazioni › Stazione › Radio"));
        assert!(super::scelta_dell_operatore("trasmissione fallita: PTT: OmniRig non risponde: la radio e' accesa e in linea in OmniRig?"));
        assert!(!super::scelta_dell_operatore("trasmissione fallita: PTT: OmniRig non risponde: apri OmniRig e controlla che Rig1 abbia la porta e il modello giusti"));
    }

    #[test]
    fn il_nominativo_si_normalizza_prima_del_codice() {
        assert_eq!(super::nominativo_base("IU3ABC").as_deref(), Some("IU3ABC"));
        assert_eq!(super::nominativo_base("iu3abc/p").as_deref(), Some("IU3ABC"));
        assert_eq!(super::nominativo_base("EA/IU3ABC").as_deref(), Some("IU3ABC"));
        assert_eq!(super::nominativo_base("IU3ABC/MM").as_deref(), Some("IU3ABC"));
        assert_eq!(super::nominativo_base("IU3ABC/QRP").as_deref(), Some("IU3ABC"));
        assert_eq!(super::nominativo_base(""), None);
        assert_eq!(super::codice_persona(""), None);
    }

    #[test]
    fn lo_stesso_nominativo_da_sempre_lo_stesso_codice() {
        let a = super::codice_persona("IU3ABC").unwrap_or_default();
        assert_eq!(a.len(), 64);
        assert!(a.bytes().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(super::codice_persona("iu3abc/p").as_deref(), Some(a.as_str()));
        assert_eq!(super::codice_persona("EA/IU3ABC").as_deref(), Some(a.as_str()));
        assert_ne!(super::codice_persona("IU3ABD").as_deref(), Some(a.as_str()));
    }

    #[test]
    fn il_segnale_non_contiene_il_nominativo_e_spento_non_parte() {
        let corpo = super::corpo_attivo(true, "abc", "IU3ABC").unwrap_or_default();
        assert!(!corpo.contains("IU3ABC"));
        assert!(corpo.contains("\"persona\""));
        assert!(!super::corpo_attivo(true, "abc", "").unwrap_or_default().contains("persona"));
        assert_eq!(super::corpo_attivo(false, "abc", "IU3ABC"), None);
    }

    #[test]
    fn il_riassunto_del_crash_va_in_un_intestazione_e_si_ferma_a_1500_caratteri() {
        let file = vec![("crash/panic-1.txt".to_string(), "Panic\nmessaggio: there is no reactor running\n".repeat(100).into_bytes())];
        let r = riassunto_crash(&file);
        assert!(r.starts_with("panic-1.txt\nPanic") && r.len() <= 1500);
        let h = per_intestazione("a b:è\n");
        assert_eq!(h, "a b%3A%C3%A8%0A");
        assert!(riassunto_crash(&[("logs/aurora.log".into(), b"x".to_vec())]).is_empty());
    }

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
        // parole intere per i valori corti; i nomi generici (User, Radio) restano: non sono dati personali
        assert_eq!(pulisci("Bob e Bobby", &["bob".into()]), "<personale> e Bobby");
        assert_eq!(pulisci("nessuna radio Cat4OM collegata", &["Radio".into(), "User".into()]), "nessuna radio Cat4OM collegata");
        assert_eq!(pulisci(r"C:\Users\User\AppData", &["User".into()]), r"C:\Users\<utente>\AppData");
        // i valori lunghi si tolgono anche dentro un'altra parola
        assert_eq!(pulisci("cartella tizianocassone2", &["tizianocassone".into()]), "cartella <personale>2");
    }

    #[test]
    fn la_fine_di_un_blocco_va_nei_rapporti_con_la_sua_area() {
        assert!(da_segnalare(tracing::Level::WARN, "aurora_desktop::blocco"));
        assert_eq!(area("aurora_desktop::blocco", "blocco del programma finito dopo 31 s"), "blocco");
        assert_eq!(area("aurora_desktop_lib::crash", "programma bloccato (finestra) da oltre 20 s"), "blocco");
    }

    #[test]
    fn la_decodifica_lenta_va_nei_rapporti_come_decoder() {
        assert!(da_segnalare(tracing::Level::WARN, "engine_native::lento"));
        assert!(!da_segnalare(tracing::Level::WARN, "engine_native"));
        assert_eq!(area("engine_native::lento", "decodifica lenta: 4 periodi su 10"), "decoder");
    }

    fn rapporti_di_prova(dir: &Path, limiti: Limiti) -> Rapporti {
        let _ = std::fs::create_dir_all(dir);
        Rapporti {
            dir: dir.to_path_buf(),
            stato: Mutex::new(Stato { impostazioni: Impostazioni::default(), id: uuid(), limiti, coda: Vec::new(), gruppi: HashMap::new(), crash_pendente: None }),
            contesto: Mutex::new(None),
            invio: Mutex::new(()),
        }
    }

    fn lento(ms: u32) -> Evento {
        Evento { livello: "warn", origine: "engine_native::lento".into(), messaggio: format!("decodifica lenta: 4 periodi su 10 oltre il 70% del tempo (FT8 in {ms} ms su 15000 ms; 4 processori, 3 thread di decodifica, profondita' 3, SWL spento, a8 spento)") }
    }

    #[test]
    fn la_decodifica_lenta_parte_una_volta_per_installazione_e_versione() {
        let dir = std::env::temp_dir().join(format!("aurora-rapporti-lento-{}", std::process::id()));
        let r = rapporti_di_prova(&dir, Limiti::default());
        r.raggruppa(vec![lento(11_709), lento(9_870)]);
        r.matura();
        let coda = r.stato.lock().map(|s| s.coda.len()).unwrap_or(0);
        assert_eq!(coda, 1, "la prima volta con questa versione parte un rapporto solo, anche con numeri di cifre diverse");
        // al riavvio (limiti riletti dal file, ore dopo): la stessa situazione non riparte, con qualunque numero
        let mut limiti: Limiti = leggi_json(&dir.join(FILE_LIMITI)).unwrap_or_default();
        limiti.ultimi.clear();
        let r = rapporti_di_prova(&dir, limiti.clone());
        r.raggruppa(vec![lento(12_001)]);
        r.matura();
        let (coda, gruppi) = r.stato.lock().map(|s| (s.coda.len(), s.gruppi.len())).unwrap_or((9, 9));
        assert_eq!((coda, gruppi), (0, 0), "gia' segnalata con questa versione: buttata, non tenuta in attesa");
        // gli errori veri partono come prima
        r.raggruppa(vec![Evento { livello: "warn", origine: "rig_control::hamlib".into(), messaggio: "rigctld fermo".into() }]);
        r.matura();
        assert_eq!(r.stato.lock().map(|s| s.coda.len()).unwrap_or(0), 1);
        // con una versione nuova si sa di nuovo una volta
        limiti.per_versione.insert("engine_native::lento".into(), "0.0.1".into());
        let r = rapporti_di_prova(&dir, limiti);
        r.raggruppa(vec![lento(11_709)]);
        r.matura();
        assert_eq!(r.stato.lock().map(|s| s.coda.len()).unwrap_or(0), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn solo_le_situazioni_della_stazione_partono_una_volta_per_versione() {
        let mut l = Limiti::default();
        l.per_versione.insert("engine_native::lento".into(), "1.0".into());
        l.per_versione.insert("rig_control".into(), "1.0".into());
        assert!(gia_segnalato(&l, "engine_native::lento", "1.0"));
        assert!(!gia_segnalato(&l, "engine_native::lento", "1.1"));
        assert!(!gia_segnalato(&l, "rig_control", "1.0"), "gli errori non sono limitati per versione");
    }

    #[test]
    fn l_associazione_non_riuscita_va_nei_rapporti_come_telefono() {
        assert!(da_segnalare(tracing::Level::WARN, "remote_api::associazione"));
        assert!(!da_segnalare(tracing::Level::WARN, "remote_api::routes"), "gli altri avvisi del server no");
        assert_eq!(area("remote_api::associazione", "associazione dal telefono non riuscita"), "telefono");
    }

    #[test]
    fn errori_uguali_stessa_chiave() {
        assert_eq!(chiave("cat", "x", "porta 4532 chiusa"), chiave("cat", "x", "porta 4533 chiusa"));
        assert_eq!(area("rig_control::hamlib", "rigctld fermo"), "cat");
        assert_eq!(area("aurora_desktop::health", "panic nel thread"), "panic");
    }

    /// Un registro dell'installatore come lo scrive `hooks.nsh` (con i dati personali da togliere).
    const REGISTRO: &str = "Registro dell'installazione di Aurora
inizio: 2026-11-12 10:22:33
versione nuova: 0.11.240 (x64)
versione installata prima: 0.11.239
modo: aggiornamento silenzioso da Aurora (/S /UPDATE)
Windows: Windows 11 Pro 24H2 (build 26100.2314)
cartella dei dati: C:\\Users\\Mario Rossi\\AppData\\Roaming\\Aurora
[10:22:34] WebView2: presente (131.0.2903.70)
[10:22:34] cartella di installazione: C:\\Users\\mario\\AppData\\Local\\Aurora (spazio libero 51234 MB)
[10:22:39] AVVISO: cartella hamlib vecchia non tolta dopo 4 tentativi (file bloccati) su PC-MARIO di mario
[10:22:41] rete: 192.168.1.20:4532 fe80::1 mario.rossi@example.com IU3ABC JN65ab token: abc123
[10:22:41] chiave 9f8e7d6c5b4a39281706f5e4d3c2b1a0 su C:\\Users\\User\\Desktop e radio User
[10:22:45] AVVISO: file mancante dopo la copia: hamlib\\rigctld.exe (tolto dall'antivirus?)
esito: riuscita
fine: 2026-11-12 10:22:46
--- dettagli dell'installazione (2 righe) ---
Estrazione: rigctld.exe
AVVISO: questa riga e' dell'elenco, non un passo
--- fine dei dettagli ---
antivirus: Windows Defender (acceso, aggiornato)
";

    fn noti_di_prova() -> Vec<String> {
        vec!["mario".into(), "PC-MARIO".into(), "User".into()]
    }

    #[test]
    fn il_registro_dell_installatore_si_ripulisce() {
        let p = pulisci(REGISTRO, &noti_di_prova());
        assert!(p.contains(r"cartella dei dati: C:\Users\<utente>\AppData\Roaming\Aurora"), "il cognome dopo il nome noto non resta: {p}");
        assert!(p.contains(r"cartella di installazione: C:\Users\<utente>\AppData\Local\Aurora"), "{p}");
        assert!(p.contains("(file bloccati) su <personale> di <personale>"), "{p}");
        assert!(p.contains("rete: <ip> <ip> <personale><email> <nominativo> <locatore> token: ***"), "{p}");
        assert!(p.contains(r"chiave <codice> su C:\Users\<utente>\Desktop e radio User"), "{p}");
        // versioni, build di Windows e WebView2 restano
        assert!(p.contains("versione nuova: 0.11.240 (x64)") && p.contains("build 26100.2314") && p.contains("131.0.2903.70"), "{p}");
        for via in ["mario", "Mario", "Rossi", "192.168", "example.com", "IU3ABC", "JN65"] {
            assert!(!p.contains(via), "{via} e' rimasto: {p}");
        }
    }

    #[test]
    fn il_registro_si_legge_in_utf16_e_in_utf8() {
        let mut utf16 = vec![0xFF, 0xFE];
        utf16.extend("esito: riuscita è\r\n".encode_utf16().flat_map(u16::to_le_bytes));
        assert_eq!(testo_registro(&utf16), "esito: riuscita è\r\n");
        assert_eq!(testo_registro(b"\xEF\xBB\xBFesito: fallita"), "esito: fallita");
        assert_eq!(testo_registro(b"esito: annullata"), "esito: annullata");
    }

    #[test]
    fn dal_registro_un_rapporto_solo_se_qualcosa_non_va() {
        // riuscita con avvisi: rapporto di livello warn con gli avvisi (non le righe dell'elenco dei dettagli), modo e versioni
        let i = leggi_installazione(REGISTRO);
        assert_eq!((i.versione_nuova.as_str(), i.versione_prima.as_str(), i.esito.as_str()), ("0.11.240", "0.11.239", "riuscita"));
        assert_eq!(i.problemi.len(), 2, "{i:?}");
        let (livello, m) = messaggio_installazione(&i).unwrap_or_default();
        assert_eq!(livello, "warn");
        assert!(m.starts_with("installazione riuscita con avvisi | AVVISO: cartella hamlib vecchia non tolta"), "{m}");
        assert!(m.contains("AVVISO: file mancante dopo la copia: hamlib\\rigctld.exe") && !m.contains("dell'elenco"), "{m}");
        assert!(m.ends_with("(aggiornamento silenzioso da Aurora (/S /UPDATE); da 0.11.239 a 0.11.240)"), "{m}");
        // riuscita senza avvisi: niente
        let ok = "versione nuova: 0.11.240 (x64)\nversione installata prima: nessuna\nmodo: normale\n[10:00:01] WebView2: presente (131)\nesito: riuscita\n";
        assert_eq!(messaggio_installazione(&leggi_installazione(ok)), None);
        // annullata dalla persona: niente
        assert_eq!(messaggio_installazione(&leggi_installazione("modo: normale\nesito: annullata\n")), None);
        // fallita: errore con il motivo; se l'installatore l'ha gia' mandato Aurora non lo rimanda
        let ko = "versione nuova: 0.11.240 (x64)\nversione installata prima: nessuna\nmodo: normale, con le finestre\n[10:00:01] ERRORE: WebView2: installazione non riuscita (codice di uscita 1603)\nesito: fallita\n";
        let (livello, m) = messaggio_installazione(&leggi_installazione(ko)).unwrap_or_default();
        assert_eq!(livello, "error");
        assert_eq!(m, "installazione non riuscita (fallita) | ERRORE: WebView2: installazione non riuscita (codice di uscita 1603) (normale, con le finestre; prima installazione di 0.11.240)");
        let mandato = format!("{ko}rapporto dell'installatore: mandato (rapporto mandato, pacchetto 1A2B3C4D)\n");
        assert_eq!(messaggio_installazione(&leggi_installazione(&mandato)), None);
        // interrotta (senza esito): errore con l'ultimo passo
        let rotto = "versione nuova: 0.11.240 (x64)\nmodo: aggiornamento silenzioso da Aurora (/S /UPDATE)\n[10:00:01] WebView2: presente (131)\n[10:00:02] copia dei file\n";
        let (livello, m) = messaggio_installazione(&leggi_installazione(rotto)).unwrap_or_default();
        assert_eq!(livello, "error");
        assert!(m.starts_with("installazione interrotta prima della fine (ultimo passo: copia dei file)"), "{m}");
    }

    #[test]
    fn il_rapporto_dell_installazione_parte_una_volta_e_non_con_i_rapporti_spenti() {
        let dir = std::env::temp_dir().join(format!("aurora-rapporti-installazione-{}", std::process::id()));
        let r = rapporti_di_prova(&dir, Limiti::default());
        let mut utf16 = vec![0xFF, 0xFE];
        utf16.extend(REGISTRO.encode_utf16().flat_map(u16::to_le_bytes));
        let _ = std::fs::write(dir.join(FILE_INSTALLAZIONE), &utf16);
        r.segnala_installazione();
        let coda = r.stato.lock().map(|s| s.coda.clone()).unwrap_or_default();
        assert_eq!(coda.len(), 1);
        assert_eq!(coda[0]["area"], "installatore");
        assert_eq!(coda[0]["livello"], "warn");
        let m = coda[0]["messaggio"].as_str().unwrap_or_default();
        assert!(m.contains("file mancante dopo la copia") && !m.contains("192.168"), "{m}");
        // lo stesso registro non riparte
        r.segnala_installazione();
        assert_eq!(r.stato.lock().map(|s| s.coda.len()).unwrap_or(0), 1);
        // un registro nuovo con i rapporti spenti: nulla in coda, ma dato per visto
        let spenti = rapporti_di_prova(&dir, Limiti::default());
        if let Ok(mut s) = spenti.stato.lock() {
            s.impostazioni.auto_errors = false;
        }
        let _ = std::fs::write(dir.join(FILE_INSTALLAZIONE), format!("{REGISTRO}\nfine bis\n"));
        spenti.segnala_installazione();
        assert_eq!(spenti.stato.lock().map(|s| s.coda.len()).unwrap_or(9), 0);
        let firma = impronta(&std::fs::read(dir.join(FILE_INSTALLAZIONE)).unwrap_or_default());
        assert_eq!(std::fs::read_to_string(dir.join(FILE_INSTALLAZIONE_VISTA)).unwrap_or_default(), firma);
        // nel pacchetto del crash il registro c'e', ripulito
        let file = r.pacchetto_crash();
        let (_, dati) = file.iter().find(|(n, _)| n == FILE_INSTALLAZIONE).cloned().unwrap_or_default();
        let testo = String::from_utf8_lossy(&dati);
        assert!(testo.contains("esito: riuscita") && !testo.contains("192.168"), "{testo}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn percorsi_con_un_nome_gia_tolto_in_parte() {
        assert_eq!(pulisci(r"C:\Users\Mario Rossi\AppData", &["mario".into()]), r"C:\Users\<utente>\AppData");
        assert_eq!(p(r"C:\Users\<utente>\AppData"), r"C:\Users\<utente>\AppData");
    }

    #[test]
    fn uuid_versione_4() {
        let u = uuid();
        assert_eq!(u.len(), 36);
        assert_eq!(&u[14..15], "4");
    }
}
