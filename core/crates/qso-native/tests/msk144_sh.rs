//! MSK144 con «Sh» (`m_bShMsgs` di WSJT-X): Tx3-Tx5 brevi «<DX MIO> R+03 / RRR / 73» (`mainwindow.cpp:10017-10065`) e la sequenza che avanza
//! sui messaggi brevi ricevuti «<MIO DX> R+03 / RRR / 73», che WSJT-X legge togliendo le parentesi (`parseDecodedMessage`, `decodedtext.cpp:57`).
//! Periodi MSK144 di 15 s (il periodo di fabbrica); le 12:00:00 UTC sono il periodo 2880, pari.

use qso_native::*;

const ME: &str = "IU3ABC";

struct Bench {
    seq: Sequencer,
    mode: Mode,
    events: Vec<Event>,
}

impl Bench {
    fn new(mode: Mode, msk_short: bool) -> Self {
        let opts = Options { autolog: false, msk_short, ..Options::default() };
        let seq = Sequencer::new(mode, Station::new(ME, "IO91wm"), opts).expect("stazione valida");
        Bench { seq, mode, events: Vec::new() }
    }

    fn slot(&self, n: u64) -> Slot {
        Slot(2880 + n)
    }

    fn collect(&mut self) {
        let mut ev = self.seq.drain_events();
        self.events.append(&mut ev);
    }

    fn rx(&mut self, n: u64, msgs: &[(&str, i32)]) {
        let decodes: Vec<Decoded> = msgs.iter().map(|(t, s)| Decoded::new(t, *s, 1500)).collect();
        let now = self.slot(n + 1).start_s(self.mode) + 0.3;
        let s = self.slot(n);
        self.seq.on_decodes_at(s, &decodes, Some(now));
        self.collect();
    }

    fn tx(&mut self, n: u64) -> Option<String> {
        let s = self.slot(n);
        let now = s.start_s(self.mode) + 0.4;
        let tx = self.seq.next_tx(s, now).map(|t| t.text);
        self.collect();
        tx
    }

    fn reply(&mut self, n: u64, text: &str, snr: i32) {
        let s = self.slot(n);
        self.seq.reply_to(s, &Decoded::new(text, snr, 1500)).expect("risposta possibile");
        self.collect();
    }

    fn logged(&self) -> usize {
        self.events.iter().filter(|e| matches!(e, Event::QsoComplete(_))).count()
    }
}

/// Chi chiama CQ: il rapporto a 77 bit, poi l'R+rapporto breve del DX fa partire «<DX MIO> RRR» e il suo 73 breve chiude il QSO.
#[test]
fn cq_caller_advances_on_short_messages() {
    let mut b = Bench::new(Mode::Msk144, true);
    b.seq.call_cq(None).expect("cq");
    assert_eq!(b.tx(0).as_deref(), Some("CQ IU3ABC IO91"));
    b.rx(1, &[("IU3ABC K1ABC FN42", 4)]);
    assert_eq!(b.tx(2).as_deref(), Some("K1ABC IU3ABC +04"), "Tx2 resta a 77 bit con il rapporto vero");
    b.rx(3, &[("<IU3ABC K1ABC> R+03", 2)]);
    assert_eq!(b.tx(4).as_deref(), Some("<K1ABC IU3ABC> RRR"));
    assert_eq!(b.logged(), 0, "RRR non e' la chiusura");
    b.rx(5, &[("<IU3ABC K1ABC> 73", 2)]);
    assert_eq!(b.logged(), 1, "il 73 breve del DX chiude e registra");
}

/// Chi risponde: il rapporto a 77 bit del DX fa partire «<DX MIO> R+rapporto» arrotondato, il suo RRR breve il «<DX MIO> 73».
#[test]
fn answerer_sends_short_r_report_and_73() {
    let mut b = Bench::new(Mode::Msk144, true);
    b.reply(0, "CQ K1ABC FN42", 9);
    assert_eq!(b.tx(1).as_deref(), Some("K1ABC IU3ABC IO91"));
    b.rx(2, &[("IU3ABC K1ABC -05", 9)]);
    assert_eq!(b.tx(3).as_deref(), Some("<K1ABC IU3ABC> R+10"), "rapporto 9 arrotondato a +10 (`mainwindow.cpp:10025`)");
    b.rx(4, &[("<IU3ABC K1ABC> RRR", 9)]);
    assert_eq!(b.tx(5).as_deref(), Some("<K1ABC IU3ABC> 73"));
    assert_eq!(b.logged(), 1);
}

/// Senza «Sh» MSK144 usa i messaggi a 77 bit (Tx4 RR73 di fabbrica); «Sh» non tocca FT8.
#[test]
fn without_sh_or_outside_msk144_messages_are_standard() {
    let mut off = Bench::new(Mode::Msk144, false);
    off.reply(0, "CQ K1ABC FN42", 9);
    off.tx(1);
    off.rx(2, &[("IU3ABC K1ABC -05", 9)]);
    assert_eq!(off.tx(3).as_deref(), Some("K1ABC IU3ABC R+09"));
    off.rx(4, &[("IU3ABC K1ABC RR73", 9)]);
    assert_eq!(off.tx(5).as_deref(), Some("K1ABC IU3ABC 73"));

    let mut ft8 = Bench::new(Mode::Ft8, true);
    ft8.reply(0, "CQ K1ABC FN42", 9);
    ft8.tx(1);
    ft8.rx(2, &[("IU3ABC K1ABC -05", 9)]);
    assert_eq!(ft8.tx(3).as_deref(), Some("K1ABC IU3ABC R+09"));
}
