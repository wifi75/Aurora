//! Traccia dei comandi mandati alla radio (a `rigctld`) e delle risposte, per il pacchetto di assistenza.
//!
//! Il registro di Aurora dice cosa succede ad Aurora, non cosa si dicono Aurora e la radio: per capire perche' una radio finisce sul VFO B,
//! perche' una lettura scade o perche' il PTT non parte serve la sequenza dei comandi, con l'ora e il tempo che ognuno ha impiegato.
//! Si tengono due archivi: l'**inizio** di ogni collegamento (i primi [`HEAD_MAX`] comandi: e' li' che la radio viene portata
//! sull'ultima frequenza e nel modo scelto) e gli **ultimi** [`TAIL_MAX`] (cosa stava succedendo prima del guaio).
//! Nessun dato personale: comandi, frequenze, modi e risposte della radio; i nominativi non passano di qui.

use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// Quanti comandi si tengono dell'inizio di un collegamento.
pub const HEAD_MAX: usize = 300;
/// Quanti degli ultimi comandi.
pub const TAIL_MAX: usize = 600;
/// Una risposta lunga si taglia (le risposte estese di `get_vfo_info` sono di qualche riga).
const RESP_MAX: usize = 160;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Entry {
    /// Millisecondi Unix.
    t_ms: u64,
    /// Il comando mandato, oppure `#` e un avviso («collegamento aperto», «chiuso»...).
    text: String,
    /// La risposta (o l'errore).
    resp: String,
    /// Tempo tra l'invio e la risposta, in ms.
    ms: u32,
    ok: bool,
}

/// Gli archivi. Quello del programma e' uno solo ([`BUFFERS`]); le prove ne creano uno proprio.
#[derive(Default)]
struct Buffers {
    head: Vec<Entry>,
    /// Il collegamento di cui `head` e' l'inizio e' ancora aperto: finche' non si riempie raccoglie i comandi.
    head_open: bool,
    tail: VecDeque<Entry>,
    /// Quante volte e' stato aperto un collegamento (per dire a quale appartiene l'inizio).
    sessions: u32,
}

static BUFFERS: Mutex<Buffers> = Mutex::new(Buffers { head: Vec::new(), head_open: false, tail: VecDeque::new(), sessions: 0 });

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

fn clip(text: &str, max: usize) -> String {
    let one_line = text.replace(['\r', '\n'], " | ");
    if one_line.chars().count() <= max {
        one_line
    } else {
        let cut: String = one_line.chars().take(max).collect();
        format!("{cut}…")
    }
}

fn clock(t_ms: u64) -> String {
    let secs = t_ms / 1000;
    format!("{:02}:{:02}:{:02}.{:03}", (secs / 3600) % 24, (secs / 60) % 60, secs % 60, t_ms % 1000)
}

fn line(e: &Entry) -> String {
    if e.text.starts_with('#') {
        return format!("{}  {}", clock(e.t_ms), e.text);
    }
    let mark = if e.ok { ' ' } else { '!' };
    format!("{}{mark} {:>5} ms  {}  ->  {}", clock(e.t_ms), e.ms, e.text, e.resp)
}

impl Buffers {
    fn push(&mut self, entry: Entry) {
        if self.head_open {
            if self.head.len() < HEAD_MAX {
                self.head.push(entry.clone());
            } else {
                self.head_open = false;
            }
        }
        if self.tail.len() >= TAIL_MAX {
            self.tail.pop_front();
        }
        self.tail.push_back(entry);
    }

    fn note(&mut self, text: &str, new_session: bool) {
        if new_session {
            self.sessions += 1;
            self.head.clear();
            self.head_open = true;
        }
        self.push(Entry { t_ms: now_ms(), text: format!("# {}", clip(text, RESP_MAX)), resp: String::new(), ms: 0, ok: true });
    }

    fn command(&mut self, text: &str, resp: &str, ms: u128, ok: bool) {
        self.push(Entry { t_ms: now_ms(), text: clip(text, RESP_MAX), resp: clip(resp, RESP_MAX), ms: ms.min(u32::MAX as u128) as u32, ok });
    }

