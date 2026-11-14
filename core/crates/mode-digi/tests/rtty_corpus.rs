//! Registrazioni vere di RTTY (passo 4 di `docs/progetto-rtty-oltre.md`): ogni cartella di `rtty-corpus/` (o di quella indicata da
//! `AURORA_RTTY_CORPUS`) contiene
//! - `audio.wav`: mono o stereo (si usa il canale sinistro; il destro serve alla diversity), PCM a 16 bit o virgola mobile a 32 bit, a
//!   qualunque frequenza di campionamento (si porta a 12 kHz);
//! - `meta.toml`: righe `chiave = valore` con `centre_hz` (centro fra i toni sull'audio), `shift_hz` (170 di base), `baud` (45.45 di base),
//!   `reverse` (`true` se il mark e' il tono basso), `nota` (libera);
//! - `riferimento.txt` (facoltativo): il testo trasmesso, se noto (bollettini W1AW, prove fatte in casa);
//! - `nominativi.txt` (facoltativo): i nominativi da trovare, uno per riga (dal log del contest);
//! - `uscita-<programma>.txt` (facoltativi): il testo ricevuto da altri programmi (2Tone, MMTTY, fldigi) sullo stesso file.
//!
//! La prova decodifica ogni file con Aurora (e con fldigi e MMTTY se compilati), scrive `uscita-aurora.txt` (e le altre) nella cartella e
//! stampa per ogni programma il CER rispetto al riferimento e i nominativi trovati.
//!
//! `cargo test --release -j 12 -p mode-digi --features fldigi-oracle,mmtty-oracle --test rtty_corpus -- --ignored --nocapture`

mod common;

use common::levenshtein;
use mode_digi::rtty::{RttyDemodulator, RttyParams};
use std::path::{Path, PathBuf};

/// Un file WAV letto: campioni per canale e frequenza di campionamento.
pub struct Wav {
    pub rate: u32,
    pub canali: Vec<Vec<f32>>,
}

/// Legge un WAV PCM a 16 bit o in virgola mobile a 32 bit.
pub fn leggi_wav(path: &Path) -> Result<Wav, String> {
    let b = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if b.len() < 12 || &b[0..4] != b"RIFF" || &b[8..12] != b"WAVE" {
        return Err(format!("{}: non e' un WAV", path.display()));
    }
    let u16_at = |i: usize| u16::from_le_bytes([b[i], b[i + 1]]);
    let u32_at = |i: usize| u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]);
    let (mut fmt, mut canali, mut rate, mut bits) = (0u16, 1usize, 0u32, 0u16);
    let mut i = 12;
    while i + 8 <= b.len() {
        let id = &b[i..i + 4];
        let len = u32_at(i + 4) as usize;
        let body = i + 8;
        if id == b"fmt " && body + 16 <= b.len() {
            fmt = u16_at(body);
            canali = usize::from(u16_at(body + 2)).max(1);
            rate = u32_at(body + 4);
            bits = u16_at(body + 14);
            if fmt == 0xFFFE && body + 26 <= b.len() {
                fmt = u16_at(body + 24);
            }
        } else if id == b"data" {
            let end = (body + len).min(b.len());
            let data = &b[body..end];
            let mut out = vec![Vec::new(); canali];
            match (fmt, bits) {
                (1, 16) => {
                    for (k, s) in data.chunks_exact(2).enumerate() {
                        out[k % canali].push(f32::from(i16::from_le_bytes([s[0], s[1]])) / 32768.0);
                    }
                }
                (3, 32) => {
                    for (k, s) in data.chunks_exact(4).enumerate() {
                        out[k % canali].push(f32::from_le_bytes([s[0], s[1], s[2], s[3]]));
                    }
                }
                _ => return Err(format!("{}: formato {fmt}/{bits} bit non letto (serve PCM 16 o float 32)", path.display())),
            }
            return Ok(Wav { rate, canali: out });
        }
        i = body + len + (len & 1);
    }
    Err(format!("{}: manca il blocco data", path.display()))
}

