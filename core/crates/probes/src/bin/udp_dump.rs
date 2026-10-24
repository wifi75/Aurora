//! Probe usa-e-getta: decodifica con `udp-bridge` tutti i file `.bin` di una cartella e li stampa.
//! Uso: udp_dump <cartella>

use std::{fs, process::ExitCode};

use udp_bridge::{decode, Message};

fn main() -> ExitCode {
    let Some(dir) = std::env::args().nth(1) else {
        eprintln!("uso: udp_dump <cartella>");
        return ExitCode::from(2);
    };
    let mut files: Vec<_> = match fs::read_dir(&dir) {
        Ok(rd) => rd.filter_map(Result::ok).map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "bin")).collect(),
        Err(e) => {
            eprintln!("{dir}: {e}");
            return ExitCode::from(1);
        }
    };
    files.sort();
    let (mut ok, mut failed) = (0, 0);
    for path in files {
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let bytes = match fs::read(&path) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("{name}: {e}");
                failed += 1;
                continue;
            }
        };
        match decode(&bytes) {
            Ok(packet) => {
                ok += 1;
                match packet.message {
                    Message::Decode(d) => println!(
                        "{name} DECODE schema={} new={} t={} snr={} dt={:.1} df={} mode={:?} testo={:?} bassa_conf={} fuori_onda={}",
                        packet.schema, d.is_new, d.time_ms, d.snr, d.delta_time, d.delta_freq, d.mode, d.message,
                        d.low_confidence, d.off_air
                    ),
                    other => println!("{name} schema={} {other:?}", packet.schema),
                }
            }
            Err(e) => {
                failed += 1;
                println!("{name} ERRORE: {e}");
            }
        }
    }
    println!("--- decodificati {ok}, falliti {failed}");
    if failed == 0 { ExitCode::SUCCESS } else { ExitCode::from(1) }
}
