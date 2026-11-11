//! Emettitore UDP del motore proprio.
//!
//! Con JTDX, a parlare il protocollo UDP di WSJT-X verso GridTracker (e, tramite lui, verso Log4OM2) era JTDX: il suo `MessageClient`
//! manda Heartbeat ogni 15 s, Status a ogni cambiamento, un Decode per messaggio, Clear, QSO Logged e Logged ADIF. Con il motore proprio
//! JTDX non c'e' piu': questo crate ascolta il bus di Aurora (`Event::Status`, `Event::Decode`, `Event::QsoLogged`) e manda agli stessi
//! destinatari gli **stessi datagrammi** (formato e ordine dei campi letti in `jtdx.exe`, vedi `docs/analisi-jtdx-reverse-rea.md`),
//! con lo stesso identificativo (`JTDX`) e la stessa regola contro i doppioni (un datagramma identico al precedente non si rimanda).
//! Sulla stessa porta arrivano anche i comandi di ritorno (Reply, HaltTx, FreeText, ...), girati ad Aurora come [`Command`].

use std::{
    collections::VecDeque,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    sync::{Arc, Mutex},
    time::Duration,
};

use aurora_core::{Command, DecodedMessage, Event, QsoRecord, SharedUdpFilters, StationStatus};
use logbook::adif;
use tokio::{
    net::UdpSocket,
    sync::{broadcast, broadcast::error::RecvError, mpsc},
    task::JoinHandle,
};
use udp_bridge::{
    encode_inhibit_status, parse_tx_inhibit, InhibitParse, InhibitStatus,
    decode_request, encode_clear, encode_close, encode_decode, encode_heartbeat, encode_logged_adif, encode_qso_logged, encode_status, message::LOGGED_ADIF, Decode, QsoLoggedData, Request, Status, HEARTBEAT_SECS, INITIAL_SCHEMA, MAX_SCHEMA,
};

pub use udp_bridge::Configure;

/// Quanti messaggi decodificati si ricordano: servono al Reply (che li indica per testo) e al Replay.
pub const RECENT_DECODES: usize = 300;

/// Identificativo di istanza di JTDX: `QApplication::applicationName()`, impostato in `main()` a «JTDX» (con `--rig-name` diventa «JTDX - nome»).
pub const JTDX_ID: &str = "JTDX";
/// Versione che JTDX dichiara nel Heartbeat.
pub const JTDX_VERSION: &str = "2.2.159";

/// TTL di base dei datagrammi multicast: 1 = solo la rete locale (come WSJT-X).
pub const DEFAULT_MULTICAST_TTL: u32 = 1;

/// Le opzioni di un destinatario multicast: quanti salti (TTL) e da quale interfaccia escono i datagrammi (`None` = scelta del sistema).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Multicast {
    pub ttl: u32,
    pub interface: Option<IpAddr>,
}

impl Default for Multicast {
    fn default() -> Self {
        Multicast { ttl: DEFAULT_MULTICAST_TTL, interface: None }
    }
}

/// Un destinatario del flusso con le sue opzioni.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Destination {
    pub addr: SocketAddr,
    /// Riceve il flusso binario di WSJT-X (Heartbeat, Status, Decode, Clear, QSO Logged, Close).
    pub stream: bool,
    /// Riceve il Logged ADIF (tipo 12) dopo ogni QSO Logged.
    pub logged_adif: bool,
    /// `Some` per un gruppo multicast (l'indirizzo deve esserlo): TTL e interfaccia di uscita.
    pub multicast: Option<Multicast>,
}

impl Destination {
    /// Il flusso completo verso un indirizzo; se l'indirizzo e' un gruppo multicast, con TTL 1 e interfaccia scelta dal sistema.
    pub fn stream(addr: SocketAddr, logged_adif: bool) -> Destination {
        Destination { addr, stream: true, logged_adif, multicast: addr.ip().is_multicast().then(Multicast::default) }
    }

    /// Vero se il datagramma (gia' codificato) va a questo destinatario: il Logged ADIF solo a chi lo vuole, il resto a chi vuole il flusso.
    pub fn wants(&self, datagram: &[u8]) -> bool {
        if is_logged_adif(datagram) { self.logged_adif } else { self.stream }
    }
}

/// Il tipo del messaggio sta nei byte 8-11 (dopo numero magico e schema).
fn is_logged_adif(datagram: &[u8]) -> bool {
    datagram.get(8..12).and_then(|b| <[u8; 4]>::try_from(b).ok()).is_some_and(|b| u32::from_be_bytes(b) == LOGGED_ADIF)
}

#[derive(Clone, Debug)]
pub struct EmitterConfig {
    pub id: String,
    pub version: String,
    /// I destinatari del flusso (il «primo server UDP» di JTDX e chi l'hub inoltrava: GridTracker2 su 127.0.0.1:2333). Ricevono il flusso e,
    /// con `send_adif`, il Logged ADIF; un indirizzo multicast si tratta come tale (TTL 1).
    pub targets: Vec<SocketAddr>,
    /// Altri destinatari, ciascuno con le sue opzioni (flusso, Logged ADIF, multicast).
    pub destinations: Vec<Destination>,
    /// Nominativo e locatore propri, per il QSO Logged e per il record ADIF (lo Status porta i suoi).
    pub de_call: String,
    pub de_grid: String,
    /// Potenza in watt scritta nel QSO (vuota: nessuna, come JTDX con «Save Tx power» spento).
    pub tx_power: String,
    /// `AcceptUDPRequests`: senza, i comandi di ritorno (tranne Replay) si ignorano, come in JTDX.
    pub accept_requests: bool,
    /// `AcceptUDPReplyMessages`: 1 solo CQ, 2 CQ e 73, 3 tutti.
    pub accept_reply: u8,
    /// `EnableUDP1adifSending`: dopo ogni QSO Logged manda anche il Logged ADIF (ai `targets`; i `destinations` hanno la loro opzione).
    pub send_adif: bool,
    /// L'invio a eQSL e' acceso: il record ADIF porta `EQSL_QSL_SENT` come in JTDX.
    pub eqsl_sent: bool,
    /// `ApplyFiltersToUDPmessages`: solo i messaggi che i filtri di testo lasciano vedere escono via UDP.
    pub apply_filters: bool,
    pub heartbeat: Duration,
}

