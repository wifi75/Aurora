//! Confronto della ricezione JT65 (sottomodi A, B, C, funzioni VHF e media dei periodi) con `jt9.exe` di WSJT-X: genera periodi di un
//! minuto con il modulatore di Aurora (WAV a 12 kHz, 16 bit, mono, messaggio a un secondo dall'inizio come in WSJT-X, rumore bianco
//! gaussiano, rapporto in 2500 Hz), li decodifica con `jt9.exe` e con `jt::rx` alle stesse impostazioni e stampa una tabella.
//!
//! Uso (serve `jt9.exe` di WSJT-X, per esempio quello del pacchetto `wsjtx-3.3.0-beta6-windows-x86_64-jt9`):
//! `cargo run --release -p jt --example oracolo_jt65 -- <jt9.exe> <cartella di lavoro> [singoli|media|tutto] [A|B|C|A VHF]`
//!
//! Le opzioni di `jt9` sono quelle di WSJT-X: `-6` JT65, `-b` sottomodo, `-X 64` «Enable VHF and submode features» (`nexp_decode & 64`),
//! `-d 3` profondita', `+16` «Include averaging», `-f`/`-F` frequenza e tolleranza di ricezione (`jt9_wav_request.f90`). La media di `jt9`
//! si tiene fra i file di una stessa chiamata: il numero del periodo viene dal nome (`_hhmm.wav`, `jt9_input_validation.f90:185-188`), e si
//! sommano solo minuti della stessa parita'. `jt9` gira con priorita' bassa (Windows), un file alla volta. I nominativi per l'analisi a
//! priori sono gli stessi da tutte e due le parti (`-c`, `-x`, `-g`). Il sincronismo di `jt9` si legge da `decoded.txt` nella cartella di
//! lavoro (`nint(sync)`); le righe grezze di `jt9` vanno su stderr. Non e' una prova automatica: serve `jt9.exe` su questo PC.
//!
//! Esito con `jt9.exe` di WSJT-X 3.3.0-beta6: stessi messaggi, ritardo, frequenza, sincronismo (entro 0,5) e rapporto (entro 1 dB) in A,
//! B e C, con e senza VHF, e la media esce allo stesso periodo; Aurora trova in piu' qualche decodifica con l'analisi a priori al limite
//! (-25/-26 dB in B e C: WSJT-X la perde se la lisciatura successiva fallisce, `decode65a.f90:130-148`).

use jt::jt65::{Submode, modulate_drifting, modulate_submode, tones};
use jt::rs::Codec;
use jt::rx::{ApCalls, Averager, Decoded, Options, decode, decode_averaged};
use std::path::{Path, PathBuf};
use std::process::Command;

const FS: usize = 12_000;
const MSG: &str = "CQ IU3ABC IO91";
const MY_CALL: &str = "K1ABC";
const DX_CALL: &str = "W9XYZ";
const DX_GRID: &str = "EN37";
/// Deviazione del rumore in unita' di campione a 16 bit.
const SIGMA: f64 = 1000.0;

struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.0 >> 11) as f64) / ((1u64 << 53) as f64)
    }
    fn gauss(&mut self) -> f64 {
        let u1 = self.next().max(1e-12);
        let u2 = self.next();
        (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
    }
}

/// Un caso: sottomodo, VHF, rapporto, frequenza, ritardo, deriva (Hz sul messaggio, solo in A), seme.
#[derive(Clone)]
struct Case {
    sub: Submode,
    vhf: bool,
    snr_db: f64,
    f0: f64,
    dt: f64,
    drift: f64,
    seed: u64,
}

/// Un minuto di audio a 16 bit.
fn minute(c: &Case) -> Vec<i16> {
    let codec = Codec::new();
    let mut rng = Lcg(c.seed);
    let n = 60 * FS;
    let mut audio: Vec<f64> = (0..n).map(|_| SIGMA * rng.gauss()).collect();
    let amp = (SIGMA * (2.0 * 10f64.powf(c.snr_db / 10.0) * 2500.0 / 6000.0).sqrt()) as f32;
    let t = tones(&codec, MSG);
    let wave = if c.drift != 0.0 {
        modulate_drifting(&t, c.f0, amp, FS as f64, c.drift)
    } else {
        modulate_submode(&t, c.f0, c.sub, amp, FS as f64)
    };
    let start = ((1.0 + c.dt) * FS as f64).round() as usize;
    for (i, v) in wave.iter().enumerate() {
        if start + i < n {
            audio[start + i] += f64::from(*v);
        }
    }
    audio.iter().map(|v| v.round().clamp(-32768.0, 32767.0) as i16).collect()
}

