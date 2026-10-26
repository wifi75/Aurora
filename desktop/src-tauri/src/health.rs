//! Diagnostica del processo: i panic finiscono nel log e ogni minuto si scrivono memoria, maniglie, oggetti grafici e thread, cosi' un
//! arresto o un consumo anomalo lasciano una traccia leggibile (il programma e' senza console e i simboli sono tolti).

use std::time::Duration;

/// Oltre queste soglie si scrive un avviso: Windows tiene 10000 oggetti USER/GDI per processo e la sessione ha una memoria grafica comune.
const WARN_HANDLES: u32 = 8_000;
const WARN_GUI_OBJECTS: u32 = 5_000;
const WARN_PRIVATE_MB: u64 = 1_500;

/// Fa scrivere nel log ogni panic (con il thread e il punto) prima che il processo si fermi.
pub fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let thread = std::thread::current();
        tracing::error!("panic nel thread «{}»: {info}", thread.name().unwrap_or("senza nome"));
        previous(info);
    }));
}

/// Misure del processo, `None` fuori da Windows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Usage {
    pub private_mb: u64,
    pub working_set_mb: u64,
    pub handles: u32,
    pub user_objects: u32,
    pub gdi_objects: u32,
}

#[cfg(windows)]
pub fn sample() -> Option<Usage> {
    use windows_sys::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS_EX};
    use windows_sys::Win32::System::Threading::{GR_GDIOBJECTS, GR_USEROBJECTS, GetCurrentProcess, GetGuiResources, GetProcessHandleCount};
    // SAFETY: `GetCurrentProcess` restituisce lo pseudo-handle del processo, valido sempre; le strutture sono inizializzate a zero e
    // `cb` dice a Windows quanto sono grandi.
    unsafe {
        let process = GetCurrentProcess();
        let mut counters: PROCESS_MEMORY_COUNTERS_EX = std::mem::zeroed();
        counters.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32;
        if GetProcessMemoryInfo(process, (&mut counters as *mut PROCESS_MEMORY_COUNTERS_EX).cast(), counters.cb) == 0 {
            return None;
        }
        let mut handles = 0u32;
        if GetProcessHandleCount(process, &mut handles) == 0 {
            return None;
        }
        Some(Usage {
            private_mb: counters.PrivateUsage as u64 / (1024 * 1024),
            working_set_mb: counters.WorkingSetSize as u64 / (1024 * 1024),
            handles,
            user_objects: GetGuiResources(process, GR_USEROBJECTS),
            gdi_objects: GetGuiResources(process, GR_GDIOBJECTS),
        })
    }
}

#[cfg(not(windows))]
pub fn sample() -> Option<Usage> {
    None
}

/// `true` se una misura supera una soglia (il log passa a `warn` ogni minuto; altrimenti una riga `info` ogni 10 minuti).
pub fn is_heavy(u: &Usage) -> bool {
    u.handles > WARN_HANDLES || u.user_objects > WARN_GUI_OBJECTS || u.gdi_objects > WARN_GUI_OBJECTS || u.private_mb > WARN_PRIVATE_MB
}

/// Avvia il controllo periodico in un thread a parte (costa una manciata di chiamate al minuto).
pub fn spawn_monitor() {
    let _ = std::thread::Builder::new().name("aurora-salute".into()).spawn(|| {
        let mut minutes = 0u32;
        loop {
            std::thread::sleep(Duration::from_secs(60));
            minutes += 1;
            let Some(u) = sample() else { return };
            let line = format!(
                "risorse del processo: {} MB privati, {} MB in memoria, {} maniglie, {} oggetti USER, {} oggetti GDI",
                u.private_mb, u.working_set_mb, u.handles, u.user_objects, u.gdi_objects
            );
            if is_heavy(&u) {
                tracing::warn!("{line}");
            } else if minutes.is_multiple_of(10) {
                tracing::info!("{line}");
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn usage(private_mb: u64, handles: u32, user: u32, gdi: u32) -> Usage {
        Usage { private_mb, working_set_mb: private_mb, handles, user_objects: user, gdi_objects: gdi }
    }

    #[test]
    fn a_normal_process_is_not_heavy() {
        assert!(!is_heavy(&usage(300, 900, 200, 300)));
    }

    #[test]
    fn each_limit_alone_makes_it_heavy() {
        assert!(is_heavy(&usage(1_600, 900, 200, 300)));
        assert!(is_heavy(&usage(300, 9_000, 200, 300)));
        assert!(is_heavy(&usage(300, 900, 6_000, 300)));
        assert!(is_heavy(&usage(300, 900, 200, 6_000)));
    }

    #[cfg(windows)]
    #[test]
    fn the_running_test_process_can_be_measured() {
        let u = sample().expect("misura");
        assert!(u.handles > 0 && u.private_mb > 0);
    }
}
