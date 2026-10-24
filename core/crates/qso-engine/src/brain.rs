//! Il cervello dell'automazione: decide, non trasmette. Il tempo (secondi UTC) lo passa il chiamante,
//! cosi' ogni salvaguardia si prova senza orologio.

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use aurora_core::{DecodeTag, DecodedMessage};
use engine_jtdx::text::parse_message;
use serde::{Deserialize, Serialize};

use crate::ghost;

const LOG_CAPACITY: usize = 100;
/// Quante stazioni sentite si ricordano al massimo (poi si riparte da zero).
const HEARD_CAPACITY: usize = 5_000;
/// Dopo una risposta, per tanto tempo nessun'altra (a meno che il QSO non venga registrato prima).
const ACTIVE_QSO_SECS: u64 = 60;
const DAY_SECS: u64 = 86_400;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    /// Decide e registra, ma non risponde mai.
    DryRun,
    Live,
}

/// Quali CQ interessano. Modificabili anche dal telefono (non fanno trasmettere di per se').
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rules {
    pub reply_to: Vec<DecodeTag>,
    pub min_snr: i32,
    pub excluded_calls: Vec<String>,
    pub max_attempts_per_call: u32,
    /// Risponde anche ai CQ delle stazioni della lista dei desiderati (nominativo, prefisso, paese o griglia), oltre alle etichette sopra.
    #[serde(default)]
    pub reply_to_wanted: bool,
}

/// Dice se un messaggio e' di una stazione desiderata (e perche'), data la frequenza della radio.
pub type WantedMatcher = Arc<dyn Fn(&DecodedMessage, Option<u64>) -> Option<String> + Send + Sync>;

impl Default for Rules {
    fn default() -> Self {
        Rules {
            reply_to: vec![DecodeTag::NewDxcc, DecodeTag::NewDxccBand, DecodeTag::NewZone, DecodeTag::NewGrid],
            min_snr: -18,
            excluded_calls: Vec::new(),
            max_attempts_per_call: 2,
            reply_to_wanted: false,
        }
    }
}

impl Rules {
    /// Regole sensate: niente CQ comuni (si risponderebbe a tutti), soglie in un intervallo plausibile.
    pub fn is_valid(&self) -> bool {
        // senza etichette si risponde solo ai desiderati; ne' etichette ne' desiderati vorrebbe dire non rispondere mai
        let tags_ok = (!self.reply_to.is_empty() || self.reply_to_wanted)
            && self
                .reply_to
                .iter()
                .all(|t| matches!(t, DecodeTag::NewDxcc | DecodeTag::NewDxccBand | DecodeTag::NewZone | DecodeTag::NewGrid));
        let calls_ok = self.excluded_calls.len() <= 200
            && self
                .excluded_calls
                .iter()
                .all(|c| (3..=13).contains(&c.trim().len()) && c.trim().chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '/'));
        tags_ok && calls_ok && (-30..=20).contains(&self.min_snr) && (1..=10).contains(&self.max_attempts_per_call)
    }
}

/// Salvaguardie: si impostano sul PC, l'API non le cambia.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Limits {
    /// Risposte di fila senza un QSO registrato; al tetto l'automazione si spegne.
    pub max_consecutive_cycles: u32,
    pub max_per_day: u32,
    /// Secondi senza nessun client collegato dopo i quali l'automazione si spegne.
    pub dead_man_secs: u64,
    /// Se vero il dead-man non si applica (stazione senza nessuno collegato).
    pub run_unattended: bool,
}