    fn dump(&self) -> String {
        let mut out = String::new();
        out.push_str("Comandi mandati alla radio (rigctld): ora UTC, tempo della risposta, comando e risposta. «!» = errore o risposta inattesa.\n");
        out.push_str(&format!("Collegamenti aperti da quando Aurora e' accesa: {}\n\n", self.sessions));
        out.push_str(&format!("== Inizio dell'ultimo collegamento (primi {} comandi) ==\n", self.head.len()));
        for e in &self.head {
            out.push_str(&line(e));
            out.push('\n');
        }
        out.push_str(&format!("\n== Ultimi {} comandi ==\n", self.tail.len()));
        for e in &self.tail {
            out.push_str(&line(e));
            out.push('\n');
        }
        out
    }
}

/// Un comando mandato a `rigctld` e come e' andata (`resp` e' la risposta o il testo dell'errore).
pub fn command(text: &str, resp: &str, ms: u128, ok: bool) {
    if let Ok(mut b) = BUFFERS.lock() {
        b.command(text, resp, ms, ok);
    }
}

/// Un fatto che non e' un comando (collegamento aperto o chiuso, avvio di `rigctld`...). `new_session` apre un nuovo inizio di collegamento.
pub fn note(text: &str, new_session: bool) {
    if let Ok(mut b) = BUFFERS.lock() {
        b.note(text, new_session);
    }
}

/// La traccia da mettere nel pacchetto: l'inizio dell'ultimo collegamento e gli ultimi comandi (ora UTC, come il registro).
pub fn dump() -> String {
    BUFFERS.lock().map(|b| b.dump()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_start_of_a_connection_is_kept_apart_from_the_latest_commands() {
        let mut b = Buffers::default();
        b.note("collegamento aperto", true);
        for i in 0..(HEAD_MAX + 50) {
            b.command(&format!("f {i}"), "14074000", 3, true);
        }
        let out = b.dump();
        assert!(out.contains("collegamento aperto"));
        assert!(out.contains("f 0  ->"));
        assert!(out.contains(&format!("f {}  ->", HEAD_MAX + 49)), "gli ultimi comandi stanno nella coda");
        let head_part = out.split("== Ultimi").next().unwrap_or("");
        assert!(!head_part.contains(&format!("f {}  ->", HEAD_MAX + 49)), "l'inizio si ferma a {HEAD_MAX} comandi");
    }

    #[test]
    fn the_tail_keeps_only_the_latest_commands() {
        let mut b = Buffers::default();
        for i in 0..(TAIL_MAX + 25) {
            b.command(&format!("m {i}"), "USB", 1, true);
        }
        let out = b.dump();
        assert!(!out.contains("m 0  ->") && out.contains(&format!("m {}  ->", TAIL_MAX + 24)));
        assert_eq!(b.tail.len(), TAIL_MAX);
    }

    #[test]
    fn a_new_connection_starts_a_new_head_and_errors_are_marked() {
        let mut b = Buffers::default();
        b.note("primo collegamento", true);
        b.command("v", "VFOA", 5, true);
        b.note("secondo collegamento", true);
        b.command("V VFOB", "RPRT -9", 40, false);
        let out = b.dump();
        let head_part = out.split("== Ultimi").next().unwrap_or("");
        assert!(head_part.contains("secondo collegamento") && !head_part.contains("primo collegamento"));
        assert!(out.contains("Collegamenti aperti da quando Aurora e' accesa: 2"));
        assert!(out.lines().any(|l| l.contains("V VFOB") && l.contains("! ")), "{out}");
    }

    #[test]
    fn long_and_multiline_answers_are_cut_to_one_line() {
        let mut b = Buffers::default();
        b.command("+\\get_vfo_info VFOB", &format!("Freq: 1\nMode: USB\n{}", "x".repeat(400)), 12, true);
        let out = b.dump();
        let l = out.lines().find(|l| l.contains("get_vfo_info")).unwrap_or("");
        assert!(l.contains(" | ") && l.chars().count() < 400 && l.ends_with('…'), "{l}");
    }

    #[test]
    fn the_real_client_leaves_its_commands_in_the_trace() {
        // la radio finta: un comando vero passa dalla traccia del programma (le altre prove ne aggiungono: si cerca il proprio)
        let fake = crate::fake_rigctld::FakeRigctld::start(14_074_000);
        let client = crate::hamlib::RigctldClient::connect(fake.addr, std::time::Duration::from_secs(2)).expect("collegamento");
        assert!(client.vfo().is_ok());
        let out = dump();
        assert!(out.contains("collegato a rigctld"), "{out}");
        assert!(out.lines().any(|l| l.contains("  v  ->  VFOA")), "{out}");
    }
}