impl EmitterConfig {
    /// Come JTDX 2.2.159 con la sua configurazione di fabbrica: nessun comando accettato, niente Logged ADIF.
    pub fn jtdx(targets: Vec<SocketAddr>) -> EmitterConfig {
        EmitterConfig {
            id: JTDX_ID.to_string(),
            version: JTDX_VERSION.to_string(),
            targets,
            destinations: Vec::new(),
            de_call: String::new(),
            de_grid: String::new(),
            tx_power: String::new(),
            accept_requests: false,
            accept_reply: 1,
            send_adif: false,
            eqsl_sent: false,
            apply_filters: false,
            heartbeat: Duration::from_secs(HEARTBEAT_SECS),
        }
    }

    /// Tutti i destinatari: prima `targets` (con `send_adif`), poi `destinations`; lo stesso indirizzo una volta sola (vince il primo).
    pub fn all_destinations(&self) -> Vec<Destination> {
        let mut out: Vec<Destination> = Vec::new();
        for d in self.targets.iter().map(|a| Destination::stream(*a, self.send_adif)).chain(self.destinations.iter().cloned()) {
            if !(d.stream || d.logged_adif) || out.iter().any(|o| o.addr == d.addr) {
                continue;
            }
            out.push(d);
        }
        out
    }
}

/// Un comando in ingresso che non e' un comando di trasmissione: chi costruisce l'emettitore decide che cosa farne (`EmitterHooks::on_external`).
/// Arrivano solo con «accetta le richieste UDP» acceso.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum External {
    /// Clear (3): svuota le liste come «Cancella». `window` 0 attivita' di banda, 1 frequenza Rx, 2 entrambe.
    Clear { window: u8 },
    /// Location (11): il locatore della stazione per la sessione, gia' controllato e scritto come `FN42ab`.
    Location { grid: String },
    /// Highlight Callsign (13): colori RGB di sfondo e testo; entrambi `None` = togli.
    Highlight { call: String, background: Option<[u8; 3]>, foreground: Option<[u8; 3]>, last_only: bool },
    /// Switch Configuration (14): Aurora non ha configurazioni con nome; chi la riceve la registra e la ignora.
    SwitchConfiguration { name: String },
    /// Configure (15): solo i campi presenti.
    Configure(Configure),
    /// AnnotationInfo (16): valore di ordinamento di un Hound per la Fox; `None` lo toglie.
    Annotation { call: String, order: Option<u32> },
}

/// Il locatore di un Location come lo legge WSJT-X (`MainWindow::locationChange`): oltre sei caratteri serve il prefisso `GRID:`; poi 4, 6 o 8
/// caratteri Maidenhead. Restituisce il locatore con le lettere del campo maiuscole e quelle del sottoquadrato minuscole.
pub fn parse_location(text: &str) -> Option<String> {
    let mut grid = text.trim().to_string();
    if grid.chars().count() > 6 {
        let upper = grid.to_ascii_uppercase();
        let rest = upper.strip_prefix("GRID:")?;
        grid = rest.trim().to_string();
    }
    let b = grid.as_bytes();
    let pair = |i: usize, lo: u8, hi: u8| b.get(i).zip(b.get(i + 1)).is_some_and(|(x, y)| (lo..=hi).contains(&x.to_ascii_uppercase()) && (lo..=hi).contains(&y.to_ascii_uppercase()));
    let ok = matches!(b.len(), 4 | 6 | 8) && pair(0, b'A', b'R') && pair(2, b'0', b'9') && (b.len() < 6 || pair(4, b'A', b'X')) && (b.len() < 8 || pair(6, b'0', b'9'));
    if !ok {
        return None;
    }
    Some(grid.chars().enumerate().map(|(i, c)| if i < 2 { c.to_ascii_uppercase() } else { c.to_ascii_lowercase() }).collect())
}

#[derive(Clone)]
struct Recent {
    id: u64,
    time_ms: u32,
    freq_hz: u32,
    snr: i32,
    delta_time: f32,
    text: String,
    hint: bool,
}

/// Chi applica i blocchi della trasmissione chiesti con TxInhibit (tipo 18): la catena di trasmissione di Aurora (`tx_native::TxInterlock`).
pub trait InhibitControl: Send + Sync {
    /// Un comando valido: `ttl_ms` 0 rilascia, 100-30000 prenota o rinnova.
    fn command(&self, controller: &str, ttl_ms: u32, station: &str);
    /// Comandi malformati ricevuti.
    fn invalid(&self, count: u32);
    /// Fa scadere le prenotazioni.
    fn tick(&self);
    /// Lo stato attuale (i campi `id` e `supported` li mette l'emettitore).
    fn snapshot(&self) -> InhibitStatus;
}

/// Ogni quanto si fanno scadere le prenotazioni e si controlla se lo stato del blocco e' cambiato (la durata minima e' 100 ms).
pub const INHIBIT_TICK: Duration = Duration::from_millis(50);

