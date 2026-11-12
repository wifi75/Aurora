//! Confronto della ricezione WSPR-2 con `wsprd.exe` di WSJT-X: genera periodi di due minuti con il modulatore di Aurora (WAV a 12 kHz,
//! 16 bit, mono, messaggio a un secondo dall'inizio come in WSJT-X, rumore bianco gaussiano, rapporto in 2500 Hz), li decodifica con
//! `wsprd.exe` e con `jt::wspr::rx` alle stesse impostazioni e stampa una tabella (rapporto, ritardo, frequenza, deriva).
//!
//! Uso (serve `wsprd.exe` di WSJT-X con le sue DLL, per esempio estratto dall'installatore `wsjtx-3.3.0-beta6-win64.exe`):
//! `cargo run --release -p jt --example oracolo_wspr -- <wsprd.exe> <cartella di lavoro> [profondita' 1|2|3]`
//!
//! Le opzioni di `wsprd` sono quelle di WSJT-X (`widgets/mainwindow.cpp:2882-2887`): profondita' 3 (predefinita) `-C 500 -o 4 -d`, 2
//! `-C 500 -o 4`, 1 `-qB`; `-a` la cartella di lavoro, `-f` la frequenza del VFO (qui 0, cosi' la frequenza e' quella audio). La tabella degli
//! hash (`hashtable.txt`) si tiene fra un file e l'altro da tutte e due le parti, nello stesso ordine; la si azzera all'inizio. Rapporto,
//! ritardo, frequenza (0,1 Hz) e deriva di `wsprd` si leggono da `ALL_WSPR.TXT` (`wsprd.c:1582-1588`). `wsprd` gira con priorita' bassa
//! (Windows), un file alla volta. Non e' una prova automatica: serve `wsprd.exe` su questo PC.
//!
//! Esito con `wsprd.exe` di WSJT-X 3.3.0-beta6 alle profondita' 1, 2 e 3: in tutti i 48 casi stessi messaggi, stesso rapporto, ritardo
//! (0,01 s), frequenza (0,1 Hz), deriva, sincronismo (0,01), passata e decodifiche OSD; stesse mancate decodifiche sotto soglia
//! (-30/-32 dB, -31 dB con l'OSD sui nominativi gia' nella tabella degli hash).

use jt::wspr::rx::{Decoder, Options};
use jt::wspr::tx::{modulate, tones};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

const FS: usize = 12_000;
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

/// Un segnale: messaggio, rapporto in 2500 Hz, frequenza audio del centro, ritardo, deriva sul messaggio (Hz).
#[derive(Clone)]
struct Sig {
    msg: &'static str,
    snr_db: f64,
    f0: f64,
    dt: f64,
    drift: f64,
}

struct Case {
    name: String,
    sigs: Vec<Sig>,
    seed: u64,
}

fn sig(msg: &'static str, snr_db: f64, f0: f64, dt: f64, drift: f64) -> Sig {
    Sig { msg, snr_db, f0, dt, drift }
}

