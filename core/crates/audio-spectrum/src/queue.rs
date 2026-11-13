//! Coda dell'audio verso il motore con un tetto in secondi.
//!
//! Il motore svuota la coda nello stesso thread in cui decodifica: durante una decodifica lunga (fino a 11 s, poi la scadenza) i blocchi si
//! accumulano e poi si recuperano. Con un canale senza limite, se quel thread restasse fermo (CAT o PTT appesi) la memoria crescerebbe senza
//! fine (circa 190 kB al secondo a 48 kHz). Qui, oltre il tetto, si buttano i blocchi piu' vecchi e si scrive un avviso nel registro (al
//! massimo una riga ogni `WARN_EVERY`). Il tetto va tenuto ben sopra la decodifica piu' lunga, cosi' nell'uso normale non si perde niente.

use std::collections::VecDeque;
use std::sync::mpsc::RecvTimeoutError;
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use crate::AudioChunk;

/// Tetto di base: 30 secondi di audio (circa 5,8 MB a 48 kHz), quasi tre volte la decodifica piu' lunga del motore.
pub const DEFAULT_MAX_SECS: f64 = 30.0;
/// Al massimo un avviso nel registro ogni tanto, anche se si buttano blocchi di continuo.
const WARN_EVERY: Duration = Duration::from_secs(30);

/// Chi riceve i blocchi audio: il canale con il tetto di questo modulo o un `mpsc::Receiver` (prove).
pub trait ChunkSource: Send + 'static {
    fn recv_timeout(&self, timeout: Duration) -> Result<AudioChunk, RecvTimeoutError>;
}

impl ChunkSource for std::sync::mpsc::Receiver<AudioChunk> {
    fn recv_timeout(&self, timeout: Duration) -> Result<AudioChunk, RecvTimeoutError> {
        std::sync::mpsc::Receiver::recv_timeout(self, timeout)
    }
}

struct State {
    chunks: VecDeque<AudioChunk>,
    /// Durata dei blocchi in coda, in campioni divisi per la frequenza (secondi).
    queued_secs: f64,
    senders: usize,
    receiver: bool,
    /// Blocchi e secondi buttati dall'ultimo avviso.
    dropped_chunks: u64,
    dropped_secs: f64,
    last_warn: Option<Instant>,
}

struct Shared {
    state: Mutex<State>,
    ready: Condvar,
    max_secs: f64,
}

impl Shared {
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|p| p.into_inner())
    }
}

fn secs_of(chunk: &AudioChunk) -> f64 {
    if chunk.rate == 0 { 0.0 } else { chunk.samples.len() as f64 / f64::from(chunk.rate) }
}

/// Crea la coda: `max_secs` e' il tetto in secondi di audio.
pub fn channel(max_secs: f64) -> (AudioSender, AudioReceiver) {
    let shared = Arc::new(Shared {
        state: Mutex::new(State {
            chunks: VecDeque::new(),
            queued_secs: 0.0,
            senders: 1,
            receiver: true,
            dropped_chunks: 0,
            dropped_secs: 0.0,
            last_warn: None,
        }),
        ready: Condvar::new(),
        max_secs,
    });
    (AudioSender { shared: Arc::clone(&shared) }, AudioReceiver { shared })
}

/// Il destinatario non c'e' piu' (motore fermo): il blocco torna indietro.
#[derive(Debug)]
pub struct Disconnected(pub AudioChunk);

pub struct AudioSender {
    shared: Arc<Shared>,
}

impl AudioSender {
    /// Mette in coda un blocco; se si supera il tetto si buttano i piu' vecchi (mai quello appena arrivato).
    pub fn send(&self, chunk: AudioChunk) -> Result<(), Disconnected> {
        let mut st = self.shared.lock();
        if !st.receiver {
            return Err(Disconnected(chunk));
        }
        st.queued_secs += secs_of(&chunk);
        st.chunks.push_back(chunk);
        while st.queued_secs > self.shared.max_secs && st.chunks.len() > 1 {
            let Some(old) = st.chunks.pop_front() else { break };
            let s = secs_of(&old);
            st.queued_secs -= s;
            st.dropped_chunks += 1;
            st.dropped_secs += s;
        }
        if st.dropped_chunks > 0 && st.last_warn.is_none_or(|t| t.elapsed() >= WARN_EVERY) {
            tracing::warn!(
                "audio verso il motore: il motore resta indietro, buttati {} blocchi vecchi ({:.1} s di audio) per non superare {:.0} s in coda",
                st.dropped_chunks,
                st.dropped_secs,
                self.shared.max_secs
            );
            st.dropped_chunks = 0;
            st.dropped_secs = 0.0;
            st.last_warn = Some(Instant::now());
        }
        drop(st);
        self.shared.ready.notify_one();
        Ok(())
    }

    /// Secondi di audio in coda adesso.
    pub fn queued_secs(&self) -> f64 {
        self.shared.lock().queued_secs
    }
}

