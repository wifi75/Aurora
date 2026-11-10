//! Il cervello dell'automazione: decide, non trasmette. Il tempo (secondi UTC) lo passa il chiamante,
//! cosi' ogni salvaguardia si prova senza orologio.

use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::Arc,
};

use aurora_core::{DecodeTag, DecodedMessage, SharedUdpFilters, StationStatus};
use engine_jtdx::text::parse_message;
use serde::{Deserialize, Serialize};

use crate::ghost;

const LOG_CAPACITY: usize = 100;
/// Quante stazioni sentite si ricordano al massimo (poi si riparte da zero).
const HEARD_CAPACITY: usize = 5_000;
/// Dopo una risposta, per tanto tempo nessun'altra (a meno che il QSO non venga registrato prima).
const ACTIVE_QSO_SECS: u64 = 60;
const DAY_SECS: u64 = 86_400;
const HOUR_SECS: u64 = 3_600;
/// Lo stesso blocco generale (limite orario) si scrive nel registro al massimo una volta ogni tanto, non a ogni ciclo.
const BLOCK_REPEAT_SECS: u64 = 300;
/// Dopo aver chiamato CQ, per tanto tempo il CQ si considera «nostro» anche se la radio non risulta ancora in trasmissione.
const CQ_GRACE_SECS: u64 = 30;
/// I continenti ammessi nel filtro «solo certi continenti».
pub const CONTINENTS: [&str; 7] = ["EU", "NA", "SA", "AF", "AS", "OC", "AN"];

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
    /// Quando in un ciclo ci sono piu' CQ interessanti, a quale rispondere.
    #[serde(default)]
    pub prefer: Prefer,
    /// Dopo quanti secondi i tentativi falliti verso una stazione si dimenticano e si puo' riprovare (banda affollata: una stazione
    /// che non risponde oggi puo' rispondere tra un quarto d'ora).
    #[serde(default = "default_cooldown_secs")]
    pub cooldown_secs: u64,
    /// Rispetta i CQ direzionali: «CQ NA» solo dal Nord America, «CQ DX» solo da fuori dal proprio continente.
    #[serde(default = "default_true")]
    pub respect_cq_direction: bool,
    /// Risponde anche ai paesi mai collegati nel modo in uso (FT8, FT4...): lo stesso calcolo di «NEW MODE» della scheda DXCC.
    #[serde(default)]
    pub reply_to_new_mode: bool,
    /// Al massimo tanti QSO in un'ora (0 = senza limite). In prova a vuoto contano le risposte che avrebbe dato.
    #[serde(default)]
    pub max_qso_per_hour: u32,
    /// Agisce solo nella fascia oraria qui sotto (ora locale del PC); spenta = a qualunque ora.
    #[serde(default)]
    pub window_enabled: bool,
    /// Inizio della fascia, minuti dalla mezzanotte locale (0-1439).
    #[serde(default = "default_window_from")]
    pub window_from_min: u32,
    /// Fine della fascia, minuti dalla mezzanotte locale (0-1439); se e' prima dell'inizio la fascia passa la mezzanotte.
    #[serde(default = "default_window_to")]
    pub window_to_min: u32,
    /// Paesi (nomi del file cty.dat) a cui non rispondere mai.
    #[serde(default)]
    pub excluded_countries: Vec<String>,
    /// Solo i CQ da questi continenti (EU, NA, SA, AF, AS, OC, AN); vuoto = tutti.
    #[serde(default)]
    pub continents: Vec<String>,
    /// I continenti scelti sono da escludere invece che i soli ammessi.
    #[serde(default)]
    pub continents_exclude: bool,
    /// Anche ad Automazioni spente (CQ a mano): non rispondere a chi i filtri di Attivita' di banda e le esclusioni nascondono.
    #[serde(default)]
    pub filter_callers_always: bool,
    /// Risponde ai CQ di stazioni USA di uno stato mai lavorato («nuovo stato»).
    #[serde(default)]
    pub reply_to_new_state: bool,
    /// Risponde ai CQ di stazioni USA di uno stato non ancora lavorato su questa banda.
    #[serde(default)]
    pub reply_to_new_state_band: bool,
    /// Dopo tanti minuti senza nessun CQ a cui rispondere Aurora chiama CQ da sola (0 = mai). Trasmette solo in modo reale.
    #[serde(default)]
    pub auto_cq_after_min: u32,
}

fn default_cooldown_secs() -> u64 {
    900
}

fn default_window_from() -> u32 {
    8 * 60
}

fn default_window_to() -> u32 {
    22 * 60
}

fn default_true() -> bool {
    true
}

/// Come scegliere tra piu' CQ interessanti arrivati nello stesso ciclo.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Prefer {
    /// La novita' piu' rara (lista dei desiderati, paese nuovo, nuovo su banda, zona, griglia); a parita' il segnale piu' forte.
    #[default]
    Rarity,
    /// Il segnale piu' forte; a parita' la novita' piu' rara.
    Snr,
    /// Un punteggio che pesa la novita' (10 per ogni livello di rarita') e il segnale (da 0 a 18 punti tra -24 e +12 dB).
    Score,
}

/// Dice se un messaggio e' di una stazione desiderata (e perche'), data la frequenza della radio.
pub type WantedMatcher = Arc<dyn Fn(&DecodedMessage, Option<u64>) -> Option<String> + Send + Sync>;

/// Dice se il paese di un nominativo non e' mai stato collegato nel modo indicato (`FT8`, `FT4`...).
pub type NewModeMatcher = Arc<dyn Fn(&str, Option<&str>) -> bool + Send + Sync>;

