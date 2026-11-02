//! Ricerca dei nominativi su QRZ.com con la sua interfaccia XML (`https://xmldata.qrz.com/xml/current/`, documentazione «QRZ XML Logbook Data
//! Specification», versione 1.34). E' un servizio diverso dal logbook ([`crate::qrz`]): vuole utente e password di QRZ, non la chiave del logbook.
//!
//! - Accesso: `?username=..&password=..&agent=..` risponde un nodo `<Session>` con `<Key>` (la chiave della sessione), `<Count>`, `<SubExp>`
//!   (scadenza dell'abbonamento XML oppure `non-subscriber`) e a volte `<Message>`; con credenziali sbagliate `<Error>` e nessuna `<Key>`.
//! - Ricerca: `?s=KEY&callsign=..` risponde `<Callsign>` (campi `call`, `fname`, `name`, `addr2` = citta', `country`, ...) e di nuovo `<Session>`.
//!   Una risposta senza `<Key>` vuol dire che la sessione non vale piu' (`Session Timeout`, `Invalid session key`): si rifa' l'accesso una volta.
//!   Un nominativo che non c'e' da' `<Error>Not found: XX1XX</Error>`.
//! - Senza abbonamento XML QRZ da' solo dati ridotti (la documentazione non dice quali campi: si prende quello che arriva) e lo dice con
//!   `<Message>A subscription is required to obtain the complete data</Message>`.
//!
//!
//! Lo stesso servizio fa anche la ricerca su HamQTH (`https://www.hamqth.com/xml.php`, API v1, gratuita con utente e password di HamQTH):
//! accesso `?u=..&p=..` -> `<session><session_id>`; ricerca `?id=..&callsign=..&prg=..` -> `<search>` (`nick`, `adr_name`, `adr_city`, `qth`,
//! `country`); errori in `<session><error>`: «Wrong user name or password», «Session does not exist or expired» (la sessione dura un'ora),
//! «Callsign not found».
//!
//! Del nominativo cercato si tengono **solo** nome, cognome, citta' e paese: email, indirizzo, telefono, data di nascita e il resto non si
//! leggono nemmeno. I risultati (anche «non trovato») restano in memoria per [`CACHE_SECS`]; la password non entra mai nel registro.

use std::{
    collections::HashMap,
    sync::Mutex,
    time::Duration,
};

use serde::Serialize;

use crate::upload::{agent, short};

pub const URL: &str = "https://xmldata.qrz.com/xml/current/";
pub const HAMQTH_URL: &str = "https://www.hamqth.com/xml.php";
/// Il nome della password di QRZ.com nella protezione dei segreti.
pub const SECRET: &str = "qrz-xml";
/// La password di HamQTH.
pub const HAMQTH_SECRET: &str = "hamqth";

/// Quale servizio di ricerca.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Provider {
    Qrz,
    HamQth,
}

impl Provider {
    pub fn name(self) -> &'static str {
        match self {
            Provider::Qrz => "QRZ.com",
            Provider::HamQth => "HamQTH",
        }
    }
}
/// Quanto resta valido un risultato in memoria (tre giorni).
pub const CACHE_SECS: u64 = 3 * 24 * 3600;
/// Quanti nominativi al massimo in memoria.
pub const CACHE_MAX: usize = 5000;
pub const TIMEOUT: Duration = Duration::from_secs(8);

/// Quello che si mostra del nominativo cercato.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Person {
    pub call: String,
    /// Nome (`fname`).
    pub fname: String,
    /// Cognome (`name`).
    pub name: String,
    /// Citta' (`addr2`).
    pub city: String,
    pub country: String,
}

/// L'esito di una ricerca.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Found {
    Yes(Person),
    NotFound,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LookupError {
    /// Utente o password rifiutati (con il testo di QRZ).
    Login(String),
    /// Rete, tempo scaduto, risposta d'errore.
    Network(String),
    /// Risposta non leggibile o errore di QRZ.
    Service(String),
}

impl LookupError {
    pub fn message(&self) -> String {
        match self {
            LookupError::Login(m) => format!("accesso rifiutato ({m})"),
            LookupError::Network(m) => format!("servizio non raggiungibile ({m})"),
            LookupError::Service(m) => m.clone(),
        }
    }
}

#[derive(Default)]
struct State {
    /// Per quale utente valgono sessione e risultati.
    user: String,
    key: Option<String>,
    /// `Some(false)` = sessione senza abbonamento XML (dati ridotti).
    subscriber: Option<bool>,
    cache: HashMap<String, (u64, Found)>,
}

