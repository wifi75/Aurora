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
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Duration,
};

use aurora_core::{Command, DecodedMessage, Event, QsoRecord, StationStatus};
use logbook::adif;
use tokio::{
    net::UdpSocket,
    sync::{broadcast, broadcast::error::RecvError, mpsc},
    task::JoinHandle,
};
use udp_bridge::{
    decode_request, encode_clear, encode_close, encode_decode, encode_heartbeat, encode_logged_adif, encode_qso_logged, encode_status, Decode, QsoLoggedData, Request, Status,
    HEARTBEAT_SECS, INITIAL_SCHEMA, MAX_SCHEMA,
};

/// Quanti messaggi decodificati si ricordano: servono al Reply (che li indica per testo) e al Replay.
pub const RECENT_DECODES: usize = 300;

/// Identificativo di istanza di JTDX: `QApplication::applicationName()`, impostato in `main()` a «JTDX» (con `--rig-name` diventa «JTDX - nome»).
pub const JTDX_ID: &str = "JTDX";
/// Versione che JTDX dichiara nel Heartbeat.
pub const JTDX_VERSION: &str = "2.2.159";

#[derive(Clone, Debug)]
pub struct EmitterConfig {
    pub id: String,
    pub version: String,
    /// I destinatari del flusso (il «primo server UDP» di JTDX e chi l'hub inoltrava: GridTracker2 su 127.0.0.1:2333).
    pub targets: Vec<SocketAddr>,
    /// Nominativo e locatore propri, per il QSO Logged e per il record ADIF (lo Status porta i suoi).
    pub de_call: String,
    pub de_grid: String,
    /// Potenza in watt scritta nel QSO (vuota: nessuna, come JTDX con «Save Tx power» spento).
    pub tx_power: String,
    /// `AcceptUDPRequests`: senza, i comandi di ritorno (tranne Replay) si ignorano, come in JTDX.
    pub accept_requests: bool,
    /// `AcceptUDPReplyMessages`: 1 solo CQ, 2 CQ e 73, 3 tutti.
    pub accept_reply: u8,
    /// `EnableUDP1adifSending`: dopo ogni QSO Logged manda anche il Logged ADIF.
    pub send_adif: bool,
    pub heartbeat: Duration,
}

impl EmitterConfig {
    /// Come JTDX 2.2.159 con la sua configurazione di fabbrica: nessun comando accettato, niente Logged ADIF.
    pub fn jtdx(targets: Vec<SocketAddr>) -> EmitterConfig {
        EmitterConfig {
            id: JTDX_ID.to_string(),
            version: JTDX_VERSION.to_string(),
            targets,
            de_call: String::new(),
            de_grid: String::new(),
            tx_power: String::new(),
            accept_requests: false,
            accept_reply: 1,
            send_adif: false,
            heartbeat: Duration::from_secs(HEARTBEAT_SECS),
        }
    }
}

#[derive(Clone)]
struct Recent {
    id: u64,
    time_ms: u32,
    freq_hz: u32,
    snr: i32,
    delta_time: f32,
    text: String,
}

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
}

/// Cosa fare dopo un datagramma ricevuto.
#[derive(Debug, Default)]
pub struct Handled {
    pub commands: Vec<Command>,
    /// Datagrammi da rimandare ai destinatari (il Replay rimanda i decode correnti).
    pub datagrams: Vec<Vec<u8>>,
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
        Session { config, schema: INITIAL_SCHEMA, last: None, mode: "FT8".to_string(), last_status: None, recent: VecDeque::new(), last_qso: None }
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
                low_confidence: false,
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
                let recent = self.remember(m);
                let datagram = self.decode_datagram(&recent, true);
                self.push_recent(recent);
                self.send(datagram).into_iter().collect()
            }
            Event::QsoLogged(q) => self.on_qso(q, now_unix_ms),
            Event::TxHalted | Event::Wanted(_) | Event::UpdateAvailable(_) | Event::AudioLevel(_) | Event::BandChanged { .. } => Vec::new(),
        }
    }

    fn remember(&self, m: &DecodedMessage) -> Recent {
        Recent { id: m.id, time_ms: m.time_ms, freq_hz: m.freq_hz, snr: m.snr, delta_time: m.delta_time, text: m.text.clone() }
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
        let data = QsoLoggedData {
            time_off_ms: instant(q.time_off_unix_ms),
            dx_call: q.call.clone(),
            dx_grid: q.grid.clone(),
            dial_freq: q.freq_hz,
            mode: q.mode.clone(),
            report_sent: q.rst_sent.clone(),
            report_received: q.rst_rcvd.clone(),
            tx_power: self.config.tx_power.clone(),
            comments: String::new(),
            name: String::new(),
            time_on_ms: instant(q.time_on_unix_ms),
            operator_call: self.config.de_call.clone(),
            my_call: self.config.de_call.clone(),
            my_grid: self.config.de_grid.clone(),
        };
        let mut out: Vec<Vec<u8>> = self.send(encode_qso_logged(&self.config.id, self.schema, &data)).into_iter().collect();
        if self.config.send_adif {
            let station = adif::Station { call: self.config.de_call.clone(), grid: self.config.de_grid.clone(), tx_power: self.config.tx_power.clone() };
            let record = adif::jtdx_record(q, &station, &adif::utc_date(now_unix_ms / 1000));
            out.extend(self.send(encode_logged_adif(&self.config.id, self.schema, &record)));
        }
        out
    }

    /// Un datagramma arrivato da un programma collegato: comandi per Aurora e, per il Replay, datagrammi da rimandare.
    pub fn on_datagram(&mut self, data: &[u8], tx_first: bool) -> Handled {
        let mut handled = Handled::default();
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
                    Some(r) => handled.commands.push(Command::Reply { decode_id: r.id }),
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
            Request::Replay => {}
        }
        handled
    }
}