/// Lo stato USA di chi trasmette e se e' nuovo: `Some((stato, true))` mai lavorato, `Some((stato, false))` nuovo su questa banda,
/// `None` se non e' una stazione USA, lo stato non si sa o e' gia' lavorato sulla banda. Riceve il messaggio, il nominativo e la frequenza.
pub type NewStateMatcher = Arc<dyn Fn(&DecodedMessage, &str, Option<u64>) -> Option<(String, bool)> + Send + Sync>;

impl Default for Rules {
    fn default() -> Self {
        Rules {
            reply_to: vec![DecodeTag::NewDxcc, DecodeTag::NewDxccBand, DecodeTag::NewZone, DecodeTag::NewGrid],
            min_snr: -18,
            excluded_calls: Vec::new(),
            max_attempts_per_call: 2,
            reply_to_wanted: false,
            prefer: Prefer::Rarity,
            cooldown_secs: default_cooldown_secs(),
            respect_cq_direction: true,
            reply_to_new_mode: false,
            max_qso_per_hour: 0,
            window_enabled: false,
            window_from_min: default_window_from(),
            window_to_min: default_window_to(),
            excluded_countries: Vec::new(),
            continents: Vec::new(),
            continents_exclude: false,
            filter_callers_always: false,
            reply_to_new_state: false,
            reply_to_new_state_band: false,
            auto_cq_after_min: 0,
        }
    }
}

impl Rules {
    /// Regole sensate: niente CQ comuni (si risponderebbe a tutti), soglie in un intervallo plausibile.
    pub fn is_valid(&self) -> bool {
        // senza etichette si risponde solo ai desiderati (o ai modi nuovi); nessuna delle tre vorrebbe dire non rispondere mai
        let tags_ok = (!self.reply_to.is_empty() || self.reply_to_wanted || self.reply_to_new_mode || self.reply_to_new_state || self.reply_to_new_state_band)
            && self
                .reply_to
                .iter()
                // anche griglia su nuova banda e nominativo nuovo o su nuova banda: la pagina le offriva ma qui mancavano, e le regole con una
                // di loro venivano rifiutate in blocco (Luc ON7KEC, 2026-11-10: «non si possono salvare»)
                .all(|t| {
                    matches!(
                        t,
                        DecodeTag::NewDxcc | DecodeTag::NewDxccBand | DecodeTag::NewZone | DecodeTag::NewGrid | DecodeTag::NewGridBand | DecodeTag::NewCall | DecodeTag::NewCallBand
                    )
                });
        let calls_ok = self.excluded_calls.len() <= 200
            && self
                .excluded_calls
                .iter()
                .all(|c| (3..=13).contains(&c.trim().len()) && c.trim().chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '/'));
        let countries_ok = self.excluded_countries.len() <= 400 && self.excluded_countries.iter().all(|c| (1..=64).contains(&c.trim().len()));
        let continents_ok = self.continents.len() <= CONTINENTS.len() && self.continents.iter().all(|c| CONTINENTS.contains(&c.trim().to_uppercase().as_str()));
        let window_ok = self.window_from_min < 1440 && self.window_to_min < 1440;
        // il CQ da solo: o spento, o almeno due minuti (non si chiama CQ a raffica)
        let cq_ok = self.auto_cq_after_min == 0 || (2..=120).contains(&self.auto_cq_after_min);
        tags_ok && calls_ok && countries_ok && continents_ok && window_ok && cq_ok
            && (-30..=20).contains(&self.min_snr)
            && (1..=10).contains(&self.max_attempts_per_call)
            && (self.cooldown_secs == 0 || (60..=7200).contains(&self.cooldown_secs))
            && self.max_qso_per_hour <= 120
    }

    /// La fascia oraria come «HH:MM-HH:MM», per il registro.
    fn window_text(&self) -> String {
        let hm = |m: u32| format!("{:02}:{:02}", m / 60, m % 60);
        format!("{}-{}", hm(self.window_from_min), hm(self.window_to_min))
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
    /// Nessun CQ a cui rispondere da tempo: Aurora chiama CQ (modo reale).
    CallCq,
    /// Come `CallCq`, in prova a vuoto: non trasmette.
    WouldCallCq,
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
    /// Risposte che avrebbe dato oggi in prova a vuoto.
    pub would_replies_today: u32,
    /// QSO registrati oggi con stazioni chiamate dall'automazione o arrivate al suo CQ (modo reale).
    pub qsos_today: u32,
    /// QSO contati nell'ultima ora per il limite orario (in prova a vuoto: le stazioni a cui avrebbe risposto).
    pub qsos_last_hour: u32,
    /// Falso quando la fascia oraria e' accesa e adesso si e' fuori: l'automazione aspetta.
    pub in_window: bool,
    /// Le ultime decisioni, le piu' vecchie per prime.
    pub log: Vec<Decision>,
    /// Le prossime stazioni: i CQ interessanti dell'ultimo ciclo non scelti, dal migliore secondo la preferenza (idea di ON7KEC, issue #6).
    pub queue: Vec<QueueEntry>,
}

/// Una stazione in coda: la si chiamerebbe se ricomincia il CQ quando l'automazione e' libera.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct QueueEntry {
    pub call: String,
    pub snr: i32,
    pub why: String,
}

