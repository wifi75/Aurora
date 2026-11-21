//! IC-820H (modello Hamlib 3032): la frequenza di trasmissione dei satelliti scritta sulla banda secondaria con trame CI-V grezze mandate
//! tramite `rigctld` (`+w`), perche' Hamlib per questo modello non ha la banda «Sub» (`V Sub` -> RPRT -9).
//!
//! Dalla tabella dei comandi CI-V del manuale della famiglia IC-821H: `03` legge la frequenza della banda selezionata (5 byte BCD, cifra
//! degli Hz nel semibyte basso del primo byte), `05` la scrive, `07 D0` seleziona la banda principale e `07 D1` la secondaria. Con il modo
//! satellite acceso la radio puo' scambiare le due selezioni («al contrario»): la prova automatica scopre quale dei due comandi porta
//! sull'altra banda e quale riporta indietro, e non scrive mai una frequenza senza aver riletto di essere sulla banda giusta.
//!
//! Regola di sicurezza: nessuna `05` parte se la lettura appena fatta non e' nella banda della trasmissione e diversa dalla banda di
//! ricezione; ogni uscita dopo un cambio di banda rimanda il comando di ritorno.

/// Indirizzo CI-V di fabbrica dell'IC-820H.
pub const RADIO_ADDR: u8 = 0x42;
/// Indirizzo del controllore (il PC).
pub const CTRL_ADDR: u8 = 0xE0;
/// `07 D0`: banda principale.
pub const SEL_MAIN: u8 = 0xD0;
/// `07 D1`: banda secondaria.
pub const SEL_SUB: u8 = 0xD1;
/// Scarto massimo fra frequenza scritta e riletta (Hz).
pub const TOLERANCE_HZ: u64 = 10;

/// La trama CI-V completa (`FE FE 42 E0 <comando> FD`) come la vuole il comando `w` di `rigctld`: `\0xFE\0xFE...`.
pub fn frame_text(body: &[u8]) -> String {
    let mut bytes = vec![0xFE, 0xFE, RADIO_ADDR, CTRL_ADDR];
    bytes.extend_from_slice(body);
    bytes.push(0xFD);
    bytes.iter().map(|b| format!("\\0x{b:02X}")).collect()
}

/// La frequenza in 5 byte BCD, dal byte meno significativo (1 e 10 Hz) al piu' significativo (100 MHz e 1 GHz). `None` oltre 9,99 GHz.
pub fn encode_freq(hz: u64) -> Option<[u8; 5]> {
    if hz >= 10_000_000_000 {
        return None;
    }
    let mut out = [0u8; 5];
    let mut rest = hz;
    for byte in &mut out {
        let low = (rest % 10) as u8;
        rest /= 10;
        let high = (rest % 10) as u8;
        rest /= 10;
        *byte = (high << 4) | low;
    }
    Some(out)
}

/// La frequenza dai byte BCD della risposta a `03` (5 byte; le radio piu' vecchie ne danno 4). `None` se un semibyte non e' una cifra.
pub fn decode_freq(bytes: &[u8]) -> Option<u64> {
    if !(4..=5).contains(&bytes.len()) {
        return None;
    }
    let mut hz: u64 = 0;
    for b in bytes.iter().rev() {
        let (high, low) = (u64::from(b >> 4), u64::from(b & 0x0F));
        if high > 9 || low > 9 {
            return None;
        }
        hz = hz * 100 + high * 10 + low;
    }
    Some(hz)
}

/// I byte scritti da `rigctld` nella risposta a `w` (`\0xFE\0xFE...` o `0xFE 0xFE ...`); le altre parole (il conteggio, «Reply:») si saltano.
pub fn parse_bytes(lines: &[String]) -> Vec<u8> {
    let mut out = Vec::new();
    for line in lines {
        let mut rest = line.as_str();
        while let Some(pos) = rest.find("0x").or_else(|| rest.find("0X")) {
            let hex = rest.get(pos + 2..pos + 4).unwrap_or("");
            if hex.len() == 2
                && let Ok(b) = u8::from_str_radix(hex, 16)
            {
                out.push(b);
                rest = rest.get(pos + 4..).unwrap_or("");
            } else {
                rest = rest.get(pos + 2..).unwrap_or("");
            }
        }
    }
    out
}

/// Cosa ha risposto la radio a una trama.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Answer {
    /// `FB`: comando accettato.
    Ack,
    /// `FA`: comando rifiutato.
    Nak,
    /// Risposta a `03`: la frequenza della banda selezionata.
    Freq(u64),
    /// Solo l'eco della trama mandata (o niente): la radio non ha risposto.
    Nothing,
    /// Una risposta della radio che non si sa leggere.
    Other(Vec<u8>),
}