/// Il servizio di ricerca: una sessione per volta, risultati in memoria.
pub struct Lookup {
    provider: Provider,
    url: String,
    user_agent: String,
    timeout: Duration,
    state: Mutex<State>,
}

/// Il testo del primo `<tag>...</tag>` (con le entita' XML sciolte).
fn tag(body: &str, name: &str) -> Option<String> {
    let open = format!("<{name}>");
    let close = format!("</{name}>");
    let start = body.find(&open)? + open.len();
    let end = body[start..].find(&close)? + start;
    let text = body[start..end].replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&apos;", "'").replace("&amp;", "&");
    Some(text.trim().to_string())
}

/// Il blocco `<name>...</name>` intero (vuoto se manca).
fn block<'a>(body: &'a str, name: &str) -> &'a str {
    let open = format!("<{name}>");
    let close = format!("</{name}>");
    match (body.find(&open), body.find(&close)) {
        (Some(a), Some(b)) if b > a => &body[a..b],
        _ => "",
    }
}

/// La sessione di una risposta: chiave, abbonamento, errore.
#[derive(Debug, PartialEq, Eq)]
pub struct Session {
    pub key: Option<String>,
    pub subscriber: bool,
    pub expires: String,
    pub error: Option<String>,
}

pub fn read_session(body: &str) -> Option<Session> {
    if body.contains("<session>") {
        let s = block(body, "session");
        return Some(Session { key: tag(s, "session_id").filter(|k| !k.is_empty()), subscriber: true, expires: String::new(), error: tag(s, "error").filter(|e| !e.is_empty()) });
    }
    let s = block(body, "Session");
    if s.is_empty() {
        return None;
    }
    let expires = tag(s, "SubExp").unwrap_or_default();
    let message = tag(s, "Message").unwrap_or_default();
    let subscriber = !expires.is_empty() && !expires.eq_ignore_ascii_case("non-subscriber") && !message.to_lowercase().contains("subscription is required");
    Some(Session { key: tag(s, "Key").filter(|k| !k.is_empty()), subscriber, expires, error: tag(s, "Error").filter(|e| !e.is_empty()) })
}

/// Solo i campi ammessi del nodo `<Callsign>`.
pub fn read_person(body: &str) -> Option<Person> {
    let h = block(body, "search");
    if !h.is_empty() {
        // HamQTH: `nick` e' il nome con cui la stazione si fa chiamare, `adr_name` il nome completo (vuoto se nascosto)
        let get = |n: &str| short(&tag(h, n).unwrap_or_default(), 60);
        let full = get("adr_name");
        let city = if get("adr_city").is_empty() { get("qth") } else { get("adr_city") };
        let (fname, name) = if full.is_empty() { (get("nick"), String::new()) } else { (String::new(), full) };
        return Some(Person { call: get("callsign").to_uppercase(), fname, name, city, country: get("country") });
    }
    let c = block(body, "Callsign");
    if c.is_empty() {
        return None;
    }
    let get = |n: &str| short(&tag(c, n).unwrap_or_default(), 60);
    Some(Person { call: get("call").to_uppercase(), fname: get("fname"), name: get("name"), city: get("addr2"), country: get("country") })
}

impl Lookup {
    pub fn new(provider: Provider, url: &str, user_agent: &str, timeout: Duration) -> Lookup {
        Lookup { provider, url: url.to_string(), user_agent: user_agent.to_string(), timeout, state: Mutex::default() }
    }