impl Clone for AudioSender {
    fn clone(&self) -> Self {
        self.shared.lock().senders += 1;
        Self { shared: Arc::clone(&self.shared) }
    }
}

impl Drop for AudioSender {
    fn drop(&mut self) {
        let mut st = self.shared.lock();
        st.senders = st.senders.saturating_sub(1);
        drop(st);
        self.shared.ready.notify_all();
    }
}

pub struct AudioReceiver {
    shared: Arc<Shared>,
}

impl AudioReceiver {
    fn take(&self, st: &mut State) -> Option<AudioChunk> {
        let chunk = st.chunks.pop_front()?;
        st.queued_secs = (st.queued_secs - secs_of(&chunk)).max(0.0);
        if st.chunks.is_empty() {
            st.queued_secs = 0.0;
        }
        Some(chunk)
    }
}

impl ChunkSource for AudioReceiver {
    fn recv_timeout(&self, timeout: Duration) -> Result<AudioChunk, RecvTimeoutError> {
        let until = Instant::now() + timeout;
        let mut st = self.shared.lock();
        loop {
            if let Some(chunk) = self.take(&mut st) {
                return Ok(chunk);
            }
            if st.senders == 0 {
                return Err(RecvTimeoutError::Disconnected);
            }
            let now = Instant::now();
            if now >= until {
                return Err(RecvTimeoutError::Timeout);
            }
            st = self.shared.ready.wait_timeout(st, until - now).map(|(g, _)| g).unwrap_or_else(|p| p.into_inner().0);
        }
    }
}

impl Drop for AudioReceiver {
    fn drop(&mut self) {
        let mut st = self.shared.lock();
        st.receiver = false;
        st.chunks.clear();
        st.queued_secs = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk(rate: u32, n: usize, mark: u64) -> AudioChunk {
        AudioChunk { rate, samples: vec![0.0; n], received_unix_ms: mark }
    }

    #[test]
    fn under_the_limit_nothing_is_lost_and_the_order_is_kept() {
        let (tx, rx) = channel(30.0);
        // 11 s di decodifica a 48 kHz in blocchi da 10 ms: tutto deve arrivare
        for i in 0..1100 {
            assert!(tx.send(chunk(48_000, 480, i)).is_ok());
        }
        for i in 0..1100 {
            let got = rx.recv_timeout(Duration::from_millis(10)).map(|c| c.received_unix_ms);
            assert_eq!(got.ok(), Some(i));
        }
        assert!(matches!(rx.recv_timeout(Duration::from_millis(1)), Err(RecvTimeoutError::Timeout)));
    }

    #[test]
    fn a_stuck_engine_keeps_only_the_newest_seconds() {
        let (tx, rx) = channel(3.0);
        // motore fermo per 60 s: in coda restano gli ultimi 3 s, la memoria non cresce
        for i in 0..6000 {
            assert!(tx.send(chunk(48_000, 480, i)).is_ok());
            assert!(tx.queued_secs() <= 3.0 + 1e-9);
        }
        let first = rx.recv_timeout(Duration::from_millis(10)).map(|c| c.received_unix_ms).ok();
        assert_eq!(first, Some(6000 - 300));
        let mut n = 1;
        while rx.recv_timeout(Duration::from_millis(1)).is_ok() {
            n += 1;
        }
        assert_eq!(n, 300);
    }

    #[test]
    fn a_single_chunk_longer_than_the_limit_still_arrives() {
        let (tx, rx) = channel(0.5);
        assert!(tx.send(chunk(12_000, 12_000, 7)).is_ok());
        assert_eq!(rx.recv_timeout(Duration::from_millis(10)).map(|c| c.received_unix_ms).ok(), Some(7));
    }

    #[test]
    fn closing_either_side_is_seen_by_the_other() {
        let (tx, rx) = channel(30.0);
        let tx2 = tx.clone();
        drop(tx);
        assert!(tx2.send(chunk(48_000, 480, 1)).is_ok());
        drop(tx2);
        // i blocchi gia' in coda arrivano ancora, poi «scollegato»
        assert!(rx.recv_timeout(Duration::from_millis(10)).is_ok());
        assert!(matches!(rx.recv_timeout(Duration::from_millis(10)), Err(RecvTimeoutError::Disconnected)));

        let (tx, rx) = channel(30.0);
        drop(rx);
        assert!(tx.send(chunk(48_000, 480, 1)).is_err());
    }

    #[test]
    fn a_waiting_receiver_wakes_up_when_a_chunk_arrives() {
        let (tx, rx) = channel(30.0);
        let t = std::thread::spawn(move || rx.recv_timeout(Duration::from_secs(5)).map(|c| c.received_unix_ms).ok());
        std::thread::sleep(Duration::from_millis(20));
        assert!(tx.send(chunk(48_000, 480, 42)).is_ok());
        assert_eq!(t.join().ok().flatten(), Some(42));
    }
}