fn write_wav(path: &Path, samples: &[i16]) -> std::io::Result<()> {
    let data_len = (samples.len() * 2) as u32;
    let mut b = Vec::with_capacity(44 + samples.len() * 2);
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36 + data_len).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&(FS as u32).to_le_bytes());
    b.extend_from_slice(&(FS as u32 * 2).to_le_bytes());
    b.extend_from_slice(&2u16.to_le_bytes());
    b.extend_from_slice(&16u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        b.extend_from_slice(&s.to_le_bytes());
    }
    std::fs::write(path, b)
}

fn as_f32(samples: &[i16]) -> Vec<f32> {
    samples.iter().map(|&s| f32::from(s) / 32768.0).collect()
}

/// Una riga decodificata da `jt9`, a colonne fisse (`format(i4.4,i4,f5.1,i5,1x,a2,1x,a22,1x,a3)`, `decoder_callbacks.f90:303`): periodo,
/// rapporto, ritardo, frequenza, sincronismo (`# `, `#*`, `##`), testo e indicatori (in VHF `f`, `d`, `a1`, piu' i periodi sommati).
#[derive(Debug, Clone)]
struct Jt9Line {
    raw: String,
    snr: i32,
    dt: f32,
    freq: i32,
    text: String,
    flags: String,
}

fn parse_jt9(line: &str) -> Option<Jt9Line> {
    let col = |a: usize, b: usize| line.get(a..b.min(line.len())).map(str::trim);
    if line.len() < 22 || col(19, 20)? != "#" {
        return None;
    }
    Some(Jt9Line {
        raw: line.trim_end().to_string(),
        snr: col(4, 8)?.parse().ok()?,
        dt: col(8, 13)?.parse().ok()?,
        freq: col(13, 18)?.parse().ok()?,
        text: col(22, 44).unwrap_or("").to_string(),
        flags: col(45, 48).unwrap_or("").to_string(),
    })
}

/// Il sincronismo che `jt9` scrive in `decoded.txt` (`nint(sync)`, `decoder_callbacks.f90:310-312`), per periodo (`hhmm`) e testo.
fn sync_of(decoded: &[String], nutc: u32, text: &str) -> Option<i32> {
    decoded.iter().find_map(|l| {
        let w: Vec<&str> = l.split_whitespace().collect();
        let utc: u32 = w.first()?.parse().ok()?;
        let end = w.iter().position(|x| *x == "JT65")?;
        (utc == nutc && w.get(6..end)?.join(" ") == text).then(|| w[1].parse().ok()).flatten()
    })
}

/// Lancia `jt9` su una lista di file e ritorna le righe di ogni file (separate da `<DecodeFinished>`) e quelle di `decoded.txt`.
fn run_jt9(jt9: &Path, work: &Path, args: &[String], files: &[PathBuf]) -> (Vec<Vec<String>>, Vec<String>) {
    let _ = std::fs::remove_file(work.join("decoded.txt"));
    let mut cmd = Command::new(jt9);
    cmd.current_dir(work).arg("-a").arg(work).arg("-t").arg(work).args(args).args(files);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // IDLE_PRIORITY_CLASS: il PC resta libero
        cmd.creation_flags(0x0000_0040);
    }
    let out = match cmd.output() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("jt9 non parte: {e}");
            return (Vec::new(), Vec::new());
        }
    };
    let stdout = String::from_utf8_lossy(&out.stdout);
    let mut per_file: Vec<Vec<String>> = vec![Vec::new()];
    for line in stdout.lines() {
        // le righe grezze su stderr, per controllare la lettura delle colonne
        eprintln!("jt9> {line}");
        if line.contains("<DecodeFinished>") {
            per_file.push(Vec::new());
        } else if !line.trim().is_empty() {
            if let Some(last) = per_file.last_mut() {
                last.push(line.to_string());
            }
        }
    }
    if !out.stderr.is_empty() {
        eprintln!("jt9 stderr: {}", String::from_utf8_lossy(&out.stderr).trim());
    }
    let decoded = std::fs::read_to_string(work.join("decoded.txt")).unwrap_or_default().lines().map(str::to_string).collect();
    (per_file, decoded)
}

