//! Risorse di Aurora per la «spia di salute» (`GET /api/v1/health/resources`): CPU del processo (normalizzata sul numero di
//! processori, tra due letture), memoria privata, ultima decodifica e storia degli ultimi 10 minuti. Un thread a bassa frequenza
//! prende un campione ogni 5 s (`aurora_core::resources`); la richiesta legge solo i dati gia' pronti.

use std::{
    sync::OnceLock,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use aurora_core::resources;
use serde_json::{Value, json};

/// Tempo di CPU del processo (utente + sistema) consumato finora.
#[cfg(windows)]
fn process_cpu_time() -> Option<Duration> {
    use windows::Win32::{Foundation::FILETIME, System::Threading::{GetCurrentProcess, GetProcessTimes}};
    let (mut c, mut e, mut k, mut u) = (FILETIME::default(), FILETIME::default(), FILETIME::default(), FILETIME::default());
    // SAFETY: lo pseudo-handle del processo corrente e' sempre valido; le quattro strutture sono nostre e scrivibili.
    unsafe { GetProcessTimes(GetCurrentProcess(), &mut c, &mut e, &mut k, &mut u) }.ok()?;
    let t = |f: FILETIME| (u64::from(f.dwHighDateTime) << 32) | u64::from(f.dwLowDateTime);
    // unita' da 100 ns
    Some(Duration::from_nanos((t(k) + t(u)).saturating_mul(100)))
}

#[cfg(target_os = "linux")]
fn process_cpu_time() -> Option<Duration> {
    let stat = std::fs::read_to_string("/proc/self/stat").ok()?;
    // dopo il nome tra parentesi: campi dal terzo in poi; utime e stime sono il 14° e il 15°
    let rest = stat.rsplit_once(')')?.1;
    let f: Vec<&str> = rest.split_whitespace().collect();
    let utime: u64 = f.get(11)?.parse().ok()?;
    let stime: u64 = f.get(12)?.parse().ok()?;
    // tick di sistema: quasi ovunque 100 al secondo
    Some(Duration::from_millis((utime + stime) * 10))
}

#[cfg(not(any(windows, target_os = "linux")))]
fn process_cpu_time() -> Option<Duration> {
    None
}

/// Memoria privata del processo in MB.
#[cfg(windows)]
fn private_mb() -> Option<u64> {
    use windows::Win32::System::{
        ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS, PROCESS_MEMORY_COUNTERS_EX},
        Threading::GetCurrentProcess,
    };
    let mut counters = PROCESS_MEMORY_COUNTERS_EX { cb: std::mem::size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32, ..Default::default() };
    // SAFETY: la struttura e' nostra, `cb` ne dice la dimensione; l'estesa comincia come quella base.
    unsafe { GetProcessMemoryInfo(GetCurrentProcess(), (&mut counters as *mut PROCESS_MEMORY_COUNTERS_EX).cast::<PROCESS_MEMORY_COUNTERS>(), counters.cb) }.ok()?;
    Some(counters.PrivateUsage as u64 / (1024 * 1024))
}

#[cfg(target_os = "linux")]
fn private_mb() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let line = status.lines().find(|l| l.starts_with("RssAnon:"))?;
    let kb: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
    Some(kb / 1024)
}

#[cfg(not(any(windows, target_os = "linux")))]
fn private_mb() -> Option<u64> {
    None
}

fn cores() -> usize {
    std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1)
}

fn now_unix_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

/// Percentuale di CPU tra due letture, su tutti i processori (0-100).
pub fn cpu_percent(cpu_delta: Duration, wall_delta: Duration, cores: usize) -> f64 {
    let wall = wall_delta.as_secs_f64() * cores.max(1) as f64;
    if wall <= 0.0 {
        return 0.0;
    }
    (cpu_delta.as_secs_f64() / wall * 100.0).clamp(0.0, 100.0)
}

/// Avvia (una volta sola) il thread che prende un campione ogni 5 s.
pub fn spawn_sampler() {
    static STARTED: OnceLock<()> = OnceLock::new();
    if STARTED.set(()).is_err() {
        return;
    }
    let _ = std::thread::Builder::new().name("aurora-risorse".into()).spawn(|| {
        let mut prev = (Instant::now(), process_cpu_time());
        loop {
            std::thread::sleep(Duration::from_secs(resources::SAMPLE_EVERY_SECS));
            let now = (Instant::now(), process_cpu_time());
            let pct = match (prev.1, now.1) {
                (Some(a), Some(b)) => cpu_percent(b.saturating_sub(a), now.0.duration_since(prev.0), cores()),
                _ => 0.0,
            };
            resources::push_sample(now_unix_ms(), (pct * 10.0).round() / 10.0);
            prev = now;
        }
    });
}

/// Il corpo della risposta di `GET /api/v1/health/resources`.
pub fn snapshot() -> Value {
    let last = resources::last_decode();
    let history: Vec<Value> = resources::history()
        .iter()
        .map(|s| json!({ "t": s.t_unix_ms, "cpu_pct": s.cpu_pct, "decode_ms": s.decode_ms, "period_ms": s.period_ms }))
        .collect();
    json!({
        "cpu_pct": resources::latest().map(|s| s.cpu_pct).unwrap_or(0.0),
        "mem_mb": private_mb(),
        "cores": cores(),
        "decode_threads": last.map(|l| l.threads),
        "last_decode_ms": last.map(|l| l.decode_ms),
        "period_ms": last.map(|l| l.period_ms),
        "last_decode_at": last.map(|l| l.at_unix_ms),
        "sample_every_s": resources::SAMPLE_EVERY_SECS,
        "history": history,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_is_normalised_on_the_cores() {
        let p = cpu_percent(Duration::from_secs(2), Duration::from_secs(5), 4);
        assert!((p - 10.0).abs() < 1e-9);
        assert_eq!(cpu_percent(Duration::from_secs(1), Duration::ZERO, 4), 0.0);
        assert_eq!(cpu_percent(Duration::from_secs(50), Duration::from_secs(5), 2), 100.0);
    }

    #[test]
    fn the_snapshot_has_the_fields() {
        let v = snapshot();
        assert!(v.get("cpu_pct").is_some() && v.get("history").is_some_and(Value::is_array));
    }
}
