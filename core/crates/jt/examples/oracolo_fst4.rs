//! Confronto della ricezione FST4 e FST4W con `jt9.exe` di WSJT-X: genera periodi con il modulatore di Aurora (WAV a 12 kHz, 16 bit,
//! mono, messaggio a 1 s dall'inizio, 0,5 s in FST4-15, rumore bianco gaussiano, rapporto in 2500 Hz), li decodifica con `jt9.exe` e
//! con `jt::fst4` alle stesse impostazioni e stampa una tabella (ritardo, frequenza del tono 0, rapporto, sincronismo, ipotesi AP).
//!
//! Uso (serve `jt9.exe` di WSJT-X, per esempio quello di `wsjtx-3.3.0-beta6-windows-x86_64-jt9`):
//! `cargo run --release -p jt --example oracolo_fst4 -- <jt9.exe> <cartella di lavoro> [fst4|fst4w|ap|statistica|tutto] [periodo massimo]`
//!
//! Le opzioni di `jt9` sono quelle di WSJT-X (`jt9.f90:131-132`, `jt9_wav_request.f90:103-124`): `-7` FST4, `-W` FST4W, `-p` periodo,
//! `-d 3` profondita', `-f`/`-F` frequenza e tolleranza di ricezione, `-L`/`-H` banda di ricerca di FST4, `-c`/`-x` i nominativi per
//! l'analisi a priori (gli stessi da tutte e due le parti). `jt9` gira con priorita' bassa (Windows), un file alla volta; il
//! sincronismo di `jt9` si legge da `decoded.txt` (`nint(sync)`, `decoder_callbacks.f90:843,848`). Non e' una prova automatica.

use ft8::message::CallHashes;
use jt::fst4::{Decoded, FST4_PERIODS, FST4W_PERIODS, Options, State, decode, encode, modulate, start_s};
use std::path::{Path, PathBuf};
use std::process::Command;

const FS: usize = 12_000;
const MSG4: &str = "CQ IU3ABC IO91";
const MSG4W: &str = "IU3ABC IO91 37";
const MY_CALL: &str = "K1ABC";
const DX_CALL: &str = "W9XYZ";
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

#[derive(Clone)]
struct Case {
    wspr: bool,
    period: u32,
    snr_db: f64,
    f0: f64,
    dt: f64,
    seed: u64,
    /// Testo trasmesso e passo del QSO per le ipotesi AP (`-Q`).
    msg: &'static str,
    qp: u8,
}

fn text(c: &Case) -> &'static str {
    c.msg
}