/// Due minuti di audio a 16 bit.
fn period(c: &Case) -> Vec<i16> {
    let mut rng = Lcg(c.seed);
    let n = 120 * FS;
    let mut audio: Vec<f64> = (0..n).map(|_| SIGMA * rng.gauss()).collect();
    for s in &c.sigs {
        let amp = (SIGMA * (2.0 * 10f64.powf(s.snr_db / 10.0) * 2500.0 / 6000.0).sqrt()) as f32;
        let Some(t) = tones(s.msg) else {
            eprintln!("messaggio non valido: {}", s.msg);
            continue;
        };
        let wave = modulate(&t, s.f0, amp, FS as f64, s.drift);
        let start = ((1.0 + s.dt) * FS as f64).round() as i64;
        for (i, v) in wave.iter().enumerate() {
            let k = start + i as i64;
            if k >= 0 && (k as usize) < n {
                audio[k as usize] += f64::from(*v);
            }
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

/// Una decodifica di `wsprd` da `ALL_WSPR.TXT`: `%6s %4s %3.0f %5.2f %11.7f  %-22s %2d %5.2f %2d %2d %4d %2d %3d %5u %5d`.
#[derive(Debug, Clone)]
struct WsprdLine {
    snr: f32,
    dt: f32,
    freq_hz: f64,
    message: String,
    drift: i32,
    sync: f32,
    pass: i32,
    osd: i32,
}

fn parse_all(line: &str) -> Option<WsprdLine> {
    let w: Vec<&str> = line.split_whitespace().collect();
    if w.len() < 5 + 1 + 9 {
        return None;
    }
    let tail = &w[w.len() - 9..];
    Some(WsprdLine {
        snr: w[2].parse().ok()?,
        dt: w[3].parse().ok()?,
        freq_hz: w[4].parse::<f64>().ok()? * 1e6,
        message: w[5..w.len() - 9].join(" "),
        drift: tail[0].parse().ok()?,
        sync: tail[1].parse().ok()?,
        pass: tail[2].parse().ok()?,
        osd: tail[5].parse().ok()?,
    })
}

fn wsprd_args(depth: u8) -> Vec<String> {
    match depth {
        1 => vec!["-qB".into()],
        2 => ["-C", "500", "-o", "4"].map(String::from).to_vec(),
        _ => ["-C", "500", "-o", "4", "-d"].map(String::from).to_vec(),
    }
}

/// Lancia `wsprd` su un file e ritorna le righe nuove di `ALL_WSPR.TXT` e il tempo impiegato.
fn run_wsprd(wsprd: &Path, work: &Path, depth: u8, file: &Path) -> (Vec<WsprdLine>, f64) {
    let all = work.join("ALL_WSPR.TXT");
    let _ = std::fs::remove_file(&all);
    let mut cmd = Command::new(wsprd);
    cmd.current_dir(work).args(wsprd_args(depth)).arg("-a").arg(work).arg("-f").arg("0").arg(file);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // IDLE_PRIORITY_CLASS: il PC resta libero
        cmd.creation_flags(0x0000_0040);
    }
    let t0 = Instant::now();
    let out = match cmd.output() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("wsprd non parte: {e}");
            return (Vec::new(), 0.0);
        }
    };
    let secs = t0.elapsed().as_secs_f64();
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        eprintln!("wsprd> {line}");
    }
    if !out.stderr.is_empty() {
        eprintln!("wsprd stderr: {}", String::from_utf8_lossy(&out.stderr).trim());
    }
    let lines = std::fs::read_to_string(&all).unwrap_or_default().lines().filter_map(parse_all).collect();
    (lines, secs)
}

fn cases() -> Vec<Case> {
    let mut v = Vec::new();
    let mut seed = 100u64;
    let mut next = || {
        seed += 1;
        seed
    };
    v.push(Case { name: "forte".into(), sigs: vec![sig("IU3ABC JN55 37", -10.0, 1500.0, 0.0, 0.0)], seed: next() });
    v.push(Case { name: "dt+1".into(), sigs: vec![sig("K1ABC FN42 30", -20.0, 1460.0, 1.0, 0.0)], seed: next() });
    v.push(Case { name: "dt-0.8".into(), sigs: vec![sig("G4ABC IO91 23", -22.0, 1540.0, -0.8, 0.0)], seed: next() });
    v.push(Case { name: "dt+2".into(), sigs: vec![sig("W9XYZ EN37 10", -22.0, 1420.0, 2.0, 0.0)], seed: next() });
    for snr in [-24.0, -26.0, -27.0, -28.0, -29.0, -30.0, -31.0, -32.0, -33.0] {
        for k in 0..3 {
            let f = 1430.0 + 47.0 * k as f64;
            v.push(Case { name: format!("soglia{snr}"), sigs: vec![sig("IU3ABC IO91 37", snr, f, 0.3 * k as f64 - 0.3, 0.0)], seed: next() });
        }
    }
    // nominativi mai visti (niente OSD: la tabella degli hash non li conosce)
    for (k, (msg, snr)) in [("DL1ABC JO62 27", -28.0), ("VK2XYZ QF56 33", -29.0), ("JA1ABC PM95 40", -30.0), ("ZS6ABC KG33 20", -31.0)].into_iter().enumerate() {
        v.push(Case { name: format!("nuovo{snr}"), sigs: vec![sig(msg, snr, 1450.0 + 30.0 * k as f64, 0.1, 0.0)], seed: next() });
    }
    for drift in [1.0, -2.0, 3.0, -4.0] {
        v.push(Case { name: format!("deriva{drift:+}"), sigs: vec![sig("K1ABC FN42 37", -22.0, 1510.0, 0.2, drift)], seed: next() });
    }
    v.push(Case {
        name: "tre stazioni".into(),
        sigs: vec![
            sig("K1ABC FN42 37", -12.0, 1500.0, 0.0, 0.0),
            sig("G4ABC IO91 23", -24.0, 1505.0, 0.4, 0.0),
            sig("W9XYZ EN37 10", -20.0, 1580.0, -0.2, 1.0),
        ],
        seed: next(),
    });
    // tipo 2 e poi tipo 3 dello stesso nominativo (la tabella degli hash si riempie nel primo periodo)
    v.push(Case {
        name: "tipo2".into(),
        sigs: vec![sig("PJ4/K1ABC 37", -18.0, 1470.0, 0.0, 0.0), sig("IU3ABC JN55 30", -18.0, 1530.0, 0.0, 0.0)],
        seed: next(),
    });
    v.push(Case {
        name: "tipo3".into(),
        sigs: vec![sig("<PJ4/K1ABC> FK52UD 37", -18.0, 1470.0, 0.0, 0.0), sig("<IU3ABC> IO91WM 30", -18.0, 1530.0, 0.0, 0.0)],
        seed: next(),
    });
    v.push(Case { name: "suffisso".into(), sigs: vec![sig("IU3ABC/P 33", -20.0, 1490.0, 0.0, 0.0)], seed: next() });
    v.push(Case { name: "solo rumore".into(), sigs: vec![], seed: next() });
    v
}

