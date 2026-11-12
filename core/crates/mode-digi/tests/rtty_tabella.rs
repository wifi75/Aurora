//! Tabella del ricevitore RTTY sugli scenari comuni (`tests/common/rtty_scenari.rs`, passi 1 e 3 di `docs/progetto-rtty-oltre.md`):
//! per ogni scenario e ogni ricevitore la curva del tasso di caratteri sbagliati (CER) sulla griglia di S/N, la soglia al 5 e al 10 % e il
//! CER medio sulla griglia; a parte i caratteri inventati su un minuto di solo rumore. Con le feature `fldigi-oracle` e `mmtty-oracle`
//! entrano anche fldigi 4.2.13 e i sette profili di MMTTY 1.70L (per ogni scenario si riporta il migliore).
//!
//! Aurora lavora con le impostazioni di base del servizio, AFC spento come negli arbitri.
//!
//! `cargo test --release -j 12 -p mode-digi --features fldigi-oracle,mmtty-oracle --test rtty_tabella -- --ignored --nocapture --test-threads=1`
//!
//! Variabili: `RTTY_RUNS` prove da 120 caratteri per cella (4 di base), `RTTY_PASSO` passo della griglia in dB (1 di base),
//! `RTTY_SCENARIO` solo alcune righe (nomi separati da virgole), `RTTY_THREADS` (6 di base), `RTTY_CSV` file dove scrivere la tabella, `RTTY_BASE` una tabella
//! scritta prima con cui confrontare Aurora (la prova fallisce se una soglia peggiora di oltre 0,5 dB o il CER medio di oltre 1 punto),
//! `AURORA_WAV` cartella dove scrivere i WAV a 0 dB di ogni scenario (per 2Tone,  e gli altri sul PC del proprietario).

mod common;

use common::rtty_scenari::{CENTRE, Scenario, errori, genera, scenari, scenario, soglia, solo_rumore};
use mode_digi::rtty::RttyDemodulator;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Caratteri per prova.
const CHARS: usize = 120;
/// La griglia di S/N (dB in 3 kHz).
const SNR_MIN: f64 = -15.0;
const SNR_MAX: f64 = 6.0;

fn env_num(name: &str, default: f64) -> f64 {
    std::env::var(name).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

/// Il ricevitore di Aurora con le impostazioni di base del servizio (`DigiSettings::default`), AFC spento.
pub fn aurora(sc: &Scenario) -> RttyDemodulator {
    let mut d = RttyDemodulator::new(sc.params(CENTRE));
    d.set_uos(true);
    d.set_discriminator(sc.shift < 40.0);
    d
}

/// I ricevitori della tabella: nome e funzione che decodifica l'audio a 12 kHz.
fn ricevitori() -> Vec<&'static str> {
    #[allow(unused_mut)]
    let mut v = vec!["Aurora"];
    #[cfg(feature = "fldigi-oracle")]
    v.push("fldigi");
    #[cfg(feature = "mmtty-oracle")]
    for p in mode_digi::mmtty_oracle::MmttyProfile::ALL {
        v.push(p.name());
    }
    v
}

fn decodifica(nome: &str, sc: &Scenario, audio: &[f32]) -> Vec<u8> {
    let mut got = Vec::new();
    match nome {
        "Aurora" => {
            let mut rx = aurora(sc);
            for block in audio.chunks(1024) {
                got.extend(rx.process(block));
            }
        }
        #[cfg(feature = "fldigi-oracle")]
        "fldigi" => {
            let mut rx = mode_digi::fldigi_oracle::FldigiRtty::new(f64::from(sc.baud), f64::from(sc.shift), f64::from(CENTRE), 8000.0);
            for block in common::a_8k(audio).chunks(1024) {
                got.extend(rx.process(block));
            }
        }
        #[cfg(feature = "mmtty-oracle")]
        other => {
            if let Some(p) = mode_digi::mmtty_oracle::MmttyProfile::ALL.into_iter().find(|p| p.name() == other) {
                let mut rx = mode_digi::mmtty_oracle::MmttyRtty::new(p, f64::from(sc.baud), f64::from(sc.shift), f64::from(CENTRE), false);
                for block in audio.chunks(1024) {
                    got.extend(rx.process(block));
                }
            }
        }
        #[cfg(not(feature = "mmtty-oracle"))]
        _ => {}
    }
    got
}