/// Ricampiona a 12 kHz con un seno cardinale a finestra di Blackman (±32 campioni), taglio a 0,45 della frequenza piu' bassa.
pub fn a_12k(x: &[f32], rate: u32) -> Vec<f32> {
    if rate == 12_000 {
        return x.to_vec();
    }
    const HALF: i64 = 32;
    let r = f64::from(rate);
    let fc = 0.45 * r.min(12_000.0) / r;
    let n_out = (x.len() as f64 * 12_000.0 / r) as usize;
    (0..n_out)
        .map(|n| {
            let t = n as f64 * r / 12_000.0;
            let k0 = t.floor() as i64;
            let mut acc = 0.0;
            for k in (k0 - HALF + 1)..=(k0 + HALF) {
                if k < 0 || k as usize >= x.len() {
                    continue;
                }
                let d = t - k as f64;
                let sinc = if d.abs() < 1e-12 { 2.0 * fc } else { (2.0 * std::f64::consts::PI * fc * d).sin() / (std::f64::consts::PI * d) };
                let w = 0.42 + 0.5 * (std::f64::consts::PI * d / HALF as f64).cos() + 0.08 * (2.0 * std::f64::consts::PI * d / HALF as f64).cos();
                acc += f64::from(x[k as usize]) * sinc * w;
            }
            acc as f32
        })
        .collect()
}

/// I parametri da `meta.toml` (righe `chiave = valore`, niente sezioni).
fn leggi_meta(path: &Path) -> RttyParams {
    let mut p = RttyParams { centre_hz: 1500.0, shift_hz: 170.0, baud: 45.45, reverse: false };
    let Ok(s) = std::fs::read_to_string(path) else { return p };
    for line in s.lines() {
        let Some((k, v)) = line.split_once('=') else { continue };
        let v = v.trim().trim_matches('"');
        match k.trim() {
            "centre_hz" => p.centre_hz = v.parse().unwrap_or(p.centre_hz),
            "shift_hz" => p.shift_hz = v.parse().unwrap_or(p.shift_hz),
            "baud" => p.baud = v.parse().unwrap_or(p.baud),
            "reverse" => p.reverse = v == "true",
            _ => {}
        }
    }
    p
}

fn normalizza(s: &str) -> Vec<u8> {
    s.bytes().filter(|&c| c != b'\r' && c != b'\n').map(|c| c.to_ascii_uppercase()).collect()
}

fn cartella_corpus() -> PathBuf {
    std::env::var_os("AURORA_RTTY_CORPUS")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../rtty-corpus"))
}

fn decodifica(nome: &str, p: RttyParams, audio: &[f32]) -> Option<Vec<u8>> {
    let mut got = Vec::new();
    match nome {
        "aurora" => {
            let mut rx = RttyDemodulator::new(p);
            rx.set_discriminator(p.shift_hz < 40.0);
            for b in audio.chunks(1024) {
                got.extend(rx.process(b));
            }
        }
        #[cfg(feature = "fldigi-oracle")]
        "fldigi-oracolo" => {
            if p.reverse {
                return None;
            }
            let mut rx = mode_digi::fldigi_oracle::FldigiRtty::new(f64::from(p.baud), f64::from(p.shift_hz), f64::from(p.centre_hz), 8000.0);
            got = rx.process(&common::a_8k(audio));
        }
        #[cfg(feature = "mmtty-oracle")]
        "mmtty-oracolo" => {
            if p.reverse {
                return None;
            }
            let mut rx = mode_digi::mmtty_oracle::MmttyRtty::new(
                mode_digi::mmtty_oracle::MmttyProfile::Standard,
                f64::from(p.baud),
                f64::from(p.shift_hz),
                f64::from(p.centre_hz),
                false,
            );
            got = rx.process(audio);
        }
        _ => return None,
    }
    Some(got)
}