fn jt9_args(c: &Case, depth: u32, tol: u32) -> Vec<String> {
    let mut a = vec!["-6".to_string(), "-b".into(), c.sub.letter().to_string(), "-d".into(), depth.to_string()];
    // gli stessi nominativi per l'analisi a priori da tutte e due le parti (quelli di fabbrica di `jt9`)
    a.extend(["-c", MY_CALL, "-x", DX_CALL, "-g", DX_GRID, "-Q", "0"].map(String::from));
    if c.vhf {
        a.extend(["-X".into(), "64".into()]);
    }
    a.extend(["-f".into(), format!("{}", c.f0.round() as i32), "-F".into(), tol.to_string()]);
    a
}

fn aurora_options(c: &Case, tol: f32, average: bool) -> Options {
    Options {
        submode: c.sub,
        vhf: c.vhf,
        rx_hz: Some(c.f0 as f32),
        tol_hz: tol,
        depth: 3,
        average,
        ap: Some(ApCalls { my_call: MY_CALL.into(), dx_call: DX_CALL.into(), dx_grid: DX_GRID.into(), qso_progress: 0 }),
        ..Options::default()
    }
}

fn label(c: &Case) -> String {
    format!("{}{}", c.sub.letter(), if c.vhf { " VHF" } else { "" })
}

fn fmt_aurora(d: Option<&Decoded>) -> String {
    match d {
        Some(d) => format!("si  dt {:+.2} f {:7.1} sync {:5.1} snr {:3} n{} ap{}", d.dt_s, d.freq_hz, d.sync, d.snr_db, d.averaged, d.ap_type),
        None => "no".to_string(),
    }
}

fn fmt_jt9(l: Option<&Jt9Line>, sync: Option<i32>) -> String {
    match l {
        Some(l) => format!("si  dt {:+.1} f {:4} sync {:>3} snr {:3} [{}]", l.dt, l.freq, sync.map_or("-".into(), |s| s.to_string()), l.snr, l.flags),
        None => "no".to_string(),
    }
}

fn singles(jt9: &Path, work: &Path, only: &str) {
    let mut cases = Vec::new();
    let mut seed = 1000u64;
    for (sub, vhf, f0) in [(Submode::A, false, 1270.0), (Submode::A, true, 1270.0), (Submode::B, true, 1500.0), (Submode::C, true, 1500.0)] {
        for snr in [-16.0, -20.0, -22.0, -24.0, -25.0, -26.0, -27.0] {
            for k in 0..2 {
                seed += 1;
                cases.push(Case { sub, vhf, snr_db: snr, f0: f0 + 37.0 * k as f64, dt: 0.4 * k as f64 - 0.2, drift: 0.0, seed });
            }
        }
    }
    // deriva (A, HF) e un ritardo grande
    cases.push(Case { sub: Submode::A, vhf: false, snr_db: -20.0, f0: 1500.0, dt: 0.0, drift: 3.0, seed: 2001 });
    cases.push(Case { sub: Submode::A, vhf: false, snr_db: -22.0, f0: 900.0, dt: 1.5, drift: 0.0, seed: 2002 });
    cases.push(Case { sub: Submode::B, vhf: true, snr_db: -20.0, f0: 1000.0, dt: 1.5, drift: 0.0, seed: 2003 });
    println!("caso;snr;f0;dt0;jt9;aurora");
    for (i, c) in cases.iter().enumerate() {
        if !label(c).starts_with(only) {
            continue;
        }
        let audio = minute(c);
        let path = work.join(format!("s{i:03}_0000.wav"));
        if let Err(e) = write_wav(&path, &audio) {
            eprintln!("{}: {e}", path.display());
            continue;
        }
        let tol = 100;
        let (lines, dec) = run_jt9(jt9, work, &jt9_args(c, 3, tol), std::slice::from_ref(&path));
        let all: Vec<Jt9Line> = lines.iter().flatten().filter_map(|l| parse_jt9(l)).collect();
        let j = all.iter().find(|l| l.text == MSG);
        let out = decode(&as_f32(&audio), &aurora_options(c, tol as f32, false));
        let a = out.iter().find(|d| d.text == MSG);
        let other_j: Vec<&str> = all.iter().filter(|l| l.text != MSG && !l.text.is_empty()).map(|l| l.raw.as_str()).collect();
        let other_a: Vec<&str> = out.iter().filter(|d| d.text != MSG).map(|d| d.text.as_str()).collect();
        println!(
            "{:6};{:4};{:6.0};{:+.1};{};{}{}",
            label(c),
            c.snr_db,
            c.f0,
            c.dt,
            fmt_jt9(j, sync_of(&dec, 0, MSG)),
            fmt_aurora(a),
            if other_j.is_empty() && other_a.is_empty() { String::new() } else { format!(";ALTRO jt9 {other_j:?} aurora {other_a:?}") }
        );
        let _ = std::fs::remove_file(&path);
    }
}