/// Il cuore dell'emettitore, senza rete: trasforma eventi in datagrammi e datagrammi in comandi.
pub struct Session {
    config: EmitterConfig,
    /// Schema in uso: parte da 2 e sale a quello piu' alto che il server dichiara (`MessageClient::impl::parse_message`).
    schema: u32,
    /// L'ultimo datagramma mandato (di qualunque tipo): uno identico non si rimanda (`MessageClient::impl::send_message`).
    last: Option<Vec<u8>>,
    /// Il modo corrente dell'ultimo Status («FT8», «FT4»): decide il carattere del modo nei Decode.
    mode: String,
    last_status: Option<StationStatus>,
    recent: VecDeque<Recent>,
    last_qso: Option<(String, i64)>,
    /// I filtri della scheda Filtri (copia mandata dalla pagina) e il continente di chi opera: servono con `apply_filters`.
    filters: Option<SharedUdpFilters>,
    own_continent: Option<String>,
    /// Il blocco della trasmissione (TxInhibit): c'e' solo se la catena di trasmissione lo sa applicare.
    inhibit: Option<Arc<dyn InhibitControl>>,
    /// L'ultimo InhibitStatus mandato (per mandarlo solo ai cambi, oltre che dopo ogni Heartbeat).
    last_inhibit: Option<InhibitStatus>,
}

/// Cosa fare dopo un datagramma ricevuto.
#[derive(Debug, Default)]
pub struct Handled {
    pub commands: Vec<Command>,
    /// Una richiesta `Reply` e' stata accettata: chi vuole puo' portare la finestra in primo piano (`udpWindowToFront`, `udpWindowRestore`).
    pub reply_accepted: bool,
    /// Datagrammi da rimandare ai destinatari (il Replay rimanda i decode correnti).
    pub datagrams: Vec<Vec<u8>>,
    /// Comandi che non riguardano la trasmissione (colori, Clear, Location, Configure...).
    pub external: Vec<External>,
}

/// Il carattere che JTDX scrive dopo la frequenza e manda nel campo `mode` del Decode: `~` per FT8, `:` per FT4 (letto in `ALL.TXT` e in `replyToUDP`).
pub fn mode_char(mode: &str) -> &'static str {
    if mode.eq_ignore_ascii_case("FT4") { ":" } else { "~" }
}

/// `replyToUDP`: quali messaggi si accettano come Reply secondo `AcceptUDPReplyMessages` (`m_acceptUDP`).
pub fn reply_accepted(accept_reply: u8, text: &str) -> bool {
    let cq = text.starts_with("CQ ") || text.starts_with("CQDX ") || text.starts_with("QRZ");
    let signoff = |needle: &str| text.match_indices(needle).any(|(i, _)| i >= 1);
    match accept_reply {
        1 => cq,
        2 => cq || signoff(" 73") || signoff(" RR73"),
        3 => true,
        _ => false,
    }
}

impl Session {
    pub fn new(config: EmitterConfig) -> Session {
        Session { config, schema: INITIAL_SCHEMA, last: None, mode: "FT8".to_string(), last_status: None, recent: VecDeque::new(), last_qso: None, filters: None, own_continent: None, inhibit: None, last_inhibit: None }
    }

    /// Chi applica i TxInhibit (`None`: i comandi si ignorano e l'InhibitStatus non si manda).
    pub fn set_inhibit(&mut self, inhibit: Option<Arc<dyn InhibitControl>>) {
        self.inhibit = inhibit;
    }

    /// WSJT-X: «supportato» vuol dire che l'istanza applica i blocchi (PTT comandabile) e che «accetta le richieste UDP» e' acceso.
    pub fn inhibit_supported(&self) -> bool {
        self.config.accept_requests && self.inhibit.is_some()
    }

    fn inhibit_snapshot(&self, supported: bool) -> Option<InhibitStatus> {
        let control = self.inhibit.as_ref()?;
        Some(InhibitStatus { id: self.config.id.clone(), supported, ..control.snapshot() })
    }

    /// InhibitStatus (tipo 17): con `force` sempre (dopo ogni Heartbeat), altrimenti solo se lo stato e' cambiato dall'ultimo mandato. Niente se il
    /// blocco non e' supportato. Non passa dal controllo dei doppioni del flusso.
    pub fn inhibit_status(&mut self, force: bool) -> Option<Vec<u8>> {
        if !self.inhibit_supported() {
            return None;
        }
        let snapshot = self.inhibit_snapshot(true)?;
        if !force && self.last_inhibit.as_ref() == Some(&snapshot) {
            return None;
        }
        let datagram = encode_inhibit_status(self.schema, &snapshot);
        self.last_inhibit = Some(snapshot);
        Some(datagram)
    }

    /// Fa scadere le prenotazioni e restituisce l'InhibitStatus se lo stato e' cambiato.
    pub fn inhibit_tick(&mut self) -> Option<Vec<u8>> {
        if let Some(control) = &self.inhibit {
            control.tick();
        }
        self.inhibit_status(false)
    }

    /// Alla chiusura: l'ultimo InhibitStatus «non supportato» ritira la disponibilita' ai controller.
    pub fn inhibit_withdrawal(&self) -> Option<Vec<u8>> {
        if !self.inhibit_supported() {
            return None;
        }
        self.inhibit_snapshot(false).map(|s| encode_inhibit_status(self.schema, &s))
    }

    /// I filtri di testo e il continente proprio, per `apply_filters`.
    pub fn set_filters(&mut self, filters: Option<SharedUdpFilters>, own_continent: Option<String>) {
        self.filters = filters;
        self.own_continent = own_continent;
    }

