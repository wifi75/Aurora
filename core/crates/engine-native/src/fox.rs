//! Il Fox classico di WSJT-X 3.3 (`widgets/mainwindow.cpp`: `houndCallers`, `selectHound`, `foxRxSequencer`, `foxTxSequencer`, ramo non
//! SuperFox): la DXpedition lavora fino a 5 Hound per periodo con un'onda sola che porta fino a 5 segnali FT8 a 60 Hz l'uno dall'altro.
//!
//! Qui sta solo la logica (chi chiama, la coda, i messaggi del prossimo periodo, chi va a log); il motore la chiama a ogni periodo e
//! trasforma i messaggi in un'onda con [`fox_wave_hz`].

use std::collections::{BTreeMap, HashMap, VecDeque};

/// Valori di fabbrica di WSJT-X 3.3 (`mainwindow.h`).
pub const MAX_SLOTS: usize = 5;
pub const MAX_STRIKES: i64 = 3;
pub const MAX_FOX_WAIT: i64 = 3;
pub const FOX_CQ_TIME: i64 = 10;
pub const MAX_HOUNDS_IN_QUEUE: usize = 10;
/// Distanza fra i segnali dei posti (`foxgen`: `fstep=60`).
pub const SLOT_STEP_HZ: u32 = 60;

/// Un Hound che chiama la Fox (una riga di `houndcallers.txt`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HoundCaller {
    pub call: String,
    pub grid: String,
    pub snr: i32,
    pub freq_hz: u32,
    /// Periodi della Fox da quando si e' sentito l'ultima volta (0 = in questo periodo).
    pub age: u32,
}

/// Un QSO in corso (`m_foxQSO`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoxQso {
    pub grid: String,
    pub sent: String,
    pub rcvd: Option<String>,
    pub ncall: i64,
    pub n_rr73: i64,
    /// Periodo della Fox in cui e' arrivato il suo R+rapporto (`tFoxRrpt`, -1 = mai).
    pub t_rrpt: i64,
    /// Periodo in cui gli si e' mandato RR73 (`tFoxTxRR73`, -1 = mai).
    pub t_tx_rr73: i64,
}

/// Un Hound in coda (`m_houndQueue`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Queued {
    pub call: String,
    pub report: String,
    pub grid: String,
}

/// Un QSO concluso dalla Fox, da registrare.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoxLogged {
    pub call: String,
    pub grid: String,
    pub sent: String,
    pub rcvd: String,
    pub band: String,
}

/// Le scelte dell'operatore Fox.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoxOptions {
    /// Posti per periodo (1-5).
    pub slots: usize,
    /// «More CQs»: un CQ ogni ~4 periodi in un posto libero e comunque ogni 10.
    pub more_cqs: bool,
    /// Il testo del CQ («CQ», «CQ NA»...).
    pub cq: String,
    /// Messaggio libero da mandare in un posto (`cbSendMsg`).
    pub free_text: Option<String>,
    /// Aggiunge da solo in coda il primo dei chiamanti quando la coda e' vuota.
    pub auto_queue: bool,
}

impl Default for FoxOptions {
    fn default() -> Self {
        FoxOptions { slots: MAX_SLOTS, more_cqs: false, cq: "CQ".into(), free_text: None, auto_queue: false }
    }
}

/// Lo stato della Fox.
#[derive(Debug, Default)]
pub struct Fox {
    pub opts: FoxOptions,
    my_call: String,
    base_call: String,
    my_grid: String,
    pub qsos: BTreeMap<String, FoxQso>,
    pub in_progress: VecDeque<String>,
    pub queue: VecDeque<Queued>,
    /// Chi chiama adesso (per la tabella dei chiamanti), il piu' recente per nominativo.
    pub callers: BTreeMap<String, HoundCaller>,
    /// Bande su cui la Fox ha gia' registrato ciascun nominativo (`m_loggedByFox`).
    logged: HashMap<String, Vec<String>>,
    t_tx: i64,
    t_tx0: i64,
    t_since_cq: i64,
}

fn base_call(call: &str) -> String {
    // come `Radio::base_callsign`: la parte piu' lunga fra le barre
    call.split('/').max_by_key(|p| p.len()).unwrap_or(call).to_string()
}

/// Il rapporto che si manda a un Hound: pari, fra -30 e +32, con il segno (`selectHound`).
pub fn hound_report(snr: i32) -> String {
    let s = snr.clamp(-30, 32);
    let s = 2 * ((s + 30) / 2) - 30;
    format!("{s:+03}")
}