pub struct Brain {
    state: BrainState,
    day: u64,
    /// Istanti (secondi UTC) dei QSO contati per il limite orario.
    hour_marks: VecDeque<u64>,
    /// Scarto dell'ora locale del PC da UTC, in secondi: serve alla fascia oraria.
    utc_offset_secs: i64,
    /// Il modo della radio (`FT8`, `FT4`...), per «nuovo modo».
    radio_mode: Option<String>,
    new_mode: Option<NewModeMatcher>,
    new_state: Option<NewStateMatcher>,
    /// La trasmissione del motore e' accesa (CQ, QSO in corso): il CQ da solo non parte.
    engine_tx: bool,
    /// Da quando non c'e' nessun CQ a cui rispondere (secondi UTC): il CQ da solo parte dopo `auto_cq_after_min`.
    idle_since: u64,
    /// Quando l'automazione ha chiamato CQ l'ultima volta, finche' quel CQ e' in onda.
    cq_since: Option<u64>,
    /// L'ultimo blocco generale scritto nel registro, per non ripeterlo a ogni ciclo.
    last_block: Option<(String, u64)>,
    /// L'ultimo istante visto (secondi UTC), per chi non lo passa.
    clock: u64,
    /// L'automazione si e' spenta da sola (o dall'operatore) con un CQ suo in onda: chi la fa girare deve fermare la trasmissione.
    halt_request: bool,
    /// Tentativi per stazione con l'istante dell'ultimo (secondi UTC): scadono dopo `cooldown_secs`.
    attempts: HashMap<String, (u32, u64)>,
    own_continent: Option<String>,
    active: Option<(String, u64)>,
    no_client_since: Option<u64>,
    /// Stazioni gia' sentite in questa sessione: un segnale debole di una mai sentita e' sospetto.
    heard: HashSet<String>,
    dial_hz: Option<u64>,
    wanted: Option<WantedMatcher>,
    /// I filtri di Attivita' di banda: una stazione che nascondono non riceve mai una risposta automatica (vale anche in prova a vuoto).
    band_filters: Option<SharedUdpFilters>,
    /// Com'era «Abilita Tx» quando le automazioni hanno cominciato a trasmettere (modo reale acceso): `Some` finche' trasmettono,
    /// poi si rimette com'era.
    tx_before: Option<bool>,
    /// Da quando (secondi UTC) «Abilita Tx» risulta spento mentre le automazioni trasmettono: si riaccende solo dopo `TX_OFF_GRACE_SECS`,
    /// cosi' uno STOP dell'operatore arriva prima al cervello (`on_halt`) e non viene scavalcato.
    tx_off_since: Option<u64>,
    /// Chiamanti del nostro CQ scartati dal motore per le regole: l'ultimo istante annotato nel registro, per non ripeterlo a ogni periodo.
    refused_callers: HashMap<String, u64>,
}

