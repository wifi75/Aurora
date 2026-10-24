//! Test sui pacchetti reali catturati da JTDX con `udp_capture` (Task 2, Step 5).
//! Le catture vanno copiate in `tests/fixtures/` come file `.bin`.

use std::{fs, path::PathBuf};

use udp_bridge::{decode, Message};

fn fixtures() -> Vec<(String, Vec<u8>)> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut out = Vec::new();
    for entry in fs::read_dir(dir).expect("cartella fixtures mancante") {
        let path = entry.expect("voce illeggibile").path();
        if path.extension().is_some_and(|e| e == "bin") {
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            out.push((name, fs::read(&path).expect("file illeggibile")));
        }
    }
    out
}

#[test]
#[ignore = "mancano le catture reali di JTDX in tests/fixtures: eseguire udp_capture (Task 2, Step 5) e togliere questo ignore"]
fn every_real_capture_decodes_and_is_plausible() {
    let all = fixtures();
    assert!(!all.is_empty(), "nessuna cattura in tests/fixtures");
    for (name, bytes) in all {
        let packet = decode(&bytes).unwrap_or_else(|e| panic!("{name}: {e}"));
        match packet.message {
            Message::Status(s) => {
                assert!((1_000_000..=450_000_000).contains(&s.dial_freq), "{name}: frequenza {}", s.dial_freq);
            }
            Message::Decode(d) => {
                assert!(d.message.as_deref().is_some_and(|m| !m.is_empty()), "{name}: testo vuoto");
            }
            _ => {}
        }
    }
}