/// Le trame della radio verso il PC (`FE FE E0 42 ... FD`) dentro i byte letti; l'eco della trama mandata (`FE FE 42 E0`) si scarta.
pub fn classify(bytes: &[u8]) -> Answer {
    let mut found = None;
    for chunk in bytes.split(|b| *b == 0xFD) {
        // la trama comincia dall'ultimo preambolo (byte di disturbo prima di FE FE si scartano)
        let Some(start) = chunk.windows(2).position(|w| w == [0xFE, 0xFE]) else { continue };
        let mut frame = &chunk[start..];
        while frame.len() > 2 && frame[2] == 0xFE {
            frame = &frame[1..];
        }
        if frame.len() < 5 || frame[2] != CTRL_ADDR || frame[3] != RADIO_ADDR {
            continue;
        }
        let body = &frame[4..];
        let answer = match body {
            [0xFB] => Answer::Ack,
            [0xFA] => Answer::Nak,
            [0x03, rest @ ..] => decode_freq(rest).map_or_else(|| Answer::Other(body.to_vec()), Answer::Freq),
            _ => Answer::Other(body.to_vec()),
        };
        found = Some(answer);
    }
    found.unwrap_or(Answer::Nothing)
}

/// Chi manda le trame alla radio e restituisce la risposta letta, con il testo grezzo per il registro.
pub trait CivPort {
    fn exchange(&mut self, body: &[u8]) -> Result<(Answer, String), String>;
}

/// Il risultato della prova riuscita: quale selezione porta sulla banda di trasmissione e quale riporta su quella di ricezione.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Probe {
    pub to_tx: u8,
    pub to_rx: u8,
    /// Frequenze lette durante la prova: banda di ricezione e banda di trasmissione.
    pub rx_hz: u64,
    pub tx_band_hz: u64,
}

/// La frequenza e' sulla banda alta (430 MHz)?
pub fn is_uhf(hz: u64) -> bool {
    hz >= 300_000_000
}

fn close(a: u64, b: u64) -> bool {
    a.abs_diff(b) <= TOLERANCE_HZ
}

fn sel_name(code: u8) -> &'static str {
    if code == SEL_SUB { "07 D1" } else { "07 D0" }
}

fn mhz(hz: u64) -> String {
    format!("{:.6} MHz", hz as f64 / 1e6)
}

/// Legge la frequenza della banda selezionata (`03`).
fn read(port: &mut dyn CivPort, log: &mut Vec<String>, what: &str) -> Result<u64, String> {
    match port.exchange(&[0x03]) {
        Ok((Answer::Freq(hz), raw)) => {
            log.push(format!("03 ({what}): {} [{raw}]", mhz(hz)));
            Ok(hz)
        }
        Ok((Answer::Nak, raw)) => {
            log.push(format!("03 ({what}): FA [{raw}]"));
            Err(format!("la radio risponde FA (rifiutato) alla lettura 03 ({what})"))
        }
        Ok((Answer::Nothing, raw)) => {
            log.push(format!("03 ({what}): nessuna risposta [{raw}]"));
            Err(format!("nessuna risposta della radio alla lettura 03 ({what}): solo l'eco o niente"))
        }
        Ok((other, raw)) => {
            log.push(format!("03 ({what}): risposta non leggibile {other:?} [{raw}]"));
            Err(format!("risposta non leggibile alla lettura 03 ({what})"))
        }
        Err(e) => {
            log.push(format!("03 ({what}): errore {e}"));
            Err(format!("lettura 03 ({what}) non riuscita: {e}"))
        }
    }
}

/// Seleziona una banda (`07 D0`/`07 D1`): serve `FB`.
fn select(port: &mut dyn CivPort, log: &mut Vec<String>, code: u8) -> Result<(), String> {
    let name = sel_name(code);
    match port.exchange(&[0x07, code]) {
        Ok((Answer::Ack, raw)) => {
            log.push(format!("{name}: FB [{raw}]"));
            Ok(())
        }
        Ok((Answer::Nak, raw)) => {
            log.push(format!("{name}: FA [{raw}]"));
            Err(format!("la radio risponde FA (rifiutato) a {name}"))
        }
        Ok((Answer::Nothing, raw)) => {
            log.push(format!("{name}: nessuna risposta [{raw}]"));
            Err(format!("nessuna risposta della radio a {name}: solo l'eco o niente"))
        }
        Ok((other, raw)) => {
            log.push(format!("{name}: risposta inattesa {other:?} [{raw}]"));
            Err(format!("risposta inattesa a {name}"))
        }
        Err(e) => {
            log.push(format!("{name}: errore {e}"));
            Err(format!("{name} non riuscito: {e}"))
        }
    }
}