fn averaging(jt9: &Path, work: &Path, only: &str) {
    println!("media;snr;periodo;jt9;aurora");
    for (sub, snr) in [(Submode::B, -27.0), (Submode::B, -29.0), (Submode::C, -27.0), (Submode::C, -29.0), (Submode::A, -27.0)] {
        let vhf = sub != Submode::A;
        if !sub.letter().to_string().starts_with(only) && !only.starts_with(sub.letter()) {
            continue;
        }
        let n = 8u64;
        let cases: Vec<Case> =
            (0..n).map(|k| Case { sub, vhf, snr_db: snr, f0: 1500.0, dt: 0.1 + 0.01 * k as f64, drift: 0.0, seed: 5000 + 10 * sub.0 as u64 + k }).collect();
        let mut files = Vec::new();
        let mut audios = Vec::new();
        for (k, c) in cases.iter().enumerate() {
            let a = minute(c);
            let p = work.join(format!("m{}{}_{:04}.wav", sub.letter(), -snr as i32, 2 * k));
            if write_wav(&p, &a).is_ok() {
                files.push(p);
                audios.push(a);
            }
        }
        let tol = 100;
        let (lines, dec) = run_jt9(jt9, work, &jt9_args(&cases[0], 3 + 16, tol), &files);
        let mut avg = Averager::new();
        let o = aurora_options(&cases[0], tol as f32, true);
        for (k, a) in audios.iter().enumerate() {
            let jl: Vec<Jt9Line> = lines.get(k).map(|v| v.iter().filter_map(|l| parse_jt9(l)).collect()).unwrap_or_default();
            let j = jl.iter().find(|l| l.text == MSG);
            let out = decode_averaged(&as_f32(a), &o, &mut avg, 2 * k as u64);
            let d = out.iter().find(|d| d.text == MSG);
            println!("{:6};{:4};{};{};{}", label(&cases[0]), snr, k + 1, fmt_jt9(j, sync_of(&dec, 2 * k as u32, MSG)), fmt_aurora(d));
        }
        for f in &files {
            let _ = std::fs::remove_file(f);
        }
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let (Some(jt9), Some(work)) = (args.next(), args.next()) else {
        eprintln!("uso: oracolo_jt65 <jt9.exe> <cartella di lavoro> [singoli|media|tutto]");
        return;
    };
    let what = args.next().unwrap_or_else(|| "tutto".into());
    // facoltativo: solo i casi la cui etichetta comincia cosi' (per esempio `A`, `B VHF`, `C`)
    let only = args.next().unwrap_or_default();
    let work = PathBuf::from(work);
    if let Err(e) = std::fs::create_dir_all(&work) {
        eprintln!("{}: {e}", work.display());
        return;
    }
    let jt9 = PathBuf::from(jt9);
    if what == "singoli" || what == "tutto" {
        singles(&jt9, &work, &only);
    }
    if what == "media" || what == "tutto" {
        averaging(&jt9, &work, &only);
    }
}
