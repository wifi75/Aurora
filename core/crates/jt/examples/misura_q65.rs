//! Confronto della decodifica Q65 con `jt9.exe` (voce 21 di `docs/tasks.md`, tabella in `docs/q65-confronto.md`): legge WAV a 12 kHz,
//! 16 bit, mono (per esempio quelli di `q65sim` di WSJT-X) e li decodifica con il periodo e il sottomodo indicati.
//!
//! Uso: `cargo run --release -p jt --example misura_q65 -- <periodo> <sottomodo A-E> [--gauss] [--media] [--senza-evanescenza]
//! [--rx <Hz>] [--thread <n>] [--mio <nominativo>] [--dx <nominativo>] [--locatore <griglia>] <file.wav> ...`
//!
//! Con `--media` i file sono periodi successivi della stessa parita' e si sommano come fa la media di Q65 (`decode_averaged`).
//! `--rx` e' la frequenza di ricezione (come `-f` di `jt9`, tolleranza 50 Hz); `--thread` i thread per i candidati. Con `--mio`,
//! `--dx` e `--locatore` (come `-c`, `-x` e il locatore di `jt9`, che di suo usa K1ABC, W9XYZ, EN37) si provano le ipotesi AP e la
//! lista dei messaggi del QSO.
//! Stampa una riga per file: `nome;messaggi;testo|snr|dt|freq|ap|periodi|b90` per ogni messaggio.

use jt::q65::{Averager, Fading, Options, Setup, decode, decode_averaged};
use ft8::message::CallHashes;

/// I campioni di un WAV PCM 16 bit mono (si cerca il blocco `data`).
fn read_wav(path: &str) -> Vec<f32> {
    let bytes = std::fs::read(path).expect("file WAV");
    let mut i = 12;
    while i + 8 <= bytes.len() {
        let id = &bytes[i..i + 4];
        let len = u32::from_le_bytes([bytes[i + 4], bytes[i + 5], bytes[i + 6], bytes[i + 7]]) as usize;
        if id == b"data" {
            let data = &bytes[i + 8..(i + 8 + len).min(bytes.len())];
            return data.chunks_exact(2).map(|c| f32::from(i16::from_le_bytes([c[0], c[1]])) / 32768.0).collect();
        }
        i += 8 + len + (len & 1);
    }
    panic!("nessun blocco data in {path}");
}

fn main() {
    let mut args = std::env::args().skip(1);
    let period: u32 = args.next().and_then(|p| p.parse().ok()).expect("periodo in secondi");
    let sub = args.next().and_then(|s| s.bytes().next()).map(|b| b.to_ascii_uppercase().wrapping_sub(b'A')).expect("sottomodo A-E");
    let setup = Setup::new(period, sub).expect("periodo o sottomodo non validi");
    let mut fading = Some(Fading::Lorentzian);
    let mut average = false;
    let mut rx_hz = None;
    let mut threads = 1;
    let (mut my_call, mut dx_call, mut dx_grid) = (String::new(), String::new(), String::new());
    let mut files = Vec::new();
    while let Some(a) = args.next() {
        match a.as_str() {
            "--gauss" => fading = Some(Fading::Gaussian),
            "--senza-evanescenza" => fading = None,
            "--media" => average = true,
            "--rx" => rx_hz = args.next().and_then(|f| f.parse().ok()),
            "--thread" => threads = args.next().and_then(|n| n.parse().ok()).unwrap_or(1),
            "--mio" => my_call = args.next().unwrap_or_default(),
            "--dx" => dx_call = args.next().unwrap_or_default(),
            "--locatore" => dx_grid = args.next().unwrap_or_default(),
            _ => files.push(a),
        }
    }
    assert!(!files.is_empty(), "nessun file WAV");
    let options = Options { setup, fading, rx_hz, threads, my_call, dx_call, dx_grid, ..Options::default() };
    let t0 = std::time::Instant::now();
    let mut averager = Averager::new();
    for (n, f) in files.iter().enumerate() {
        let audio = read_wav(f);
        let mut hashes = CallHashes::default();
        // nella media ogni file e' un periodo della stessa parita' (indici pari)
        let found = if average { decode_averaged(&audio, &options, &mut hashes, &mut averager, 2 * n as u64) } else { decode(&audio, &options, &mut hashes) };
        let name = std::path::Path::new(f).file_name().and_then(|n| n.to_str()).unwrap_or(f);
        let texts: Vec<String> = found
            .iter()
            .map(|d| format!("{}|{}|{:.1}|{:.0}|{}|{}|{}", d.text, d.snr_db, d.dt_s, d.freq_hz, d.ap, d.averaged, d.b90_hz.map_or("-".to_string(), |b| format!("{b:.1}"))))
            .collect();
        println!("{name};{};{}", found.len(), texts.join(";"));
    }
    eprintln!("tempo medio per file: {:.2} s", t0.elapsed().as_secs_f64() / files.len() as f64);
}