/// Tentativo di ritorno sulla banda di ricezione dopo un errore della prova (le selezioni non sono ancora note): `07 D0`, e se la
/// lettura non e' nella banda di partenza anche `07 D1`. Solo selezioni e letture, nessuna scrittura. Senza conferma si rimanda `07 D0`.
fn restore_unknown(port: &mut dyn CivPort, log: &mut Vec<String>, rx_hz: u64) {
    for code in [SEL_MAIN, SEL_SUB] {
        if select(port, log, code).is_ok() && read(port, log, "ritorno").is_ok_and(|hz| is_uhf(hz) == is_uhf(rx_hz)) {
            return;
        }
    }
    let _ = select(port, log, SEL_MAIN);
    log.push("ritorno sulla banda di ricezione non confermato: controlla la radio".into());
}

/// La prova di sicurezza, senza scrivere frequenze: legge la banda selezionata (quella di ricezione), passa all'altra e controlla che la
/// lettura sia nell'altra banda, torna e controlla che la ricezione sia rimasta com'era. Prima prova `07 D1` per andare e `07 D0` per
/// tornare; se `07 D1` lascia la radio sulla stessa banda (modo satellite «al contrario»), prova la coppia inversa.
pub fn probe(port: &mut dyn CivPort, log: &mut Vec<String>) -> Result<Probe, String> {
    let rx = read(port, log, "banda selezionata, prima")?;
    let mut same_band = Vec::new();
    for (to_tx, to_rx) in [(SEL_SUB, SEL_MAIN), (SEL_MAIN, SEL_SUB)] {
        if let Err(e) = select(port, log, to_tx) {
            restore_unknown(port, log, rx);
            return Err(e);
        }
        let other = match read(port, log, &format!("dopo {}", sel_name(to_tx))) {
            Ok(hz) => hz,
            Err(e) => {
                restore_unknown(port, log, rx);
                return Err(e);
            }
        };
        if is_uhf(other) == is_uhf(rx) {
            if close(other, rx) {
                // stessa banda: questa selezione non porta all'altra banda, si prova la coppia inversa
                same_band.push(sel_name(to_tx));
                continue;
            }
            restore_unknown(port, log, rx);
            return Err(format!("dopo {} la frequenza e' cambiata ({} -> {}) ma la banda e' la stessa: non e' la banda secondaria", sel_name(to_tx), mhz(rx), mhz(other)));
        }
        if let Err(e) = select(port, log, to_rx) {
            restore_unknown(port, log, rx);
            return Err(e);
        }
        let back = match read(port, log, &format!("dopo {}", sel_name(to_rx))) {
            Ok(hz) => hz,
            Err(e) => {
                restore_unknown(port, log, rx);
                return Err(e);
            }
        };
        if !close(back, rx) {
            restore_unknown(port, log, rx);
            return Err(format!("dopo il ritorno con {} la banda di ricezione legge {} invece di {}", sel_name(to_rx), mhz(back), mhz(rx)));
        }
        return Ok(Probe { to_tx, to_rx, rx_hz: rx, tx_band_hz: other });
    }
    Err(format!("{} non cambiano banda: la frequenza letta resta {}", same_band.join(" e "), mhz(rx)))
}

/// Il risultato di una scrittura riuscita: frequenza di trasmissione riletta e frequenza di ricezione riletta dopo il ritorno.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Written {
    pub tx_hz: u64,
    pub rx_hz: u64,
}