    fn state(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// Le credenziali sono cambiate (o il servizio e' stato spento): sessione e risultati si dimenticano.
    pub fn reset(&self) {
        *self.state() = State::default();
    }

    /// La sessione attuale e' senza abbonamento XML (`None` = nessuna sessione ancora).
    pub fn subscriber(&self) -> Option<bool> {
        self.state().subscriber
    }

    fn get(&self, query: &[(&str, &str)]) -> Result<String, LookupError> {
        let mut req = agent(self.timeout, &self.user_agent).get(&self.url);
        for (k, v) in query {
            req = req.query(*k, *v);
        }
        let mut response = req.call().map_err(|e| LookupError::Network(crate::eqsl::describe(&e)))?;
        let code = response.status().as_u16();
        let text = response.body_mut().read_to_string().map_err(|e| LookupError::Network(e.to_string()))?;
        if code != 200 {
            return Err(LookupError::Network(format!("risposta {code}")));
        }
        Ok(text)
    }

    /// Apre una sessione nuova (bloccante).
    fn login(&self, user: &str, password: &str) -> Result<Session, LookupError> {
        let agent_name = self.user_agent.clone();
        let body = match self.provider {
            Provider::Qrz => self.get(&[("username", user), ("password", password), ("agent", &agent_name)])?,
            Provider::HamQth => self.get(&[("u", user), ("p", password)])?,
        };
        let session = read_session(&body).ok_or_else(|| LookupError::Service("risposta non leggibile".into()))?;
        if session.key.is_none() {
            return Err(LookupError::Login(short(session.error.as_deref().unwrap_or("nessuna sessione"), 120)));
        }
        let mut st = self.state();
        if st.user != user {
            st.cache.clear();
        }
        st.user = user.to_string();
        st.key.clone_from(&session.key);
        st.subscriber = Some(session.subscriber);
        Ok(session)
    }

    /// Prova le credenziali con un accesso nuovo; dice se c'e' l'abbonamento XML.
    pub fn test(&self, user: &str, password: &str) -> Result<String, String> {
        self.reset();
        let s = self.login(user, password).map_err(|e| e.message())?;
        Ok(if self.provider == Provider::HamQth {
            "accesso a HamQTH riuscito".to_string()
        } else if s.subscriber {
            format!("accesso riuscito, abbonamento XML fino a {}", short(&s.expires, 40))
        } else {
            "accesso riuscito, ma senza abbonamento XML: QRZ da' solo dati ridotti".to_string()
        })
    }

    /// Cerca un nominativo (bloccante): prima in memoria, poi su QRZ; la sessione scaduta si rinnova una volta.
    pub fn lookup(&self, user: &str, password: &str, call: &str, now_secs: u64) -> Result<Found, LookupError> {
        let call = call.trim().to_uppercase();
        {
            let mut st = self.state();
            if st.user != user {
                *st = State::default();
            }
            if let Some((at, found)) = st.cache.get(&call)
                && now_secs.saturating_sub(*at) < CACHE_SECS
            {
                return Ok(found.clone());
            }
        }
        for attempt in 0..2 {
            // la chiave si legge e il lucchetto si lascia subito: dentro il `match` resterebbe chiuso e `login` lo aspetterebbe per sempre
            let saved = self.state().key.clone();
            let key = match saved {
                Some(k) => k,
                None => self.login(user, password)?.key.unwrap_or_default(),
            };
            let program: String = self.user_agent.chars().filter(|c| !c.is_whitespace()).collect();
            let body = match self.provider {
                Provider::Qrz => self.get(&[("s", &key), ("callsign", &call)])?,
                Provider::HamQth => self.get(&[("id", &key), ("callsign", &call), ("prg", &program)])?,
            };
            let session = read_session(&body);
            if let Some(person) = read_person(&body) {
                let found = Found::Yes(person);
                self.remember(&call, now_secs, &found, session.as_ref());
                return Ok(found);
            }
            let error = session.as_ref().and_then(|s| s.error.clone()).unwrap_or_default();
            let lower = error.to_lowercase();
            if lower.starts_with("not found") || lower.contains("callsign not found") {
                self.remember(&call, now_secs, &Found::NotFound, session.as_ref());
                return Ok(Found::NotFound);
            }
            let expired = session.as_ref().is_some_and(|s| s.key.is_none()) || lower.contains("expired") || lower.contains("does not exist") || lower.contains("timeout") || lower.contains("invalid session");
            if expired && attempt == 0 {
                // sessione scaduta o non valida: si rifa' l'accesso e si riprova una volta
                self.state().key = None;
                continue;
            }
            return Err(LookupError::Service(if error.is_empty() { "risposta non leggibile".into() } else { short(&error, 120) }));
        }
        Err(LookupError::Service("sessione non rinnovata".into()))
    }

    fn remember(&self, call: &str, now_secs: u64, found: &Found, session: Option<&Session>) {
        let mut st = self.state();
        if let Some(s) = session
            && s.key.is_some()
        {
            st.key.clone_from(&s.key);
            st.subscriber = Some(s.subscriber);
        }
        if st.cache.len() >= CACHE_MAX {
            st.cache.retain(|_, (at, _)| now_secs.saturating_sub(*at) < CACHE_SECS);
            if st.cache.len() >= CACHE_MAX {
                st.cache.clear();
            }
        }
        st.cache.insert(call.to_string(), (now_secs, found.clone()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{BufRead, BufReader, Write},
        net::TcpListener,
        sync::{Arc, atomic::{AtomicUsize, Ordering}},
    };

    const LOGIN_OK: &str = "<?xml version=\"1.0\" ?><QRZDatabase version=\"1.34\"><Session><Key>K1</Key><Count>1</Count><SubExp>Wed Jan 1 12:34:03 2031</SubExp></Session></QRZDatabase>";

    /// Un finto QRZ: risponde secondo la richiesta e conta gli accessi e le ricerche.
    fn fake(expire_first: bool) -> (String, Arc<AtomicUsize>, Arc<AtomicUsize>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/xml/current/", listener.local_addr().unwrap());
        let logins = Arc::new(AtomicUsize::new(0));
        let searches = Arc::new(AtomicUsize::new(0));
        let (l, s) = (logins.clone(), searches.clone());
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let mut line = String::new();
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                reader.read_line(&mut line).unwrap();
                loop {
                    let mut h = String::new();
                    if reader.read_line(&mut h).unwrap_or(0) == 0 || h == "\r\n" {
                        break;
                    }
                }
                let body = if line.contains("username=") {
                    let n = l.fetch_add(1, Ordering::SeqCst);
                    if line.contains("password=giusta") {
                        LOGIN_OK.replace("K1", &format!("K{}", n + 1))
                    } else {
                        "<QRZDatabase><Session><Error>Username/password incorrect</Error></Session></QRZDatabase>".to_string()
                    }
                } else {
                    let n = s.fetch_add(1, Ordering::SeqCst);
                    if expire_first && n == 0 {
                        "<QRZDatabase><Session><Error>Session Timeout</Error></Session></QRZDatabase>".to_string()
                    } else if line.contains("callsign=IU3ABC") {
                        "<QRZDatabase><Callsign><call>IU3ABC</call><fname>Mario</fname><name>Rossi</name><addr1>Via Segreta 1</addr1><addr2>Padova</addr2><country>Italy</country><email>nascosta@example.com</email></Callsign><Session><Key>K9</Key><SubExp>non-subscriber</SubExp></Session></QRZDatabase>".to_string()
                    } else {
                        "<QRZDatabase><Session><Key>K9</Key><Error>Not found: XX1XX</Error></Session></QRZDatabase>".to_string()
                    }
                };
                let _ = write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: text/xml\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
            }
        });
        (url, logins, searches)
    }

