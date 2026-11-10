//! I thread della decodifica, creati una volta sola e riusati: prima ogni periodo (ogni 15 s, e di nuovo per "a8") costruiva un
//! gruppo di thread nuovo e lo buttava. I thread hanno priorita' sotto il normale su Windows, cosi' audio, CAT e finestra restano
//! pronti anche quando la decodifica occupa tutti i processori.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock},
};

/// Il gruppo di `threads` thread (uno per numero di thread chiesto; di solito ne serve uno solo). `None` con un thread solo o se il
/// sistema non ne crea: si decodifica nel thread che chiama.
pub fn shared(threads: usize) -> Option<Arc<rayon::ThreadPool>> {
    if threads <= 1 {
        return None;
    }
    static POOLS: OnceLock<Mutex<HashMap<usize, Arc<rayon::ThreadPool>>>> = OnceLock::new();
    let mut pools = POOLS.get_or_init(|| Mutex::new(HashMap::new())).lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(pool) = pools.get(&threads) {
        return Some(Arc::clone(pool));
    }
    let pool = Arc::new(
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .thread_name(|i| format!("aurora-decodifica-{i}"))
            .start_handler(|_| lower_priority())
            .build()
            .ok()?,
    );
    pools.insert(threads, Arc::clone(&pool));
    Some(pool)
}

/// Quanti thread usare per decodificare: tutti i processori meno uno (lasciato ad audio, CAT e finestra), almeno uno.
pub fn decode_threads() -> usize {
    std::thread::available_parallelism().map_or(1, std::num::NonZero::get).saturating_sub(1).max(1)
}

#[cfg(windows)]
fn lower_priority() {
    use windows::Win32::System::Threading::{GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_BELOW_NORMAL};
    // SAFETY: GetCurrentThread e' una pseudo-maniglia del thread corrente, sempre valida
    let _ = unsafe { SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_BELOW_NORMAL) };
}

#[cfg(not(windows))]
fn lower_priority() {}

#[cfg(test)]
mod tests {
    #[test]
    fn the_pool_is_reused_and_one_thread_needs_none() {
        assert!(super::shared(1).is_none());
        let a = super::shared(3).expect("gruppo");
        let b = super::shared(3).expect("gruppo");
        assert!(std::sync::Arc::ptr_eq(&a, &b));
        assert!(super::decode_threads() >= 1);
    }
}
