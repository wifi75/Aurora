//! Le opzioni dei decodificatori di WSJT-X 3.3 scelte nelle Impostazioni (riquadro «Decodifica» della scheda Filtri): profondita'
//! «Fast/Normal/Deep», «Enable averaging», «Enable deep search», «Auto Clear Avg after decode» ed «Enable AP» (menu Decode,
//! `widgets/mainwindow.cpp:12544-12578`), il livello «Aggressive» di JT65 (Settings → Advanced) e «Single decode» (Settings → General),
//! «F Tol» di JT65, JT9, MSK144 e Q65 (`mainwindow_settings.cpp:370-395`).
//!
//! Una copia sola per programma, come gli interruttori di Q65 in `rig`: la scrive chi applica le impostazioni, la legge il ciclo della
//! decodifica a ogni periodo (vale dal periodo che segue, come in WSJT-X che legge `m_ndepth` a ogni `decode()`).

use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;

/// Le opzioni, gia' controllate. I valori di fabbrica ([`DecodeSettings::WSJTX`]) sono quelli di WSJT-X 3.3, letti anche nel `wsjtx.exe`
/// compilato: `NDepth` 3 e `JT65AP` falso (`FUN_1401e5f90`), `Aggressive` 4 e `SingleDecode` falso (`FUN_14036d650`), `Ftol_*` 50.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecodeSettings {
    /// 1 Fast, 2 Normal, 3 Deep (`NDepth & 7`).
    pub depth: u8,
    /// «Enable averaging» (bit 16 di `NDepth`): Q65 prova la media dei periodi.
    pub include_averaging: bool,
    /// «Enable deep search» (bit 32 di `NDepth`, «Include correlation»): JT65 prova `hint65`.
    pub include_correlation: bool,
    /// «Auto Clear Avg after decode» (bit 128 di `NDepth`): Q65 svuota la media dopo un messaggio sulla frequenza di ricezione.
    pub auto_clear_avg: bool,
    /// «Enable AP» di JT65 (`JT65AP`, `ljt65apon`).
    pub jt65_ap: bool,
    /// «Aggressive decoding level» di JT65, da 0 a 10 (`naggressive`).
    pub jt65_aggressive: u8,
    /// «Single decode» (`nexp_decode & 32`): JT65 e Q65 cercano solo attorno alla frequenza di ricezione.
    pub single_decode: bool,
    /// F Tol in hertz (`ntol`).
    pub ftol_jt65: u32,
    pub ftol_jt9: u32,
    pub ftol_msk144: u32,
    pub ftol_q65: u32,
}

impl DecodeSettings {
    /// I valori di fabbrica di WSJT-X 3.3.
    pub const WSJTX: DecodeSettings = DecodeSettings {
        depth: 3,
        include_averaging: false,
        include_correlation: false,
        auto_clear_avg: false,
        jt65_ap: false,
        jt65_aggressive: 4,
        single_decode: false,
        ftol_jt65: 50,
        ftol_jt9: 50,
        ftol_msk144: 50,
        ftol_q65: 50,
    };

    /// I valori portati nei campi che WSJT-X ammette: profondita' 1-3, «Aggressive» al piu' 10, F Tol da 1 a 1000 Hz (da 20 a 500 in MSK144,
    /// la tendina di `mainwindow.cpp:12114`).
    fn normalized(self) -> DecodeSettings {
        DecodeSettings {
            depth: self.depth.clamp(1, 3),
            jt65_aggressive: self.jt65_aggressive.min(10),
            ftol_jt65: self.ftol_jt65.clamp(1, 1000),
            ftol_jt9: self.ftol_jt9.clamp(1, 1000),
            ftol_msk144: self.ftol_msk144.clamp(20, 500),
            ftol_q65: self.ftol_q65.clamp(1, 1000),
            ..self
        }
    }
}

impl Default for DecodeSettings {
    fn default() -> Self {
        DecodeSettings::WSJTX
    }
}