#[test]
#[ignore]
fn corpus() {
    let dir = cartella_corpus();
    let Ok(voci) = std::fs::read_dir(&dir) else {
        println!("nessun corpus in {} (vedi docs/progetto-rtty-oltre.md, passo 4)", dir.display());
        return;
    };
    let mut cartelle: Vec<PathBuf> = voci.flatten().map(|e| e.path()).filter(|p| p.join("audio.wav").is_file()).collect();
    cartelle.sort();
    for c in cartelle {
        let wav = match leggi_wav(&c.join("audio.wav")) {
            Ok(w) => w,
            Err(e) => {
                println!("{e}");
                continue;
            }
        };
        let p = leggi_meta(&c.join("meta.toml"));
        let audio = a_12k(&wav.canali[0], wav.rate);
        let rif = std::fs::read_to_string(c.join("riferimento.txt")).ok().map(|s| normalizza(&s));
        let calls: Vec<String> = std::fs::read_to_string(c.join("nominativi.txt"))
            .map(|s| s.lines().map(|l| l.trim().to_ascii_uppercase()).filter(|l| !l.is_empty()).collect())
            .unwrap_or_default();
        println!("== {} ({:.0} s, {} Hz, centro {} Hz, {} Hz / {} baud{})", c.display(), audio.len() as f64 / 12_000.0, wav.rate, p.centre_hz, p.shift_hz, p.baud, if p.reverse { ", Rev" } else { "" });
        for nome in ["aurora", "fldigi-oracolo", "mmtty-oracolo"] {
            if let Some(got) = decodifica(nome, p, &audio) {
                let _ = std::fs::write(c.join(format!("uscita-{nome}.txt")), &got);
            }
        }
        // tutte le uscite nella cartella: le nostre appena scritte e quelle degli altri programmi
        let mut uscite: Vec<PathBuf> = std::fs::read_dir(&c)
            .map(|r| r.flatten().map(|e| e.path()).filter(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("uscita-") && n.ends_with(".txt"))).collect())
            .unwrap_or_default();
        uscite.sort();
        for u in uscite {
            let nome = u.file_stem().and_then(|n| n.to_str()).unwrap_or("?").trim_start_matches("uscita-").to_string();
            let testo = std::fs::read(&u).unwrap_or_default();
            let testo_n = normalizza(&String::from_utf8_lossy(&testo));
            let cer = rif.as_ref().map(|r| 100.0 * levenshtein(r, &testo_n) as f64 / r.len().max(1) as f64);
            let s = String::from_utf8_lossy(&testo_n).into_owned();
            let trovati = calls.iter().filter(|k| s.contains(k.as_str())).count();
            println!(
                "  {nome:<16} {:>6} caratteri  CER {}  nominativi {trovati}/{}",
                testo_n.len(),
                cer.map_or("  --  ".to_string(), |v| format!("{v:5.1}%")),
                calls.len()
            );
        }
    }
}

#[test]
fn il_wav_scritto_dalle_prove_si_rilegge() {
    let x: Vec<f32> = (0..1200).map(|n| (n as f32 * 0.05).sin() * 0.5).collect();
    let dir = std::env::temp_dir().join(format!("aurora-rtty-corpus-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    // salva_wav scrive solo con AURORA_WAV: qui si scrive a mano lo stesso formato
    let path = dir.join("prova.wav");
    let mut b = Vec::new();
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36 + 2 * x.len() as u32).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&8000u32.to_le_bytes());
    b.extend_from_slice(&16000u32.to_le_bytes());
    b.extend_from_slice(&2u16.to_le_bytes());
    b.extend_from_slice(&16u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&(2 * x.len() as u32).to_le_bytes());
    for &s in &x {
        b.extend_from_slice(&((s * 32767.0) as i16).to_le_bytes());
    }
    std::fs::write(&path, b).expect("scrittura");
    let w = leggi_wav(&path).expect("lettura");
    assert_eq!(w.rate, 8000);
    assert_eq!(w.canali[0].len(), x.len());
    assert!((w.canali[0][100] - x[100]).abs() < 1e-3);
    let y = a_12k(&w.canali[0], w.rate);
    assert_eq!(y.len(), 1800);
    let _ = std::fs::remove_dir_all(&dir);
}

/// Aurora BIGScan RTTY su ogni file del corpus: quanti segnali trova in tutta la banda e i nominativi verificati di ognuno.
#[test]
#[ignore]
fn banda_intera() {
    use mode_digi::skim::BandDecoder;
    use mode_digi::skim::rtty::RttyBandDecoder;
    let dir = cartella_corpus();
    let Ok(voci) = std::fs::read_dir(&dir) else { return };
    let mut cartelle: Vec<PathBuf> = voci.flatten().map(|e| e.path()).filter(|p| p.join("audio.wav").is_file()).collect();
    cartelle.sort();
    for c in cartelle.into_iter().take(1) {
        let Ok(wav) = leggi_wav(&c.join("audio.wav")) else { continue };
        let audio = a_12k(&wav.canali[0], wav.rate);
        let mut d = RttyBandDecoder::new(170.0, 45.45, 30);
        for b in audio.chunks(4800) {
            d.process(b);
        }
        println!("== {}", c.display());
        for ch in d.channels() {
            let calls: Vec<String> = ch.calls.iter().map(|k| format!("{}{}", k.call, if k.verified() { "*" } else { "" })).collect();
            let tail: String = ch.text.chars().rev().take(70).collect::<Vec<_>>().into_iter().rev().collect();
            println!("{:7.1} Hz {calls:?} | {}", ch.freq_hz, tail.replace('\n', " "));
        }
    }
}
