//! Registrazione dell'audio di ogni cartina in un file WAV (PCM 16 bit mono a 12000 Hz, la frequenza interna del ricevitore) nella
//! sottocartella `audio` della cartella delle cartine. Il ricevitore non aspetta mai: i campioni vanno a un thread di scrittura con una coda
//! limitata, e gli errori del disco fermano solo la registrazione in corso (segnalati una volta nel log).
//!
//! Il file si scrive con un nome provvisorio (`wefax-<data>.wav.part`) e alla chiusura della cartina prende il nome definitivo
//! `wefax-<data e ora della cartina>.wav`; se la cartina viene scartata il file si cancella.

use std::{
    collections::VecDeque,
    fs::File,
    io::{BufWriter, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::mpsc::{Receiver, SyncSender, TrySendError, sync_channel},
    time::{Duration, SystemTime},
};

use crate::RATE;

/// Secondi di audio gia' ricevuto tenuti in memoria e messi in testa al file: il tono d'inizio si riconosce dopo qualche secondo.
pub const PREROLL_SECS: usize = 10;
/// Durata massima di una registrazione (poi si smette di scrivere, il file resta): tre ore.
pub const MAX_SECS: u64 = 3 * 3600;
/// Byte al secondo del file (16 bit mono a 12000 Hz).
pub const BYTES_PER_SEC: u32 = RATE * 2;
/// Blocchi in coda verso il thread di scrittura (blocchi di qualche decimo di secondo: decine di secondi di margine).
const QUEUE: usize = 1024;

/// Nome della sottocartella dell'audio.
pub const AUDIO_DIR: &str = "audio";

/// Il nome del WAV di una cartina salvata: `wefax_20261121-101500_ioc576` diventa `wefax-20261121-101500_ioc576.wav`.
pub fn audio_name_for(id: &str) -> String {
    format!("wefax-{}.wav", id.strip_prefix("wefax_").unwrap_or(id))
}

/// Un file della cartella dell'audio che la pulizia puo' toccare: `wefax-*.wav` (e i provvisori `wefax-*.wav.part` rimasti da un arresto).
fn is_ours(name: &str) -> bool {
    name.starts_with("wefax-") && (name.ends_with(".wav") || name.ends_with(".wav.part"))
}

/// Cancella dalla cartella `dir` i file `wefax-*.wav` piu' vecchi di `days` giorni rispetto a `now` (0: nessuno). Solo i file di quella
/// cartella, niente sottocartelle ne' altri nomi. Restituisce quanti ne ha cancellati.
pub fn prune(dir: &Path, days: u32, now: SystemTime) -> usize {
    if days == 0 {
        return 0;
    }
    let Ok(list) = std::fs::read_dir(dir) else { return 0 };
    let limit = Duration::from_secs(u64::from(days) * 86_400);
    let mut gone = 0;
    for item in list.flatten() {
        let name = item.file_name();
        let Some(name) = name.to_str() else { continue };
        if !is_ours(name) {
            continue;
        }
        let Ok(meta) = item.metadata() else { continue };
        if !meta.is_file() {
            continue;
        }
        let old = meta.modified().ok().and_then(|m| now.duration_since(m).ok()).is_some_and(|age| age > limit);
        if old && std::fs::remove_file(item.path()).is_ok() {
            gone += 1;
        }
    }
    gone
}

/// Intestazione WAV di 44 byte per `data_bytes` byte di campioni.
fn header(data_bytes: u32) -> [u8; 44] {
    let mut h = [0u8; 44];
    h[0..4].copy_from_slice(b"RIFF");
    h[4..8].copy_from_slice(&data_bytes.saturating_add(36).to_le_bytes());
    h[8..16].copy_from_slice(b"WAVEfmt ");
    h[16..20].copy_from_slice(&16u32.to_le_bytes());
    h[20..22].copy_from_slice(&1u16.to_le_bytes());
    h[22..24].copy_from_slice(&1u16.to_le_bytes());
    h[24..28].copy_from_slice(&RATE.to_le_bytes());
    h[28..32].copy_from_slice(&BYTES_PER_SEC.to_le_bytes());
    h[32..34].copy_from_slice(&2u16.to_le_bytes());
    h[34..36].copy_from_slice(&16u16.to_le_bytes());
    h[36..40].copy_from_slice(b"data");
    h[40..44].copy_from_slice(&data_bytes.to_le_bytes());
    h
}

fn to_i16(v: f32) -> i16 {
    if v.is_finite() { (v.clamp(-1.0, 1.0) * 32767.0).round() as i16 } else { 0 }
}

/// Chi aspetta l'esito della chiusura: `Some(nome)` se il file e' stato scritto, `None` altrimenti.
pub type OnDone = Box<dyn FnOnce(Option<String>) + Send>;

enum Cmd {
    /// Comincia un file nuovo in `dir` con il pre-roll; prima pulisce i file vecchi.
    Begin { dir: PathBuf, stamp: String, keep_days: u32, preroll: Vec<i16> },
    Data(Vec<i16>),
    /// Chiude il file e gli da' il nome definitivo.
    End { name: String, done: OnDone },
    /// Cancella il file in corso.
    Abort,
    /// Pulizia dei file vecchi (all'avvio).
    Prune { dir: PathBuf, keep_days: u32 },
}

/// Il lato del ricevitore: tiene il pre-roll e manda i campioni al thread di scrittura senza mai aspettare.
pub struct Recorder {
    tx: Option<SyncSender<Cmd>>,
    preroll: VecDeque<i16>,
    active: bool,
    /// Un blocco perso con la coda piena gia' segnalato (per questa registrazione).
    lost_warned: bool,
}

impl Recorder {
    /// Accende il thread di scrittura.
    pub fn new() -> Recorder {
        let (tx, rx) = sync_channel::<Cmd>(QUEUE);
        let tx = match std::thread::Builder::new().name("aurora-wefax-wav".into()).spawn(move || writer(rx)) {
            Ok(_) => Some(tx),
            Err(e) => {
                tracing::error!("WEFAX: thread di registrazione non avviato: {e}");
                None
            }
        };
        Recorder { tx, preroll: VecDeque::with_capacity(PREROLL_SECS * RATE as usize), active: false, lost_warned: false }
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    fn send(&mut self, cmd: Cmd) -> bool {
        let Some(tx) = &self.tx else { return false };
        match tx.try_send(cmd) {
            Ok(()) => true,
            Err(TrySendError::Full(_)) => {
                if !self.lost_warned {
                    self.lost_warned = true;
                    tracing::warn!("WEFAX: il disco non tiene il passo, parte dell'audio della cartina non e' registrata");
                }
                false
            }
            Err(TrySendError::Disconnected(_)) => false,
        }
    }

    /// Audio a 12 kHz appena ricevuto: va nel file se si registra, altrimenti nel pre-roll.
    pub fn push(&mut self, audio: &[f32]) {
        if self.active {
            let block: Vec<i16> = audio.iter().map(|&v| to_i16(v)).collect();
            self.send(Cmd::Data(block));
            return;
        }
        let cap = PREROLL_SECS * RATE as usize;
        self.preroll.extend(audio.iter().map(|&v| to_i16(v)));
        let extra = self.preroll.len().saturating_sub(cap);
        self.preroll.drain(..extra);
    }

    /// Comincia una registrazione in `dir` (con il pre-roll in testa).
    pub fn begin(&mut self, dir: &Path, stamp: &str, keep_days: u32) {
        if self.active {
            return;
        }
        self.lost_warned = false;
        let preroll: Vec<i16> = self.preroll.drain(..).collect();
        self.active = self.send(Cmd::Begin { dir: dir.to_path_buf(), stamp: stamp.to_string(), keep_days, preroll });
    }

    /// Chiude la registrazione con il nome `name`; `done` riceve l'esito dal thread di scrittura.
    pub fn end(&mut self, name: String, done: OnDone) {
        if !self.active {
            done(None);
            return;
        }
        self.active = false;
        if let Some(tx) = &self.tx {
            // la chiusura non si perde: se la coda e' piena si consegna l'esito negativo
            if let Err(e) = tx.try_send(Cmd::End { name, done }) {
                tracing::warn!("WEFAX: registrazione non chiusa (coda piena)");
                let (TrySendError::Full(Cmd::End { done, .. }) | TrySendError::Disconnected(Cmd::End { done, .. })) = e else { return };
                done(None);
            }
        } else {
            done(None);
        }
    }

    /// Butta la registrazione in corso (cartina scartata, falso allarme, ascolto spento).
    pub fn abort(&mut self) {
        if self.active {
            self.active = false;
            self.send(Cmd::Abort);
        }
    }

    /// Pulizia dei file vecchi nel thread di scrittura.
    pub fn prune_later(&mut self, dir: &Path, keep_days: u32) {
        self.send(Cmd::Prune { dir: dir.to_path_buf(), keep_days });
    }
}

impl Default for Recorder {
    fn default() -> Self {
        Recorder::new()
    }
}

struct Open {
    file: BufWriter<File>,
    part: PathBuf,
    dir: PathBuf,
    bytes: u64,
}

fn writer(rx: Receiver<Cmd>) {
    let mut open: Option<Open> = None;
    // un errore del disco si segnala una volta finche' una registrazione non riesce di nuovo
    let mut failed = false;
    let fail = |what: &str, e: &dyn std::fmt::Display, failed: &mut bool| {
        if !*failed {
            *failed = true;
            tracing::warn!("WEFAX: registrazione dell'audio fermata ({what}): {e}; la ricezione continua");
        }
    };
    let drop_open = |open: &mut Option<Open>| {
        if let Some(o) = open.take() {
            drop(o.file);
            let _ = std::fs::remove_file(&o.part);
        }
    };
    while let Ok(cmd) = rx.recv() {
        match cmd {
            Cmd::Begin { dir, stamp, keep_days, preroll } => {
                drop_open(&mut open);
                prune(&dir, keep_days, SystemTime::now());
                if let Err(e) = std::fs::create_dir_all(&dir) {
                    fail("cartella", &e, &mut failed);
                    continue;
                }
                let part = dir.join(format!("wefax-{stamp}.wav.part"));
                let started = File::create(&part).and_then(|f| {
                    let mut w = BufWriter::with_capacity(256 * 1024, f);
                    w.write_all(&header(0))?;
                    write_samples(&mut w, &preroll)?;
                    Ok(w)
                });
                match started {
                    Ok(file) => open = Some(Open { file, part, dir, bytes: preroll.len() as u64 * 2 }),
                    Err(e) => {
                        fail("file", &e, &mut failed);
                        let _ = std::fs::remove_file(&part);
                    }
                }
            }
            Cmd::Data(block) => {
                let Some(o) = open.as_mut() else { continue };
                if o.bytes >= MAX_SECS * u64::from(BYTES_PER_SEC) {
                    continue;
                }
                match write_samples(&mut o.file, &block) {
                    Ok(()) => o.bytes += block.len() as u64 * 2,
                    Err(e) => {
                        fail("scrittura", &e, &mut failed);
                        drop_open(&mut open);
                    }
                }
            }
            Cmd::End { name, done } => {
                let Some(o) = open.take() else {
                    done(None);
                    continue;
                };
                match close(o, &name) {
                    Ok(()) => {
                        failed = false;
                        done(Some(name));
                    }
                    Err(e) => {
                        fail("chiusura", &e, &mut failed);
                        done(None);
                    }
                }
            }
            Cmd::Abort => drop_open(&mut open),
            Cmd::Prune { dir, keep_days } => {
                let n = prune(&dir, keep_days, SystemTime::now());
                if n > 0 {
                    tracing::info!("WEFAX: {n} registrazioni piu' vecchie di {keep_days} giorni cancellate");
                }
            }
        }
    }
    drop_open(&mut open);
}

fn write_samples(w: &mut impl Write, samples: &[i16]) -> std::io::Result<()> {
    let mut buf = Vec::with_capacity(samples.len() * 2);
    for s in samples {
        buf.extend_from_slice(&s.to_le_bytes());
    }
    w.write_all(&buf)
}

/// Scrive le misure vere nell'intestazione e da' al file il nome definitivo (con `_2`, `_3`... se esiste gia').
fn close(o: Open, name: &str) -> std::io::Result<()> {
    let Open { file, part, dir, bytes } = o;
    let result = (|| {
        let mut f = file.into_inner().map_err(|e| e.into_error())?;
        let data = u32::try_from(bytes).unwrap_or(u32::MAX - 36);
        f.seek(SeekFrom::Start(0))?;
        f.write_all(&header(data))?;
        f.sync_all()?;
        drop(f);
        let mut target = dir.join(name);
        let mut n = 2;
        while target.exists() {
            let stem = name.strip_suffix(".wav").unwrap_or(name);
            target = dir.join(format!("{stem}_{n}.wav"));
            n += 1;
        }
        std::fs::rename(&part, &target)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&part);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("aurora-wefax-rec-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("cartella di prova");
        dir
    }

    fn file_at(path: &Path, age_days: u64) {
        std::fs::write(path, b"x").expect("file");
        let when = SystemTime::now() - Duration::from_secs(age_days * 86_400 + 60);
        File::options().write(true).open(path).and_then(|f| f.set_modified(when)).expect("data");
    }

    #[test]
    fn la_pulizia_cancella_solo_i_vecchi_wefax_wav_della_cartella_audio() {
        let base = temp("pulizia");
        let audio = base.join(AUDIO_DIR);
        std::fs::create_dir_all(audio.join("sotto")).expect("cartelle");
        file_at(&audio.join("wefax-a.wav"), 10);
        file_at(&audio.join("wefax-p.wav.part"), 10);
        file_at(&audio.join("wefax-b.wav"), 2);
        file_at(&audio.join("altro.wav"), 30);
        file_at(&audio.join("wefax-c.png"), 30);
        file_at(&audio.join("sotto").join("wefax-e.wav"), 30);
        file_at(&base.join("wefax-d.wav"), 30);
        assert_eq!(prune(&audio, 0, SystemTime::now()), 0, "0 giorni: non si cancella niente");
        assert!(audio.join("wefax-a.wav").exists());
        assert_eq!(prune(&audio, 7, SystemTime::now()), 2);
        assert!(!audio.join("wefax-a.wav").exists());
        assert!(!audio.join("wefax-p.wav.part").exists());
        for keep in [audio.join("wefax-b.wav"), audio.join("altro.wav"), audio.join("wefax-c.png"), audio.join("sotto").join("wefax-e.wav"), base.join("wefax-d.wav")] {
            assert!(keep.exists(), "{} doveva restare", keep.display());
        }
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn il_nome_segue_quello_della_cartina() {
        assert_eq!(audio_name_for("wefax_20261121-101500_ioc576"), "wefax-20261121-101500_ioc576.wav");
        assert_eq!(audio_name_for("wefax_20261121-101500_ioc576_2"), "wefax-20261121-101500_ioc576_2.wav");
    }

    #[test]
    fn registrazione_chiusa_e_buttata() {
        let dir = temp("chiusa");
        let mut rec = Recorder::new();
        rec.push(&vec![0.25; 1200]);
        rec.begin(&dir, "uno", 7);
        rec.push(&vec![-0.5; 2400]);
        let (tx, rx) = std::sync::mpsc::channel();
        rec.end("wefax-uno.wav".into(), Box::new(move |n| {
            let _ = tx.send(n);
        }));
        assert_eq!(rx.recv_timeout(Duration::from_secs(5)).expect("esito"), Some("wefax-uno.wav".into()));
        let bytes = std::fs::read(dir.join("wefax-uno.wav")).expect("wav");
        let wav = crate::wav::parse(&bytes).expect("wav valido");
        assert_eq!((wav.rate, wav.channels, wav.frames()), (RATE, 1, 3600));
        // buttata: il file provvisorio sparisce
        rec.begin(&dir, "due", 7);
        rec.push(&vec![0.1; 1200]);
        rec.abort();
        let (tx, rx) = std::sync::mpsc::channel();
        rec.begin(&dir, "tre", 7);
        rec.end("wefax-tre.wav".into(), Box::new(move |n| {
            let _ = tx.send(n);
        }));
        rx.recv_timeout(Duration::from_secs(5)).expect("esito");
        assert!(!dir.join("wefax-due.wav.part").exists());
        assert!(!dir.join("wefax-due.wav").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