static CURRENT: Mutex<DecodeSettings> = Mutex::new(DecodeSettings::WSJTX);

/// Le opzioni in uso.
pub fn current() -> DecodeSettings {
    CURRENT.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clone()
}

/// Cambia le opzioni: valgono dal periodo che segue.
pub fn set(settings: DecodeSettings) {
    *CURRENT.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = settings.normalized();
}

/// JT65: frequenza di ricezione e F Tol (`nfqso`, `ntol`), profondita', «Aggressive», «Single decode»; l'analisi a priori solo con
/// «Enable AP» (in WSJT-X `ljt65apon` e' una voce a se', non legata all'AP di FT8: `mainwindow.cpp:5469-5470`) e, come prima in Aurora,
/// solo con Abilita Tx (`my_call_active`, contro i falsi del rumore); la ricerca profonda (`hint65`) con «Enable deep search», sui
/// nominativi del QSO anche senza AP e sulla lista dei nominativi con locatore sentiti (il `CALL3.TXT` di WSJT-X, qui imparato dai messaggi).
pub(crate) fn jt65_options(s: &DecodeSettings, rx_hz: Option<f32>, calls: Option<jt::rx::ApCalls>, my_call_active: bool) -> jt::rx::Options {
    jt::rx::Options {
        rx_hz,
        tol_hz: s.ftol_jt65 as f32,
        depth: s.depth,
        aggressive: s.jt65_aggressive,
        single_decode: s.single_decode,
        ap: calls.clone().filter(|_| s.jt65_ap && my_call_active),
        deep_search: s.include_correlation,
        hint_calls: if s.include_correlation { hint_calls() } else { Vec::new() },
        deep_calls: calls,
        ..jt::rx::Options::default()
    }
}

/// JT9: la finestra di ricezione (`nqd = 1` di `jt9_decode.f90`) con F Tol, e la profondita' (`iand(ndepth,7)`, `jt9_decode.f90:210-214`).
pub(crate) fn jt9_options(s: &DecodeSettings, rx_hz: Option<f32>) -> jt::jt9rx::Options {
    jt::jt9rx::Options { rx_hz, tol_hz: s.ftol_jt9 as f32, depth: s.depth, ..jt::jt9rx::Options::default() }
}

/// MSK144: F Tol (`Ftol_MSK144`) e profondita' (`ndepth` di `mskrtd.f90:146-148`).
pub(crate) fn tune_msk144(s: &DecodeSettings, o: &mut jt::msk144::Options) {
    o.tolerance_hz = s.ftol_msk144;
    o.depth = s.depth;
}

/// Q65: F Tol (`ntol`), «Enable averaging», «Auto Clear Avg after decode» e «Single decode». La profondita' no: il decodificatore Q65 di
/// Aurora prova sempre le larghezze e le 100 iterazioni di «Deep» (`q65_decode.f90:445-450`), il valore di fabbrica di WSJT-X, che lo
/// usa comunque per la decodifica a mano (`effort=ior(effort,3)`, `:389`).
pub(crate) fn tune_q65(s: &DecodeSettings, o: &mut jt::q65::Options) {
    o.rx_tol_hz = s.ftol_q65 as f32;
    o.average_decode = s.include_averaging;
    o.auto_clear_avg = s.auto_clear_avg;
    o.single_decode = s.single_decode;
}

/// Quanti nominativi con locatore al massimo (`hint65` ne prende 10000, `set_calls`).
const MAX_HINT_CALLS: usize = 10_000;

/// I nominativi sentiti con il loro locatore a 4 caratteri, il piu' recente in fondo.
#[derive(Default)]
struct HintBook {
    grids: HashMap<String, String>,
    order: VecDeque<String>,
}