impl Default for Limits {
    fn default() -> Self {
        Limits { max_consecutive_cycles: 6, max_per_day: 40, dead_man_secs: 120, run_unattended: false }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionKind {
    Reply,
    WouldReply,
    Skipped,
    Blocked,
    Halted,
    Paused,
    Info,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Decision {
    /// Secondi UTC.
    pub at: u64,
    pub kind: DecisionKind,
    pub call: Option<String>,
    pub decode_id: Option<u64>,
    /// Il perche', in italiano.
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LiveNotAllowed;

impl std::fmt::Display for LiveNotAllowed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "la trasmissione non e' abilitata su questo PC")
    }
}

impl std::error::Error for LiveNotAllowed {}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct BrainState {
    pub enabled: bool,
    pub mode: Mode,
    /// Falso se la trasmissione e' chiusa (`AURORA_ALLOW_TX=0`): il modo reale non e' scegliibile.
    pub live_allowed: bool,
    pub rules: Rules,
    pub limits: Limits,
    pub replies_today: u32,
    pub consecutive: u32,
    /// Le ultime decisioni, le piu' vecchie per prime.
    pub log: Vec<Decision>,
}

pub struct Brain {
    state: BrainState,
    day: u64,
    attempts: HashMap<String, u32>,
    active: Option<(String, u64)>,
    no_client_since: Option<u64>,
    /// Stazioni gia' sentite in questa sessione: un segnale debole di una mai sentita e' sospetto.
    heard: HashSet<String>,
    dial_hz: Option<u64>,
    wanted: Option<WantedMatcher>,
}

impl Brain {
    pub fn new(rules: Rules, limits: Limits, live_allowed: bool) -> Brain {
        Brain {
            state: BrainState {
                enabled: false,
                mode: Mode::DryRun,
                live_allowed,
                rules,
                limits,
                replies_today: 0,
                consecutive: 0,
                log: Vec::new(),
            },
            day: 0,
            attempts: HashMap::new(),
            active: None,
            no_client_since: None,
            heard: HashSet::new(),
            dial_hz: None,
            wanted: None,
        }
    }

    /// Come riconoscere le stazioni desiderate (la lista sta fuori dal cervello).
    pub fn set_wanted_matcher(&mut self, matcher: WantedMatcher) {
        self.wanted = Some(matcher);
    }

    /// La frequenza della radio, per i filtri di banda della lista dei desiderati.
    pub fn on_dial(&mut self, hz: u64) {
        self.dial_hz = (hz > 0).then_some(hz);
    }

    pub fn state(&self) -> &BrainState {
        &self.state
    }

    pub fn set_rules(&mut self, rules: Rules) {
        self.state.rules = rules;
    }

    pub fn set_mode(&mut self, mode: Mode) -> Result<(), LiveNotAllowed> {
        if mode == Mode::Live && !self.state.live_allowed {
            return Err(LiveNotAllowed);
        }
        self.state.mode = mode;
        Ok(())
    }

    pub fn set_enabled(&mut self, enabled: bool, now: u64) {
        if enabled == self.state.enabled {
            return;
        }
        self.state.enabled = enabled;
        self.no_client_since = None;
        if enabled {
            self.state.consecutive = 0;
            let how = if self.state.mode == Mode::Live { "reale" } else { "in prova a vuoto" };
            self.push(now, DecisionKind::Info, None, None, format!("Automazione accesa, modo {how}"));
        } else {
            self.active = None;
            self.push(now, DecisionKind::Paused, None, None, "Automazione spenta dall'operatore".into());
        }
    }

    /// STOP TX: spegne tutto, da qualunque via sia arrivato.
    pub fn on_halt(&mut self, now: u64) {
        if !self.state.enabled {
            return;
        }
        self.state.enabled = false;
        self.active = None;
        self.push(now, DecisionKind::Halted, None, None, "STOP TX: automazione spenta".into());
    }

    pub fn on_qso_logged(&mut self, call: &str) {
        let call = call.trim().to_uppercase();
        self.state.consecutive = 0;
        self.attempts.remove(&call);
        if self.active.as_ref().is_some_and(|(c, _)| *c == call) {
            self.active = None;
        }
    }

    /// Da chiamare circa una volta al secondo. `clients` = quanti client sono collegati ora.
    pub fn tick(&mut self, now: u64, clients: usize) {
        if !self.state.enabled || self.state.limits.run_unattended {
            return;
        }
        if clients > 0 {
            self.no_client_since = None;
            return;
        }
        let since = *self.no_client_since.get_or_insert(now);
        if now.saturating_sub(since) >= self.state.limits.dead_man_secs {
            self.state.enabled = false;
            self.active = None;
            self.no_client_since = None;
            let reason = format!("Nessun dispositivo collegato da {} s: automazione spenta", self.state.limits.dead_man_secs);
            self.push(now, DecisionKind::Paused, None, None, reason);
        }
    }