/// Scrive la frequenza di trasmissione sulla banda secondaria: lettura della ricezione, selezione dell'altra banda, lettura di controllo
/// (deve essere nella banda della trasmissione e diversa da quella di ricezione), `05`, rilettura di conferma, ritorno, rilettura della
/// ricezione. Qualunque passo fallito rimanda il ritorno e restituisce l'errore (il chiamante spegne il ripiego).
pub fn write_tx(port: &mut dyn CivPort, log: &mut Vec<String>, probe: &Probe, tx_hz: u64) -> Result<Written, String> {
    if is_uhf(tx_hz) != is_uhf(probe.tx_band_hz) {
        return Err(format!("la trasmissione {} non e' nella banda secondaria (la prova l'ha letta a {}): niente scritto", mhz(tx_hz), mhz(probe.tx_band_hz)));
    }
    let Some(bcd) = encode_freq(tx_hz) else {
        return Err(format!("frequenza di trasmissione non valida: {tx_hz} Hz"));
    };
    let rx = read(port, log, "ricezione, prima")?;
    if is_uhf(rx) == is_uhf(tx_hz) {
        return Err(format!("la banda selezionata ({}) e' quella della trasmissione: niente scritto", mhz(rx)));
    }
    let back = |port: &mut dyn CivPort, log: &mut Vec<String>| {
        if select(port, log, probe.to_rx).is_err() {
            let _ = select(port, log, probe.to_rx);
        }
    };
    if let Err(e) = select(port, log, probe.to_tx) {
        back(port, log);
        return Err(e);
    }
    let here = match read(port, log, "banda di trasmissione, controllo") {
        Ok(hz) => hz,
        Err(e) => {
            back(port, log);
            return Err(e);
        }
    };
    if is_uhf(here) != is_uhf(tx_hz) || close(here, rx) {
        back(port, log);
        return Err(format!("dopo {} la radio legge {}: non e' la banda di trasmissione, niente scritto", sel_name(probe.to_tx), mhz(here)));
    }
    let mut body = vec![0x05];
    body.extend_from_slice(&bcd);
    let wrote = match port.exchange(&body) {
        Ok((Answer::Ack, raw)) => {
            log.push(format!("05 {}: FB [{raw}]", mhz(tx_hz)));
            Ok(())
        }
        Ok((answer, raw)) => {
            log.push(format!("05 {}: {answer:?} [{raw}]", mhz(tx_hz)));
            Err(format!("la radio non conferma la scrittura 05 ({answer:?})"))
        }
        Err(e) => {
            log.push(format!("05 {}: errore {e}", mhz(tx_hz)));
            Err(format!("scrittura 05 non riuscita: {e}"))
        }
    };
    if let Err(e) = wrote {
        back(port, log);
        return Err(e);
    }
    let confirmed = match read(port, log, "trasmissione, conferma") {
        Ok(hz) => hz,
        Err(e) => {
            back(port, log);
            return Err(e);
        }
    };
    if !close(confirmed, tx_hz) {
        back(port, log);
        return Err(format!("la trasmissione riletta e' {} invece di {}", mhz(confirmed), mhz(tx_hz)));
    }
    if let Err(e) = select(port, log, probe.to_rx) {
        // un secondo tentativo: la radio non deve restare sulla banda di trasmissione
        if select(port, log, probe.to_rx).is_err() {
            return Err(format!("{e}: la radio potrebbe essere rimasta sulla banda di trasmissione, controlla il display"));
        }
    }
    let rx_after = read(port, log, "ricezione, dopo il ritorno")?;
    if !close(rx_after, rx) {
        return Err(format!("dopo il ritorno la ricezione legge {} invece di {}", mhz(rx_after), mhz(rx)));
    }
    Ok(Written { tx_hz: confirmed, rx_hz: rx_after })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_frequency_goes_to_bcd_and_back() {
        let bcd = encode_freq(145_950_000).unwrap();
        assert_eq!(bcd, [0x00, 0x00, 0x95, 0x45, 0x01]);
        assert_eq!(decode_freq(&bcd), Some(145_950_000));
        for hz in [0, 1, 437_790_123, 1_296_000_000, 9_999_999_999] {
            assert_eq!(decode_freq(&encode_freq(hz).unwrap()), Some(hz));
        }
        assert_eq!(encode_freq(10_000_000_000), None);
        assert_eq!(decode_freq(&[0x0A, 0, 0, 0, 0]), None);
    }

    #[test]
    fn the_frame_text_is_the_one_rigctld_wants() {
        assert_eq!(frame_text(&[0x07, 0xD1]), "\\0xFE\\0xFE\\0x42\\0xE0\\0x07\\0xD1\\0xFD");
    }

    #[test]
    fn the_echo_is_skipped_and_the_radio_answer_is_read() {
        let echo_only = parse_bytes(&["Reply: \\0xFE\\0xFE\\0x42\\0xE0\\0x07\\0xD1\\0xFD 7".into()]);
        assert_eq!(classify(&echo_only), Answer::Nothing);
        let ack = parse_bytes(&["Reply: \\0xFE\\0xFE\\0x42\\0xE0\\0x07\\0xD1\\0xFD\\0xFE\\0xFE\\0xE0\\0x42\\0xFB\\0xFD 13".into()]);
        assert_eq!(classify(&ack), Answer::Ack);
        let nak = parse_bytes(&["0xFE 0xFE 0xE0 0x42 0xFA 0xFD".into()]);
        assert_eq!(classify(&nak), Answer::Nak);
        let freq = parse_bytes(&["\\0xFE\\0xFE\\0x42\\0xE0\\0x03\\0xFD 6".into(), "\\0xFE\\0xFE\\0xE0\\0x42\\0x03\\0x00\\0x00\\0x95\\0x45\\0x01\\0xFD 11".into()]);
        assert_eq!(classify(&freq), Answer::Freq(145_950_000));
        assert_eq!(classify(&[]), Answer::Nothing);
    }
}