impl HintBook {
    fn learn(&mut self, call: String, grid: String) {
        if self.grids.insert(call.clone(), grid).is_some() {
            self.order.retain(|c| *c != call);
        }
        self.order.push_back(call);
        while self.order.len() > MAX_HINT_CALLS {
            if let Some(old) = self.order.pop_front() {
                self.grids.remove(&old);
            }
        }
    }

    fn list(&self) -> Vec<(String, String)> {
        self.order.iter().rev().filter_map(|c| self.grids.get(c).map(|g| (c.clone(), g.clone()))).collect()
    }
}

static HINTS: Mutex<Option<HintBook>> = Mutex::new(None);

/// Il locatore a 4 caratteri (`AA00`-`RR99`), non `RR73`.
fn is_grid4(word: &str) -> bool {
    let b = word.as_bytes();
    b.len() == 4 && (b'A'..=b'R').contains(&b[0]) && (b'A'..=b'R').contains(&b[1]) && b[2].is_ascii_digit() && b[3].is_ascii_digit() && word != "RR73"
}

/// Un nominativo plausibile (lettere e cifre, con almeno una di ciascuna, eventualmente con `/`).
fn is_call(word: &str) -> bool {
    (3..=11).contains(&word.len())
        && word.chars().all(|c| c.is_ascii_alphanumeric() || c == '/')
        && word.chars().any(|c| c.is_ascii_digit())
        && word.chars().any(|c| c.is_ascii_alphabetic())
}

/// Nominativo e locatore di un messaggio che finisce con il locatore («CQ K1ABC FN42», «IU3ABC K1ABC FN42»): e' chi trasmette.
fn call_and_grid(text: &str) -> Option<(String, String)> {
    let words: Vec<&str> = text.split_whitespace().collect();
    let [.., call, grid] = words.as_slice() else { return None };
    let call = call.trim_matches(|c| c == '<' || c == '>');
    (words.len() >= 3 && is_grid4(grid) && is_call(call)).then(|| (call.to_string(), (*grid).to_string()))
}

/// Impara nominativi e locatori dai messaggi decodificati senza aiuti (non a priori, non dalla lista): sono la lista di `hint65`.
pub(crate) fn learn_grids(decoded: &[ft8::decode::Decoded]) {
    let found: Vec<(String, String)> = decoded.iter().filter(|d| !d.hint).filter_map(|d| call_and_grid(&d.text)).collect();
    if found.is_empty() {
        return;
    }
    let mut guard = HINTS.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let book = guard.get_or_insert_with(HintBook::default);
    for (call, grid) in found {
        book.learn(call, grid);
    }
}