    /// Vero se il messaggio puo' uscire via UDP: con `apply_filters` solo se i filtri di testo lo lasciano vedere (`readFromStdout`: `!filterUDP || notified & 2`).
    fn passes_filters(&self, m: &DecodedMessage) -> bool {
        if !self.config.apply_filters {
            return true;
        }
        let Some(filters) = &self.filters else { return true };
        filters.read().map_or(true, |f| f.allows(m, self.own_continent.as_deref()))
    }

    pub fn schema(&self) -> u32 {
        self.schema
    }

    /// Il Heartbeat (ogni 15 s e al primo contatto): non passa dal controllo dei doppioni.
    pub fn heartbeat(&self) -> Vec<u8> {
        encode_heartbeat(&self.config.id, self.schema, MAX_SCHEMA, &self.config.version)
    }

    /// Alla chiusura.
    pub fn close(&self) -> Vec<u8> {
        encode_close(&self.config.id, self.schema)
    }

    /// Cio' che JTDX manda appena sa a chi scrivere: il Heartbeat e il Clear del costruttore (`on_EraseButton_clicked`).
    pub fn start(&mut self) -> Vec<Vec<u8>> {
        let mut out = vec![self.heartbeat()];
        out.extend(self.send(encode_clear(&self.config.id, self.schema)));
        out
    }

    /// `send_message`: un datagramma uguale all'ultimo mandato non si rimanda.
    fn send(&mut self, datagram: Vec<u8>) -> Option<Vec<u8>> {
        if self.last.as_deref() == Some(datagram.as_slice()) {
            return None;
        }
        self.last = Some(datagram.clone());
        Some(datagram)
    }

    fn status_datagram(&self, s: &StationStatus, tx_first: bool) -> Vec<u8> {
        let text = |v: &Option<String>| Some(v.clone().unwrap_or_default());
        let non_empty = |own: &str, config: &str| Some(if own.is_empty() { config.to_string() } else { own.to_string() });
        let mode = if s.mode.is_empty() { self.mode.clone() } else { s.mode.clone() };
        let wire = Status {
            id: Some(self.config.id.clone()),
            dial_freq: s.dial_hz,
            mode: Some(mode.clone()),
            dx_call: text(&s.dx_call),
            report: text(&s.report),
            tx_mode: Some(mode),
            tx_enabled: s.tx_enabled,
            transmitting: s.transmitting,
            decoding: s.decoding,
            rx_df: s.rx_df.unwrap_or(0),
            tx_df: s.tx_df.unwrap_or(0),
            de_call: non_empty(&s.de_call, &self.config.de_call),
            de_grid: non_empty(&s.de_grid, &self.config.de_grid),
            dx_grid: text(&s.dx_grid),
            tx_watchdog: false,
            // `QChar submode {0}` -> stringa nulla
            submode: None,
            fast_mode: false,
            // JTDX: dopo `fast_mode` un solo byte, `tx_first`; niente `freq_tolerance`, `tr_period`, `config_name`, `tx_message`
            special_op_mode: Some(u8::from(tx_first)),
            freq_tolerance: None,
            tr_period: None,
            config_name: None,
            tx_message: None,
        };
        encode_status(self.schema, &wire)
    }

    fn decode_datagram(&self, m: &Recent, is_new: bool) -> Vec<u8> {
        encode_decode(
            self.schema,
            &Decode {
                id: Some(self.config.id.clone()),
                is_new,
                time_ms: m.time_ms,
                snr: m.snr,
                delta_time: f64::from(m.delta_time),
                delta_freq: m.freq_hz,
                mode: Some(mode_char(&self.mode).to_string()),
                message: Some(m.text.trim().to_string()),
                // JTDX: `postDecode` passa il marcatore `*` o `^` della riga come bit di bassa fiducia
                low_confidence: m.hint,
                off_air: false,
            },
        )
    }

    /// Trasforma un evento del bus nei datagrammi da mandare (zero, uno o due).
    pub fn on_event(&mut self, event: &Event, tx_first: bool, now_unix_ms: u64) -> Vec<Vec<u8>> {
        match event {
            Event::Status(s) => {
                if !s.mode.is_empty() {
                    self.mode.clone_from(&s.mode);
                }
                self.last_status = Some(s.clone());
                let datagram = self.status_datagram(s, tx_first);
                self.send(datagram).into_iter().collect()
            }
            Event::Decode(m) => {
                // la riga di verifica di un SuperFox non va ai programmi collegati (`postDecode` di WSJT-X: «Don't send SuperFox OTP messages»)
                if m.text.contains("$VERIFY$") {
                    return Vec::new();
                }
                // il Reply e il Replay di GridTracker usano solo cio' che e' uscito: una riga nascosta dai filtri non si ricorda nemmeno
                if !self.passes_filters(m) {
                    return Vec::new();
                }
                let recent = self.remember(m);
                let datagram = self.decode_datagram(&recent, true);
                self.push_recent(recent);
                self.send(datagram).into_iter().collect()
            }
            Event::QsoLogged(q) => self.on_qso(q, now_unix_ms),
            Event::QsoCompleted(_)
            | Event::QsoPending(_)
            | Event::QsoPendingDone { .. }
            | Event::TxHalted
            | Event::Wanted(_)
            | Event::UpdateAvailable(_)
            | Event::AudioLevel(_)
            | Event::BandChanged { .. }
            | Event::HighlightCallsign { .. }
            | Event::ClearDecodes { .. }
            | Event::Fox(_)
            | Event::Notice { .. }
            | Event::Sstv(_)
            | Event::Digi(_) => Vec::new(),
        }
    }

    fn remember(&self, m: &DecodedMessage) -> Recent {
        Recent { id: m.id, time_ms: m.time_ms, freq_hz: m.freq_hz, snr: m.snr, delta_time: m.delta_time, text: m.text.clone(), hint: m.hint }
    }