    #[test]
    fn session_and_person_are_read_without_private_fields() {
        let s = read_session(LOGIN_OK).unwrap();
        assert_eq!(s.key.as_deref(), Some("K1"));
        assert!(s.subscriber);
        let reduced = read_session("<Session><Key>A</Key><SubExp>non-subscriber</SubExp><Message>A subscription is required to obtain the complete data</Message></Session>").unwrap();
        assert!(!reduced.subscriber);
        let p = read_person("<Callsign><call>iu3abc</call><fname>A &amp; B</fname><email>x@y</email></Callsign>").unwrap();
        assert_eq!(p, Person { call: "IU3ABC".into(), fname: "A & B".into(), ..Person::default() });
    }

    #[test]
    fn login_search_cache_and_not_found() {
        let (url, logins, searches) = fake(false);
        let q = Lookup::new(Provider::Qrz, &url, "Aurora/test", Duration::from_secs(5));
        let Found::Yes(p) = q.lookup("IU3ABC", "giusta", "iu3abc", 100).unwrap() else { panic!("trovato") };
        assert_eq!((p.fname.as_str(), p.name.as_str(), p.city.as_str(), p.country.as_str()), ("Mario", "Rossi", "Padova", "Italy"));
        assert_eq!(q.subscriber(), Some(false), "senza abbonamento: dati ridotti");
        // dalla memoria: nessuna nuova richiesta
        assert!(matches!(q.lookup("IU3ABC", "giusta", "IU3ABC", 200).unwrap(), Found::Yes(_)));
        assert_eq!(searches.load(Ordering::SeqCst), 1);
        assert_eq!(q.lookup("IU3ABC", "giusta", "XX1XX", 200).unwrap(), Found::NotFound);
        assert_eq!(q.lookup("IU3ABC", "giusta", "XX1XX", 300).unwrap(), Found::NotFound);
        assert_eq!(searches.load(Ordering::SeqCst), 2);
        // scaduta la memoria si richiede
        q.lookup("IU3ABC", "giusta", "IU3ABC", 100 + CACHE_SECS).unwrap();
        assert_eq!(searches.load(Ordering::SeqCst), 3);
        assert_eq!(logins.load(Ordering::SeqCst), 1, "una sola sessione");
    }