    pub fn on_decode(&mut self, d: &DecodedMessage, now: u64) -> Option<Decision> {
        let parsed = parse_message(&d.text);
        // chi e' gia' stato sentito si ricorda anche a automazione spenta: serve a giudicare i segnali deboli
        let heard_before = parsed.sender.as_ref().is_some_and(|c| self.heard.contains(c));
        if let Some(sender) = &parsed.sender {
            if self.heard.len() >= HEARD_CAPACITY {
                self.heard.clear();
            }
            self.heard.insert(sender.clone());
        }
        if !self.state.enabled {
            return None;
        }
        self.roll_day(now);
        let tag_wanted = d.tag.is_some_and(|t| self.state.rules.reply_to.contains(&t));
        let list_reason = if self.state.rules.reply_to_wanted && parsed.is_cq { self.wanted.as_ref().and_then(|m| m(d, self.dial_hz)) } else { None };
        if !(tag_wanted || list_reason.is_some()) || !parsed.is_cq {
            return None;
        }
        let Some(call) = parsed.sender else {
            return Some(self.push(now, DecisionKind::Skipped, None, Some(d.id), "Nominativo non leggibile nel messaggio".into()));
        };
        let call_opt = Some(call.clone());
        if let Some(why) = ghost::suspicion(&d.text, &call, d.snr, heard_before) {
            return Some(self.push(now, DecisionKind::Skipped, call_opt, Some(d.id), format!("{call}: {why}")));
        }
        if d.snr < self.state.rules.min_snr {
            let reason = format!("{call}: segnale {} dB sotto il minimo di {} dB", d.snr, self.state.rules.min_snr);
            return Some(self.push(now, DecisionKind::Skipped, call_opt, Some(d.id), reason));
        }
        if self.state.rules.excluded_calls.iter().any(|c| c.trim().eq_ignore_ascii_case(&call)) {
            return Some(self.push(now, DecisionKind::Skipped, call_opt, Some(d.id), format!("{call} e' nell'elenco delle esclusioni")));
        }
        if self.attempts.get(&call).copied().unwrap_or(0) >= self.state.rules.max_attempts_per_call {
            return Some(self.push(now, DecisionKind::Skipped, call_opt, Some(d.id), format!("{call}: tentativi esauriti senza QSO")));
        }
        if let Some((busy, since)) = &self.active
            && now.saturating_sub(*since) < ACTIVE_QSO_SECS
        {
            let reason = format!("{call}: QSO gia' in corso con {busy}");
            return Some(self.push(now, DecisionKind::Skipped, call_opt, Some(d.id), reason));
        }
        let live = self.state.mode == Mode::Live;
        if live {
            if self.state.consecutive >= self.state.limits.max_consecutive_cycles {
                self.state.enabled = false;
                self.active = None;
                let reason = format!("Raggiunte {} risposte di fila senza QSO: automazione spenta", self.state.limits.max_consecutive_cycles);
                return Some(self.push(now, DecisionKind::Blocked, call_opt, Some(d.id), reason));
            }
            if self.state.replies_today >= self.state.limits.max_per_day {
                let reason = format!("Raggiunto il tetto di {} risposte al giorno", self.state.limits.max_per_day);
                return Some(self.push(now, DecisionKind::Blocked, call_opt, Some(d.id), reason));
            }
            self.state.consecutive += 1;
            self.state.replies_today += 1;
        }
        *self.attempts.entry(call.clone()).or_insert(0) += 1;
        self.active = Some((call.clone(), now));
        let why = match (&list_reason, d.tag) {
            (Some(reason), _) if !tag_wanted => format!("nella lista dei desiderati ({reason})"),
            (_, Some(tag)) => tag_text(tag).to_string(),
            _ => "CQ di interesse".to_string(),
        };
        let (kind, reason) = if live {
            (DecisionKind::Reply, format!("Rispondo a {call}: {why}"))
        } else {
            (DecisionKind::WouldReply, format!("Risponderei a {call}: {why} (prova a vuoto)"))
        };
        Some(self.push(now, kind, call_opt, Some(d.id), reason))
    }

    fn roll_day(&mut self, now: u64) {
        let today = now / DAY_SECS;
        if today != self.day {
            self.day = today;
            self.state.replies_today = 0;
        }
    }

    fn push(&mut self, at: u64, kind: DecisionKind, call: Option<String>, decode_id: Option<u64>, reason: String) -> Decision {
        let decision = Decision { at, kind, call, decode_id, reason };
        self.state.log.push(decision.clone());
        if self.state.log.len() > LOG_CAPACITY {
            self.state.log.remove(0);
        }
        decision
    }
}

fn tag_text(tag: DecodeTag) -> &'static str {
    match tag {
        DecodeTag::NewDxcc => "paese nuovo",
        DecodeTag::NewDxccBand => "paese nuovo su questa banda",
        DecodeTag::NewZone => "zona nuova",
        DecodeTag::NewGrid => "griglia nuova",
        _ => "CQ",
    }
}
