//! Invio facoltativo degli spot WSPR e FST4W a WSPRnet, come «Upload spots» di WSJT-X 3.3.0-beta6 (`Network/wsprnet.cpp`, spento di
//! fabbrica: `mainwindow_settings.cpp:594`). Spento non si accoda e non si manda niente.
//!
//! - **Quando**: dopo la decodifica di ogni periodo WSPR o FST4W ricevuto dal vivo, con un ritardo a caso fra 0 e 20 s
//!   (`uploadTimer`, `mainwindow.cpp:6789-6795,14209-14214`), solo con la frequenza della radio nota (WSJT-X vuole il CAT acceso).
//! - **Cosa**: una richiesta POST `application/x-www-form-urlencoded` a `http://wsprnet.org/post/` per ogni messaggio ricevuto
//!   (`function=wspr`, `urlEncodeSpot`), oppure, se nel periodo non c'e' niente da segnalare, una sola richiesta di stato
//!   (`function=wsprstat`, `urlEncodeNoSpot`) con la frequenza di ricezione e trasmissione, «Tx Pct» (0 con Abilita Tx spento) e la potenza.
//!   I messaggi WSPR con il nominativo non risolto (`<...>`) o di forma sconosciuta si scartano (`decodeLine`), come quelli fuori di 10 kHz
//!   dalla frequenza di ricezione.
//! - **Nuovi tentativi**: una richiesta alla volta; se fallisce (rete, risposta HTTP d'errore, o uno spot senza «spot(s) added» nella
//!   risposta) si riprova dopo 15 s, 1, 2, 5, 10 e 20 minuti con un margine a caso del 25%, al piu' 7 volte e per un'ora; in coda al piu'
//!   1024 richieste, le piu' vecchie escono prima (`RetryPolicy`, `wsprnet.h:33-40`).
//!
//! WSJT-X 3.3.0-beta6 manda gli spot con il locatore anche a un secondo indirizzo (`wsprnet.eu:3000`, `wsprNetUrl2`): Aurora manda solo a
//! wsprnet.org, l'unico indirizzo detto nelle Impostazioni e nell'informativa.

use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

use aurora_core::WsprCycle;

/// L'indirizzo di WSPRnet (`wsprNetUrl`, `wsprnet.cpp:55`).
pub const URL: &str = "http://wsprnet.org/post/";
/// Attesa massima di una richiesta.
pub const TIMEOUT: Duration = Duration::from_secs(30);
/// Ritardo massimo, a caso, fra la fine della decodifica e l'invio (`bounded (0, 20000)`).
pub const MAX_DELAY: Duration = Duration::from_secs(20);
/// `RetryPolicy` di `wsprnet.h`.
pub const MAX_ATTEMPTS: u32 = 7;
pub const MAX_PENDING: usize = 1024;
pub const TTL: Duration = Duration::from_secs(3600);
pub const RETRY_DELAYS_S: [u64; 6] = [15, 60, 120, 300, 600, 1200];
pub const JITTER: f64 = 0.25;

/// La stazione che riceve: nominativo, locatore e programma (`version`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Station {
    pub call: String,
    pub grid: String,
    pub version: String,
}

/// Uno spot o lo stato della stazione.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Spot,
    Status,
}

/// Una richiesta pronta: il corpo gia' codificato.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    pub kind: Kind,
    pub body: String,
}

/// Il campo `mode` (`encode_mode`, `wsprnet.cpp:721-735`): WSPR-2 «2»; FST4W i minuti del periodo, con 2 minuti che diventa «3» per non
/// confondersi con WSPR-2.
pub fn encode_mode(mode: &str, period_s: u32) -> String {
    match mode {
        "WSPR" => "2".to_string(),
        "WSPR-15" => "15".to_string(),
        "FST4W" => {
            let mut tr = ((f64::from(period_s) / 60.0) + 0.5) as u32;
            if tr == 2 {
                tr += 1;
            }
            tr.to_string()
        }
        _ => String::new(),
    }
}

/// MHz con sei decimali, come `QString ("%1").arg (f / 1e6, 0, 'f', 6)`.
fn mhz(hz: f64) -> String {
    format!("{:.6}", hz / 1e6)
}