/// La lista per la ricerca profonda di JT65: nominativo e locatore, i piu' recenti per primi.
pub(crate) fn hint_calls() -> Vec<(String, String)> {
    HINTS.lock().unwrap_or_else(std::sync::PoisonError::into_inner).as_ref().map(HintBook::list).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn calls() -> jt::rx::ApCalls {
        jt::rx::ApCalls { my_call: "IU3ABC".into(), dx_call: "K1ABC".into(), dx_grid: "FN42".into(), qso_progress: 1 }
    }

    #[test]
    fn the_defaults_are_those_of_wsjtx_and_values_are_kept_in_range() {
        let d = DecodeSettings::default();
        assert_eq!((d.depth, d.jt65_aggressive, d.ftol_jt65, d.ftol_jt9, d.ftol_msk144, d.ftol_q65), (3, 4, 50, 50, 50, 50));
        assert!(!d.include_averaging && !d.include_correlation && !d.auto_clear_avg && !d.jt65_ap && !d.single_decode);
        let n = DecodeSettings { depth: 0, jt65_aggressive: 40, ftol_jt65: 0, ftol_jt9: 5000, ftol_msk144: 1, ftol_q65: 2000, ..d }.normalized();
        assert_eq!((n.depth, n.jt65_aggressive, n.ftol_jt65, n.ftol_jt9, n.ftol_msk144, n.ftol_q65), (1, 10, 1, 1000, 20, 1000));
    }

    #[test]
    fn jt65_gets_every_option_and_ap_only_with_enable_ap() {
        let s = DecodeSettings { depth: 2, jt65_aggressive: 7, single_decode: true, ftol_jt65: 100, ..DecodeSettings::WSJTX };
        let o = jt65_options(&s, Some(1270.0), Some(calls()), true);
        assert_eq!((o.rx_hz, o.tol_hz, o.depth, o.aggressive, o.single_decode), (Some(1270.0), 100.0, 2, 7, true));
        assert!(o.ap.is_none(), "«Enable AP» spento di fabbrica, anche con a7/a8 acceso");
        assert!(!o.deep_search && o.hint_calls.is_empty());
        let ap = DecodeSettings { jt65_ap: true, ..s.clone() };
        assert!(jt65_options(&ap, None, Some(calls()), true).ap.is_some());
        assert!(jt65_options(&ap, None, Some(calls()), false).ap.is_none(), "senza Abilita Tx niente AP, come prima");
        // ricerca profonda senza AP: i nominativi del QSO arrivano lo stesso a `hint65`
        let deep = jt65_options(&DecodeSettings { include_correlation: true, ..s }, None, Some(calls()), false);
        assert!(deep.deep_search && deep.ap.is_none() && deep.deep_calls.is_some());
    }

    #[test]
    fn jt9_msk144_and_q65_get_their_ftol_and_depth() {
        let s = DecodeSettings { depth: 1, ftol_jt9: 20, ftol_msk144: 200, ftol_q65: 10, include_averaging: true, auto_clear_avg: true, single_decode: true, ..DecodeSettings::WSJTX };
        let j = jt9_options(&s, Some(1000.0));
        assert_eq!((j.rx_hz, j.tol_hz, j.depth), (Some(1000.0), 20.0, 1));
        let mut m = jt::msk144::Options::default();
        tune_msk144(&s, &mut m);
        assert_eq!((m.tolerance_hz, m.depth), (200, 1));
        let mut q = jt::q65::Options::default();
        tune_q65(&s, &mut q);
        assert_eq!(q.rx_tol_hz, 10.0);
        assert!(q.average_decode && q.auto_clear_avg && q.single_decode);
        tune_q65(&DecodeSettings::WSJTX, &mut q);
        assert!(!q.average_decode && !q.auto_clear_avg && !q.single_decode, "WSJT-X di fabbrica: media spenta");
    }

    #[test]
    fn calls_and_grids_come_from_messages_ending_with_a_locator() {
        assert_eq!(call_and_grid("CQ K1ABC FN42"), Some(("K1ABC".into(), "FN42".into())));
        assert_eq!(call_and_grid("CQ DX IU3ABC JN55"), Some(("IU3ABC".into(), "JN55".into())));
        assert_eq!(call_and_grid("IU3ABC <K1ABC/P> FN42"), Some(("K1ABC/P".into(), "FN42".into())));
        assert_eq!(call_and_grid("IU3ABC K1ABC RR73"), None);
        assert_eq!(call_and_grid("IU3ABC K1ABC -15"), None);
        assert_eq!(call_and_grid("K1ABC W9XYZ R EN37"), None);
        assert_eq!(call_and_grid("FN42"), None);
        let mut book = HintBook::default();
        book.learn("K1ABC".into(), "FN42".into());
        book.learn("W9XYZ".into(), "EN37".into());
        book.learn("K1ABC".into(), "FN43".into());
        assert_eq!(book.list(), vec![("K1ABC".to_string(), "FN43".to_string()), ("W9XYZ".to_string(), "EN37".to_string())]);
        for i in 0..MAX_HINT_CALLS {
            book.learn(format!("K{i}AA"), "AA00".into());
        }
        assert_eq!(book.list().len(), MAX_HINT_CALLS);
        assert!(!book.grids.contains_key("W9XYZ"), "i piu' vecchi escono");
    }
}