impl Fox {
    pub fn new(my_call: &str, my_grid: &str) -> Fox {
        Fox { my_call: my_call.to_uppercase(), base_call: base_call(&my_call.to_uppercase()), my_grid: my_grid.to_uppercase(), t_since_cq: FOX_CQ_TIME, ..Fox::default() }
    }

    /// Tutto da capo (`FoxReset`): niente coda, niente QSO in corso, niente chiamanti.
    pub fn reset(&mut self) {
        self.qsos.clear();
        self.in_progress.clear();
        self.queue.clear();
        self.callers.clear();
    }

    /// I messaggi decodificati di un periodo: chi chiama la Fox va fra i chiamanti, chi manda R+rapporto fa avanzare il suo QSO.
    pub fn on_decodes(&mut self, decoded: &[(String, i32, u32)]) {
        for c in self.callers.values_mut() {
            c.age += 1;
        }
        for (text, snr, freq) in decoded {
            let w: Vec<&str> = text.split_whitespace().collect();
            if w.len() < 3 || !w[0].eq_ignore_ascii_case(&self.my_call) && !w[0].eq_ignore_ascii_case(&self.base_call) {
                continue;
            }
            let hound = w[1].trim_matches(['<', '>']).to_uppercase();
            let third = w[2].to_uppercase();
            if third.starts_with("R+") || third.starts_with("R-") {
                // `foxRxSequencer`
                let key = if self.qsos.contains_key(&hound) {
                    Some(hound.clone())
                } else {
                    self.qsos.keys().find(|hc| hc.contains(&format!("/{hound}")) || hc.contains(&format!("{hound}/"))).cloned()
                };
                if let Some(k) = key
                    && let Some(q) = self.qsos.get_mut(&k)
                {
                    q.rcvd = Some(third[1..].to_string());
                    q.t_rrpt = self.t_tx;
                }
                continue;
            }
            // «FOX HOUND GRID» (o un rapporto): un Hound che chiama
            if self.qsos.contains_key(&hound) || self.queue.iter().any(|q| q.call == hound) {
                continue;
            }
            let grid = if third.len() == 4 && third.as_bytes()[2].is_ascii_digit() { third.clone() } else { String::new() };
            self.callers.insert(hound.clone(), HoundCaller { call: hound, grid, snr: *snr, freq_hz: *freq, age: 0 });
        }
        // chi non si sente da 4 periodi esce dalla lista (WSJT-X tiene le ultime 4 sequenze)
        self.callers.retain(|_, c| c.age < 4);
        if self.opts.auto_queue
            && self.queue.is_empty()
            && let Some(best) = self.callers.values().max_by_key(|c| c.snr).map(|c| c.call.clone())
        {
            self.select(&best, false);
        }
    }

    /// Mette in coda un chiamante (`selectHound`); `top` lo mette in testa. Falso se non c'e' o la coda e' piena.
    pub fn select(&mut self, call: &str, top: bool) -> bool {
        let call = call.to_uppercase();
        if self.queue.len() >= MAX_HOUNDS_IN_QUEUE || self.queue.iter().any(|q| q.call == call) || self.qsos.contains_key(&call) {
            return false;
        }
        let Some(c) = self.callers.remove(&call) else { return false };
        let q = Queued { call: c.call, report: hound_report(c.snr), grid: c.grid };
        if top {
            self.queue.push_front(q);
        } else {
            self.queue.push_back(q);
        }
        true
    }

    /// Toglie un Hound dalla coda.
    pub fn unqueue(&mut self, call: &str) -> bool {
        let before = self.queue.len();
        self.queue.retain(|q| !q.call.eq_ignore_ascii_case(call));
        before != self.queue.len()
    }

    /// Chiude un QSO in corso (doppio clic nella lista dei QSO in corso di WSJT-X: «timeout»).
    pub fn drop_qso(&mut self, call: &str) {
        let call = call.to_uppercase();
        self.qsos.remove(&call);
        self.in_progress.retain(|c| *c != call);
    }

    fn cq_message(&self) -> String {
        let mut fm = format!("{} {}", self.opts.cq.trim(), self.my_call);
        if !fm.contains('/') && self.my_grid.len() >= 4 {
            fm.push(' ');
            fm.push_str(&self.my_grid[..4]);
        }
        fm
    }