fn main() {
    let mut args = std::env::args().skip(1);
    let (Some(wsprd), Some(work)) = (args.next(), args.next()) else {
        eprintln!("uso: oracolo_wspr <wsprd.exe> <cartella di lavoro> [profondita' 1|2|3]");
        return;
    };
    let depth: u8 = args.next().and_then(|s| s.parse().ok()).unwrap_or(3);
    let work = PathBuf::from(work);
    if let Err(e) = std::fs::create_dir_all(&work) {
        eprintln!("{}: {e}", work.display());
        return;
    }
    let _ = std::fs::remove_file(work.join("hashtable.txt"));
    let wsprd = PathBuf::from(wsprd);
    let mut decoder = Decoder::new();
    let opts = Options::depth(depth);
    let (mut t_wsprd, mut t_aurora) = (0.0, 0.0);
    println!("caso;inviato;messaggio;wsprd snr/dt/f/deriva/sync/passata/osd;aurora snr/dt/f/deriva/sync/passata/osd");
    for (i, c) in cases().iter().enumerate() {
        let audio = period(c);
        // `wsprd` legge data e ora dal nome: aammgg_hhmm.wav (`wsprd.c:1005-1008`)
        let path = work.join(format!("260101_{:04}.wav", 2 * i));
        if let Err(e) = write_wav(&path, &audio) {
            eprintln!("{}: {e}", path.display());
            continue;
        }
        let (wl, tw) = run_wsprd(&wsprd, &work, depth, &path);
        t_wsprd += tw;
        let f: Vec<f32> = audio.iter().map(|&s| f32::from(s) / 32768.0).collect();
        let t0 = Instant::now();
        let al = decoder.decode(&f, &opts);
        let ta = t0.elapsed().as_secs_f64();
        t_aurora += ta;
        let mut msgs: Vec<String> = wl.iter().map(|l| l.message.clone()).chain(al.iter().map(|d| d.message.clone())).collect();
        msgs.dedup();
        let mut seen = std::collections::BTreeSet::new();
        msgs.retain(|m| seen.insert(m.clone()));
        let sent: Vec<&str> = c.sigs.iter().map(|s| s.msg).collect();
        if msgs.is_empty() {
            println!("{};{:?};-;no;no", c.name, sent);
        }
        for m in msgs {
            let w = wl.iter().find(|l| l.message == m);
            let a = al.iter().find(|d| d.message == m);
            let ws = w.map_or("no".to_string(), |l| {
                format!("{:4.0} {:+5.2} {:7.1} {:+2} {:.2} p{} o{}", l.snr, l.dt, l.freq_hz, l.drift, l.sync, l.pass, l.osd)
            });
            let as_ = a.map_or("no".to_string(), |d| {
                format!(
                    "{:4.0} {:+5.2} {:7.1} {:+2} {:.2} p{} o{}",
                    d.snr_db,
                    d.dt_s,
                    d.audio_hz,
                    d.drift_hz as i32,
                    d.sync,
                    d.pass + 1,
                    i32::from(d.osd)
                )
            });
            println!("{};{:?};{};{};{}", c.name, sent, m, ws, as_);
        }
        eprintln!("{}: wsprd {tw:.1} s, aurora {ta:.1} s", c.name);
        let _ = std::fs::remove_file(&path);
    }
    println!("tempo totale: wsprd {t_wsprd:.1} s, aurora {t_aurora:.1} s");
}