/// Quanto «Abilita Tx» deve restare spento (con le automazioni che trasmettono) prima che lo riaccendano.
const TX_OFF_GRACE_SECS: u64 = 2;
/// Ogni quanto si ripete nel registro lo scarto dello stesso chiamante.
const REFUSED_LOG_SECS: u64 = 600;

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
                would_replies_today: 0,
                qsos_today: 0,
                qsos_last_hour: 0,
                in_window: true,
                log: Vec::new(),
                queue: Vec::new(),
            },
            day: 0,
            hour_marks: VecDeque::new(),
            utc_offset_secs: 0,
            radio_mode: None,
            new_mode: None,
            new_state: None,
            engine_tx: false,
            idle_since: 0,
            cq_since: None,
            last_block: None,
            clock: 0,
            halt_request: false,
            attempts: HashMap::new(),
            own_continent: None,
            active: None,
            no_client_since: None,
            heard: HashSet::new(),
            dial_hz: None,
            wanted: None,
            band_filters: None,
            tx_before: None,
            tx_off_since: None,
            refused_callers: HashMap::new(),
        }
    }

    /// Come riconoscere le stazioni desiderate (la lista sta fuori dal cervello).
    pub fn set_wanted_matcher(&mut self, matcher: WantedMatcher) {
        self.wanted = Some(matcher);
    }

    /// I filtri di Attivita' di banda (li manda la pagina, li tiene chi fa girare il cervello).
    pub fn set_band_filters(&mut self, filters: SharedUdpFilters) {
        self.band_filters = Some(filters);
    }

    /// La stazione e' nascosta dai filtri di Attivita' di banda?
    fn hidden_by_band_filters(&self, d: &DecodedMessage) -> bool {
        self.band_filters.as_ref().is_some_and(|f| !f.read().unwrap_or_else(|p| p.into_inner()).allows(d, self.own_continent.as_deref()))
    }

    /// Le automazioni possono trasmettere adesso (accese, in modo reale)?
    fn transmits(&self) -> bool {
        self.state.enabled && self.state.mode == Mode::Live
    }

    /// Vero se «Abilita Tx» va sistemato: le automazioni hanno cominciato o smesso di trasmettere.
    pub fn tx_switch_pending(&self) -> bool {
        // mentre trasmettono si controlla sempre: il motore puo' averlo spento a fine QSO
        self.transmits() || self.tx_before.is_some()
    }

    /// Le automazioni sono accese in modo reale (il motore lo usa per non spegnere «Abilita Tx» e per filtrare chi ci chiama).
    pub fn transmitting(&self) -> bool {
        self.transmits()
    }

    /// «Abilita Tx» segue le automazioni: quando cominciano a trasmettere (accese in modo reale) si accende, quando smettono (spente a mano,
    /// per un tetto, dal dead-man, da STOP o passate in prova a vuoto) torna com'era prima. `current` e' com'e' adesso (`None` = non si sa:
    /// vale spento). Ritorna il valore da impostare, se va cambiato.
    pub fn tx_switch_step(&mut self, current: Option<bool>, now: u64) -> Option<bool> {
        match (self.transmits(), self.tx_before) {
            (true, None) => {
                let before = current.unwrap_or(false);
                self.tx_before = Some(before);
                if before {
                    return None;
                }
                self.push(now, DecisionKind::Info, None, None, "Abilita Tx acceso dalle automazioni (si rispegne quando le automazioni si fermano)".into());
                Some(true)
            }
            (true, Some(_)) => {
                if current != Some(false) {
                    self.tx_off_since = None;
                    return None;
                }
                let since = *self.tx_off_since.get_or_insert(now);
                if now.saturating_sub(since) < TX_OFF_GRACE_SECS {
                    return None;
                }
                self.tx_off_since = None;
                self.push(now, DecisionKind::Info, None, None, "Abilita Tx riacceso dalle automazioni (era stato spento a fine QSO)".into());
                Some(true)
            }
            (false, Some(before)) => {
                self.tx_before = None;
                self.tx_off_since = None;
                if before {
                    return None;
                }
                self.push(now, DecisionKind::Info, None, None, "Abilita Tx rimesso spento, com'era prima delle automazioni".into());
                Some(false)
            }
            _ => None,
        }
    }

    /// La frequenza della radio, per i filtri di banda della lista dei desiderati.
    /// Il continente di chi opera (AF, AN, AS, EU, NA, OC, SA): serve a rispettare i CQ direzionali.
    pub fn set_own_continent(&mut self, continent: Option<String>) {
        self.own_continent = continent.map(|c| c.trim().to_uppercase()).filter(|c| !c.is_empty());
    }

    pub fn on_dial(&mut self, hz: u64) {
        self.dial_hz = (hz > 0).then_some(hz);
    }

    /// Come sapere se un paese e' nuovo nel modo in uso (il log sta fuori dal cervello).
    pub fn set_new_mode_matcher(&mut self, matcher: NewModeMatcher) {
        self.new_mode = Some(matcher);
    }

    /// Come sapere se lo stato USA di una stazione e' nuovo (ricerche dei nominativi e log stanno fuori dal cervello).
    pub fn set_new_state_matcher(&mut self, matcher: NewStateMatcher) {
        self.new_state = Some(matcher);
    }

    /// Scarto dell'ora locale del PC da UTC, in secondi (+3600 = ora italiana d'inverno).
    pub fn set_utc_offset(&mut self, secs: i64) {
        self.utc_offset_secs = secs.clamp(-14 * 3600, 14 * 3600);
    }

    /// Lo stato del motore: frequenza, modo e se la trasmissione e' accesa (il CQ da solo parte solo a trasmissione spenta).
    pub fn on_status(&mut self, s: &StationStatus) {
        self.on_dial(s.dial_hz);
        let mode = s.mode.trim().to_uppercase();
        self.radio_mode = (!mode.is_empty()).then_some(mode);
        self.engine_tx = s.tx_enabled || s.transmitting;
        // il nostro CQ e' finito (fermato dal watchdog, da STOP o da un QSO concluso): da qui non e' piu' nostro
        if !self.engine_tx
            && let Some(since) = self.cq_since
            && self.clock.saturating_sub(since) > CQ_GRACE_SECS
        {
            self.cq_since = None;
        }
    }

    /// Vero una volta sola dopo che l'automazione si e' spenta con un suo CQ in onda: va mandato lo STOP della trasmissione.
    pub fn take_halt_request(&mut self) -> bool {
        std::mem::take(&mut self.halt_request)
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
        self.clock = now;
        self.state.enabled = enabled;
        self.no_client_since = None;
        if enabled {
            self.state.consecutive = 0;
            self.idle_since = now;
            self.last_block = None;
            // la fascia oraria si guarda subito: fuori fascia lo dice il registro al primo controllo
            self.state.in_window = true;
            let how = if self.state.mode == Mode::Live { "reale" } else { "in prova a vuoto" };
            self.push(now, DecisionKind::Info, None, None, format!("Automazione accesa, modo {how}"));
            self.check_window(now);
        } else {
            self.switch_off();
            self.push(now, DecisionKind::Paused, None, None, "Automazione spenta dall'operatore".into());
        }
    }

    /// Spegne e, se un CQ chiamato dall'automazione e' ancora in onda, chiede di fermare la trasmissione.
    fn switch_off(&mut self) {
        self.state.enabled = false;
        self.active = None;
        self.no_client_since = None;
        if self.cq_since.take().is_some() {
            self.halt_request = true;
        }
    }

    /// STOP TX: spegne tutto, da qualunque via sia arrivato.
    pub fn on_halt(&mut self, now: u64) {
        // lo STOP ferma gia' la trasmissione: il CQ non e' piu' in onda
        self.cq_since = None;
        if !self.state.enabled {
            return;
        }
        self.clock = now;
        self.switch_off();
        self.push(now, DecisionKind::Halted, None, None, "STOP TX: automazione spenta".into());
    }

    /// Un QSO registrato, all'ultimo istante noto.
    pub fn on_qso_logged(&mut self, call: &str) {
        let now = self.clock;
        self.on_qso_logged_at(call, now);
    }

    /// Un QSO registrato: libera la risposta successiva e, se e' dell'automazione (stazione chiamata da lei o arrivata al suo CQ),
    /// conta per i QSO del giorno e per il limite orario.
    pub fn on_qso_logged_at(&mut self, call: &str, now: u64) {
        self.clock = now.max(self.clock);
        let call = call.trim().to_uppercase();
        self.roll_day(now);
        let ours = self.attempts.contains_key(&call) || self.active.as_ref().is_some_and(|(c, _)| *c == call) || self.cq_since.is_some();
        if self.state.enabled && self.state.mode == Mode::Live && ours {
            self.state.qsos_today += 1;
            self.hour_marks.push_back(now);
            self.prune_hour(now);
        }
        self.state.consecutive = 0;
        self.idle_since = now;
        self.attempts.remove(&call);
        if self.active.as_ref().is_some_and(|(c, _)| *c == call) {
            self.active = None;
        }
    }

    /// Da chiamare circa una volta al secondo. `clients` = quanti client sono collegati ora. Ritorna la decisione di chiamare CQ
    /// da sola, se c'e' (in modo reale `CallCq`: chi la fa girare manda il CQ al motore).
    pub fn tick(&mut self, now: u64, clients: usize) -> Option<Decision> {
        self.clock = now;
        self.prune_hour(now);
        if !self.state.enabled {
            return None;
        }
        self.roll_day(now);
        if !self.state.limits.run_unattended {
            if clients > 0 {
                self.no_client_since = None;
            } else {
                let since = *self.no_client_since.get_or_insert(now);
                if now.saturating_sub(since) >= self.state.limits.dead_man_secs {
                    self.switch_off();
                    let reason = format!("Nessun dispositivo collegato da {} s: automazione spenta", self.state.limits.dead_man_secs);
                    self.push(now, DecisionKind::Paused, None, None, reason);
                    return None;
                }
            }
        }
        self.check_window(now);
        self.maybe_call_cq(now)
    }

    /// Minuti dalla mezzanotte locale.
    fn local_minute(&self, now: u64) -> u32 {
        let local = i64::try_from(now).unwrap_or(i64::MAX).saturating_add(self.utc_offset_secs);
        u32::try_from(local.rem_euclid(DAY_SECS as i64) / 60).unwrap_or(0)
    }

    /// Siamo dentro la fascia oraria? Spenta, o con inizio uguale alla fine, vale sempre; se la fine e' prima dell'inizio passa la mezzanotte.
    fn inside_window(&self, now: u64) -> bool {
        let r = &self.state.rules;
        if !r.window_enabled || r.window_from_min == r.window_to_min {
            return true;
        }
        let m = self.local_minute(now);
        if r.window_from_min < r.window_to_min {
            (r.window_from_min..r.window_to_min).contains(&m)
        } else {
            m >= r.window_from_min || m < r.window_to_min
        }
    }

    /// Il registro dice quando si esce dalla fascia oraria e quando si rientra (una volta sola, non a ogni ciclo).
    fn check_window(&mut self, now: u64) {
        let inside = self.inside_window(now);
        if inside == self.state.in_window {
            return;
        }
        self.state.in_window = inside;
        if inside {
            self.idle_since = now;
            self.push(now, DecisionKind::Info, None, None, "Di nuovo nella fascia oraria: l'automazione riprende".into());
        } else {
            let reason = format!("Fuori dalla fascia oraria {} (ora locale): automazione in attesa", self.state.rules.window_text());
            self.push(now, DecisionKind::Paused, None, None, reason);
        }
    }

    /// Toglie i QSO piu' vecchi di un'ora dal conto del limite orario.
    fn prune_hour(&mut self, now: u64) {
        while self.hour_marks.front().is_some_and(|&t| now.saturating_sub(t) >= HOUR_SECS) {
            self.hour_marks.pop_front();
        }
        self.state.qsos_last_hour = u32::try_from(self.hour_marks.len()).unwrap_or(u32::MAX);
    }

    /// Il limite di QSO all'ora e' raggiunto?
    fn hour_full(&mut self, now: u64) -> bool {
        self.prune_hour(now);
        let max = self.state.rules.max_qso_per_hour;
        max > 0 && self.state.qsos_last_hour >= max
    }

    /// Un blocco generale (non legato a una stazione) si scrive una volta ogni `BLOCK_REPEAT_SECS`, non a ogni ciclo.
    fn push_block(&mut self, now: u64, call: Option<String>, decode_id: Option<u64>, reason: String) -> Decision {
        let repeated = self.last_block.as_ref().is_some_and(|(r, at)| *r == reason && now.saturating_sub(*at) < BLOCK_REPEAT_SECS);
        if repeated {
            return Decision { at: now, kind: DecisionKind::Blocked, call, decode_id, reason };
        }
        self.last_block = Some((reason.clone(), now));
        self.push(now, DecisionKind::Blocked, call, decode_id, reason)
    }

    fn hour_block_reason(&self) -> String {
        format!("Raggiunto il limite di {} QSO all'ora", self.state.rules.max_qso_per_hour)
    }

    /// Il CQ da solo: dopo `auto_cq_after_min` minuti senza nessun CQ a cui rispondere, se la trasmissione del motore e' spenta e
    /// tutti i limiti lo permettono. In modo reale conta come una risposta di fila senza QSO (tetto e spegnimento valgono anche qui).
    fn maybe_call_cq(&mut self, now: u64) -> Option<Decision> {
        let after = self.state.rules.auto_cq_after_min;
        if after == 0 || !self.state.in_window {
            return None;
        }
        if now.saturating_sub(self.idle_since) < u64::from(after) * 60 {
            return None;
        }
        // il conto riparte comunque: il prossimo tentativo tra altri `after` minuti
        self.idle_since = now;
        // la radio sta gia' trasmettendo (un CQ, un QSO dell'operatore o del motore): non si interrompe nulla
        if self.engine_tx {
            return None;
        }
        if let Some((_, since)) = &self.active
            && now.saturating_sub(*since) < ACTIVE_QSO_SECS
        {
            return None;
        }
        if self.hour_full(now) {
            let reason = self.hour_block_reason();
            return Some(self.push_block(now, None, None, reason));
        }
        let live = self.state.mode == Mode::Live;
        if live {
            if self.state.consecutive >= self.state.limits.max_consecutive_cycles {
                self.switch_off();
                let reason = format!("Raggiunte {} risposte di fila senza QSO: automazione spenta", self.state.limits.max_consecutive_cycles);
                return Some(self.push(now, DecisionKind::Blocked, None, None, reason));
            }
            if self.state.replies_today >= self.state.limits.max_per_day {
                let reason = format!("Raggiunto il tetto di {} risposte al giorno", self.state.limits.max_per_day);
                return Some(self.push_block(now, None, None, reason));
            }
            self.state.consecutive += 1;
        }
        // in prova a vuoto non parte nulla: niente CQ «nostro» da fermare poi
        if live {
            self.cq_since = Some(now);
        }
        let (kind, reason) = if live {
            (DecisionKind::CallCq, format!("Nessun CQ a cui rispondere da {after} min: chiamo CQ"))
        } else {
            (DecisionKind::WouldCallCq, format!("Nessun CQ a cui rispondere da {after} min: chiamerei CQ (prova a vuoto)"))
        };
        Some(self.push(now, kind, None, None, reason))
    }

    /// Un solo messaggio: come un gruppo di uno.
    pub fn on_decode(&mut self, d: &DecodedMessage, now: u64) -> Option<Decision> {
        self.on_batch(std::slice::from_ref(d), now).pop()
    }

    /// I messaggi arrivati insieme a fine ciclo: si valutano tutti e si risponde **a uno solo**, il migliore secondo la preferenza
    /// delle regole (la novita' piu' rara, o il segnale piu' forte). Le decisioni sono quelle da registrare, in ordine; la risposta,
    /// se c'e', e' l'unica di tipo `Reply` o `WouldReply`.
    pub fn on_batch(&mut self, decodes: &[DecodedMessage], now: u64) -> Vec<Decision> {
        let mut decisions = Vec::new();
        let mut candidates: Vec<Candidate> = Vec::new();
        for d in decodes {
            match self.screen(d, now) {
                Screened::Ignore => {}
                Screened::Skipped(decision) => decisions.push(decision),
                Screened::Candidate(c) => candidates.push(c),
            }
        }
        if candidates.is_empty() {
            self.state.queue.clear();
            return decisions;
        }
        // c'e' qualcuno a cui rispondere: il conto del CQ da solo riparte
        self.idle_since = now;
        let prefer = self.state.rules.prefer;
        // il migliore: meglio chi ha rango o segnale piu' alti; a parita', il primo arrivato
        let mut best = 0;
        for (i, c) in candidates.iter().enumerate().skip(1) {
            if c.better_than(&candidates[best], prefer) {
                best = i;
            }
        }
        let chosen = candidates.swap_remove(best);
        // la coda: gli altri, dal migliore
        let mut rest = candidates.clone();
        rest.sort_by(|a, b| if a.better_than(b, prefer) { std::cmp::Ordering::Less } else if b.better_than(a, prefer) { std::cmp::Ordering::Greater } else { std::cmp::Ordering::Equal });
        self.state.queue = rest.into_iter().take(10).map(|c| QueueEntry { call: c.call, snr: c.snr, why: c.why }).collect();
        if !candidates.is_empty() {
            let mut others: Vec<String> = candidates.iter().map(|c| c.call.clone()).collect();
            others.sort();
            let reason = format!("Tra {} CQ interessanti scelto {} (altri: {})", candidates.len() + 1, chosen.call, others.join(", "));
            decisions.push(self.push(now, DecisionKind::Info, Some(chosen.call.clone()), Some(chosen.decode_id), reason));
        }
        decisions.push(self.commit(chosen, now));
        decisions
    }

    /// Nominativi esclusi, paesi esclusi e continenti scelti: il motivo se la stazione non va lavorata.
    fn excluded_by_rules(&self, d: &DecodedMessage, call: &str) -> Option<String> {
        if self.state.rules.excluded_calls.iter().any(|c| c.trim().eq_ignore_ascii_case(call)) {
            return Some(format!("{call} e' nell'elenco delle esclusioni"));
        }
        if let Some(country) = d.country.as_deref()
            && self.state.rules.excluded_countries.iter().any(|c| c.trim().eq_ignore_ascii_case(country.trim()))
        {
            return Some(format!("{call}: {country} e' tra i paesi esclusi"));
        }
        if !self.state.rules.continents.is_empty() {
            let allowed = |c: &str| self.state.rules.continents.iter().any(|x| x.trim().eq_ignore_ascii_case(c.trim()));
            if self.state.rules.continents_exclude {
                return d.continent.as_deref().filter(|c| allowed(c)).map(|c| format!("{call}: continente {} escluso", c.trim().to_uppercase()));
            }
            return match d.continent.as_deref() {
                Some(c) if allowed(c) => None,
                Some(c) => Some(format!("{call}: continente {} non tra quelli scelti", c.trim().to_uppercase())),
                // senza continente noto non si puo' dire che vada bene: con il filtro acceso si salta
                None => Some(format!("{call}: continente sconosciuto")),
            };
        }
        None
    }

    /// Chi risponde al nostro CQ (anche quello automatico) e' escluso dalle regole o nascosto dai filtri di Attivita' di banda? Vale solo con le
    /// automazioni accese in modo reale; il motivo resta nel registro delle decisioni (una volta ogni dieci minuti per stazione).
    pub fn refuse_caller(&mut self, d: &DecodedMessage, call: &str, now: u64) -> Option<String> {
        if !self.transmits() && !self.state.rules.filter_callers_always {
            return None;
        }
        let hidden = self.band_filters.as_ref().is_some_and(|f| !f.read().unwrap_or_else(|p| p.into_inner()).allows_station(d, self.own_continent.as_deref()));
        let reason = if hidden {
            format!("{call}: nascosta dai filtri di Attivita' di banda")
        } else {
            self.excluded_by_rules(d, call)?
        };
        let key = call.to_ascii_uppercase();
        if self.refused_callers.get(&key).is_none_or(|&at| now.saturating_sub(at) >= REFUSED_LOG_SECS) {
            if self.refused_callers.len() >= HEARD_CAPACITY {
                self.refused_callers.clear();
            }
            self.refused_callers.insert(key, now);
            self.push(now, DecisionKind::Skipped, Some(call.to_string()), Some(d.id), format!("{reason} (ci chiama: il motore non risponde)"));
        }
        Some(reason)
    }

    /// Valuta un messaggio senza cambiare lo stato della risposta: lo scarta (con il motivo) o lo propone come candidato.
    fn screen(&mut self, d: &DecodedMessage, now: u64) -> Screened {
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
            return Screened::Ignore;
        }
        self.clock = now.max(self.clock);
        self.roll_day(now);
        // fuori dalla fascia oraria non si valuta nulla: il registro l'ha gia' detto una volta (`check_window`)
        self.check_window(now);
        if !self.state.in_window {
            return Screened::Ignore;
        }
        let tag_wanted = d.tag.is_some_and(|t| self.state.rules.reply_to.contains(&t));
        let list_reason = if self.state.rules.reply_to_wanted && parsed.is_cq { self.wanted.as_ref().and_then(|m| m(d, self.dial_hz)) } else { None };
        let new_mode = self.state.rules.reply_to_new_mode
            && parsed.is_cq
            && parsed.sender.as_deref().is_some_and(|call| self.new_mode.as_ref().is_some_and(|m| m(call, self.radio_mode.as_deref())));
        let rules = &self.state.rules;
        let new_state = if parsed.is_cq && (rules.reply_to_new_state || rules.reply_to_new_state_band) {
            parsed
                .sender
                .as_deref()
                .and_then(|call| self.new_state.as_ref().and_then(|m| m(d, call, self.dial_hz)))
                .filter(|(_, never)| (*never && (rules.reply_to_new_state || rules.reply_to_new_state_band)) || (!*never && rules.reply_to_new_state_band))
        } else {
            None
        };
        if !(tag_wanted || list_reason.is_some() || new_mode || new_state.is_some()) || !parsed.is_cq {
            return Screened::Ignore;
        }
        let Some(call) = parsed.sender else {
            return Screened::Skipped(self.push(now, DecisionKind::Skipped, None, Some(d.id), "Nominativo non leggibile nel messaggio".into()));
        };
        let call_opt = Some(call.clone());
        // quello che l'operatore ha nascosto in Attivita' di banda non riceve mai una risposta automatica
        if self.hidden_by_band_filters(d) {
            let reason = format!("{call}: nascosta dai filtri di Attivita' di banda");
            return Screened::Skipped(self.push(now, DecisionKind::Skipped, call_opt, Some(d.id), reason));
        }
        if let Some(why) = ghost::suspicion(&d.text, &call, d.snr, heard_before) {
            return Screened::Skipped(self.push(now, DecisionKind::Skipped, call_opt, Some(d.id), format!("{call}: {why}")));
        }
        if d.snr < self.state.rules.min_snr {
            let reason = format!("{call}: segnale {} dB sotto il minimo di {} dB", d.snr, self.state.rules.min_snr);
            return Screened::Skipped(self.push(now, DecisionKind::Skipped, call_opt, Some(d.id), reason));
        }
        if let Some(reason) = self.excluded_by_rules(d, &call) {
            return Screened::Skipped(self.push(now, DecisionKind::Skipped, call_opt, Some(d.id), reason));
        }
        if let Some(&(_, last)) = self.attempts.get(&call)
            && now.saturating_sub(last) >= self.state.rules.cooldown_secs
        {
            // la pausa e' passata: si puo' riprovare
            self.attempts.remove(&call);
        }
        if self.state.rules.respect_cq_direction
            && let Some(why) = self.direction_mismatch(&d.text, d.continent.as_deref())
        {
            return Screened::Skipped(self.push(now, DecisionKind::Skipped, call_opt, Some(d.id), format!("{call}: {why}")));
        }
        if self.attempts.get(&call).map_or(0, |a| a.0) >= self.state.rules.max_attempts_per_call {
            return Screened::Skipped(self.push(now, DecisionKind::Skipped, call_opt, Some(d.id), format!("{call}: tentativi esauriti senza QSO")));
        }
        if let Some((busy, since)) = &self.active
            && now.saturating_sub(*since) < ACTIVE_QSO_SECS
        {
            let reason = format!("{call}: QSO gia' in corso con {busy}");
            return Screened::Skipped(self.push(now, DecisionKind::Skipped, call_opt, Some(d.id), reason));
        }
        let why = match (&list_reason, d.tag) {
            (Some(reason), _) if !tag_wanted => format!("nella lista dei desiderati ({reason})"),
            (_, Some(tag)) if tag_wanted => tag_text(tag).to_string(),
            _ if new_mode => "paese nuovo in questo modo".to_string(),
            _ if new_state.is_some() => match &new_state {
                Some((st, true)) => format!("stato nuovo ({st})"),
                Some((st, false)) => format!("stato nuovo sulla banda ({st})"),
                None => String::new(),
            },
            _ => "CQ di interesse".to_string(),
        };
        let tag_rank = if tag_wanted { d.tag.map_or(0, rank_of) } else { 0 };
        let mode_rank = if new_mode { NEW_MODE_RANK } else { 0 }.max(match &new_state {
            Some((_, true)) => NEW_MODE_RANK,
            Some((_, false)) => NEW_MODE_RANK.saturating_sub(1),
            None => 0,
        });
        let rank = if list_reason.is_some() { WISHLIST_RANK.max(tag_rank) } else { tag_rank.max(mode_rank) };
        Screened::Candidate(Candidate { call, decode_id: d.id, snr: d.snr, rank, why })
    }

    /// Risponde al candidato scelto, se i tetti lo permettono.
    fn commit(&mut self, c: Candidate, now: u64) -> Decision {
        let call_opt = Some(c.call.clone());
        let live = self.state.mode == Mode::Live;
        // il limite orario vale in tutti e due i modi: in prova a vuoto il registro dice che si sarebbe fermata
        if self.hour_full(now) {
            let reason = self.hour_block_reason();
            return self.push_block(now, call_opt, Some(c.decode_id), reason);
        }
        if live {
            if self.state.consecutive >= self.state.limits.max_consecutive_cycles {
                self.switch_off();
                let reason = format!("Raggiunte {} risposte di fila senza QSO: automazione spenta", self.state.limits.max_consecutive_cycles);
                return self.push(now, DecisionKind::Blocked, call_opt, Some(c.decode_id), reason);
            }
            if self.state.replies_today >= self.state.limits.max_per_day {
                let reason = format!("Raggiunto il tetto di {} risposte al giorno", self.state.limits.max_per_day);
                return self.push(now, DecisionKind::Blocked, call_opt, Some(c.decode_id), reason);
            }
            self.state.consecutive += 1;
            self.state.replies_today += 1;
            // la risposta prende il posto del CQ chiamato da sola
            self.cq_since = None;
        } else {
            self.state.would_replies_today += 1;
            // in prova a vuoto non ci sono QSO veri: per il limite orario conta ogni stazione nuova a cui avrebbe risposto
            if !self.attempts.contains_key(&c.call) {
                self.hour_marks.push_back(now);
                self.prune_hour(now);
            }
        }
        let entry = self.attempts.entry(c.call.clone()).or_insert((0, now));
        entry.0 += 1;
        entry.1 = now;
        self.active = Some((c.call.clone(), now));
        let (kind, reason) = if live {
            (DecisionKind::Reply, format!("Rispondo a {}: {}", c.call, c.why))
        } else {
            (DecisionKind::WouldReply, format!("Risponderei a {}: {} (prova a vuoto)", c.call, c.why))
        };
        self.push(now, kind, call_opt, Some(c.decode_id), reason)
    }

    /// Il CQ e' diretto a un'altra zona? «CQ NA» e' per il Nord America, «CQ DX» per chi sta fuori dal continente di chi chiama.
    /// Senza conoscere il proprio continente (o quello di chi chiama, per «DX») non si esclude nulla.
    fn direction_mismatch(&self, text: &str, sender_continent: Option<&str>) -> Option<String> {
        let own = self.own_continent.as_deref()?;
        let modifier = cq_modifier(text)?;
        match modifier.as_str() {
            "AF" | "AN" | "AS" | "EU" | "NA" | "OC" | "SA" if modifier != own => Some(format!("CQ {modifier}: non e' per il tuo continente ({own})")),
            "DX" if sender_continent.is_some_and(|c| c.eq_ignore_ascii_case(own)) => Some(format!("CQ DX dal tuo stesso continente ({own})")),
            _ => None,
        }
    }

    fn roll_day(&mut self, now: u64) {
        let today = now / DAY_SECS;
        if today != self.day {
            self.day = today;
            self.state.replies_today = 0;
            self.state.would_replies_today = 0;
            self.state.qsos_today = 0;
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

/// Un CQ che ha passato tutti i controlli e puo' essere scelto.
#[derive(Clone)]
struct Candidate {
    call: String,
    decode_id: u64,
    snr: i32,
    /// Quanto e' interessante: piu' alto = piu' raro.
    rank: u8,
    why: String,
}

/// Il modificatore di un CQ («CQ NA K1ABC FN42» -> NA; «CQDX» -> DX); `None` se non c'e'.
fn cq_modifier(text: &str) -> Option<String> {
    let mut tokens = text.split_whitespace().map(str::to_uppercase);
    match tokens.next()?.as_str() {
        "CQDX" => Some("DX".to_string()),
        "CQ" => tokens.next().filter(|t| (2..=4).contains(&t.len()) && t.chars().all(|c| c.is_ascii_uppercase())),
        _ => None,
    }
}

impl Candidate {
    fn score(&self) -> i32 {
        i32::from(self.rank) * 10 + (self.snr + 24).clamp(0, 36) / 2
    }

    fn better_than(&self, other: &Candidate, prefer: Prefer) -> bool {
        match prefer {
            Prefer::Rarity => (self.rank, self.snr) > (other.rank, other.snr),
            Prefer::Snr => (self.snr, self.rank) > (other.snr, other.rank),
            Prefer::Score => (self.score(), self.snr) > (other.score(), other.snr),
        }
    }
}

enum Screened {
    /// Non interessa: non si registra nulla.
    Ignore,
    Skipped(Decision),
    Candidate(Candidate),
}

/// Una stazione della lista dei desiderati vale piu' di qualunque novita': l'ha scelta l'operatore.
const WISHLIST_RANK: u8 = 5;

/// Un paese nuovo nel modo in uso vale quanto un paese nuovo sulla banda.
const NEW_MODE_RANK: u8 = 3;

/// Quanto e' rara una novita': un paese mai collegato batte uno nuovo solo su questa banda, poi zona e griglia.
fn rank_of(tag: DecodeTag) -> u8 {
    match tag {
        DecodeTag::NewDxcc => 4,
        DecodeTag::NewDxccBand => 3,
        DecodeTag::NewZone => 2,
        DecodeTag::NewGrid => 1,
        _ => 0,
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
