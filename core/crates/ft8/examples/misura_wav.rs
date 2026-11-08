//! Misura della decodifica FT8 su periodi veri (voce 10 di `docs/tasks.md`): legge WAV a 12 kHz, 16 bit, mono (le registrazioni di Aurora in
//! `%APPDATA%\Aurora\recordings`) e decodifica ognuno con 1, 2, 4 e 8 processori, profondita' 2 e 3, stampando tempo e messaggi trovati.
//!
//! Uso: `cargo run --release -p ft8 --example misura_wav -- <file.wav> [altri.wav ...]`

use std::time::Instant;

use ft8::decode::{decode_slot, Options};
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
    let files: Vec<String> = std::env::args().skip(1).collect();
    assert!(!files.is_empty(), "uso: misura_wav <file.wav> ...");
    println!("file;profondita';thread;messaggi;ms");
    for depth in [2u8, 3] {
        for threads in [1usize, 2, 4, 8] {
            let mut total_ms = 0.0;
            let mut total_msgs = 0;
            for f in &files {
                let audio = read_wav(f);
                let options = Options { threads, ..Options::depth(depth) };
                let mut hashes = CallHashes::default();
                let start = Instant::now();
                let found = decode_slot(&audio, &options, &mut hashes);
                let ms = start.elapsed().as_secs_f64() * 1000.0;
                total_ms += ms;
                total_msgs += found.len();
                let name = std::path::Path::new(f).file_name().and_then(|n| n.to_str()).unwrap_or(f);
                println!("{name};{depth};{threads};{};{ms:.0}", found.len());
            }
            println!("TOTALE;{depth};{threads};{total_msgs};{total_ms:.0}");
        }
    }
}