    /// I messaggi del prossimo periodo di trasmissione (`foxTxSequencer`, Fox classico) e i QSO da registrare su `band`.
    pub fn plan_tx(&mut self, band: &str) -> (Vec<String>, Vec<FoxLogged>) {
        self.t_tx += 1;
        let slots = self.opts.slots.clamp(1, MAX_SLOTS);
        let mut msgs: Vec<String> = Vec::new();
        let mut logged = Vec::new();

        // CQ da solo, se e' ora (almeno ogni 10 periodi con «More CQs»)
        if self.t_since_cq >= FOX_CQ_TIME && self.opts.more_cqs {
            self.t_tx0 = self.t_tx;
            msgs.push(self.cq_message());
            self.t_since_cq = 0;
            self.age_out();
            return (msgs, logged);
        }
        if let Some(text) = self.opts.free_text.clone().filter(|t| !t.trim().is_empty())
            && msgs.len() < slots
        {
            msgs.push(text);
        }

        // lista 1: a chi mandare RR73, in tre priorita'
        let mut list1: Vec<String> = Vec::new();
        let room1 = slots - msgs.len();
        let keys: Vec<String> = self.qsos.keys().cloned().collect();
        for pass in 0..3 {
            for hc in &keys {
                if list1.len() >= room1 || list1.contains(hc) {
                    continue;
                }
                let Some(q) = self.qsos.get(hc) else { continue };
                if q.t_rrpt < 0 {
                    continue;
                }
                let take = match pass {
                    0 => q.t_rrpt - q.t_tx_rr73 > 3,
                    1 => q.t_tx_rr73 < 0,
                    _ => q.t_tx_rr73 <= q.t_rrpt,
                };
                if take {
                    list1.push(hc.clone());
                    if let Some(q) = self.qsos.get_mut(hc) {
                        q.t_tx_rr73 = self.t_tx;
                        q.n_rr73 += 1;
                    }
                }
            }
        }

        // lista 2: a chi mandare il rapporto (prima chi e' in corso, poi i nuovi dalla coda)
        let room2 = slots - msgs.len();
        let mut list2: Vec<String> = Vec::new();
        for hc in self.in_progress.iter() {
            if list2.len() >= room2 {
                break;
            }
            if let Some(q) = self.qsos.get(hc)
                && q.t_rrpt < 0
                && q.ncall < MAX_STRIKES
            {
                list2.push(hc.clone());
            }
        }
        while list2.len() < room2 {
            let Some(q) = self.queue.pop_front() else { break };
            list2.push(q.call.clone());
            self.in_progress.push_back(q.call.clone());
            self.qsos.insert(q.call, FoxQso { grid: q.grid, sent: q.report, rcvd: None, ncall: 0, n_rr73: 0, t_rrpt: -1, t_tx_rr73: -1 });
        }

        let n3 = list1.len().max(list2.len()).min(slots - msgs.len());
        let mut sent_calls = 0;
        for i in 0..n3 {
            let hc1 = list1.get(i).cloned();
            let hc2 = list2.get(i).cloned();
            let fm = match (&hc1, &hc2) {
                (Some(a), Some(b)) => {
                    let q = self.qsos.get_mut(b);
                    let sent = q.map(|q| {
                        q.ncall += 1;
                        q.sent.clone()
                    });
                    format!("{} RR73; {} <{}> {}", base_call(a), base_call(b), self.my_call, sent.unwrap_or_default())
                }
                (Some(a), None) => format!("{} {} RR73", base_call(a), self.base_call),
                (None, Some(b)) => {
                    let sent = self.qsos.get_mut(b).map(|q| {
                        q.ncall += 1;
                        q.sent.clone()
                    });
                    format!("{} {} {}", base_call(b), self.base_call, sent.unwrap_or_default())
                }
                (None, None) => continue,
            };
            if let Some(a) = hc1 {
                let bands = self.logged.entry(a.clone()).or_default();
                if !bands.iter().any(|b| b == band)
                    && let Some(q) = self.qsos.get(&a)
                {
                    logged.push(FoxLogged { call: a.clone(), grid: q.grid.clone(), sent: q.sent.clone(), rcvd: q.rcvd.clone().unwrap_or_default(), band: band.to_string() });
                    bands.push(band.to_string());
                }
            }
            msgs.push(fm);
            sent_calls += 1;
        }

        // un posto libero: CQ se non si e' chiamato nessuno, o ogni ~4 periodi con «More CQs»
        if msgs.len() < slots && (sent_calls == 0 || (self.t_tx - self.t_tx0 >= 4 && self.opts.more_cqs)) {
            let fm = self.cq_message();
            if !fm.contains('/') {
                self.t_tx0 = self.t_tx;
            }
            msgs.push(fm);
        }
        self.t_since_cq += 1;
        self.age_out();
        (msgs, logged)
    }

