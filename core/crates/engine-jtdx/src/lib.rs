//! Motore Aurora per JTDX: traduce il flusso UDP in eventi e i comandi in pacchetti.
//!
//! Sicurezza: `Reply` fa trasmettere JTDX (provato dal vivo), quindi viene inviato solo con
//! `allow_tx`; `HaltTx` invece parte sempre.

pub mod enrich;
pub mod ini;
pub mod ini_edit;
pub mod map;
pub mod matrix;
pub mod text;
pub mod worked;
pub mod store;

use std::{
    net::SocketAddr,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use aurora_core::{Command, Engine, EngineIo, Event};
use tokio::sync::broadcast::{self, error::RecvError};
use udp_bridge::{decode, encode_free_text, encode_halt_tx, encode_replay, encode_reply, encode_set_tx_freq, encode_trigger_cq, Message};
use udp_hub::Hub;

use dxcc::DxccDb;

use crate::{
    enrich::Enrichment,
    map::{band_of, to_decoded, to_qso, to_status},
    store::DecodeStore,
    worked::WorkedLog,
};

const DAY_MS: u64 = 86_400_000;

pub trait Clock: Send + Sync {
    fn now_ms(&self) -> u64;
}

pub struct SystemClock;

impl Clock for SystemClock {
    fn now_ms(&self) -> u64 {
        SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
    }
}

#[derive(Debug, Clone)]
pub struct JtdxConfig {
    /// Se falso, `Reply` viene rifiutato: un Reply fa trasmettere JTDX.
    pub allow_tx: bool,
    /// Un decode piu' vecchio di cosi' non e' piu' nella finestra di JTDX e non si puo' rispondere.
    pub reply_max_age_ms: u64,
    /// Chiede a JTDX di rimandare i decode correnti appena lo vede (o lo rivede riavviato).
    pub replay_on_connect: bool,
    /// File dove ricordare indirizzo e identificativo di JTDX: dopo un riavvio di Aurora i comandi (anche STOP) partono subito,
    /// senza aspettare il primo pacchetto di JTDX (che arriva a fine periodo, anche 15 secondi dopo).
    pub peer_cache: Option<std::path::PathBuf>,
}

impl Default for JtdxConfig {
    fn default() -> Self {
        JtdxConfig { allow_tx: false, reply_max_age_ms: 45_000, replay_on_connect: true, peer_cache: None }
    }
}

/// Indirizzo, identificativo e schema di JTDX come si ricordano su disco: tre righe di testo.
pub fn encode_peer(addr: SocketAddr, id: &str, schema: u32) -> String {
    format!("{addr}\n{id}\n{schema}\n")
}

/// Il contrario di `encode_peer`; `None` se il file e' vuoto, rovinato o con un identificativo con a capo.
pub fn decode_peer(text: &str) -> Option<(SocketAddr, String, u32)> {
    let mut lines = text.lines();
    let addr = lines.next()?.trim().parse().ok()?;
    let id = lines.next()?.trim().to_string();
    let schema = lines.next()?.trim().parse().ok()?;
    if id.is_empty() || lines.next().is_some_and(|l| !l.trim().is_empty()) {
        return None;
    }
    Some((addr, id, schema))
}

pub struct JtdxEngine {
    hub: Arc<Hub>,
    config: JtdxConfig,
    clock: Arc<dyn Clock>,
    datagrams: broadcast::Receiver<Vec<u8>>,
    enrichment: Option<Enrichment>,
}

impl JtdxEngine {
    /// Si iscrive subito all'hub: nessun datagramma inviato dopo `new` va perso.
    pub fn new(hub: Arc<Hub>, config: JtdxConfig, clock: Arc<dyn Clock>) -> Self {
        let datagrams = hub.subscribe();
        JtdxEngine { hub, config, clock, datagrams, enrichment: None }
    }

    /// Aggiunge paese e novita' (nuovo DXCC, zona, griglia) ai decode.
    pub fn with_enrichment(mut self, db: Arc<DxccDb>, worked: WorkedLog) -> Self {
        self.enrichment = Some(Enrichment::new(db, worked));
        self
    }

    /// Le evidenziazioni di «nuovo» (scheda Notifiche) condivise con chi le cambia; senza, valgono quelle di fabbrica di JTDX.
    pub fn with_highlight_flags(mut self, flags: enrich::SharedFlags) -> Self {
        if let Some(e) = self.enrichment.take() {
            self.enrichment = Some(e.with_flags(flags));
        }
        self
    }

    /// Come `with_enrichment`, con i QSO gia' fatti condivisi (la scheda DXCC e l'importazione del log li aggiornano).
    pub fn with_shared_enrichment(mut self, db: Arc<DxccDb>, worked: enrich::SharedWorked) -> Self {
        self.enrichment = Some(Enrichment::with_shared(db, worked));
        self
    }
}

struct State {
    store: DecodeStore,
    /// Identificativo e schema dell'ultimo pacchetto di JTDX: i comandi devono riusarli.
    ident: Option<(String, u32)>,
    last_peer: Option<SocketAddr>,
    /// Quello gia' scritto nel file dell'indirizzo: si riscrive solo se cambia.
    saved_peer: Option<(SocketAddr, String, u32)>,
    enrichment: Option<Enrichment>,
    /// Banda corrente, dall'ultima frequenza comunicata da JTDX.
    band: Option<&'static str>,
    /// Modo corrente (FT8, FT4...), dall'ultimo `Status`: serve alle evidenziazioni «per modo».
    mode: Option<String>,
}

fn message_id(m: &Message) -> Option<String> {
    match m {
        Message::Heartbeat { id, .. }
        | Message::Clear { id }
        | Message::QsoLogged { id, .. }
        | Message::LoggedAdif { id, .. }
        | Message::Close { id }
        | Message::Unknown { id, .. } => id.clone(),
        Message::Status(s) => s.id.clone(),
        Message::Decode(d) => d.id.clone(),
    }
}

impl Engine for JtdxEngine {
    async fn run(self, mut io: EngineIo) {
        let JtdxEngine { hub, config, clock, mut datagrams, enrichment } = self;
        let mut state = State { store: DecodeStore::new(), ident: None, last_peer: None, saved_peer: None, enrichment, band: None, mode: None };
        if let Some(path) = &config.peer_cache
            && let Some((addr, id, schema)) = std::fs::read_to_string(path).ok().as_deref().and_then(decode_peer)
        {
            hub.set_peer(Some(addr));
            state.ident = Some((id.clone(), schema));
            state.last_peer = Some(addr);
            state.saved_peer = Some((addr, id, schema));
        }
        loop {
            tokio::select! {
                received = datagrams.recv() => match received {
                    Ok(bytes) => on_datagram(&hub, &config, clock.as_ref(), &io, &mut state, &bytes).await,
                    Err(RecvError::Lagged(n)) => tracing::warn!("{n} datagrammi persi: il motore e' troppo lento"),
                    Err(RecvError::Closed) => return,
                },
                command = io.commands.recv() => match command {
                    Some(c) => on_command(&hub, &config, clock.as_ref(), &io, &mut state, c).await,
                    None => return,
                },
            }
        }
    }
}

/// Ricorda su disco dove sta JTDX (se e' cambiato), per i comandi subito dopo un riavvio di Aurora.
fn remember_peer(config: &JtdxConfig, state: &mut State, peer: Option<SocketAddr>) {
    let (Some(path), Some(addr), Some((id, schema))) = (&config.peer_cache, peer, &state.ident) else { return };
    let current = (addr, id.clone(), *schema);
    if state.saved_peer.as_ref() == Some(&current) {
        return;
    }
    match std::fs::write(path, encode_peer(current.0, &current.1, current.2)) {
        Ok(()) => state.saved_peer = Some(current),
        Err(e) => tracing::warn!("indirizzo di JTDX non salvato in {}: {e}", path.display()),
    }
}

async fn send_to_jtdx(hub: &Hub, bytes: Vec<u8>, what: &str) {
    if let Err(e) = hub.send_to_jtdx(&bytes).await {
        tracing::warn!("{what} non inviato a JTDX: {e}");
    }
}

async fn on_datagram(hub: &Hub, config: &JtdxConfig, clock: &dyn Clock, io: &EngineIo, state: &mut State, bytes: &[u8]) {
    let packet = match decode(bytes) {
        Ok(p) => p,
        Err(e) => {
            tracing::debug!("datagramma non valido ignorato: {e}");
            return;
        }
    };
    if let Some(id) = message_id(&packet.message) {
        state.ident = Some((id, packet.schema));
    }

    // JTDX nuovo (o riavviato con un'altra porta): si chiede di rimandare i decode correnti
    let peer = hub.jtdx_addr();
    remember_peer(config, state, peer);
    if peer != state.last_peer {
        state.last_peer = peer;
        if let (true, Some((id, schema))) = (config.replay_on_connect, &state.ident) {
            send_to_jtdx(hub, encode_replay(id, *schema), "Replay").await;
        }
    }

    match packet.message {
        Message::Status(s) => {
            state.band = if s.dial_freq > 0 { band_of(s.dial_freq) } else { None };
            let mut status = to_status(&s);
            state.mode = Some(status.mode.clone()).filter(|m| !m.trim().is_empty());
            status.own_continent = state.enrichment.as_ref().and_then(|e| e.continent_of(&status.de_call));
            let _ = io.events.send(Event::Status(status));
        }
        Message::Decode(d) => {
            // un decode rimandato da un Replay e gia' noto non va mostrato due volte
            if !d.is_new && state.store.find_same(&d).is_some() {
                return;
            }
            let id = state.store.insert(d.clone(), clock.now_ms());
            let mut message = to_decoded(id, &d);
            if let Some(enrichment) = &state.enrichment {
                let found = enrichment.annotate_in(&message.text, state.band, state.mode.as_deref());
                message.country = found.country;
                message.continent = found.continent;
                message.tag = found.tag;
            }
            let _ = io.events.send(Event::Decode(message));
        }
        Message::Clear { .. } => state.store.clear(),
        ref m @ Message::QsoLogged { .. } => {
            let time_ms = (clock.now_ms() % DAY_MS) as u32;
            if let Some(qso) = to_qso(m, time_ms) {
                if let Some(enrichment) = &state.enrichment {
                    enrichment.record_qso(&qso.call, &qso.grid, &qso.band, &qso.mode);
                }
                let _ = io.events.send(Event::QsoLogged(qso));
            }
        }
        _ => {}
    }
}

async fn on_command(hub: &Hub, config: &JtdxConfig, clock: &dyn Clock, io: &EngineIo, state: &mut State, command: Command) {
    // lo STOP si annuncia sempre, anche se JTDX non e' ancora noto: chi ascolta (l'automazione) deve fermarsi
    if command == Command::HaltTx {
        let _ = io.events.send(Event::TxHalted);
    }
    let Some((id, schema)) = state.ident.clone() else {
        tracing::warn!("comando scartato: JTDX non ancora noto");
        return;
    };
    match command {
        // fermare la trasmissione e' sempre ammesso
        Command::HaltTx => send_to_jtdx(hub, encode_halt_tx(&id, schema, false), "HaltTx").await,
        Command::SendFreeText { text, send } => {
            if send && !config.allow_tx {
                tracing::warn!("testo libero respinto: la trasmissione non e' abilitata");
                return;
            }
            send_to_jtdx(hub, encode_free_text(&id, schema, &text, send), "FreeText").await;
        }
        Command::CallCq { modifier } => {
            if !config.allow_tx {
                tracing::warn!("CQ respinto: la trasmissione non e' abilitata");
                return;
            }
            send_to_jtdx(hub, encode_trigger_cq(&id, schema, &modifier, true), "TriggerCQ").await;
        }
        Command::SetTxFrequency { hz } => {
            send_to_jtdx(hub, encode_set_tx_freq(&id, schema, i32::try_from(hz).unwrap_or(1500)), "SetTxFreq").await;
        }
        Command::Reply { decode_id } => {
            if !config.allow_tx {
                tracing::warn!("Reply respinto: la trasmissione non e' abilitata (un Reply fa trasmettere JTDX)");
                return;
            }
            let bytes = state
                .store
                .get(decode_id, clock.now_ms(), config.reply_max_age_ms)
                .map(|original| encode_reply(&id, schema, original, 0));
            match bytes {
                Some(b) => send_to_jtdx(hub, b, "Reply").await,
                None => tracing::warn!("Reply ignorato: decode {decode_id} sconosciuto o non piu' nella finestra di JTDX"),
            }
        }
    }
}