/// Una riga della tabella: per ogni ricevitore i punti (S/N, CER).
struct Riga {
    sc: Scenario,
    curve: Vec<Vec<(f64, f64)>>,
}

impl Riga {
    fn soglie(&self, k: usize) -> (Option<f64>, Option<f64>, f64) {
        let c = &self.curve[k];
        let media = c.iter().map(|p| p.1).sum::<f64>() / c.len().max(1) as f64;
        (soglia(c, 0.05), soglia(c, 0.10), 100.0 * media)
    }
}

fn fmt_soglia(s: Option<f64>) -> String {
    s.map_or("  --  ".to_string(), |v| format!("{v:6.1}"))
}

fn calcola(sc: Scenario, nomi: &[&str], runs: u64, passo: f64) -> Riga {
    let n_snr = ((SNR_MAX - SNR_MIN) / passo).round() as usize + 1;
    let mut curve = vec![Vec::new(); nomi.len()];
    for i in 0..n_snr {
        let snr = SNR_MIN + i as f64 * passo;
        let mut err = vec![0usize; nomi.len()];
        let mut tot = 0usize;
        for r in 0..runs {
            let (audio, sent) = genera(&sc, snr, 7000 + r * 101 + i as u64 * 13, CHARS);
            tot += sent.len();
            for (k, nome) in nomi.iter().enumerate() {
                err[k] += errori(&sent, decodifica(nome, &sc, &audio));
            }
        }
        for k in 0..nomi.len() {
            curve[k].push((snr, (err[k] as f64 / tot.max(1) as f64).min(1.5)));
        }
    }
    Riga { sc, curve }
}

/// La tabella completa (lenta: si lancia a mano).
#[test]
#[ignore]
fn tabella() {
    let runs = env_num("RTTY_RUNS", 4.0) as u64;
    let passo = env_num("RTTY_PASSO", 1.0);
    let threads = env_num("RTTY_THREADS", 6.0) as usize;
    let nomi = ricevitori();
    let lista: Vec<Scenario> = match std::env::var("RTTY_SCENARIO") {
        Ok(n) => n.split(',').filter_map(|x| scenario(x.trim())).collect(),
        Err(_) => scenari(),
    };
    if let Some(dir) = std::env::var_os("AURORA_WAV") {
        let _ = dir;
        for sc in &lista {
            let (audio, _) = genera(sc, 0.0, 1, 600);
            common::salva_wav(&format!("rtty_{}", sc.nome.replace([' ', '/', '+'], "_")), 12_000, &audio);
        }
    }
    let prossimo = AtomicUsize::new(0);
    let righe: Mutex<Vec<Option<Riga>>> = Mutex::new((0..lista.len()).map(|_| None).collect());
    std::thread::scope(|s| {
        for _ in 0..threads.max(1) {
            s.spawn(|| {
                loop {
                    let i = prossimo.fetch_add(1, Ordering::SeqCst);
                    if i >= lista.len() {
                        break;
                    }
                    let riga = calcola(lista[i], &nomi, runs, passo);
                    if let Ok(mut r) = righe.lock() {
                        r[i] = Some(riga);
                    }
                }
            });
        }
    });
    let righe: Vec<Riga> = righe.into_inner().unwrap_or_default().into_iter().flatten().collect();
    let mut csv = String::from("scenario;ricevitore;soglia5;soglia10;cer_medio\n");
    println!("soglie S/N (dB in 3 kHz) al 10 % e CER medio sulla griglia {SNR_MIN}..{SNR_MAX} dB (passo {passo}, {} caratteri per cella)", runs as usize * CHARS);
    println!("{:<18}| {:>17} | {:>17} | {:>28}", "scenario", "Aurora s10 / CER", "fldigi s10 / CER", "miglior MMTTY s10 / CER");
    for riga in &righe {
        let mut celle = Vec::new();
        for (k, nome) in nomi.iter().enumerate() {
            let (s5, s10, m) = riga.soglie(k);
            csv.push_str(&format!("{};{};{};{};{m:.2}\n", riga.sc.nome, nome, s5.map_or(String::new(), |v| format!("{v:.2}")), s10.map_or(String::new(), |v| format!("{v:.2}"))));
            celle.push((nome, s10, m));
        }
        let aurora = celle.first().map(|c| format!("{} / {:5.1}", fmt_soglia(c.1), c.2)).unwrap_or_default();
        let fldigi = celle.iter().find(|c| *c.0 == "fldigi").map(|c| format!("{} / {:5.1}", fmt_soglia(c.1), c.2)).unwrap_or_default();
        let mmtty = celle
            .iter()
            .skip(if cfg!(feature = "fldigi-oracle") { 2 } else { 1 })
            .min_by(|a, b| a.1.unwrap_or(99.0).total_cmp(&b.1.unwrap_or(99.0)).then(a.2.total_cmp(&b.2)))
            .map(|c| format!("{:<9} {} / {:5.1}", c.0, fmt_soglia(c.1), c.2))
            .unwrap_or_default();
        println!("{:<18}| {aurora:>17} | {fldigi:>17} | {mmtty:>28}", riga.sc.nome);
    }
    if let Some(path) = std::env::var_os("RTTY_CSV")
        && let Err(e) = std::fs::write(&path, &csv)
    {
        eprintln!("tabella non scritta: {e}");
    }
    // confronto con una tabella di prima (solo Aurora)
    if let Some(base) = std::env::var_os("RTTY_BASE").and_then(|p| std::fs::read_to_string(p).ok()) {
        let mut peggiori = Vec::new();
        for line in base.lines().skip(1) {
            let f: Vec<&str> = line.split(';').collect();
            if f.len() < 5 || f[1] != "Aurora" {
                continue;
            }
            let Some(riga) = righe.iter().find(|r| r.sc.nome == f[0]) else { continue };
            let (_, s10, m) = riga.soglie(0);
            let b10: Option<f64> = f[3].parse().ok();
            let bm: f64 = f[4].parse().unwrap_or(0.0);
            let ds = match (s10, b10) {
                (Some(a), Some(b)) => a - b,
                (None, Some(_)) => 99.0,
                _ => 0.0,
            };
            println!("{:<18} soglia {:+5.2} dB, CER medio {:+5.2} punti", f[0], ds, m - bm);
            if ds > 0.5 || m - bm > 1.0 {
                peggiori.push(f[0].to_string());
            }
        }
        assert!(peggiori.is_empty(), "scenari dove Aurora peggiora: {peggiori:?}");
    }
}