    fn push_recent(&mut self, recent: Recent) {
        if self.recent.len() >= RECENT_DECODES {
            self.recent.pop_front();
        }
        self.recent.push_back(recent);
    }

    fn on_qso(&mut self, q: &QsoRecord, now_unix_ms: u64) -> Vec<Vec<u8>> {
        // stessa guardia dell'esportatore: lo stesso nominativo entro due periodi non e' un altro QSO
        let stamp = i64::try_from(if q.time_off_unix_ms > 0 { q.time_off_unix_ms } else { now_unix_ms }).unwrap_or(i64::MAX);
        let window_ms: i64 = if q.mode.eq_ignore_ascii_case("FT4") { 14_000 } else { 30_000 };
        if self.last_qso.as_ref().is_some_and(|(call, at)| call.eq_ignore_ascii_case(&q.call) && (stamp - at).abs() < window_ms) {
            return Vec::new();
        }
        self.last_qso = Some((q.call.clone(), stamp));
        let instant = |ms: u64| (ms > 0).then(|| i64::try_from(ms).unwrap_or(i64::MAX));
        // il locatore dell'ultimo Status (un Location lo puo' aver cambiato per la sessione), altrimenti quello della configurazione
        let my_grid = self.last_status.as_ref().map(|s| s.de_grid.clone()).filter(|g| !g.is_empty()).unwrap_or_else(|| self.config.de_grid.clone());
        let data = QsoLoggedData {
            time_off_ms: instant(q.time_off_unix_ms),
            dx_call: q.call.clone(),
            dx_grid: q.grid.clone(),
            dial_freq: q.freq_hz,
            mode: q.mode.clone(),
            report_sent: q.rst_sent.clone(),
            report_received: q.rst_rcvd.clone(),
            tx_power: if q.tx_power.is_empty() { self.config.tx_power.clone() } else { q.tx_power.clone() },
            comments: q.comment.clone(),
            name: q.name.clone(),
            time_on_ms: instant(q.time_on_unix_ms),
            operator_call: self.config.de_call.clone(),
            my_call: self.config.de_call.clone(),
            my_grid: my_grid.clone(),
            // Aurora non fa gare: scambi vuoti come WSJT-X fuori dal modo gara; propagazione non nota
            exchange_sent: String::new(),
            exchange_received: String::new(),
            propagation_mode: String::new(),
        };
        let mut out: Vec<Vec<u8>> = self.send(encode_qso_logged(&self.config.id, self.schema, &data)).into_iter().collect();
        if self.config.send_adif {
            let station = adif::Station { call: self.config.de_call.clone(), grid: my_grid, tx_power: self.config.tx_power.clone() };
            let today = adif::utc_date(now_unix_ms / 1000);
            let record = adif::jtdx_record_with(q, &station, &today, self.config.eqsl_sent.then_some(today.as_str()));
            out.extend(self.send(encode_logged_adif(&self.config.id, self.schema, &record)));
        }
        out
    }

    /// Un datagramma arrivato da un programma collegato: comandi per Aurora e, per il Replay, datagrammi da rimandare.
    pub fn on_datagram(&mut self, data: &[u8], tx_first: bool) -> Handled {
        let mut handled = Handled::default();
        // TxInhibit (18) prima di tutto, come `parse_tx_inhibit` di WSJT-X 3.3: con «accetta le richieste» spento si ignora senza contarlo
        match parse_tx_inhibit(data, &self.config.id) {
            InhibitParse::NotInhibit => {}
            _ if !self.config.accept_requests => return handled,
            InhibitParse::OtherId => return handled,
            InhibitParse::Invalid => {
                match &self.inhibit {
                    Some(control) => control.invalid(1),
                    None => tracing::debug!("TxInhibit malformato ignorato"),
                }
                handled.datagrams.extend(self.inhibit_status(false));
                return handled;
            }
            InhibitParse::Command(c) => {
                match &self.inhibit {
                    Some(control) => {
                        tracing::info!("TxInhibit da {} ({}): {} ms", c.controller, c.station, c.ttl_ms);
                        control.command(&c.controller, c.ttl_ms, &c.station);
                    }
                    None => tracing::info!("TxInhibit da {} ignorato: nessuna catena di trasmissione", c.controller),
                }
                handled.datagrams.extend(self.inhibit_status(false));
                return handled;
            }
        }
        let Ok(received) = decode_request(data) else { return handled };
        // `parse_message`: solo i datagrammi per noi (stesso identificativo); uno schema piu' alto del 3 e' un errore, non si legge
        if received.id.as_deref() != Some(self.config.id.as_str()) || received.schema > MAX_SCHEMA {
            return handled;
        }
        if self.schema < received.schema {
            self.schema = received.schema;
        }
        let Some(request) = received.request else { return handled };
        if let Request::Replay = request {
            // il Replay si accetta anche con «accetta le richieste» spento: rimanda i decode (non nuovi) e poi lo stato
            self.last = None;
            let recent: Vec<Recent> = self.recent.iter().cloned().collect();
            for r in &recent {
                handled.datagrams.extend(self.send(self.decode_datagram(r, false)));
            }
            if let Some(status) = self.last_status.clone() {
                handled.datagrams.extend(self.send(self.status_datagram(&status, tx_first)));
            }
            return handled;
        }
        if !self.config.accept_requests {
            return handled;
        }
        match request {
            Request::Reply { decode, .. } => {
                let text = decode.message.unwrap_or_default();
                if !reply_accepted(self.config.accept_reply, &text) {
                    return handled;
                }
                let exact = self.recent.iter().rev().find(|r| r.text.trim() == text.trim() && r.freq_hz == decode.delta_freq && r.time_ms == decode.time_ms);
                let found = exact.or_else(|| self.recent.iter().rev().find(|r| r.text.trim() == text.trim() && r.freq_hz == decode.delta_freq));
                match found {
                    Some(r) => {
                        handled.commands.push(Command::Reply { decode_id: r.id });
                        handled.reply_accepted = true;
                    }
                    None => tracing::debug!("Reply UDP ignorato, messaggio non trovato tra i recenti: {text}"),
                }
            }
            // «solo Abilita Tx» o tutto: in Aurora l'unico comando e' lo STOP della trasmissione
            Request::HaltTx { .. } => handled.commands.push(Command::HaltTx),
            Request::FreeText { text, send } => handled.commands.push(Command::SendFreeText { text, send }),
            Request::SetTxFreq { hz } => {
                if (1..=5000).contains(&hz) {
                    handled.commands.push(Command::SetTxFrequency { hz });
                }
            }
            Request::TriggerCq { direction, send } => {
                if send {
                    handled.commands.push(Command::CallCq { modifier: direction });
                }
            }
            // WSJT-X: senza il campo della finestra vale 0 (attivita' di banda); un valore oltre 2 vale «entrambe»
            Request::Clear { window } => handled.external.push(External::Clear { window: window.unwrap_or(0).min(2) }),
            Request::Location { location } => match parse_location(&location) {
                Some(grid) => handled.external.push(External::Location { grid }),
                None => tracing::info!("Location UDP ignorato: «{location}» non e' un locatore"),
            },
            Request::HighlightCallsign { call, background, foreground, last_only } => {
                let call = call.trim().to_ascii_uppercase();
                if !call.is_empty() && call.len() <= 20 {
                    handled.external.push(External::Highlight { call, background, foreground, last_only });
                }
            }
            Request::SwitchConfiguration { name } => handled.external.push(External::SwitchConfiguration { name }),
            Request::Configure(c) => handled.external.push(External::Configure(c)),
            Request::AnnotationInfo { call, order } => {
                let call = call.trim().to_ascii_uppercase();
                if !call.is_empty() && call.len() <= 20 {
                    handled.external.push(External::Annotation { call, order });
                }
            }
            Request::Replay => {}
        }
        handled
    }
}