fn now_unix_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

/// L'emettitore in esecuzione: un socket UDP da cui partono i datagrammi e su cui tornano i comandi.
pub struct Emitter {
    local: SocketAddr,
    socket: Arc<UdpSocket>,
    session: Arc<Mutex<Session>>,
    targets: Vec<SocketAddr>,
    task: JoinHandle<()>,
}

impl Emitter {
    /// Avvia l'emettitore (dentro un runtime Tokio). `tx_first` dice se tocca a noi il primo periodo (byte `tx_first` dello Status).
    pub async fn start(
        config: EmitterConfig,
        mut events: broadcast::Receiver<Event>,
        commands: mpsc::Sender<Command>,
        tx_first: Arc<dyn Fn() -> bool + Send + Sync>,
    ) -> std::io::Result<Emitter> {
        // un solo socket per tutti i destinatari: i comandi tornano all'indirizzo da cui e' partito il flusso (come con JTDX)
        let loopback = config.targets.iter().all(|t| t.ip().is_loopback());
        let ipv6 = config.targets.iter().any(|t| t.is_ipv6());
        let bind = match (ipv6, loopback) {
            (false, true) => "127.0.0.1:0",
            (false, false) => "0.0.0.0:0",
            (true, true) => "[::1]:0",
            (true, false) => "[::]:0",
        };
        let socket = Arc::new(UdpSocket::bind(bind).await?);
        let local = socket.local_addr()?;
        let targets = config.targets.clone();
        let heartbeat = config.heartbeat;
        let session = Arc::new(Mutex::new(Session::new(config)));

        let first = lock(&session).map(|mut s| s.start()).unwrap_or_default();
        send_all(&socket, &targets, &first).await;

        let task = {
            let (socket, targets, session) = (Arc::clone(&socket), targets.clone(), Arc::clone(&session));
            tokio::spawn(async move {
                let mut tick = tokio::time::interval(heartbeat);
                tick.tick().await;
                let mut buf = vec![0u8; 65_535];
                loop {
                    tokio::select! {
                        event = events.recv() => match event {
                            Ok(event) => {
                                let out = lock(&session).map(|mut s| s.on_event(&event, tx_first(), now_unix_ms())).unwrap_or_default();
                                send_all(&socket, &targets, &out).await;
                            }
                            Err(RecvError::Lagged(_)) => {}
                            Err(RecvError::Closed) => break,
                        },
                        _ = tick.tick() => {
                            let heartbeat = lock(&session).map(|s| s.heartbeat());
                            if let Some(heartbeat) = heartbeat {
                                send_all(&socket, &targets, &[heartbeat]).await;
                            }
                        }
                        received = socket.recv_from(&mut buf) => match received {
                            Ok((n, _from)) => {
                                let handled = lock(&session).map(|mut s| s.on_datagram(&buf[..n], tx_first())).unwrap_or_default();
                                for command in handled.commands {
                                    if commands.send(command).await.is_err() {
                                        tracing::debug!("comando UDP non consegnato: canale chiuso");
                                    }
                                }
                                send_all(&socket, &targets, &handled.datagrams).await;
                            }
                            // su Windows un ICMP «porta chiusa» (GridTracker spento) appare come errore di ricezione: si prosegue
                            Err(e) => {
                                tracing::debug!("errore di ricezione sulla porta del flusso UDP: {e}");
                                tokio::time::sleep(Duration::from_millis(10)).await;
                            }
                        },
                    }
                }
            })
        };
        Ok(Emitter { local, socket, session, targets, task })
    }

    /// L'indirizzo da cui partono i datagrammi (e su cui i programmi collegati mandano i comandi).
    pub fn local_addr(&self) -> SocketAddr {
        self.local
    }

    /// Alla chiusura di Aurora: manda il Close e ferma l'emettitore.
    pub async fn shutdown(&self) {
        let close = lock(&self.session).map(|s| s.close());
        if let Some(close) = close {
            send_all(&self.socket, &self.targets, &[close]).await;
        }
        self.task.abort();
    }
}

impl Drop for Emitter {
    fn drop(&mut self) {
        self.task.abort();
    }
}

fn lock(session: &Mutex<Session>) -> Option<std::sync::MutexGuard<'_, Session>> {
    session.lock().ok()
}

async fn send_all(socket: &UdpSocket, targets: &[SocketAddr], datagrams: &[Vec<u8>]) {
    for datagram in datagrams {
        for target in targets {
            if let Err(e) = socket.send_to(datagram, target).await {
                tracing::debug!("invio verso {target} non riuscito: {e}");
            }
        }
    }
}