/// Caratteri inventati su un minuto di solo rumore, con le impostazioni di base (squelch spento) e, per Aurora e MMTTY, con lo squelch.
#[test]
#[ignore]
fn inventati_sul_rumore() {
    let sc = scenari()[0];
    let audio = solo_rumore(60.0, 99);
    for nome in ricevitori() {
        let n = decodifica(nome, &sc, &audio).iter().filter(|&&c| c != b'\r' && c != b'\n').count();
        println!("{nome:<10} squelch spento: {n:4} caratteri al minuto");
    }
    let mut rx = aurora(&sc);
    rx.set_squelch_snr_db(Some(6.0));
    let n = audio.chunks(1024).map(|b| rx.process(b).len()).sum::<usize>();
    println!("Aurora     squelch S/N 6 dB: {n:4} caratteri al minuto");
    #[cfg(feature = "mmtty-oracle")]
    for p in mode_digi::mmtty_oracle::MmttyProfile::ALL {
        let mut rx = mode_digi::mmtty_oracle::MmttyRtty::new(p, f64::from(sc.baud), f64::from(sc.shift), f64::from(CENTRE), true);
        let n = rx.process(&audio).len();
        println!("{:<10} squelch del profilo: {n:4} caratteri al minuto", p.name());
    }
}

/// Le misure di base funzionano: a S/N alto Aurora riceve in ogni scenario (prova veloce, sempre accesa), tranne il fading veloce
/// (flutter e alte latitudini), che sbaglia molto anche senza rumore, e l'AGC che pompa (misurato a parte nella tabella).
#[test]
fn a_snr_alto_aurora_riceve_in_ogni_scenario() {
    for sc in scenari().into_iter().filter(|s| !matches!(s.disturbo, common::rtty_scenari::Disturbo::Fading { spread, .. } if spread >= 10.0) && !matches!(s.disturbo, common::rtty_scenari::Disturbo::Agc)) {
        let (audio, sent) = genera(&sc, 20.0, 3, 60);
        let e = errori(&sent, decodifica("Aurora", &sc, &audio));
        // l'AGC che pompa e le scariche lasciano qualche errore anche a S/N alto
        assert!(e * 100 <= sent.len() * 25, "{}: {e} errori su {}", sc.nome, sent.len());
    }
}