fn now_unix_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

/// Cio' che l'emettitore riceve da chi lo costruisce oltre alla configurazione.
#[derive(Clone, Default)]
pub struct EmitterHooks {
    /// I filtri di testo (con `apply_filters`).
    pub filters: Option<SharedUdpFilters>,
    /// Il continente di chi opera (filtro «nascondi il proprio continente»).
    pub own_continent: Option<String>,
    /// Chiamata quando una richiesta `Reply` e' accettata: la finestra del programma si porta in primo piano o si ripristina.
    pub on_reply_accepted: Option<Arc<dyn Fn() + Send + Sync>>,
    /// I comandi in ingresso che non sono di trasmissione (colori, Clear, Location, Configure, cambio di configurazione).
    pub on_external: Option<Arc<dyn Fn(External) + Send + Sync>>,
    /// Chi applica i TxInhibit (tipo 18); senza, l'istanza non li supporta.
    pub inhibit: Option<Arc<dyn InhibitControl>>,
}

/// Da quale socket parte un destinatario: unicast (uno per famiglia di indirizzi) o multicast (uno per TTL e interfaccia).
#[derive(Clone, Debug, PartialEq, Eq)]
enum SocketKind {
    Unicast { ipv6: bool, loopback: bool },
    Multicast { ipv6: bool, options: Multicast },
}

impl SocketKind {
    fn of(dest: &Destination, all: &[Destination]) -> SocketKind {
        let ipv6 = dest.addr.is_ipv6();
        match &dest.multicast {
            Some(options) if dest.addr.ip().is_multicast() => SocketKind::Multicast { ipv6, options: options.clone() },
            // il socket unicast si lega al loopback solo se tutti i destinatari unicast della stessa famiglia sono su questo PC (come prima)
            _ => SocketKind::Unicast {
                ipv6,
                loopback: all.iter().filter(|d| d.addr.is_ipv6() == ipv6 && !is_multicast_route(d)).all(|d| d.addr.ip().is_loopback()),
            },
        }
    }
}

fn is_multicast_route(d: &Destination) -> bool {
    d.multicast.is_some() && d.addr.ip().is_multicast()
}

/// Apre un socket. Unicast: porta effimera sul loopback o su tutte le interfacce. Multicast: TTL, interfaccia di uscita e ritorno locale acceso
/// (i programmi sullo stesso PC ricevono il gruppo); nessuna iscrizione al gruppo: come WSJT-X si scrive al gruppo e i comandi tornano
/// all'indirizzo del mittente.
async fn open_socket(kind: &SocketKind) -> std::io::Result<UdpSocket> {
    match kind {
        SocketKind::Unicast { ipv6, loopback } => {
            let ip: IpAddr = match (ipv6, loopback) {
                (false, true) => Ipv4Addr::LOCALHOST.into(),
                (false, false) => Ipv4Addr::UNSPECIFIED.into(),
                (true, true) => Ipv6Addr::LOCALHOST.into(),
                (true, false) => Ipv6Addr::UNSPECIFIED.into(),
            };
            UdpSocket::bind(SocketAddr::new(ip, 0)).await
        }
        SocketKind::Multicast { ipv6, options } => {
            use socket2::{Domain, Protocol, Socket, Type};
            let socket = Socket::new(if *ipv6 { Domain::IPV6 } else { Domain::IPV4 }, Type::DGRAM, Some(Protocol::UDP))?;
            if *ipv6 {
                socket.set_multicast_hops_v6(options.ttl)?;
                socket.set_multicast_loop_v6(true)?;
                if options.interface.is_some() {
                    tracing::info!("multicast IPv6: l'interfaccia di uscita si sceglie per numero, qui decide il sistema");
                }
                socket.bind(&SocketAddr::new(Ipv6Addr::UNSPECIFIED.into(), 0).into())?;
            } else {
                socket.set_multicast_ttl_v4(options.ttl)?;
                socket.set_multicast_loop_v4(true)?;
                if let Some(IpAddr::V4(interface)) = options.interface {
                    socket.set_multicast_if_v4(&interface)?;
                }
                socket.bind(&SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), 0).into())?;
            }
            socket.set_nonblocking(true)?;
            UdpSocket::from_std(socket.into())
        }
    }
}

