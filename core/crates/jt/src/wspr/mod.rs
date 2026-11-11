//! WSPR-2 («Weak Signal Propagation Reporter»): radiofaro a 4-FSK, 162 simboli da 8192 campioni a 12 kHz (1,4648 baud, toni distanziati di
//! 1,4648 Hz, circa 110,6 s), periodi di due minuti con l'inizio a un secondo dall'inizio del minuto pari. Messaggio di 50 bit (nominativo,
//! locatore, potenza), codice convoluzionale K=32 r=1/2, interfogliatore a inversione di bit e vettore di sincronismo sul bit basso del tono.
//!
//! Tutto viene da WSJT-X 3.3.0-beta6 (GPL 3), `lib/wsprd/` (wsprd di K9AN e K1JT) e `lib/genwspr.f90`:
//!
//! - [`pack`]: messaggi di tipo 1, 2 e 3 e tabella degli hash (`wsprd_utils.c`, `wsprsim_utils.c`, `wqencode.f90`, `nhash.c`);
//! - [`fano`]: codice convoluzionale, interfogliatore e decodificatore di Fano (`fano.c`, `metric_tables.c`);
//! - [`osd`]: decodificatore a statistiche ordinate (`osdwspr.f90`);
//! - [`tx`]: toni e onda in trasmissione (`genwspr.f90`, `mainwindow.cpp:13586-13593`);
//! - [`rx`]: ricezione (`wsprd.c`: sottocampionamento, candidati, sincronismo, deriva, demodulazione a blocchi, sottrazione).

pub mod fano;
pub mod osd;
pub mod pack;
pub mod rx;
pub mod tx;

/// Simboli del messaggio.
pub const NSYM: usize = 162;
/// Campioni per simbolo a 12 kHz (`NSPS0` di `wspr_params.f90`).
pub const NSPS: usize = 8192;
/// Distanza dei toni e velocita' dei simboli: 12000 / 8192 Hz.
pub const TONE_SPACING_HZ: f64 = 12_000.0 / 8192.0;
/// Durata del periodo in secondi.
pub const PERIOD_S: u32 = 120;
/// Inizio del messaggio dall'inizio del periodo, in secondi.
pub const START_S: f64 = 1.0;
/// Frequenza audio predefinita del centro del segnale (`WSPRfreq`, `mainwindow_settings.cpp:524`); WSJT-X la limita a 1400-1600 Hz.
pub const DEFAULT_AUDIO_HZ: f64 = 1500.0;

/// Il vettore di sincronismo (`pr3` di `wsprd.c:56-65`, `npr3` di `genwspr.f90`): il bit basso di ogni tono.
pub const SYNC: [u8; NSYM] = [
    1, 1, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 1, 1, 1, 0, 0, 0, 1, 0, 0, 1, 0, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 1, 0, 1, 0, 0, 0, 0, 0, 0,
    1, 0, 1, 1, 0, 0, 1, 1, 0, 1, 0, 0, 0, 1, 1, 0, 1, 0, 0, 0, 0, 1, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 0, 1, 0, 0, 1, 0, 1, 1, 0, 0, 0, 1, 1, 0,
    1, 0, 1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 1, 0, 0, 1, 1, 1, 0, 1, 1, 0, 0, 1, 1, 0, 1, 0, 0, 0, 1, 1, 1, 0, 0, 0, 0, 0, 1, 0, 1, 0, 0,
    1, 1, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 1, 0, 1, 1, 0, 0, 0, 1, 1, 0, 0, 0,
];