    /// Chi ha esaurito i tentativi o non risponde piu' esce dai QSO in corso.
    fn age_out(&mut self) {
        let t = self.t_tx;
        let mut gone = Vec::new();
        for (hc, q) in self.qsos.iter_mut() {
            if q.ncall >= MAX_STRIKES {
                q.ncall += 1;
            }
            let b1 = t - q.t_rrpt > 2 * MAX_FOX_WAIT && q.t_rrpt > 0;
            let b2 = t - q.t_tx_rr73 > MAX_FOX_WAIT && q.t_tx_rr73 > 0;
            let b3 = q.ncall >= MAX_STRIKES + MAX_FOX_WAIT;
            let b4 = q.n_rr73 >= MAX_STRIKES;
            if b1 || b2 || b3 || b4 {
                gone.push(hc.clone());
            }
        }
        for hc in gone {
            self.drop_qso(&hc);
        }
    }

    /// Il periodo della Fox (quante trasmissioni ha fatto).
    pub fn periods(&self) -> i64 {
        self.t_tx
    }
}

/// La frequenza audio del posto `i` (0-4) con la Tx della Fox a `tx_hz` (`foxgen`: `f0 = nfreq + 60 (n-1)`).
pub fn fox_wave_hz(tx_hz: u32, slot: usize) -> u32 {
    tx_hz + SLOT_STEP_HZ * slot as u32
}

/// Somma le onde dei posti (stessa lunghezza o diverse) e riporta il picco a 1, come `foxgen` (`wave/peak`).
pub fn mix_waves(waves: &[Vec<f32>]) -> Vec<f32> {
    let n = waves.iter().map(Vec::len).max().unwrap_or(0);
    let mut out = vec![0.0f32; n];
    for w in waves {
        for (o, v) in out.iter_mut().zip(w) {
            *o += *v;
        }
    }
    let peak = out.iter().fold(0.0f32, |m, v| m.max(v.abs()));
    if peak > 0.0 {
        for v in &mut out {
            *v /= peak;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fox() -> Fox {
        Fox::new("KH1/KH7Z", "AJ10")
    }

    #[test]
    fn reports_are_even_and_signed() {
        assert_eq!(hound_report(-13), "-14");
        assert_eq!(hound_report(5), "+04");
        assert_eq!(hound_report(-40), "-30");
    }

    #[test]
    fn a_full_qso_with_two_hounds_and_the_combined_message() {
        let mut f = fox();
        f.on_decodes(&[("KH7Z K1ABC FN42".into(), -10, 1500), ("KH7Z W9XYZ EN52".into(), -15, 1800)]);
        assert_eq!(f.callers.len(), 2);
        assert!(f.select("K1ABC", false) && f.select("W9XYZ", false));
        // primo periodo: i rapporti a tutti e due
        let (msgs, logged) = f.plan_tx("20m");
        assert_eq!(msgs, vec!["K1ABC KH7Z -10".to_string(), "W9XYZ KH7Z -16".to_string()]);
        assert!(logged.is_empty());
        // K1ABC risponde R-12: RR73 a lui e il rapporto ancora a W9XYZ nello stesso posto
        f.on_decodes(&[("KH7Z K1ABC R-12".into(), -9, 1500)]);
        let (msgs, logged) = f.plan_tx("20m");
        assert_eq!(msgs[0], "K1ABC RR73; W9XYZ <KH1/KH7Z> -16");
        assert_eq!(logged, vec![FoxLogged { call: "K1ABC".into(), grid: "FN42".into(), sent: "-10".into(), rcvd: "-12".into(), band: "20m".into() }]);
    }

    #[test]
    fn calls_cq_in_a_free_slot_when_nobody_is_worked() {
        let mut f = Fox::new("K1ABC", "FN42");
        let (msgs, _) = f.plan_tx("20m");
        assert_eq!(msgs, vec!["CQ K1ABC FN42".to_string()]);
    }

    #[test]
    fn three_strikes_and_the_hound_is_out() {
        let mut f = fox();
        f.on_decodes(&[("KH7Z K1ABC FN42".into(), -10, 1500)]);
        f.select("K1ABC", false);
        for _ in 0..10 {
            f.plan_tx("20m");
        }
        assert!(!f.qsos.contains_key("K1ABC"));
    }

    #[test]
    fn mixing_keeps_the_peak_at_one() {
        let w = mix_waves(&[vec![0.5, 1.0, -1.0], vec![0.5, 1.0, 0.2]]);
        assert!((w.iter().fold(0.0f32, |m, v| m.max(v.abs())) - 1.0).abs() < 1e-6);
        assert_eq!(fox_wave_hz(300, 4), 540);
    }
}