/// Un destinatario e il socket (indice in `Emitter::sockets`) da cui gli si scrive.
struct Route {
    dest: Destination,
    socket: usize,
}

/// L'emettitore in esecuzione: i socket UDP da cui partono i datagrammi e su cui tornano i comandi (uno unicast per famiglia, uno per ogni
/// gruppo multicast con le sue opzioni).
pub struct Emitter {
    local: SocketAddr,
    sockets: Vec<Arc<UdpSocket>>,
    routes: Arc<Vec<Route>>,
    session: Arc<Mutex<Session>>,
    tasks: Vec<JoinHandle<()>>,
}

impl Emitter {
    /// Avvia l'emettitore (dentro un runtime Tokio). `tx_first` dice se tocca a noi il primo periodo (byte `tx_first` dello Status).
    pub async fn start(config: EmitterConfig, events: broadcast::Receiver<Event>, commands: mpsc::Sender<Command>, tx_first: Arc<dyn Fn() -> bool + Send + Sync>) -> std::io::Result<Emitter> {
        Emitter::start_with(config, events, commands, tx_first, EmitterHooks::default()).await
    }

    /// Come `start`, con i filtri di testo e la notifica delle richieste accettate.
    pub async fn start_with(
        mut config: EmitterConfig,
        mut events: broadcast::Receiver<Event>,
        commands: mpsc::Sender<Command>,
        tx_first: Arc<dyn Fn() -> bool + Send + Sync>,
        hooks: EmitterHooks,
    ) -> std::io::Result<Emitter> {
        let destinations = config.all_destinations();
        // il Logged ADIF si prepara se almeno un destinatario lo vuole; a chi va lo decide `Destination::wants`
        config.send_adif = destinations.iter().any(|d| d.logged_adif);
        let mut kinds: Vec<SocketKind> = Vec::new();
        let mut routes: Vec<Route> = Vec::new();
        for dest in destinations.iter().cloned() {
            let kind = SocketKind::of(&dest, &destinations);
            let socket = match kinds.iter().position(|k| *k == kind) {
                Some(i) => i,
                None => {
                    kinds.push(kind);
                    kinds.len() - 1
                }
            };
            routes.push(Route { dest, socket });
        }
        // senza destinatari un socket sul loopback c'e' comunque (l'indirizzo dei comandi non cambia significato)
        if kinds.is_empty() {
            kinds.push(SocketKind::Unicast { ipv6: false, loopback: true });
        }
        let mut sockets: Vec<Arc<UdpSocket>> = Vec::new();
        for kind in &kinds {
            sockets.push(Arc::new(open_socket(kind).await?));
        }
        let local = sockets.first().map(|s| s.local_addr()).transpose()?.unwrap_or_else(|| SocketAddr::from(([127, 0, 0, 1], 0)));
        let routes = Arc::new(routes);
        let heartbeat = config.heartbeat;
        let mut first_session = Session::new(config);
        first_session.set_filters(hooks.filters.clone(), hooks.own_continent.clone());
        first_session.set_inhibit(hooks.inhibit.clone());
        let session = Arc::new(Mutex::new(first_session));
        let on_accepted = hooks.on_reply_accepted.clone();
        let on_external = hooks.on_external.clone();

        let first = lock(&session)
            .map(|mut s| {
                let mut out = s.start();
                out.extend(s.inhibit_status(true));
                out
            })
            .unwrap_or_default();
        send_all(&sockets, &routes, &first).await;

        // ogni socket ascolta i comandi di ritorno e li passa al ciclo principale
        let (incoming_tx, mut incoming) = mpsc::channel::<Vec<u8>>(64);
        let mut tasks: Vec<JoinHandle<()>> = sockets
            .iter()
            .map(|socket| {
                let (socket, incoming_tx) = (Arc::clone(socket), incoming_tx.clone());
                tokio::spawn(async move {
                    let mut buf = vec![0u8; 65_535];
                    loop {
                        match socket.recv_from(&mut buf).await {
                            Ok((n, _from)) => {
                                if incoming_tx.send(buf[..n].to_vec()).await.is_err() {
                                    break;
                                }
                            }
                            // su Windows un ICMP «porta chiusa» (GridTracker spento) appare come errore di ricezione: si prosegue
                            Err(e) => {
                                tracing::debug!("errore di ricezione sulla porta del flusso UDP: {e}");
                                tokio::time::sleep(Duration::from_millis(10)).await;
                            }
                        }
                    }
                })
            })
            .collect();
        drop(incoming_tx);

        let main = {
            let (sockets, routes, session) = (sockets.clone(), Arc::clone(&routes), Arc::clone(&session));
            tokio::spawn(async move {
                let mut tick = tokio::time::interval(heartbeat);
                tick.tick().await;
                let mut inhibit_tick = tokio::time::interval(INHIBIT_TICK);
                inhibit_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
                loop {
                    tokio::select! {
                        event = events.recv() => match event {
                            Ok(event) => {
                                let out = lock(&session).map(|mut s| s.on_event(&event, tx_first(), now_unix_ms())).unwrap_or_default();
                                send_all(&sockets, &routes, &out).await;
                            }
                            Err(RecvError::Lagged(_)) => {}
                            Err(RecvError::Closed) => break,
                        },
                        _ = tick.tick() => {
                            // WSJT-X: dopo ogni Heartbeat anche l'InhibitStatus, se il blocco e' supportato
                            let out = lock(&session).map(|mut s| [Some(s.heartbeat()), s.inhibit_status(true)].into_iter().flatten().collect::<Vec<_>>());
                            if let Some(out) = out {
                                send_all(&sockets, &routes, &out).await;
                            }
                        }
                        _ = inhibit_tick.tick() => {
                            let out = lock(&session).and_then(|mut s| s.inhibit_tick());
                            if let Some(out) = out {
                                send_all(&sockets, &routes, &[out]).await;
                            }
                        }
                        received = incoming.recv() => match received {
                            Some(data) => {
                                let handled = lock(&session).map(|mut s| s.on_datagram(&data, tx_first())).unwrap_or_default();
                                if handled.reply_accepted
                                    && let Some(hook) = &on_accepted
                                {
                                    hook();
                                }
                                for command in handled.commands {
                                    if commands.send(command).await.is_err() {
                                        tracing::debug!("comando UDP non consegnato: canale chiuso");
                                    }
                                }
                                for external in handled.external {
                                    match &on_external {
                                        Some(hook) => hook(external),
                                        None => tracing::debug!("comando UDP senza destinatario: {external:?}"),
                                    }
                                }
                                send_all(&sockets, &routes, &handled.datagrams).await;
                            }
                            None => break,
                        },
                    }
                }
            })
        };
        tasks.push(main);
        Ok(Emitter { local, sockets, routes, session, tasks })
    }