fn audio(c: &Case) -> Vec<i16> {
    let mut h = CallHashes::new();
    let n = c.period as usize * FS;
    let mut rng = Lcg(c.seed);
    let mut a: Vec<f64> = (0..n).map(|_| SIGMA * rng.gauss()).collect();
    let amp = (SIGMA * (2.0 * 10f64.powf(c.snr_db / 10.0) * 2500.0 / 6000.0).sqrt()) as f32;
    let Ok(e) = encode(text(c), c.wspr, &mut h) else { return Vec::new() };
    let wave = modulate(&e.tones, c.f0, c.period, amp, FS as f64);
    let start = ((start_s(c.period) + c.dt) * FS as f64).round() as usize;
    for (i, v) in wave.iter().enumerate() {
        if let Some(x) = a.get_mut(start + i) {
            *x += f64::from(*v);
        }
    }
    a.iter().map(|v| v.round().clamp(-32768.0, 32767.0) as i16).collect()
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

/// Una riga di `jt9` (`decoder_callbacks.f90:1001,1003`): `hhmmss`/`hhmm`, rapporto, ritardo, frequenza, `` ` ``, testo, ipotesi AP.
#[derive(Debug, Clone)]
struct Jt9Line {
    snr: i32,
    dt: f32,
    freq: i32,
    text: String,
    ap: String,
}

fn parse_jt9(line: &str) -> Option<Jt9Line> {
    let (left, right) = line.split_once('`')?;
    let w: Vec<&str> = left.split_whitespace().collect();
    if w.len() < 4 {
        return None;
    }
    let right = right.get(2..).unwrap_or("");
    let msg: String = right.chars().take(37).collect();
    let rest: String = right.chars().skip(37).collect();
    Some(Jt9Line {
        snr: w[1].parse().ok()?,
        dt: w[2].parse().ok()?,
        freq: w[3].parse().ok()?,
        // con le ipotesi AP e qualita' bassa WSJT-X mette `?` nell'ultima colonna (`decoder_callbacks.f90`, `qual.lt.0.17`)
        text: msg.trim().trim_end_matches('?').trim().to_string(),
        ap: rest.split_whitespace().next().unwrap_or("").to_string(),
    })
}

/// Sincronismo da `decoded.txt` (`i4`, seconda colonna).
fn sync_of(decoded: &[String], text: &str) -> Option<i32> {
    decoded.iter().find_map(|l| {
        let w: Vec<&str> = l.split_whitespace().collect();
        let end = w.iter().position(|x| *x == "FST4")?;
        (w.get(6..end)?.join(" ").trim_end_matches('?').trim() == text).then(|| w[1].parse().ok()).flatten()
    })
}

fn run_jt9(jt9: &Path, work: &Path, args: &[String], file: &Path) -> (Vec<String>, Vec<String>) {
    let _ = std::fs::remove_file(work.join("decoded.txt"));
    let mut cmd = Command::new(jt9);
    cmd.current_dir(work).arg("-a").arg(work).arg("-t").arg(work).args(args).arg(file);
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
    let lines: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .inspect(|l| eprintln!("jt9> {l}"))
        .filter(|l| !l.contains("<DecodeFinished>") && !l.trim().is_empty())
        .map(str::to_string)
        .collect();
    if !out.stderr.is_empty() {
        eprintln!("jt9 stderr: {}", String::from_utf8_lossy(&out.stderr).trim());
    }
    let decoded = std::fs::read_to_string(work.join("decoded.txt")).unwrap_or_default().lines().map(str::to_string).collect();
    (lines, decoded)
}

fn jt9_args(c: &Case) -> Vec<String> {
    let mut a = vec![if c.wspr { "-W" } else { "-7" }.to_string(), "-p".into(), c.period.to_string(), "-d".into(), "3".into()];
    a.extend(["-c", MY_CALL, "-x", DX_CALL, "-Q"].map(String::from));
    a.push(c.qp.to_string());
    let tol = if c.wspr { 100 } else { 20 };
    a.extend(["-f".into(), format!("{}", c.f0.round() as i32), "-F".into(), tol.to_string()]);
    if !c.wspr {
        a.extend(["-L".into(), format!("{}", c.f0 as i32 - 200), "-H".into(), format!("{}", c.f0 as i32 + 200)]);
    }
    a
}

fn aurora_options(c: &Case) -> Options {
    Options {
        period_s: c.period,
        wspr: c.wspr,
        f_low_hz: c.f0 as i32 - 200,
        f_high_hz: c.f0 as i32 + 200,
        rx_hz: c.f0.round() as i32,
        tol_hz: if c.wspr { 100 } else { 20 },
        depth: 3,
        qso_progress: c.qp,
        my_call: MY_CALL.into(),
        dx_call: DX_CALL.into(),
        threads: 4,
        ..Options::default()
    }
}

fn fmt_a(d: Option<&Decoded>) -> String {
    match d {
        Some(d) => format!("si dt {:+.2} f {:7.1} snr {:4} sync {:5.2} ap{} k{}", d.dt_s, d.freq_hz, d.snr_db, d.sync, d.ap, d.keff),
        None => "no".into(),
    }
}

fn fmt_j(l: Option<&Jt9Line>, sync: Option<i32>) -> String {
    match l {
        Some(l) => format!("si dt {:+.1} f {:4} snr {:4} sync {:>2} {}", l.dt, l.freq, l.snr, sync.map_or("-".into(), |s| s.to_string()), l.ap),
        None => "no".into(),
    }
}

/// Rapporti di prova attorno alla soglia di WSJT-X per periodo (FST4 Quick Start Guide: -20,7 dB a 15 s ... -43,2 dB a 1800 s; FST4W
/// da -32,8 a -44,8 dB).
fn snrs(wspr: bool, period: u32) -> Vec<f64> {
    let thr = match (wspr, period) {
        (false, 15) => -20.7,
        (false, 30) => -24.2,
        (false, 60) => -28.1,
        (false, 120) => -31.3,
        (false, 300) => -35.3,
        (false, 900) => -40.2,
        (false, _) => -43.2,
        (true, 120) => -32.8,
        (true, 300) => -36.8,
        (true, 900) => -41.7,
        (true, _) => -44.8,
    };
    if period >= 900 { vec![thr + 6.0, thr + 1.0] } else { vec![thr + 8.0, thr + 3.0, thr + 1.0, thr - 1.0, thr - 2.5, thr - 4.0] }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let (Some(jt9), Some(work)) = (args.next(), args.next()) else {
        eprintln!("uso: oracolo_fst4 <jt9.exe> <cartella di lavoro> [fst4|fst4w|ap|statistica|tutto] [periodo massimo]");
        return;
    };
    let what = args.next().unwrap_or_else(|| "tutto".into());
    let max_period: u32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(1800);
    let work = PathBuf::from(work);
    if let Err(e) = std::fs::create_dir_all(&work) {
        eprintln!("{}: {e}", work.display());
        return;
    }
    let jt9 = PathBuf::from(jt9);
    let mut cases = Vec::new();
    let mut seed = 100u64;
    for wspr in [false, true] {
        if (wspr && what == "fst4") || (!wspr && what == "fst4w") || what == "ap" || what == "statistica" {
            continue;
        }
        let periods: &[u32] = if wspr { &FST4W_PERIODS } else { &FST4_PERIODS };
        for &p in periods.iter().filter(|&&p| p <= max_period) {
            for (k, s) in snrs(wspr, p).into_iter().enumerate() {
                seed += 1;
                let f0 = if wspr { 1500.0 } else { 1400.0 + 23.0 * k as f64 };
                let msg = if wspr { MSG4W } else { MSG4 };
                cases.push(Case { wspr, period: p, snr_db: s, f0, dt: 0.15 * k as f64 - 0.2, seed, msg, qp: 0 });
            }
        }
    }
    // ipotesi AP: il DX risponde con RR73 (passo 3: tipi 3 e 6) e con un rapporto (passo 1: tipi 2 e 3), sotto la soglia
    // statistica: tanti semi allo stesso rapporto, per confrontare le percentuali quando i casi singoli divergono al limite
    if what == "statistica" {
        for (p, snr, msg, qp) in [(15u32, -21.0, "K1ABC W9XYZ RR73", 3u8), (15, -22.0, "K1ABC W9XYZ RR73", 3), (60, -29.5, "K1ABC W9XYZ -15", 1), (15, -20.5, MSG4, 0), (120, -32.0, MSG4W, 0)] {
            for _ in 0..20 {
                seed += 1;
                cases.push(Case { wspr: msg == MSG4W, period: p, snr_db: snr, f0: 1500.0, dt: 0.1, seed, msg, qp });
            }
        }
    }
    if what == "ap" || what == "tutto" {
        for &p in [15u32, 60, 300].iter().filter(|&&p| p <= max_period) {
            for (msg, qp) in [("K1ABC W9XYZ RR73", 3u8), ("K1ABC W9XYZ -15", 1)] {
                for k in 0..4 {
                    seed += 1;
                    let thr = snrs(false, p)[2] - 1.0;
                    cases.push(Case { wspr: false, period: p, snr_db: thr - 1.5 * k as f64, f0: 1500.0, dt: 0.1, seed, msg, qp });
                }
            }
        }
    }
    // `jt9` ricorda fra un file e l'altro i nominativi/locatori sentiti in FST4W (`fst4w_calls.txt` nella cartella dei dati) per la
    // decodifica con Keff 50: si parte da vuoto e Aurora tiene il suo [`State`] per tutta la corsa, come lui
    let _ = std::fs::remove_file(work.join("fst4w_calls.txt"));
    let mut state = State::default();
    println!("modo;periodo;Q;snr;f0;dt0;messaggio;jt9;aurora");
    for (i, c) in cases.iter().enumerate() {
        let a = audio(c);
        // nome che finisce in hhmmss (sotto i 60 s) o _hhmm, come i file di WSJT-X
        let path = work.join(if c.period < 60 { format!("o{i:03}_000000.wav") } else { format!("o{i:03}_0000.wav") });
        if let Err(e) = write_wav(&path, &a) {
            eprintln!("{}: {e}", path.display());
            continue;
        }
        let (lines, dec) = run_jt9(&jt9, &work, &jt9_args(c), &path);
        let jl: Vec<Jt9Line> = lines.iter().filter_map(|l| parse_jt9(l)).collect();
        let j = jl.iter().find(|l| l.text == text(c));
        let f32a: Vec<f32> = a.iter().map(|&s| f32::from(s) / 32768.0).collect();
        let mut h = CallHashes::new();
        let t0 = std::time::Instant::now();
        let out = decode(&f32a, &aurora_options(c), &mut h, &mut state);
        let ms = t0.elapsed().as_millis();
        let d = out.iter().find(|d| d.text == text(c));
        let other_j: Vec<&str> = jl.iter().filter(|l| l.text != text(c)).map(|l| l.text.as_str()).collect();
        let other_a: Vec<&str> = out.iter().filter(|d| d.text != text(c)).map(|d| d.text.as_str()).collect();
        println!(
            "{};{};{};{:.1};{:.0};{:+.2};{};{};{} ({} ms){}",
            if c.wspr { "FST4W" } else { "FST4" },
            c.period,
            c.qp,
            c.snr_db,
            c.f0,
            c.dt,
            c.msg,
            fmt_j(j, sync_of(&dec, text(c))),
            fmt_a(d),
            ms,
            if other_j.is_empty() && other_a.is_empty() { String::new() } else { format!(";ALTRO jt9 {other_j:?} aurora {other_a:?}") }
        );
        let _ = std::fs::remove_file(&path);
    }
}
