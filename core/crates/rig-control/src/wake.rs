//! «Accendi» per le radio che Hamlib non riesce ad aprire da spente: prima di avviare `rigctld`, Aurora apre la porta seriale e manda da se'
//! il comando di accensione nella lingua della radio. Con l'FT-2000 spento Hamlib 4.7.2 non apre nemmeno il collegamento («rig power is
//! off?», «Some rigs cannot get_powerstat while off») e `auto_power_on` non parte mai (prova del 8/11/2026).
//!
//! - Yaesu (modelli Hamlib 1xxx), Kenwood ed Elecraft (2xxx): `PS1;` a coppie. Il manuale Yaesu vuole un primo invio che sveglia il
//!   processore della radio e il comando vero dopo piu' di un secondo e meno di due. Con una sola pausa di 1,0 s esatti l'FT-2000 a volte
//!   restava spento (registro del 2026-11-11) e con tre invii a 0, 0,3 e 1,5 s non si accendeva (proprietario, 2026-11-12: l'invio a
//!   0,3 s cadeva dentro la finestra e la faceva ripartire). Ora due coppie pulite: 0 e 1,2 s, poi dopo 2,5 s un'altra a 1,5 s di
//!   distanza; a radio accesa `PS1;` non fa nulla.
//! - Icom (3xxx): una serie di byte `FE` di risveglio, poi il comando CI-V 0x18 0x01 all'indirizzo della radio (serve l'indirizzo CI-V).
//!
//! Gli altri modelli non si toccano: resta `auto_power_on` di Hamlib.

use std::{io::Write, time::Duration};

use crate::cat_config::RigConfig;

/// Le pause prima del secondo, del terzo e del quarto messaggio: invii a 0, 1,2, 3,7 e 5,2 s (due coppie, una a 1,2 e una a 1,5 s).
const PAUSES: [Duration; 3] = [Duration::from_millis(1200), Duration::from_millis(2500), Duration::from_millis(1500)];

/// I messaggi da mandare per accendere questa radio, se Aurora li conosce.
pub fn power_on_bytes(config: &RigConfig) -> Option<Vec<Vec<u8>>> {
    match config.model / 1000 {
        1 | 2 => Some(vec![b"PS1;".to_vec(); 4]),
        3 if config.civ_address != 0 => {
            // risveglio: piu' byte FE quanto piu' alta e' la velocita' (come il manuale Icom: 150 a 115200, 25 a 19200)
            let wake = match config.baud {
                b if b >= 115_200 => 150,
                b if b >= 57_600 => 75,
                b if b >= 38_400 => 50,
                b if b >= 19_200 => 25,
                _ => 13,
            };
            let mut first = vec![0xFE; wake];
            first.extend_from_slice(&[0xFE, 0xFE, config.civ_address, 0xE0, 0x18, 0x01, 0xFD]);
            Some(vec![first])
        }
        _ => None,
    }
}

/// Le linee DTR e RTS durante l'accensione. Le Yaesu come l'FT-2000 hanno di fabbrica «CAT RTS» acceso: con RTS basso la radio ignora i
/// comandi, e Hamlib per questi modelli usa l'handshake hardware (RTS alto). Prima RTS era sempre basso qui: la radio si spegneva via CAT
/// (con Hamlib, RTS alto) ma «Accendi» non arrivava mai (proprietario, FT-2000, 2026-11-13). RTS alto con handshake hardware o
/// «predefinito» su una Yaesu, salvo che RTS sia il PTT di questa porta; le scelte esplicite dell'utente vincono. DTR come in `to_rigctld`.
/// CTS e DSR non si guardano (`OCTS=OFF`): a radio spenta potrebbero restare bassi e bloccare la scrittura.
fn wake_lines(config: &RigConfig) -> (&'static str, &'static str) {
    use crate::cat_config::{Handshake, PttMethod};
    let same_port = config.ptt_port.is_empty() || config.ptt_port == config.port;
    let rts_is_ptt = same_port && config.ptt == PttMethod::Rts;
    let dtr_is_ptt = same_port && config.ptt == PttMethod::Dtr;
    let on = |b: bool| if b { "ON" } else { "OFF" };
    let dtr = if dtr_is_ptt { "OFF" } else { on(config.force_dtr.unwrap_or(false)) };
    let rts = if rts_is_ptt {
        "OFF"
    } else {
        match (config.force_rts, config.handshake) {
            (_, Handshake::Hardware) => "ON",
            (Some(high), _) => on(high),
            (None, Handshake::Default) => on(config.model / 1000 == 1),
            (None, _) => "OFF",
        }
    };
    (dtr, rts)
}