    /// L'indirizzo da cui partono i datagrammi (e su cui i programmi collegati mandano i comandi): quello del primo socket.
    pub fn local_addr(&self) -> SocketAddr {
        self.local
    }

    /// Gli indirizzi di tutti i socket (unicast e multicast): i comandi possono tornare a ciascuno.
    pub fn local_addrs(&self) -> Vec<SocketAddr> {
        self.sockets.iter().filter_map(|s| s.local_addr().ok()).collect()
    }

    /// Alla chiusura di Aurora: manda il Close e ferma l'emettitore.
    pub async fn shutdown(&self) {
        let out = lock(&self.session).map(|s| s.inhibit_withdrawal().into_iter().chain([s.close()]).collect::<Vec<_>>());
        if let Some(out) = out {
            send_all(&self.sockets, &self.routes, &out).await;
        }
        for task in &self.tasks {
            task.abort();
        }
    }
}

impl Drop for Emitter {
    fn drop(&mut self) {
        for task in &self.tasks {
            task.abort();
        }
    }
}

fn lock(session: &Mutex<Session>) -> Option<std::sync::MutexGuard<'_, Session>> {
    session.lock().ok()
}

async fn send_all(sockets: &[Arc<UdpSocket>], routes: &[Route], datagrams: &[Vec<u8>]) {
    for datagram in datagrams {
        for route in routes.iter().filter(|r| r.dest.wants(datagram)) {
            let Some(socket) = sockets.get(route.socket) else { continue };
            if let Err(e) = socket.send_to(datagram, route.dest.addr).await {
                tracing::debug!("invio verso {} non riuscito: {e}", route.dest.addr);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_location_is_read_like_wsjtx() {
        assert_eq!(parse_location("fn42AB"), Some("FN42ab".to_string()));
        assert_eq!(parse_location(" FN42 "), Some("FN42".to_string()));
        assert_eq!(parse_location("GRID:fn42ab12"), Some("FN42ab12".to_string()));
        assert_eq!(parse_location("FN42ab12"), None, "oltre sei caratteri serve GRID:");
        assert_eq!(parse_location("ZZ42"), None);
        assert_eq!(parse_location("FN4"), None);
        assert_eq!(parse_location("45.0,9.0"), None);
    }

    #[test]
    fn each_destination_gets_only_what_it_asked_for() {
        let adif = udp_bridge::encode_logged_adif("JTDX", 2, "<EOR>");
        let clear = encode_clear("JTDX", 2);
        let stream_only = Destination { addr: SocketAddr::from(([127, 0, 0, 1], 2237)), stream: true, logged_adif: false, multicast: None };
        let adif_only = Destination { stream: false, logged_adif: true, ..stream_only.clone() };
        assert!(stream_only.wants(&clear) && !stream_only.wants(&adif));
        assert!(!adif_only.wants(&clear) && adif_only.wants(&adif));
    }

    #[test]
    fn targets_and_destinations_merge_without_doubles() {
        let a = SocketAddr::from(([127, 0, 0, 1], 2237));
        let group = SocketAddr::from(([239, 255, 0, 1], 2237));
        let mut config = EmitterConfig::jtdx(vec![a, group]);
        config.send_adif = true;
        config.destinations = vec![
            Destination { addr: a, stream: false, logged_adif: true, multicast: None },
            Destination { addr: SocketAddr::from(([127, 0, 0, 1], 2240)), stream: false, logged_adif: false, multicast: None },
        ];
        let all = config.all_destinations();
        assert_eq!(all.len(), 2, "lo stesso indirizzo una volta, il destinatario senza niente da ricevere si salta");
        assert!(all[0].logged_adif && all[0].multicast.is_none());
        assert_eq!(all[1].multicast, Some(Multicast::default()), "un gruppo multicast tra i destinatari semplici ha TTL 1");
    }
}