/// Codifica di un valore come `QUrlQuery::query (QUrl::FullyEncoded)`: lettere, cifre e `-._~!$'()*,;:@/?` restano, il resto (spazio
/// compreso) diventa `%XX`.
fn encode_value(v: &str) -> String {
    let mut out = String::with_capacity(v.len());
    for b in v.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~!$'()*,;:@/?".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// Il corpo `chiave=valore&...` nell'ordine dato.
pub fn form(fields: &[(&str, String)]) -> String {
    fields.iter().map(|(k, v)| format!("{k}={}", encode_value(v))).collect::<Vec<_>>().join("&")
}

/// Data (`AAMMGG`) e ora (`HHMM`) UTC dei campi `date` e `time` di WSPRnet, dalla stessa funzione dei record ADIF (`AAAAMMGG`, `HHMMSS`).
fn utc_date_time(unix_ms: u64) -> (String, String) {
    let (date, time) = logbook::adif::utc_datetime(unix_ms);
    (date.get(2..).unwrap_or_default().to_string(), time.get(..4).unwrap_or_default().to_string())
}

fn is_call_char(c: char) -> bool {
    c.is_ascii_uppercase() || c.is_ascii_digit()
}

fn is_grid4(g: &str) -> bool {
    let b = g.as_bytes();
    b.len() == 4 && (b'A'..=b'R').contains(&b[0]) && (b'A'..=b'R').contains(&b[1]) && b[2].is_ascii_digit() && b[3].is_ascii_digit()
}

fn is_grid6(g: &str) -> bool {
    let b = g.as_bytes();
    b.len() == 6 && g.get(..4).is_some_and(is_grid4) && (b'A'..=b'X').contains(&b[4]) && (b'A'..=b'X').contains(&b[5])
}

/// Le cifre all'inizio di una parola (`\d+` delle espressioni di WSJT-X), vuoto se non comincia con una cifra.
fn leading_digits(t: &str) -> String {
    t.bytes().take_while(u8::is_ascii_digit).map(char::from).collect()
}

fn is_number(t: &str) -> bool {
    !t.is_empty() && t.bytes().all(|b| b.is_ascii_digit())
}

/// Nominativo, locatore e potenza di un messaggio WSPR nei tre tipi di `decodeLine` (`wsprnet.cpp:657-714`); `None` se la forma non e' nota.
/// Come le espressioni di WSJT-X, vale l'inizio del messaggio (il tipo 3 e il tipo 2 si provano dopo il tipo 1 e, se riconosciuti, vincono).
pub fn wspr_fields(message: &str) -> Option<(String, String, String)> {
    let t: Vec<&str> = message.split_whitespace().collect();
    let mut found = None;
    // tipo 1: `^([A-Z0-9]{3,6})\s+([A-R]{2}\d{2})\s+(\d+)`
    if t.len() >= 3 && (3..=6).contains(&t[0].len()) && t[0].chars().all(is_call_char) && is_grid4(t[1]) && !leading_digits(t[2]).is_empty() {
        found = Some((t[0].to_string(), t[1].to_string(), leading_digits(t[2])));
    }
    // tipo 2: `^([A-Z0-9/]+)\s+(\d+)`
    if t.len() >= 2 && t[0].chars().all(|c| is_call_char(c) || c == '/') && !leading_digits(t[1]).is_empty() {
        found = Some((t[0].to_string(), String::new(), leading_digits(t[1])));
    }
    // tipo 3: `^<([A-Z0-9/]+)>\s+([A-R]{2}\d{2}[A-X]{2})\s+(\d+)`
    if t.len() >= 3
        && let Some(call) = t[0].strip_prefix('<').and_then(|c| c.strip_suffix('>'))
        && !call.is_empty()
        && call.chars().all(|c| is_call_char(c) || c == '/')
        && is_grid6(t[1])
        && !leading_digits(t[2]).is_empty()
    {
        found = Some((call.to_string(), t[1].to_string(), leading_digits(t[2])));
    }
    found
}

/// Nominativo, locatore e potenza di un messaggio FST4W come li legge `fst4_re` (`wsprnet.cpp:79-87`): il nominativo (senza le parentesi),
/// poi, se ci sono, il locatore a 4 o 6 caratteri e la potenza.
pub fn fst4w_fields(message: &str) -> Option<(String, String, String)> {
    let mut t = message.split_whitespace();
    let first = t.next()?;
    let call = first.strip_prefix('<').unwrap_or(first);
    let call = call.strip_suffix('>').unwrap_or(call);
    if call.is_empty() || !call.chars().all(|c| is_call_char(c) || c == '/') {
        return None;
    }
    let rest: Vec<&str> = t.collect();
    let mut i = 0;
    let mut grid = String::new();
    if let Some(g) = rest.first()
        && (is_grid4(g) || is_grid6(g))
    {
        grid = (*g).to_string();
        i = 1;
    }
    let dbm = rest.get(i).filter(|d| is_number(d)).map(|d| (*d).to_string()).unwrap_or_default();
    Some((call.to_string(), grid, dbm))
}

/// Le richieste di un periodo decodificato: uno spot per messaggio valido, o lo stato della stazione se non ce n'e' nessuno. Vuoto se la
/// frequenza della radio non e' nota o il modo non e' WSPR/FST4W.
pub fn requests(cycle: &WsprCycle, station: &Station) -> Vec<Request> {
    if cycle.dial_hz == 0 || !matches!(cycle.mode.as_str(), "WSPR" | "FST4W") {
        return Vec::new();
    }
    let dial = cycle.dial_hz as f64;
    let rqrg = mhz(dial + 1500.0);
    let tqrg_beacon = mhz(dial + f64::from(cycle.tx_df));
    let mode = encode_mode(&cycle.mode, cycle.period_s);
    let fst4w = cycle.mode == "FST4W";
    let station_fields = |q: &mut Vec<(&str, String)>| {
        q.push(("version", station.version.clone()));
        q.push(("rcall", station.call.clone()));
        q.push(("rgrid", station.grid.clone()));
        q.push(("rqrg", rqrg.clone()));
        q.push(("mode", mode.clone()));
    };
    let (date, time) = utc_date_time(cycle.slot_start_unix_ms);
    let mut out = Vec::new();
    for s in &cycle.spots {
        let mut q: Vec<(&str, String)> = Vec::new();
        if fst4w {
            // FST4W: la data e' quella di adesso meno 3/4 del periodo, cioe' quella del periodo (`queueFst4wDecode`); niente filtro sulla banda
            let Some((call, grid, dbm)) = fst4w_fields(&s.message) else { continue };
            let freq = s.audio_hz.round();
            let (date, _) = utc_date_time(cycle.slot_start_unix_ms + u64::from(cycle.period_s) * 250);
            q.push(("function", "wspr".to_string()));
            q.push(("date", date));
            q.push(("time", time.clone()));
            q.push(("sig", s.snr.to_string()));
            q.push(("dt", format!("{:.1}", s.dt_s)));
            q.push(("tqrg", mhz(dial + 1500.0 + (freq - 1500.0))));
            q.push(("tcall", call));
            q.push(("drift", "0".to_string()));
            q.push(("tgrid", grid));
            q.push(("dbm", dbm));
        } else {
            if s.message.contains("<...>") {
                continue;
            }
            let Some((call, grid, dbm)) = wspr_fields(&s.message) else { continue };
            let tqrg = mhz(dial + s.audio_hz);
            // fuori di 10 kHz dalla frequenza di ricezione non e' la banda in uso
            let off = (rqrg.parse::<f64>().unwrap_or(0.0) - tqrg.parse::<f64>().unwrap_or(0.0)).abs();
            if off >= 0.01 {
                continue;
            }
            q.push(("function", "wspr".to_string()));
            q.push(("date", date.clone()));
            q.push(("time", time.clone()));
            q.push(("sig", format!("{}", s.snr)));
            q.push(("dt", format!("{:.1}", s.dt_s)));
            q.push(("drift", s.drift_hz.to_string()));
            q.push(("tqrg", tqrg));
            q.push(("tcall", call));
            q.push(("tgrid", grid));
            q.push(("dbm", dbm));
        }
        station_fields(&mut q);
        out.push(Request { kind: Kind::Spot, body: form(&q) });
    }
    if out.is_empty() {
        let q: Vec<(&str, String)> = vec![
            ("function", "wsprstat".to_string()),
            ("rcall", station.call.clone()),
            ("rgrid", station.grid.clone()),
            ("rqrg", rqrg.clone()),
            ("tpct", cycle.tx_pct.to_string()),
            ("tqrg", tqrg_beacon),
            ("dbm", cycle.dbm.to_string()),
            ("version", station.version.clone()),
            ("mode", mode.clone()),
        ];
        out.push(Request { kind: Kind::Status, body: form(&q) });
    }
    out
}

/// Manda una richiesta (bloccante: si chiama da un thread a parte). `Ok` con il testo della risposta se WSPRnet l'ha accettata: per uno
/// spot la risposta deve dire «spot(s) added» (`replyAccepted`, `wsprnet.cpp:542-552`).
pub fn send(url: &str, req: &Request, timeout: Duration, user_agent: &str) -> Result<String, String> {
    let mut response = crate::upload::agent(timeout, user_agent)
        .post(url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .send(req.body.as_bytes())
        .map_err(|e| crate::eqsl::describe(&e))?;
    let status = response.status().as_u16();
    let text = response.body_mut().read_to_string().unwrap_or_default();
    if !(200..300).contains(&status) {
        return Err(format!("WSPRnet ha risposto {status}"));
    }
    if req.kind == Kind::Spot && !text.contains("spot(s) added") {
        let short: String = text.trim().chars().take(120).collect();
        return Err(format!("WSPRnet non ha accettato lo spot: {short}"));
    }
    Ok(text)
}

/// Una richiesta in coda con i suoi tentativi.
#[derive(Clone, Debug)]
pub struct Pending {
    pub request: Request,
    pub attempts: u32,
    pub next_at: Instant,
    pub expires_at: Instant,
}

/// La coda delle richieste, con i tempi dei nuovi tentativi di WSJT-X.
#[derive(Debug, Default)]
pub struct Queue {
    items: VecDeque<Pending>,
    /// Una richiesta tolta dalla coda e in volo (se ne aspetta l'esito): conta fra quelle da mandare.
    pub sending: bool,
}

impl Queue {
    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Le richieste ancora da mandare: quelle in coda e quella in volo.
    pub fn outstanding(&self) -> usize {
        self.items.len() + usize::from(self.sending)
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }

    /// Aggiunge le richieste di un periodo. Lo stato si accoda solo se non c'e' gia' altro da mandare (`flush`, `wsprnet.cpp:182-190`).
    /// Restituisce quante richieste sono uscite perche' troppo vecchie o oltre il limite.
    pub fn push(&mut self, requests: Vec<Request>, now: Instant) -> usize {
        let mut dropped = self.prune(now);
        for request in requests {
            if request.kind == Kind::Status && !self.items.is_empty() {
                continue;
            }
            self.items.push_back(Pending { request, attempts: 0, next_at: now, expires_at: now + TTL });
        }
        while self.items.len() > MAX_PENDING {
            self.items.pop_front();
            dropped += 1;
        }
        dropped
    }

    fn prune(&mut self, now: Instant) -> usize {
        let before = self.items.len();
        self.items.retain(|p| p.expires_at > now);
        before - self.items.len()
    }

    /// Il momento in cui la prossima richiesta e' pronta.
    pub fn next_due(&self) -> Option<Instant> {
        self.items.iter().map(|p| p.next_at).min()
    }

    /// Toglie la prima richiesta pronta (nell'ordine di coda).
    pub fn take_due(&mut self, now: Instant) -> Option<Pending> {
        self.prune(now);
        let i = self.items.iter().position(|p| p.next_at <= now)?;
        self.items.remove(i)
    }

    /// Una richiesta fallita: torna in coda con il ritardo del tentativo (`retryDelayMs`, `canRetry`), oppure si lascia perdere.
    /// `jitter` fra 0 e 1. Restituisce il ritardo, o `None` se non si riprova piu'.
    pub fn retry(&mut self, mut p: Pending, now: Instant, jitter: f64) -> Option<Duration> {
        p.attempts += 1;
        let delay = retry_delay(p.attempts, jitter);
        if p.attempts >= MAX_ATTEMPTS || now + delay >= p.expires_at {
            return None;
        }
        p.next_at = now + delay;
        self.items.push_back(p);
        while self.items.len() > MAX_PENDING {
            self.items.pop_front();
        }
        Some(delay)
    }
}

/// Il ritardo dopo il tentativo numero `attempts` (da 1): la tabella di `RetryPolicy` con un margine del 25% (`jitter` fra 0 e 1).
pub fn retry_delay(attempts: u32, jitter: f64) -> Duration {
    let index = (attempts.max(1) as usize - 1).min(RETRY_DELAYS_S.len() - 1);
    let base = RETRY_DELAYS_S[index] as f64 * 1000.0;
    let factor = 1.0 - JITTER + jitter.clamp(0.0, 1.0) * JITTER * 2.0;
    Duration::from_millis((base * factor) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use aurora_core::WsprSpot;

    // 2026-11-11 21:04:00 UTC
    const SLOT: u64 = 1_794_431_040_000;

    fn station() -> Station {
        Station { call: "IU3ABC".into(), grid: "IO91wm".into(), version: "Aurora 0.11.0".into() }
    }

    fn cycle(mode: &str, period_s: u32, spots: Vec<WsprSpot>) -> WsprCycle {
        WsprCycle { mode: mode.into(), period_s, slot_start_unix_ms: SLOT, dial_hz: 14_095_600, tx_df: 1500, tx_pct: 20, dbm: 37, spots }
    }

    fn spot(message: &str, audio_hz: f64, drift_hz: i32) -> WsprSpot {
        WsprSpot { snr: -21, dt_s: -0.34, audio_hz, drift_hz, message: message.into() }
    }

    #[test]
    fn the_date_is_utc() {
        assert_eq!(utc_date_time(SLOT), ("261111".to_string(), "2104".to_string()));
        assert_eq!(utc_date_time(0), ("700101".to_string(), "0000".to_string()));
        assert_eq!(utc_date_time(951_782_400_000), ("000229".to_string(), "0000".to_string()));
    }

    #[test]
    fn the_mode_field_follows_encode_mode() {
        assert_eq!(encode_mode("WSPR", 120), "2");
        assert_eq!(encode_mode("FST4W", 120), "3");
        assert_eq!(encode_mode("FST4W", 300), "5");
        assert_eq!(encode_mode("FST4W", 900), "15");
        assert_eq!(encode_mode("FST4W", 1800), "30");
        assert_eq!(encode_mode("FT8", 15), "");
    }

    #[test]
    fn the_three_wspr_message_types_are_read_like_decode_line() {
        assert_eq!(wspr_fields("K1ABC FN42 37"), Some(("K1ABC".into(), "FN42".into(), "37".into())));
        assert_eq!(wspr_fields("PJ4/K1ABC 37"), Some(("PJ4/K1ABC".into(), String::new(), "37".into())));
        assert_eq!(wspr_fields("<PJ4/K1ABC> FK52UD 37"), Some(("PJ4/K1ABC".into(), "FK52UD".into(), "37".into())));
        assert_eq!(wspr_fields("CQ K1ABC FN42"), None);
        assert_eq!(fst4w_fields("IU3ABC IO91 37"), Some(("IU3ABC".into(), "IO91".into(), "37".into())));
        assert_eq!(fst4w_fields("<IU3ABC> IO91WM"), Some(("IU3ABC".into(), "IO91WM".into(), String::new())));
        assert_eq!(fst4w_fields("PJ4/K1ABC 30"), Some(("PJ4/K1ABC".into(), String::new(), "30".into())));
        assert_eq!(fst4w_fields("<...> IO91WM"), None);
    }

    /// Uno spot WSPR ha i campi di `decodeLine` + `urlEncodeSpot`, nell'ordine di WSJT-X, con le frequenze in MHz a sei decimali.
    #[test]
    fn a_wspr_spot_carries_the_fields_of_wsjtx() {
        let r = requests(&cycle("WSPR", 120, vec![spot("K1ABC FN42 37", 1512.3, -1), spot("<...> FN42AA 30", 1490.0, 0)]), &station());
        assert_eq!(r.len(), 1, "il nominativo non risolto si scarta");
        assert_eq!(r[0].kind, Kind::Spot);
        assert_eq!(
            r[0].body,
            "function=wspr&date=261111&time=2104&sig=-21&dt=-0.3&drift=-1&tqrg=14.097112&tcall=K1ABC&tgrid=FN42&dbm=37\
             &version=Aurora%200.11.0&rcall=IU3ABC&rgrid=IO91wm&rqrg=14.097100&mode=2"
        );
    }

    #[test]
    fn an_fst4w_spot_carries_the_fields_of_queue_fst4w_decode() {
        let r = requests(&cycle("FST4W", 300, vec![spot("<PJ4/K1ABC> FK52UD", 1563.6, 0)]), &station());
        assert_eq!(
            r[0].body,
            "function=wspr&date=261111&time=2104&sig=-21&dt=-0.3&tqrg=14.097164&tcall=PJ4/K1ABC&drift=0&tgrid=FK52UD&dbm=\
             &version=Aurora%200.11.0&rcall=IU3ABC&rgrid=IO91wm&rqrg=14.097100&mode=5"
        );
    }

    /// Senza messaggi si manda lo stato (`urlEncodeNoSpot`); senza frequenza della radio niente.
    #[test]
    fn an_empty_period_sends_the_station_status() {
        let mut c = cycle("WSPR", 120, Vec::new());
        c.tx_df = 1450;
        let r = requests(&c, &station());
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].kind, Kind::Status);
        assert_eq!(r[0].body, "function=wsprstat&rcall=IU3ABC&rgrid=IO91wm&rqrg=14.097100&tpct=20&tqrg=14.097050&dbm=37&version=Aurora%200.11.0&mode=2");
        c.dial_hz = 0;
        assert!(requests(&c, &station()).is_empty());
    }

    #[test]
    fn the_queue_retries_with_the_delays_of_wsjtx_and_gives_up() {
        let now = Instant::now();
        let mut q = Queue::default();
        let spot = Request { kind: Kind::Spot, body: "a".into() };
        let status = Request { kind: Kind::Status, body: "s".into() };
        q.push(vec![spot.clone()], now);
        // lo stato si accoda solo con la coda vuota
        q.push(vec![status.clone()], now);
        assert_eq!(q.len(), 1);
        let p = q.take_due(now).expect("pronta");
        assert!(q.take_due(now).is_none());
        assert_eq!(q.retry(p, now, 0.5), Some(Duration::from_secs(15)));
        assert!(q.take_due(now + Duration::from_secs(14)).is_none());
        let mut p = q.take_due(now + Duration::from_secs(15)).expect("dopo 15 s");
        assert_eq!(retry_delay(2, 0.5), Duration::from_secs(60));
        assert_eq!(retry_delay(9, 0.5), Duration::from_secs(1200));
        assert_eq!(retry_delay(1, 0.0), Duration::from_millis(11_250));
        assert_eq!(retry_delay(1, 1.0), Duration::from_millis(18_750));
        // al settimo tentativo o oltre l'ora di vita si lascia perdere
        p.attempts = MAX_ATTEMPTS - 1;
        assert_eq!(q.retry(p, now, 0.5), None);
        q.push(vec![spot.clone()], now);
        let p = q.take_due(now).expect("pronta");
        assert_eq!(q.retry(p, now + TTL - Duration::from_secs(10), 0.5), None);
        // al piu' 1024 in coda: le piu' vecchie escono
        let dropped = q.push((0..MAX_PENDING + 5).map(|i| Request { kind: Kind::Spot, body: i.to_string() }).collect(), now);
        assert_eq!((q.len(), dropped), (MAX_PENDING, 5));
    }
}