/// Manda l'accensione sulla porta seriale (solo Windows: la porta si imposta con `mode`); errori solo nel registro.
pub fn wake(config: &RigConfig) {
    let Some(messages) = power_on_bytes(config) else {
        tracing::info!("«Accendi»: per il modello Hamlib {} si usa solo auto_power_on di Hamlib", config.model);
        return;
    };
    if config.port.is_empty() || config.port.contains(':') {
        return;
    }
    #[cfg(windows)]
    {
        let port = config.port.trim().to_uppercase();
        let baud = if config.baud == 0 { 38_400 } else { config.baud };
        let stop = if config.stop_bits == 2 { 2 } else { 1 };
        let (dtr, rts) = wake_lines(config);
        let _ = std::process::Command::new("mode")
            .args([port.as_str(), &format!("BAUD={baud}"), "PARITY=N", "DATA=8", &format!("STOP={stop}"), &format!("DTR={dtr}"), &format!("RTS={rts}"), "OCTS=OFF", "ODSR=OFF"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
        let path = format!("\\\\.\\{port}");
        // porta negata: la tiene un altro programma (rapporti del 2026-11-11: OmniRig rimasto aperto con la COM8). Si dice quale, a schermo
        // e non come errore; un rigctld orfano di Aurora si chiude e si riprova una volta
        let opened = crate::busy_port::open_with_diagnosis(&port, || std::fs::OpenOptions::new().write(true).open(&path), |e| format!("{e} {}", busy_hint(e)))
            .map_err(|(e, diagnosis)| {
                if let Some(d) = diagnosis {
                    crate::busy_port::notify(&d.message);
                }
                e
            });
        match opened {
            Ok(mut f) => {
                for (i, m) in messages.iter().enumerate() {
                    if i > 0 {
                        std::thread::sleep(PAUSES.get(i - 1).copied().unwrap_or(Duration::from_millis(1000)));
                    }
                    let _ = f.write_all(m);
                    let _ = f.flush();
                }
                tracing::info!("«Accendi»: comando di accensione mandato sulla {port}");
            }
            // occupata: gia' detto (avviso e registro); non e' un errore di Aurora da segnalare
            Err(e) if crate::busy_port::is_access_denied(&format!("{e} {}", busy_hint(&e))) => tracing::info!("«Accendi»: porta {port} occupata, accensione non mandata: {e}"),
            Err(e) => tracing::warn!("«Accendi»: porta {port} non aperta: {e}"),
        }
        // il tempo che la radio si accenda prima che Hamlib la interroghi
        std::thread::sleep(Duration::from_secs(4));
    }
    #[cfg(not(windows))]
    {
        let _ = (messages, Duration::ZERO, std::io::sink().flush());
    }
}

/// Il codice di sistema dell'errore detto in parole fisse (il testo di Windows cambia con la lingua): 5 e 32 sono «accesso negato» e
/// «file usato da un altro processo», cioe' porta di un altro programma.
#[cfg_attr(not(windows), allow(dead_code))]
fn busy_hint(e: &std::io::Error) -> &'static str {
    match e.raw_os_error() {
        Some(5 | 32) => "access denied",
        _ if e.kind() == std::io::ErrorKind::PermissionDenied => "access denied",
        _ => "",
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn a_yaesu_gets_rts_high_while_waking_unless_rts_is_the_ptt() {
        let mut c = RigConfig { model: 1026, port: "COM3".into(), ..RigConfig::default() };
        assert_eq!(wake_lines(&c), ("OFF", "ON"));
        c.ptt = crate::cat_config::PttMethod::Rts;
        assert_eq!(wake_lines(&c).1, "OFF");
        let k = RigConfig { model: 2014, port: "COM3".into(), ..RigConfig::default() };
        assert_eq!(wake_lines(&k).1, "OFF");
    }
    use super::*;

    #[test]
    fn yaesu_and_kenwood_get_ps1_and_icom_needs_its_address() {
        let ft2000 = RigConfig { model: 1029, ..RigConfig::default() };
        assert_eq!(power_on_bytes(&ft2000), Some(vec![b"PS1;".to_vec(); 4]));
        // ogni coppia ha il secondo invio a piu' di un secondo e meno di due dal primo
        assert!([PAUSES[0], PAUSES[2]].iter().all(|p| *p > Duration::from_millis(1000) && *p < Duration::from_millis(2000)));
        let ts590 = RigConfig { model: 2031, ..RigConfig::default() };
        assert!(power_on_bytes(&ts590).is_some());
        let icom_no_addr = RigConfig { model: 3073, ..RigConfig::default() };
        assert_eq!(power_on_bytes(&icom_no_addr), None);
        let ic7300 = RigConfig { model: 3073, civ_address: 0x94, baud: 19_200, ..RigConfig::default() };
        let m = power_on_bytes(&ic7300).expect("icom");
        assert_eq!(&m[0][m[0].len() - 7..], &[0xFE, 0xFE, 0x94, 0xE0, 0x18, 0x01, 0xFD]);
        assert_eq!(power_on_bytes(&RigConfig { model: 1, ..RigConfig::default() }), None);
    }

    #[test]
    fn a_denied_port_is_recognised_whatever_the_language_of_windows() {
        let denied = std::io::Error::from_raw_os_error(5);
        assert_eq!(busy_hint(&denied), "access denied");
        assert!(crate::busy_port::is_access_denied(&format!("Zugriff verweigert {}", busy_hint(&denied))));
        assert_eq!(busy_hint(&std::io::Error::from_raw_os_error(32)), "access denied");
        assert_eq!(busy_hint(&std::io::Error::from(std::io::ErrorKind::PermissionDenied)), "access denied");
        let missing = std::io::Error::from(std::io::ErrorKind::NotFound);
        assert_eq!(busy_hint(&missing), "");
    }
}