    #[test]
    fn an_expired_session_is_renewed_once() {
        let (url, logins, searches) = fake(true);
        let q = Lookup::new(Provider::Qrz, &url, "Aurora/test", Duration::from_secs(5));
        assert!(matches!(q.lookup("IU3ABC", "giusta", "IU3ABC", 1).unwrap(), Found::Yes(_)));
        assert_eq!(logins.load(Ordering::SeqCst), 2);
        assert_eq!(searches.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn hamqth_answers_are_read() {
        let login = read_session("<HamQTH><session><session_id>abc</session_id></session></HamQTH>").unwrap();
        assert_eq!(login.key.as_deref(), Some("abc"));
        let expired = read_session("<HamQTH><session><error>Session does not exist or expired</error></session></HamQTH>").unwrap();
        assert!(expired.key.is_none());
        let p = read_person("<HamQTH><search><callsign>iu3abc</callsign><nick>Mario</nick><qth>Padova</qth><country>Italy</country><adr_name></adr_name><email>x@y</email></search></HamQTH>").unwrap();
        assert_eq!(p, Person { call: "IU3ABC".into(), fname: "Mario".into(), name: String::new(), city: "Padova".into(), country: "Italy".into() });
    }

    #[test]
    fn hamqth_session_search_and_renewal() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/xml.php", listener.local_addr().unwrap());
        let logins = Arc::new(AtomicUsize::new(0));
        let l = logins.clone();
        std::thread::spawn(move || {
            let mut searches = 0;
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                loop {
                    let mut h = String::new();
                    if reader.read_line(&mut h).unwrap_or(0) == 0 || h == "\r\n" {
                        break;
                    }
                }
                let body = if line.contains("?u=") {
                    l.fetch_add(1, Ordering::SeqCst);
                    if line.contains("p=giusta") { "<HamQTH><session><session_id>S1</session_id></session></HamQTH>" } else { "<HamQTH><session><error>Wrong user name or password</error></session></HamQTH>" }
                } else {
                    searches += 1;
                    if searches == 1 {
                        "<HamQTH><session><error>Session does not exist or expired</error></session></HamQTH>"
                    } else if line.contains("callsign=IU3ABC") && line.contains("prg=") {
                        "<HamQTH><search><callsign>IU3ABC</callsign><nick>Mario</nick><adr_city>Padova</adr_city><country>Italy</country></search></HamQTH>"
                    } else {
                        "<HamQTH><session><error>Callsign not found</error></session></HamQTH>"
                    }
                };
                let _ = write!(stream, "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
            }
        });
        let q = Lookup::new(Provider::HamQth, &url, "Aurora 1.0", Duration::from_secs(5));
        let Found::Yes(p) = q.lookup("IU3ABC", "giusta", "IU3ABC", 1).unwrap() else { panic!("trovato") };
        assert_eq!((p.fname.as_str(), p.city.as_str()), ("Mario", "Padova"));
        assert_eq!(logins.load(Ordering::SeqCst), 2, "sessione scaduta rinnovata");
        assert_eq!(q.lookup("IU3ABC", "giusta", "XX9XX", 1).unwrap(), Found::NotFound);
        assert!(matches!(Lookup::new(Provider::HamQth, &url, "A", Duration::from_secs(5)).lookup("IU3ABC", "no", "IU3ABC", 1), Err(LookupError::Login(_))));
    }

    #[test]
    fn wrong_credentials_are_reported() {
        let (url, _, searches) = fake(false);
        let q = Lookup::new(Provider::Qrz, &url, "Aurora/test", Duration::from_secs(5));
        let e = q.lookup("IU3ABC", "sbagliata", "IU3ABC", 1).unwrap_err();
        assert!(matches!(&e, LookupError::Login(m) if m.contains("incorrect")));
        assert!(!e.message().contains("sbagliata"), "la password non compare");
        assert_eq!(searches.load(Ordering::SeqCst), 0);
        assert!(q.test("IU3ABC", "sbagliata").is_err());
        assert!(q.test("IU3ABC", "giusta").unwrap().contains("abbonamento XML fino"));
    }
}
