//! La decodifica di un periodo intero (`ft8_decode.f90` e la parte di controllo di `ft8b.f90`, JTDX 2.2.159): passate con ricerca dei
//! candidati diversa a rotazione, sottrazione di ogni segnale appena decodificato, analisi a priori, filtri sui falsi.
//!
//! # Come lavora una passata
//!
//! 1. [`sync::find_candidates_ex`] cerca i candidati sull'audio di quel momento, con la forma dello spettro della passata (ampiezza,
//!    potenza, somma degli assoluti) e la soglia `syncmin` della passata (1,5; con [`Options::low_threshold`] 1,225 / 1,5 / 1,1 a rotazione).
//! 2. Per ogni candidato ([`try_candidate`]): discesa a 200 Hz, allineamento in ritardo e frequenza, controlli di sincronismo
//!    ([`filters::sync_screen`]), le serie di LLR (a, b, c, d), e per ognuna BP e poi OSD; poi i tipi a priori ([`ap`]).
//! 3. Una parola che passa il CRC deve avere `nharderrors + dmin < 60`, un testo che si legge e che non sembra un falso
//!    ([`filters::reject_message`]).
//! 4. Il segnale decodificato si toglie **subito** dall'audio ([`subtract`]), con il ritardo affinato: i candidati dopo vedono gia' il
//!    residuo. Se un candidato e' entro 50 Hz da un segnale appena tolto la FFT lunga si rifa, come `ft8_downsample` di JTDX.
//!
//! Il calendario delle passate e' quello di JTDX: 3, 6 o 9 secondo la profondita' ([`Options::depth`]); la quarta passata legge l'audio
//! a mezzo campione, la settima a mezzo campione dall'altra parte, la 5 e la 7 leggono i simboli a tempo invertito, dalla sesta non si
//! sottrae piu' (ne' nella terza di un ciclo da tre).
//!
//! # Differenze rispetto a JTDX
//!
//! - Un solo thread (JTDX divide la banda in fasce, una per thread); la cronologia tra periodi ([`history::PeriodHistory`],
//!   `evencq`/`oddcq`/`evenmyc`/`oddmyc`/`evenqso`/`oddqso` di JTDX) quindi e' una lista sola per parita', non una per thread.
//! - Le sottopassate 3-11 (CQ, chiamata diretta a noi, segnale del canale QSO, combinate con la lettura a tempo invertito dello
//!   stesso periodo o con il periodo precedente della stessa parita') sono portate in [`try_candidate`]; restano fuori i casi piu'
//!   intrecciati del gate per tipo a priori quando piu' motivi si sovrappongono (vedi i commenti li') e `evencopy`/`oddcopy`
//!   (dalla 0.11.132 c'e' la ridecodifica `evencopy`/`oddcopy` con `ft8sd1`/`ft8mf1`/`ft8mfcq` e `ldeepsync`: [`crate::copies`]), il decoder `ft8s` del segnale di QSO e i candidati virtuali sulla
//!   frequenza di ricezione.
//! - Nessun controllo `chkgrid` (geografico) ne' `ALLCALL7.TXT`: vedi [`filters`].

use std::sync::{Arc, OnceLock};

use rustfft::num_complex::Complex32;
use rustfft::{Fft, FftPlanner};

use rayon::prelude::*;

use crate::a8::A8Query;
use crate::ap::{self, ApContext, ApOptions, ApTable};
use crate::filters::{self, CallBook, GridCheck, MessageCheck};
pub use crate::history::PeriodHistory;
use crate::history::StoredSignal;
use crate::message::{self, CallHashes};
use crate::subtract::{self, Reference};
use crate::sync::{self, Candidate, Complex, Downsampler, MetricsMode, PassCandidate, Spectrum, SyncKind};
use crate::{a7, a8, ldpc, modem, NN, SLOT_SAMPLES};

/// Un messaggio decodificato.
#[derive(Debug, Clone, PartialEq)]
pub struct Decoded {
    /// Frequenza del tono 0 (Hz): quella che JTDX scrive nella colonna delle frequenze.
    pub freq_hz: f32,
    /// Scarto di tempo come lo scrive JTDX (secondi).
    pub dt_s: f32,
    /// Rapporto segnale/rumore in 2500 Hz (dB, intero come nell'elenco dei messaggi).
    pub snr_db: i32,
    pub text: String,
    /// Trovato con un'analisi a priori (`ap_type` diverso da 0): il marcatore `*` di JTDX.
    pub hint: bool,
}

/// Un messaggio decodificato con le grandezze con cui e' stato accettato (per le misure e per tarare i filtri).
#[derive(Debug, Clone, PartialEq)]
pub struct DecodedInfo {
    pub decoded: Decoded,
    /// Passata (da 1) in cui e' uscito.
    pub pass: usize,
    /// Tipo a priori con cui e' stato trovato (0 = nessuno): vedi [`ap`].
    pub ap_type: u8,
    /// `nharderrors` e `dmin` della parola accettata.
    pub hard_errors: u32,
    pub dmin: f32,
    /// Sincronismo del candidato (`sync8`).
    pub sync: f32,
}

/// Come si toglie un segnale decodificato dall'audio.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Subtraction {
    /// `subtractft8` di JTDX: filtro passa-basso (ampiezza e fase variabili nel tempo) e ritardo affinato.
    Adaptive,
    /// Una sola ampiezza e fase per tutto il messaggio, senza affinare il ritardo (come faceva Aurora prima): solo per confronto.
    Constant,
}

#[derive(Debug, Clone)]
pub struct Options {
    pub f_low_hz: f32,
    pub f_high_hz: f32,
    /// Soglia di sincronismo dei candidati (1,5 normale; con `low_threshold` si usa invece la rotazione di JTDX).
    pub min_sync: f32,
    /// Numero massimo di candidati per passata. WSJT-X ne tiene fino a 450 (460 con quelli sulla frequenza di ricezione) per ognuna
    /// delle fette di banda dei suoi thread (`sync8var.f90:296` e `:375`); qui la banda e' una sola e il tetto resta 300, perche' il
    /// limite di tempo ([`Options::deadline`]) si controlla solo tra una passata e l'altra e una passata con 450 candidati su una banda
    /// affollata dura una volta e mezza di piu'.
    pub max_candidates: usize,
    /// Passate (JTDX: 3, 6 o 9, secondo "i cicli": vedi [`Options::depth`]).
    pub passes: usize,
    /// Iterazioni della propagazione di credenze.
    pub bp_iterations: usize,
    /// Ordine dell'OSD (0 = nessuno; JTDX usa `ndeep = 3`). E' la profondita' di base: i segnali del QSO vicino alla frequenza di
    /// ricezione o di trasmissione salgono a 4 e la ridecodifica ([`Options::again`]) a 5 (vedi `osd_depth`).
    pub osd_order: usize,
    /// `lft8lowth`: `syncmin` a rotazione 1,225 / 1,5 / 1,1 sulle passate (1, 4, 7), (2, 5, 8) e (3, 6, 9).
    pub low_threshold: bool,
    /// `lft8subpass`: seconda lettura dei simboli (a tempo invertito) per ogni candidato.
    pub subpass: bool,
    /// Modo SWL di JTDX (`nswl`): finestra di ricerca piu' ampia (+-86 passi), soglie di sincronismo come con la soglia bassa, la terza passata
    /// di un ciclo da 3 sottrae e la lettura a tempo invertito si fa sempre; la quarta lettura (`llrd`) e la prima lettura con `llra` anche alla prima passata,
    /// e non valgono i controlli sui rapporti dei toni dei tipi a priori 1 e 2 (`ft8b`, sorgenti di JTDX letti il 2026-10-26: `swl` in `ft8b.f90`).
    pub swl: bool,
    /// Come si tolgono i segnali decodificati.
    pub subtraction: Subtraction,
    /// Sottrarre subito dopo ogni decodifica (come JTDX) invece che a fine passata.
    pub immediate_subtraction: bool,
    /// Filtri sui falsi di JTDX (sincronismo, `nharderrors + dmin`, testo): vedi [`filters`]. Spenti, valgono solo CRC, lettura del
    /// testo e `nsync >= 7`.
    pub filters: bool,
    /// Ritardo medio delle stazioni sentite (`avexdt`, secondi): la finestra di ricerca del ritardo si sposta di `avexdt * 25` passi.
    /// Si aggiorna da un periodo all'altro con [`DtCenter`].
    pub dt_center_s: f32,
    /// Frequenza di ricezione (`nfqso`, Hz): entro +-3 Hz la soglia dei candidati e' 1,1 e quei candidati si provano per primi
    /// (`sync8var.f90:290-293` e `:344-377` di WSJT-X 3.3.0-beta6). `None`: nessuna.
    pub rx_freq_hz: Option<f32>,
    /// Ridecodifica filtrata (`nagainfil` di WSJT-X; la sua interfaccia non la attiva mai, e il pulsante «Decodifica» di Aurora neppure): solo +-25 Hz attorno a [`Options::rx_freq_hz`] e OSD di
    /// profondita' 5 (`ft8_engine_kernel.f90:350-362`, `ft8bvar.f90:2165`). Senza frequenza di ricezione, o con la frequenza fuori
    /// banda, non si decodifica niente (`nfqso is out of bandwidth`).
    pub again: bool,
    /// Analisi a priori per il proprio nominativo e il QSO (`None`: solo il tipo CQ, se acceso).
    pub ap: Option<ApOptions>,
    /// Tipo a priori 1, `CQ ??? ???`: non richiede di sapere niente.
    pub ap_cq: bool,
    /// Elenco completo di nominativi (come `ALLCALL7.TXT`) per `chkflscall`; con un elenco incompleto si scartano stazioni vere.
    pub call_book: Option<Arc<CallBook>>,
    /// Controllo del locatore sul paese del nominativo (vedi [`filters::grid_is_false`]); solo sui messaggi deboli o a priori.
    pub grid_check: Option<GridCheck>,
    /// Parita' di questo periodo (FT8 alterna periodi che cominciano a un multiplo pari o dispari di 15 s dall'inizio dei
    /// tempi): sceglie quale meta' della [`history::PeriodHistory`] passata a [`decode_slot_with_history`] leggere e
    /// scrivere. Senza cronologia non conta.
    pub even_period: bool,
    /// Compensazione AGC (pulsante AGCc, `nagcc` di JTDX): [`sync::agc_compensate`] sull'audio e sincronismo per tono nella ricerca.
    pub agc: bool,
    /// Decodifica "a7" ([`a7`]): i messaggi attesi dopo quelli del periodo precedente della stessa parita'. Serve la
    /// cronologia ([`decode_slot_with_history`]). In WSJT-X e' accesa con l'analisi a priori (`lft8apon`).
    pub a7: bool,
    /// Decodifica "a8" ([`a8`]) del corrispondente alla frequenza di ricezione (`None`: spenta).
    pub a8: Option<A8Query>,
    /// Identifica il periodo per la tabella "a7" (per esempio i quarti di minuto dall'inizio dei tempi): decodificare di
    /// nuovo lo stesso periodo non sposta la tabella. `None`: ogni chiamata e' un periodo nuovo.
    pub period_key: Option<u64>,
    /// Thread per la decodifica (1 = un solo thread). I decode sono gli stessi con qualunque numero di thread.
    pub threads: usize,
    /// Oltre questo momento non si comincia un'altra passata: su un PC lento il periodo finisce in tempo con i messaggi gia' trovati,
    /// invece di accumulare ritardo un periodo dopo l'altro. `None`: tutte le passate.
    pub deadline: Option<std::time::Instant>,
}

impl Default for Options {
    fn default() -> Self {
        Options::depth(2)
    }
}

impl Options {
    /// Le profondita' di JTDX: 1 = veloce (3 passate), 2 = normale (6 passate), 3 = profonda (9 passate, soglia bassa e seconda lettura).
    /// La 1 non prova nessun tipo a priori e usa OSD di ordine 2; dalla 2 in su OSD e' d'ordine 3 come `ndeep = 3` di JTDX.
    pub fn depth(depth: u8) -> Options {
        let depth = depth.clamp(1, 3);
        Options {
            f_low_hz: 200.0,
            f_high_hz: 4000.0,
            min_sync: 1.3,
            max_candidates: 300,
            passes: 3 * usize::from(depth),
            bp_iterations: 30,
            osd_order: if depth == 1 { 2 } else { 3 },
            low_threshold: depth == 3,
            subpass: depth == 3,
            swl: false,
            subtraction: Subtraction::Adaptive,
            immediate_subtraction: true,
            filters: true,
            dt_center_s: 0.0,
            rx_freq_hz: None,
            again: false,
            ap: None,
            ap_cq: depth > 1,
            call_book: None,
            grid_check: None,
            even_period: true,
            agc: false,
            a7: depth > 1,
            a8: None,
            period_key: None,
            threads: 1,
            deadline: None,
        }
    }
}

/// Mette nella tabella degli hash i nominativi dati (per esempio quelli dei `*_ALL.TXT` di JTDX o dei decodificati degli ultimi giorni),
/// cosi' un messaggio che porta solo l'impronta di un nominativo si legge subito invece di `<...>` (`fillhash`/`save_hash_call`).
/// Ritorna quanti ne sono stati presi.
pub fn seed_call_hashes<'a, I: IntoIterator<Item = &'a str>>(hashes: &mut CallHashes, calls: I) -> usize {
    let before = hashes.len();
    for call in calls {
        hashes.save_call(call);
    }
    hashes.len().saturating_sub(before)
}

/// Il ritardo medio delle stazioni sentite (`avexdt` di WSJT-X 3.3.0-beta6), da un periodo FT8 all'altro: centra la finestra in cui si
/// cerca il ritardo dei candidati ([`Options::dt_center_s`], `ft8_decodevar.f90:172-173`), cosi' con l'orologio del PC spostato
/// i segnali non finiscono al bordo della finestra.
///
/// Dopo ogni decodifica si chiama [`DtCenter::update`] con i ritardi dei messaggi nell'ordine in cui sono usciti
/// (`ft8_engine_kernel.f90:444-478`): con 0 messaggi torna a 0; con 1-5 si sposta verso la media del periodo con un peso che cresce
/// col numero dei messaggi; con piu' di 5 va a meta' strada tra il valore di prima e la media. La media e' quella delle mediane di
/// tre messaggi consecutivi (`ft8_timing_summary`, `decoder_callbacks.f90:381-419`), cosi' un ritardo strano non la tira.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct DtCenter {
    avexdt: f32,
    /// `nintcount`: dopo un cambio di modo, alla seconda decodifica con piu' di 10 messaggi si prende subito la media del periodo.
    nintcount: u8,
}

/// I ritardi che contano per `avexdt` (`xdtt(200)` di `decoder_callbacks.f90:63`).
const AVEXDT_MAX_DECODES: usize = 200;

impl DtCenter {
    /// Il ritardo medio in secondi, da mettere in [`Options::dt_center_s`].
    pub fn value(&self) -> f32 {
        self.avexdt
    }

    /// Cambio di modo: si riparte da 0 e la media veloce torna possibile (`lmodechanged`, `ft8_engine_kernel.f90:254-257`).
    pub fn mode_changed(&mut self) {
        self.avexdt = 0.0;
        self.nintcount = 3;
    }

    /// Aggiorna il ritardo medio con i ritardi (`dt_s`) dei messaggi di un periodo, nell'ordine in cui sono usciti.
    pub fn update(&mut self, dts: &[f32]) {
        let (n, sumxdt) = timing_summary(dts);
        // `ft8_engine_kernel.f90:447-472` (`forcedt` vale sempre 0)
        self.nintcount = self.nintcount.saturating_sub(1);
        let a = self.avexdt;
        let mean = if n > 0 { sumxdt / n as f32 } else { 0.0 };
        self.avexdt = match n {
            0 => 0.0,
            1 => (1.75 * a + 0.25 * sumxdt) / 2.0,
            2 => (1.5 * a + 0.5 * mean) / 2.0,
            3 => (1.35 * a + 0.65 * mean) / 2.0,
            4 => (1.25 * a + 0.75 * mean) / 2.0,
            5 => (1.1 * a + 0.9 * mean) / 2.0,
            _ => (a + mean) / 2.0,
        };
        // `ft8_engine_kernel.f90:474`: media veloce dopo un cambio di modo su una banda affollata
        if n > 10 && self.nintcount == 1 {
            self.avexdt = mean;
        }
    }
}

/// `ft8_timing_summary` (`decoder_callbacks.f90:381-419`): quanti ritardi contano (al piu' 200) e la somma delle mediane di tre
/// ritardi consecutivi; gli ultimi due ripetono l'ultima mediana. Con 1 o 2 ritardi la somma e' quella semplice.
fn timing_summary(dts: &[f32]) -> (usize, f32) {
    let x = &dts[..dts.len().min(AVEXDT_MAX_DECODES)];
    let count = x.len();
    if count <= 2 {
        return (count, x.iter().sum());
    }
    let mut sumdt = 0.0f32;
    let mut dtmed = 0.0f32;
    for i in 0..count {
        // `if(i.lt.count-1)` con gli indici da 1: c'e' ancora una terna completa
        if i + 2 < count {
            let (a, b, c) = (x[i], x[i + 1], x[i + 2]);
            dtmed = if (a > b && a < c) || (a < b && a > c) {
                a
            } else if (b > a && b < c) || (b < a && b > c) {
                b
            } else if (c > a && c < b) || (c < a && c > b) {
                c
            } else {
                a
            };
        }
        sumdt += dtmed;
    }
    (count, sumdt)
}

/// L'audio di JTDX e' a 16 bit (±32768): i campioni normalizzati a ±1 si riportano a quella scala, su cui sono tarate le soglie.
const SCALE: f32 = 32768.0;
/// Due decodifiche a meno di questa distanza in frequenza e tempo sono lo stesso segnale (per saltare i candidati su un segnale tolto).
const SAME_FREQ_HZ: f32 = 3.0;
const SAME_DT_S: f32 = 0.1;
/// Un messaggio uguale a meno di questa distanza in frequenza e' lo stesso messaggio (`allmessages` di `decode`).
const DUPE_FREQ_HZ: f32 = 45.0;
/// Entro questa distanza da un segnale appena tolto la FFT lunga va rifatta.
const REFFT_HZ: f32 = 50.0;

/// Decodifica i 15 secondi di audio di un periodo (12 kHz, campioni normalizzati a ±1).
pub fn decode_slot(samples: &[f32], options: &Options, hashes: &mut CallHashes) -> Vec<Decoded> {
    decode_slot_detailed(samples, options, hashes, None).into_iter().map(|d| d.decoded).collect()
}

/// Come [`decode_slot`], con la cronologia tra periodi delle sottopassate 3-11 (`history::PeriodHistory`): i segnali CQ,
/// le chiamate dirette a noi e il segnale del canale QSO visti in questo periodo vengono sommati, se si ripetono, con
/// quelli del periodo precedente della stessa parita' ([`Options::even_period`]).
pub fn decode_slot_with_history(samples: &[f32], options: &Options, hashes: &mut CallHashes, history: &mut PeriodHistory) -> Vec<Decoded> {
    decode_slot_detailed(samples, options, hashes, Some(history)).into_iter().map(|d| d.decoded).collect()
}

/// Un segnale decodificato in attesa di essere sottratto a fine passata.
struct Pending {
    tones: [u8; NN],
    hit_freq: f32,
    hit_dt: f32,
    cd: Vec<Complex>,
    i0: i64,
}

/// Come [`decode_slot`], con le grandezze di accettazione di ogni messaggio e, se data, la cronologia tra periodi
/// (`None` = nessuna sottopassata 3-11, come prima).
pub fn decode_slot_detailed(samples: &[f32], options: &Options, hashes: &mut CallHashes, history: Option<&mut PeriodHistory>) -> Vec<DecodedInfo> {
    decode_slot_streaming(samples, options, hashes, history, &mut |_| {})
}

/// Come [`decode_slot_with_history`], ma a fine di ogni passata chiama `on_pass` con i messaggi nuovi di quella passata:
/// chi mostra l'elenco non aspetta tutte le passate (le decodifiche a liste "a7"/"a8" arrivano solo nel risultato finale).
pub fn decode_slot_streaming(
    samples: &[f32],
    options: &Options,
    hashes: &mut CallHashes,
    history: Option<&mut PeriodHistory>,
    on_pass: &mut dyn FnMut(&[DecodedInfo]),
) -> Vec<DecodedInfo> {
    // ridecodifica a mano: banda di 50 Hz attorno alla frequenza di ricezione, niente se questa e' fuori dalla banda
    // (`nfa=max(nfa,nfqso-25)`, `nfb=min(nfb,nfqso+25)`, `ft8_engine_kernel.f90:350-362` di WSJT-X 3.3.0-beta6)
    let narrowed;
    let options = if options.again {
        let Some(rx) = options.rx_freq_hz.filter(|rx| *rx >= options.f_low_hz && *rx <= options.f_high_hz) else {
            return Vec::new();
        };
        narrowed = Options { f_low_hz: options.f_low_hz.max(rx - 25.0), f_high_hz: options.f_high_hz.min(rx + 25.0), ..options.clone() };
        &narrowed
    } else {
        options
    };
    let mut reported = 0usize;
    let mut audio: Vec<f32> = samples.iter().take(SLOT_SAMPLES).map(|&x| x * SCALE).collect();
    audio.resize(SLOT_SAMPLES, 0.0);
    // AGCc: come JTDX una volta sola sull'audio del periodo, prima di tutte le passate; se il rumore e' uniforme (`lagccbail`) resta tutto com'e'
    let agcc = options.agc && sync::agc_compensate(&mut audio, options.f_low_hz, options.f_high_hz);
    let mut found: Vec<DecodedInfo> = Vec::new();
    // i segnali CQ, le chiamate dirette a noi e il segnale del canale QSO di questo periodo, da tenere per il prossimo
    // periodo della stessa parita' (sottopassate 3-11): si popolano in `try_candidate`, si scrivono in `history` alla fine.
    let mut cq_capture: Vec<StoredSignal> = Vec::new();
    let mut my_capture: Vec<StoredSignal> = Vec::new();
    let mut qso_capture: Option<StoredSignal> = None;

    let npass = options.passes.max(1);
    let cycles = if npass >= 7 { 3 } else if npass >= 4 { 2 } else { 1 };
    let ap_options = options.ap.clone().unwrap_or_else(|| ApOptions::new(""));
    let ap_ctx = ApContext::new(&ap_options);
    // il proprio nominativo e quello del DX, completi (con /P o prefisso), nella tabella degli hash: un messaggio che li porta solo in hash
    // (i tipi a priori 11-14, 40-44, ma anche senza AP) si legge con il nominativo invece di `<...>`, come con i nominativi configurati di
    // WSJT-X 3.3.0-beta6 (`sync_configured_calls_core`, `77bit/packjt77.f90:1717-1742`; `ft8apsetvar.f90:50-51`)
    if let Some(a) = &options.ap {
        for call in [&a.mycall, &a.hiscall] {
            hashes.save_call(call.trim());
        }
    }
    let mut qso_decoded = false;
    let mut audio_before_shift: Option<Vec<f32>> = None;
    // `jzb=-62 + avexdt*25.` (`ft8_decodevar.f90:172-173`): la somma si tronca verso lo zero quando va nell'intero. Il ritardo medio
    // resta nei limiti dei candidati (-2..3 s): un valore fuori non sposta la finestra oltre lo spettrogramma.
    let jz = |center: f32, base: i32| (base as f32 + center.clamp(-2.0, 3.0) * 25.0) as i32;
    let mut history = history;
    // l'audio dopo l'ultima passata che ha tolto qualcosa: su questo lavorano "a7" e "a8"
    let mut residual = audio.clone();
    // thread: un gruppo di candidati per volta, due per thread
    let threads = options.threads.max(1);
    let pool = crate::pool::shared(threads);
    let batch = if pool.is_some() { 2 * threads } else { 1 };

    for ipass in 1..=npass {
        if ipass > 1 && options.deadline.is_some_and(|d| std::time::Instant::now() >= d) {
            break;
        }
        let subtract_on = !(ipass > 5 || (ipass == 3 && npass == 3 && !options.swl));
        // syncmin: 1,3 oppure, con la soglia bassa, 1,225 / 1,3 / 1,1 a rotazione (`ft8_decodevar.f90` di WSJT-X 3.3.0-beta6; JTDX usava 1,5)
        let syncmin = if options.low_threshold || options.swl {
            match (ipass - 1) % 3 {
                0 => 1.225,
                1 => 1.3,
                _ => 1.1,
            }
        } else {
            options.min_sync
        };
        // l'audio a mezzo campione: passata 4 verso destra, passata 7 (solo con tre cicli) verso sinistra dall'audio di prima
        if ipass == 4 {
            if npass >= 9 {
                audio_before_shift = Some(audio.clone());
            }
            for i in 0..SLOT_SAMPLES - 1 {
                audio[i] = (audio[i] + audio[i + 1]) / 2.0;
            }
        } else if let (7, Some(saved)) = (ipass, &audio_before_shift) {
            audio[0] = saved[0];
            for i in 1..SLOT_SAMPLES {
                audio[i] = (saved[i - 1] + saved[i]) / 2.0;
            }
        }

        let half_window = if options.swl { 86 } else { 62 };
        let window = (jz(options.dt_center_s, -half_window), jz(options.dt_center_s, half_window));
        let candidates = sync::find_candidates_agc(&audio, options.f_low_hz, options.f_high_hz, syncmin, options.max_candidates, Spectrum::for_pass(ipass), window, options.swl, agcc.then_some(ipass), options.rx_freq_hz);
        if candidates.is_empty() {
            continue;
        }
        let mut ds = Downsampler::new(&audio);
        let mut stale: Vec<f32> = Vec::new();
        let mut pending: Vec<Pending> = Vec::new();
        let mut subtracted = false;

        // I candidati si provano a gruppi in parallelo sulla stessa fotografia dello stato (spettro, hash, QSO gia' decodificato)
        // e si accettano poi uno per uno, nell'ordine del singolo thread. Un risultato si butta e si rifa' quando lo stato che
        // ha letto e' cambiato prima del suo turno: spettro rifatto per un segnale tolto vicino, QSO decodificato, hash
        // cambiati per un candidato che ha letto un testo. Cosi' i decode sono identici a quelli del singolo thread.
        let mut hashes_version = 0u64;
        let mut idx = 0usize;
        while idx < candidates.len() {
            let end = (idx + batch).min(candidates.len());
            let snapshot_qso = qso_decoded;
            let snapshot_hashes = hashes_version;
            let speculative: Vec<Option<(Option<Hit>, Capture)>> = {
                let ctx = PassContext { ipass, cycles, options, ap: &ap_ctx, qso_decoded, hashes: &*hashes, history: history.as_deref() };
                let eval = |cand: &PassCandidate| -> Option<(Option<Hit>, Capture)> {
                    let c = cand.candidate;
                    if found.iter().any(|d| (d.decoded.freq_hz - c.freq_hz).abs() < SAME_FREQ_HZ && (d.decoded.dt_s - c.dt_s).abs() < SAME_DT_S) {
                        return None;
                    }
                    Some(try_candidate(&ds, *cand, &ctx))
                };
                match (&pool, end - idx > 1) {
                    (Some(pool), true) => pool.install(|| candidates[idx..end].par_iter().map(eval).collect()),
                    _ => candidates[idx..end].iter().map(eval).collect(),
                }
            };
            let mut k = idx;
            for result in speculative {
                let cand = candidates[k];
                let c = cand.candidate;
                if found.iter().any(|d| (d.decoded.freq_hz - c.freq_hz).abs() < SAME_FREQ_HZ && (d.decoded.dt_s - c.dt_s).abs() < SAME_DT_S) {
                    k += 1;
                    continue;
                }
                if stale.iter().any(|&f| (f - c.freq_hz).abs() < REFFT_HZ) {
                    // lo spettro si rifa': questo candidato e i seguenti vanno provati di nuovo
                    ds = Downsampler::new(&audio);
                    stale.clear();
                    break;
                }
                if qso_decoded != snapshot_qso {
                    break;
                }
                let (hit, capture) = match result {
                    Some(r) if !(r.1.used_hashes && hashes_version != snapshot_hashes) => r,
                    _ => {
                        let ctx = PassContext { ipass, cycles, options, ap: &ap_ctx, qso_decoded, hashes: &*hashes, history: history.as_deref() };
                        try_candidate(&ds, cand, &ctx)
                    }
                };
                k += 1;
                if hit.is_none() {
                    if let Some(s) = capture.cq {
                        cq_capture.push(s);
                    }
                    if let Some(s) = capture.my_call {
                        my_capture.push(s);
                    }
                    if let Some(s) = capture.qso {
                        qso_capture = Some(s);
                    }
                }
                let Some(hit) = hit else { continue };
                *hashes = hit.hashes;
                hashes_version += 1;

                if subtract_on {
                    let freq = hit.info.decoded.freq_hz;
                    subtracted = true;
                    if options.immediate_subtraction {
                        remove_signal(&mut audio, options, &hit.tones, freq, hit.info.decoded.dt_s, &hit.cd, hit.i0);
                        stale.push(freq);
                    } else {
                        pending.push(Pending { tones: hit.tones, hit_freq: freq, hit_dt: hit.info.decoded.dt_s, cd: hit.cd, i0: hit.i0 });
                    }
                }
                let info = hit.info;
                if found.iter().any(|d| d.decoded.text == info.decoded.text && (d.decoded.freq_hz - info.decoded.freq_hz).abs() < DUPE_FREQ_HZ) {
                    continue;
                }
                if let Some(a) = &options.ap {
                    let root = format!("{} {} ", a.mycall, a.hiscall);
                    if a.mycall.len() >= 3 && a.hiscall.len() > 3 && info.decoded.text.starts_with(&root) {
                        qso_decoded = true;
                    }
                }
                found.push(info);
                if qso_decoded != snapshot_qso {
                    break;
                }
            }
            idx = k;
        }
        for p in pending {
            remove_signal(&mut audio, options, &p.tones, p.hit_freq, p.hit_dt, &p.cd, p.i0);
        }
        if subtracted {
            residual.clone_from(&audio);
        }
        if found.len() > reported {
            on_pass(&found[reported..]);
            reported = found.len();
        }
    }
    if let Some(h) = history.as_deref_mut() {
        h.store_cq(options.even_period, cq_capture);
        h.store_my_call(options.even_period, my_capture);
        h.store_qso(options.even_period, qso_capture);
        // i messaggi di questo periodo restano per la ridecodifica di quello seguente della stessa parita'
        let copies = found
            .iter()
            .filter(|d| crate::copies::worth_keeping(&d.decoded.text, &ap_options.mycall))
            .map(|d| crate::copies::CopyMessage { text: d.decoded.text.clone(), freq_hz: d.decoded.freq_hz, dt_s: d.decoded.dt_s })
            .collect();
        h.store_copies(options.even_period, copies);
    }
    list_decoding(&residual, options, hashes, history, &mut found, npass);
    found.sort_by(|a, b| a.decoded.freq_hz.total_cmp(&b.decoded.freq_hz));
    found
}

/// Le decodifiche a liste di WSJT-X dopo le passate ([`a7`], [`a8`]), sul residuo. Ogni decode del periodo entra nella
/// tabella "a7" della sua parita'.
fn list_decoding(residual: &[f32], options: &Options, hashes: &CallHashes, history: Option<&mut PeriodHistory>, found: &mut Vec<DecodedInfo>, npass: usize) {
    let even = options.even_period;
    let mut table = history.map(|h| &mut h.a7);
    if let Some(t) = table.as_mut() {
        t.begin_period(even, options.period_key);
        for d in found.iter() {
            t.save(even, d.decoded.dt_s, d.decoded.freq_hz, &d.decoded.text);
        }
    }
    let a8 = options.a8.as_ref().filter(|q| q.usable());
    let a7_entries = match (&table, options.a7) {
        (Some(t), true) => t.candidates(even),
        _ => Vec::new(),
    };
    if a7_entries.is_empty() && a8.is_none() {
        return;
    }
    let ds = Downsampler::new(residual);
    let dupe = |found: &[DecodedInfo], text: &str, freq: f32| found.iter().any(|d| d.decoded.text == text && (d.decoded.freq_hz - freq).abs() < DUPE_FREQ_HZ);
    let mut try_a8 = true;
    let dxcall = a8.map(|q| q.dxcall.trim().to_uppercase()).unwrap_or_default();
    let has_word = |text: &str, word: &str| !word.is_empty() && text.split_whitespace().any(|w| w == word);
    for entry in &a7_entries {
        let Some(hit) = a7::a7_decode(&ds, entry, hashes) else { continue };
        if dupe(found, &hit.text, hit.freq_hz) {
            continue;
        }
        if has_word(&hit.text, &dxcall) {
            try_a8 = false;
        }
        if let Some(t) = table.as_mut() {
            t.save(even, hit.dt_s, hit.freq_hz, &hit.text);
        }
        found.push(DecodedInfo {
            decoded: Decoded { freq_hz: hit.freq_hz, dt_s: hit.dt_s, snr_db: hit.snr_db.round() as i32, text: hit.text, hint: true },
            pass: npass + 1,
            ap_type: 7,
            hard_errors: hit.hard_errors,
            dmin: hit.dmin,
            sync: 0.0,
        });
    }
    let Some(query) = a8 else { return };
    // il corrispondente gia' decodificato vicino alla frequenza di ricezione: non serve cercarlo per liste
    if found.iter().any(|d| has_word(&d.decoded.text, &dxcall) && (d.decoded.freq_hz - query.rx_freq_hz).abs() <= 10.0) {
        try_a8 = false;
    }
    if !try_a8 {
        return;
    }
    let Some(hit) = a8::a8_decode(&ds, query, hashes) else { return };
    if dupe(found, &hit.text, hit.freq_hz) {
        return;
    }
    if let Some(t) = table.as_mut() {
        t.save(even, hit.dt_s, query.rx_freq_hz, &hit.text);
    }
    found.push(DecodedInfo {
        decoded: Decoded { freq_hz: hit.freq_hz, dt_s: hit.dt_s, snr_db: hit.snr_db.round() as i32, text: hit.text, hint: true },
        pass: npass + 2,
        ap_type: 8,
        hard_errors: hit.hard_errors,
        dmin: -hit.plog,
        sync: 0.0,
    });
}

/// Toglie un segnale dall'audio nel modo scelto dalle opzioni.
fn remove_signal(audio: &mut [f32], options: &Options, tones: &[u8; NN], freq_hz: f32, dt_s: f32, cd: &[Complex], i0: i64) {
    let dt_abs = f64::from(dt_s) + 0.5;
    match options.subtraction {
        Subtraction::Constant => subtract::subtract_constant(audio, tones, freq_hz, dt_abs),
        Subtraction::Adaptive => {
            let reference = Reference::new(tones);
            let correction = subtract::refine_dt(cd, &reference, i0);
            // Modo SWL: il filtro corto di JTDX (`NFILT2`)
            subtract::subtract_with(audio, &reference, freq_hz, dt_abs + f64::from(correction), options.swl);
        }
    }
}

/// Lo stato che `try_candidate` legge e (per gli hash) aggiorna.
struct PassContext<'a> {
    ipass: usize,

    cycles: usize,
    options: &'a Options,
    ap: &'a ApContext,
    qso_decoded: bool,
    hashes: &'a CallHashes,
    /// La cronologia tra periodi, per le sottopassate 3-11 (`None` = nessuna, come senza [`decode_slot_with_history`]).
    history: Option<&'a PeriodHistory>,
}

/// Il segnale di questo candidato da tenere per il periodo successivo della stessa parita' (sottopassate 3-11 di
/// `ft8b`): si riempie solo sulla passata giusta (penultima per CQ e chiamata diretta, ultima per il canale QSO) e solo
/// se il candidato ha il motivo giusto. Il chiamante lo tiene solo se il candidato non decodifica affatto in questo
/// periodo (corrisponde al confronto di JTDX con i CQ/chiamate gia' decodificati, qui semplificato).
#[derive(Default)]
struct Capture {
    cq: Option<StoredSignal>,
    my_call: Option<StoredSignal>,
    qso: Option<StoredSignal>,
    /// Il candidato e' arrivato a leggere il testo: il risultato dipende dalla tabella degli hash.
    used_hashes: bool,
}

/// Un candidato decodificato, con quello che serve per sottrarlo.
struct Hit {
    info: DecodedInfo,
    tones: [u8; NN],
    cd: Vec<Complex>,
    i0: i64,
    /// La tabella degli hash dopo aver letto il messaggio: si adotta quando il decode si accetta.
    hashes: CallHashes,
}

/// Indicatori sul segnale che decidono quali tipi a priori valgono la pena (`lcqsignal`, `lmycsignal`, `lqso73`...).
#[derive(Default)]
struct Flags {
    cq_signal: bool,
    cq_ratio: f32,
    my_signal: bool,
    my_ratio: f32,
    my_matches: usize,
    qso_type3: bool,
    /// `lqsosig` di `ft8bvar.f90:808`: piu' di 6 dei primi 19 simboli di dati hanno i toni di "MYCALL DXCALL" (serve solo alla profondita' dell'OSD).
    qso_signal: bool,
    /// `ldxcsig` di `ft8bvar.f90:864` (DX con nominativo standard): piu' di 3 dei simboli 10-19 hanno i toni del nominativo del DX;
    /// con il DX non standard (`ft8bvar.f90:868-887`) piu' di 4 (5 lontano dalla ricezione) dei simboli 5-23 di `<...> DXCALL 73`.
    dx_signal: bool,
    /// `lcqdxcnssig` di `ft8bvar.f90:868-887`: i simboli 1-23 sono quelli di `CQ DXCALL` (DX non standard, tipo 4).
    cq_dxns: bool,
    qso_73: bool,
    qso_rr73: bool,
    qso_rrr: bool,
    cq_dx: bool,
    /// Simboli del nominativo del DX (10-19) che tornano (`ndxt` di `ft8b`): serve a `qso_candidate` e a `cq_dx`.
    ndxt: usize,
    /// `lqsocandave`: il candidato e' il segnale del canale QSO in corso (AP mio attivo, QSO non ancora decodificato,
    /// entrambi i nominativi standard, candidato vicino alla frequenza di ricezione). Abilita le sottopassate 9-11.
    qso_candidate: bool,
}

/// FFT di 256 punti per l'indicatore di CQ e la sua rotazione di 3,125 Hz (`ctwk256`).
fn cq_fft() -> &'static (Arc<dyn Fft<f32>>, Vec<Complex>) {
    static PLAN: OnceLock<(Arc<dyn Fft<f32>>, Vec<Complex>)> = OnceLock::new();
    PLAN.get_or_init(|| {
        let plan = FftPlanner::<f32>::new().plan_fft_forward(256);
        let dphi = 2.0 * std::f64::consts::PI * 3.125 * 0.005;
        let twist = (0..256).map(|i| Complex32::new((dphi * i as f64).cos() as f32, (dphi * i as f64).sin() as f32)).collect();
        (plan, twist)
    })
}

/// `lcqsignal` di `ft8b`: gli otto simboli dopo il primo Costas, dove un CQ ha sempre il tono 0, danno una riga spettrale unica.
fn cq_signal(cd: &[Complex], ibest: i64, rscq: f32) -> bool {
    let (plan, twist) = cq_fft();
    let mut buf: Vec<Complex> = (0..256).map(|i| sync::cd_at(cd, ibest + 224 + i as i64) * twist[i]).collect();
    plan.process(&mut buf);
    let mut s256 = [0.0f32; 9];
    for (s, b) in s256.iter_mut().zip(buf.iter()) {
        *s = b.norm();
    }
    let argmax = |v: &[f32]| -> usize {
        let mut best = 0;
        for (i, &x) in v.iter().enumerate() {
            if x > v[best] {
                best = i;
            }
        }
        best
    };
    let ip = argmax(&s256);
    let mut cq = ip == 4 || rscq > 3.1;
    if (!cq && ip == 3) || ip == 5 {
        let mut wide = [0.0f32; 27];
        wide[..9].copy_from_slice(&s256);
        for (m, w) in wide[9..].iter_mut().enumerate() {
            *w = buf[8 + m].norm();
        }
        let ip2 = argmax(&wide);
        if ip2 == 3 || ip2 == 5 {
            cq = true;
        }
    }
    cq
}

/// Gli indicatori che decidono i tipi a priori, dal segnale e dalle opzioni.
fn signal_flags(ctx: &PassContext, cd: &[Complex], ibest: i64, s8: &[[f32; 8]; NN], f1: f32) -> Flags {
    let o = &ctx.ap.opts;
    let rscq = ap::cq_score(s8);
    let cq_signal = cq_signal(cd, ibest, rscq);
    let mut fl = Flags { cq_signal, cq_ratio: ap::expected_ratio(s8, &ctx.ap.cq_tones), ..Flags::default() };
    if ctx.ap.my_ok {
        fl.my_ratio = ap::expected_ratio(s8, &ctx.ap.my_tones);
        let nmic = ap::tone_matches(s8, &ctx.ap.my_tones, 0);
        fl.my_matches = nmic;
        fl.my_signal = o.my_call_active && nmic > 2;
    }
    let dfqso = o.rx_freq_hz.map_or(f32::MAX, |r| (r - f1).abs());
    let near_tx = o.tx_freq_hz.is_some_and(|t| (t - f1).abs() < o.width_hz);
    // `idtone56_valid(1)`: entrambi standard, o uno solo non standard con i messaggi in hash di `tone8.f90:81-129` (`ft8bvar.f90:799-839`)
    if ctx.ap.qso_tones_ok {
        if !ctx.qso_decoded && (dfqso < o.width_hz || near_tx) && o.my_call_active {
            let n19 = ap::tone_matches(s8, &ctx.ap.qso_tones[..19], 0);
            fl.qso_signal = n19 > 6;
            let n22 = n19 + ap::tone_matches(s8, &ctx.ap.qso_tones[19..22], 19);
            fl.qso_type3 = n22 > 3;
        }
        if dfqso < o.width_hz && (o.qso_progress == 3 || o.qso_progress == 4) {
            // i simboli 24-58 (da 1) del messaggio atteso: 73, RR73 o RRR
            let count = |t: &[u8; 58]| ap::tone_matches(s8, &t[23..], 23);
            let (n73, nrr73, nrrr) = (count(&ctx.ap.tones_73), count(&ctx.ap.tones_rr73), count(&ctx.ap.tones_rrr));
            let best = n73.max(nrr73).max(nrrr);
            if best > 6 {
                if n73 == best {
                    fl.qso_73 = true;
                } else if nrr73 == best {
                    fl.qso_rr73 = true;
                } else {
                    fl.qso_rrr = true;
                }
            }
        }
    }
    if ctx.ap.his_ok {
        // simboli 10-19 di dati: il nominativo del DX (`idtone56(1,k)`, k = 10..19), solo se `idtone56_valid(1)` (`ft8bvar.f90:857-866`)
        let ndxt = if ctx.ap.qso_tones_ok { ap::tone_matches(s8, &ctx.ap.qso_tones[9..19], 9) } else { 0 };
        fl.ndxt = ndxt;
        fl.dx_signal = ndxt > 3;
        fl.cq_dx = fl.cq_signal && ndxt > 3;
    }
    // DX non standard (`ft8bvar.f90:868-887`): simboli 1-4 e 5-23 di `CQ DXCALL`, 5-23 di `<AA1AAA> DXCALL 73`
    if !ctx.ap.his_std && o.hiscall.trim().len() > 2 {
        let ncq = ctx.ap.dxns_cq_tones.map_or(0, |t| ap::tone_matches(s8, &t[..23], 0));
        let ndxt = ctx.ap.dxns_73_tones.map_or(0, |t| ap::tone_matches(s8, &t[4..23], 4));
        fl.ndxt = ndxt;
        if dfqso < o.width_hz {
            fl.dx_signal = ndxt > 4;
            fl.cq_dxns = ncq > 5;
        } else {
            fl.dx_signal = ndxt > 5;
            fl.cq_dxns = ncq > 6;
        }
    }
    if ctx.ap.my_ok && ctx.ap.his_ok {
        // `lqsocandave`: il segnale del canale QSO in corso (sottopassate 9-11, `docs/analisi-jtdx-reverse-rea.md` 7.2.4)
        fl.qso_candidate =
            o.my_call_active && fl.ndxt > 2 && fl.my_matches > 2 && !ctx.qso_decoded && dfqso < o.width_hz / 2.0;
    }
    fl
}

/// La profondita' dell'OSD per questo candidato e tipo a priori (`ndeep` di `ft8bvar.f90:2153-2165`, WSJT-X 3.3.0-beta6): quella di base
/// ([`Options::osd_order`], 3 come in WSJT-X), 4 per un segnale del QSO o per noi (`lqsosig`, `lmycsignal`) entro `napwid` dalla
/// frequenza di ricezione o, con il proprio nominativo attivo, da quella di trasmissione, tranne che senza tipo a priori, a QSO gia'
/// decodificato o mentre si trasmette a un DX (li' lo prende il decoder del QSO); 4 anche per il nominativo del DX cercato in ricezione
/// libera entro `napwid` dalla ricezione (`ldxcsig .and. stophint`); 5 nella ridecodifica a mano (`nagainfil`). Il costo resta
/// limitato: il 4 tocca solo i candidati vicini alle frequenze del QSO con i toni attesi, il 5 solo la banda di 50 Hz della ridecodifica.
/// `ldxcsig` vale anche per un DX con nominativo non standard (i toni di `<...> DXCALL 73`, `ft8bvar.f90:868-887`) e `lqsosig` anche per i
/// messaggi del QSO con un nominativo in hash (`tone8.f90:81-129`). Manca `lhound` (in Hound qui non c'e' analisi a priori, quindi resta la base).
fn osd_depth(ctx: &PassContext, fl: &Flags, aptype: u8, f1: f32) -> usize {
    let base = ctx.options.osd_order;
    if base == 0 {
        return 0;
    }
    if ctx.options.again {
        return 5;
    }
    let o = &ctx.ap.opts;
    let dfqso = o.rx_freq_hz.map_or(f32::MAX, |r| (r - f1).abs());
    let near_tx = o.tx_freq_hz.is_some_and(|t| (t - f1).abs() < o.width_hz);
    let mut deep = false;
    if fl.qso_signal || fl.my_signal {
        if dfqso < o.width_hz || (near_tx && o.my_call_active) {
            deep = true;
        }
        // `if(lapmyc .and. ft8_qso_hints_suppressed() .or. iaptype.eq.0) ndeep=3`: in Fortran `.and.` lega piu' di `.or.`
        if (o.my_call_active && ctx.qso_decoded) || aptype == 0 {
            deep = false;
        }
        if !o.idle && o.hiscall.trim().len() > 2 {
            deep = false;
        }
    }
    if fl.dx_signal && o.idle && dfqso < o.width_hz {
        deep = true;
    }
    if deep { base.max(4) } else { base }
}

/// Quali tipi a priori valgono la pena su questo candidato: le condizioni `cycle` del ramo di `ft8bvar.f90` che vale per i due
/// nominativi ([`ApTable`]).
fn ap_allowed(aptype: u8, isubp1: usize, isubp2: usize, ctx: &PassContext, fl: &Flags, cq_only: bool, f1: f32) -> bool {
    if aptype == 1 && !ctx.options.ap_cq {
        return false;
    }
    match ctx.ap.table {
        ApTable::Standard => ap_allowed_standard(aptype, isubp2, ctx, fl, cq_only, f1),
        ApTable::DxNonStandard => ap_allowed_dx_non_standard(aptype, isubp2, ctx, fl, cq_only, f1),
        ApTable::BothNonStandard => ap_allowed_both_non_standard(aptype, isubp2, ctx, fl, cq_only),
        ApTable::MyNonStandard => ap_allowed_my_non_standard(aptype, isubp1, ctx, fl, cq_only, f1),
    }
}

/// Il candidato e' lontano sia dalla frequenza di ricezione sia da quella di trasmissione (`loutapwid`, `ft8bvar.f90:1363`).
fn out_of_ap_width(o: &ApOptions, f1: f32) -> bool {
    o.rx_freq_hz.is_none_or(|r| (r - f1).abs() > o.width_hz) && o.tx_freq_hz.is_none_or(|t| (t - f1).abs() > o.width_hz)
}

/// I rapporti segnale/rumore dei simboli attesi di CQ (tipo 1, letture 20 e 21) e del proprio nominativo (tipo 2, letture 17 e 18), come
/// nei rami dei nominativi non standard di WSJT-X 3.3.0-beta6 (`ft8bvar.f90:1626-1663`, `:1777-1797`): senza le eccezioni per SWL.
fn expected_ratio_ok(aptype: u8, isubp2: usize, ctx: &PassContext, fl: &Flags) -> bool {
    let lowth = ctx.options.low_threshold || ctx.options.subpass;
    match (aptype, isubp2) {
        (1, 20) => !(fl.cq_ratio < 1.0 && !fl.cq_signal),
        (1, 21) => !(fl.cq_ratio < if lowth { 1.2 } else { 1.3 } && !fl.cq_signal),
        (2, 17) => !(fl.my_ratio < 1.0 && !fl.my_signal),
        (2, 18) => !(lowth && fl.my_ratio < 1.2 && !fl.my_signal),
        _ => true,
    }
}

/// Proprio nominativo standard, DX non standard: `ft8bvar.f90:1606-1663` (tabella [`ap::NDXNSAPTYPES`]).
fn ap_allowed_dx_non_standard(aptype: u8, isubp2: usize, ctx: &PassContext, fl: &Flags, cq_only: bool, f1: f32) -> bool {
    let o = &ctx.ap.opts;
    let far = out_of_ap_width(o, f1);
    let blocked = (aptype == 2 && cq_only)
        || (!o.idle && aptype > 30)
        // per i tipi con il proprio nominativo: QSO gia' decodificato o proprio nominativo spento (2-3 minuti dopo l'ultima trasmissione)
        || ((ctx.qso_decoded || !o.my_call_active) && aptype > 1 && aptype < 15)
        || (aptype == 12 && !fl.qso_rrr)
        || (aptype == 13 && !fl.qso_73)
        || (aptype == 14 && !fl.qso_rr73)
        || (aptype > 30 && !o.dx_search)
        || (aptype > 30 && !o.wide_dx_search && far)
        || (aptype > 30 && cq_only)
        || (aptype == 31 && !fl.cq_dxns)
        || (aptype == 35 && !fl.qso_73)
        || (aptype == 36 && !fl.qso_rr73)
        // `lqsocandave` (riga 1623) vale solo con entrambi i nominativi standard: qui e' sempre spento
        || (aptype > 2 && aptype < 15 && far);
    !blocked && expected_ratio_ok(aptype, isubp2, ctx, fl)
}

/// Entrambi i nominativi non standard (o il proprio vuoto) con il DX non standard: `ft8bvar.f90:1766-1797`. Un QSO non e' possibile
/// (non c'e' un messaggio che porti due nominativi non standard): restano CQ e, in ricezione libera, la ricerca del DX su tutta la banda.
fn ap_allowed_both_non_standard(aptype: u8, isubp2: usize, ctx: &PassContext, fl: &Flags, cq_only: bool) -> bool {
    let o = &ctx.ap.opts;
    let blocked = (aptype > 1 && aptype < 31)
        || (!o.idle && aptype > 1)
        || (aptype > 30 && cq_only)
        || (aptype == 31 && !fl.cq_dxns)
        || (aptype > 34 && !fl.dx_signal);
    !blocked && expected_ratio_ok(aptype, isubp2, ctx, fl)
}

/// Proprio nominativo non standard, DX standard o vuoto: `ft8bvar.f90:1849-1867` (tabella [`ap::NMYCNSAPTYPES`]). Qui i rapporti dei
/// simboli attesi non si guardano; `CQ ??? ???` si prova solo su un segnale che ha la riga del CQ (`lcqsignal`, riga 1869: altrimenti
/// il Fortran non costruisce la maschera e decodifica di nuovo gli LLR della lettura precedente, cosa che qui non ha senso ripetere).
fn ap_allowed_my_non_standard(aptype: u8, isubp1: usize, ctx: &PassContext, fl: &Flags, cq_only: bool, f1: f32) -> bool {
    let o = &ctx.ap.opts;
    let no_his = o.hiscall.trim().len() < 3;
    let far = out_of_ap_width(o, f1);
    let blocked = isubp1 > 5 // per ora solo le medie del CQ
        || (aptype == 40 && cq_only)
        || (aptype > 40 && aptype < 45 && ctx.qso_decoded)
        || (aptype == 42 && !fl.qso_rrr)
        || (aptype == 43 && !fl.qso_73)
        || (aptype == 44 && !fl.qso_rr73)
        || (aptype > 39 && !o.my_call_active)
        || (no_his && aptype != 1 && aptype != 40)
        || (aptype > 30 && aptype < 40 && !o.idle)
        || (aptype == 31 && !fl.cq_dx)
        || ((aptype == 35 || aptype == 36) && (!fl.dx_signal || cq_only))
        || (aptype > 30 && aptype < 40 && !o.wide_dx_search && far)
        || (aptype == 1 && !fl.cq_signal);
    !blocked
}

/// Proprio nominativo standard, DX standard o vuoto (le condizioni `cycle` di `ft8b` di JTDX per nominativi standard).
fn ap_allowed_standard(aptype: u8, isubp2: usize, ctx: &PassContext, fl: &Flags, cq_only: bool, f1: f32) -> bool {
    let o = &ctx.ap.opts;
    let his_given = o.hiscall.trim().len() >= 3;
    // con un nominativo del DX non standard restano solo CQ e "MYCALL ??? ???"
    if his_given && !ctx.ap.his_ok && !matches!(aptype, 1 | 2) {
        return false;
    }
    if aptype == 1 && !ctx.options.ap_cq {
        return false;
    }
    let far = o.rx_freq_hz.is_none_or(|r| (r - f1).abs() > o.width_hz) && o.tx_freq_hz.is_none_or(|t| (t - f1).abs() > o.width_hz);
    let qso_type = (3..31).contains(&aptype);
    if ctx.qso_decoded && qso_type {
        return false;
    }
    let lowth = ctx.options.low_threshold || ctx.options.subpass;
    if aptype == 2 {
        if !o.my_call_active || cq_only {
            return false;
        }
        // in un QSO si prova "MYCALL ??? ???" solo se almeno due dei primi nove simboli hanno il tono del proprio nominativo
        if o.qso_progress != 0 && fl.my_matches < 2 {
            return false;
        }
    }
    if !o.idle && aptype > 30 {
        return false;
    }
    if o.idle && qso_type {
        return false;
    }
    let no_his = !his_given;
    if aptype > 2 && no_his {
        return false;
    }
    if qso_type && far {
        return false;
    }
    match aptype {
        3 if !fl.qso_type3 => return false,
        4 if !fl.qso_rrr => return false,
        5 if !fl.qso_73 => return false,
        6 if !fl.qso_rr73 => return false,
        _ => {}
    }
    if aptype > 30 {
        if !o.dx_search || (!o.wide_dx_search && far) {
            return false;
        }
        if aptype == 31 && !fl.cq_dx {
            return false;
        }
        if aptype > 31 && cq_only {
            return false;
        }
        if aptype == 35 && !fl.qso_73 {
            return false;
        }
        if aptype == 36 && !fl.qso_rr73 {
            return false;
        }
    }
    // rapporti segnale/rumore dei simboli attesi, valutati alla prima lettura di ogni tipo
    // in SWL (`.not.swl` davanti a ognuna di queste condizioni di `ft8b`) non si applicano
    let swl = ctx.options.swl;
    match (aptype, isubp2) {
        (1, 20) if !swl && fl.cq_ratio < 1.0 && !fl.cq_signal => return false,
        (1, 21) if !swl && fl.cq_ratio < if lowth { 1.2 } else { 1.3 } && !fl.cq_signal => return false,
        (2, 17) if !swl && fl.my_ratio < 1.0 && !fl.my_signal => return false,
        (2, 18) if !swl && lowth && fl.my_ratio < 1.2 && !fl.my_signal => return false,
        (3, 5) if fl.my_ratio < 1.0 => return false,
        (3, 6) if fl.my_ratio < 1.2 => return false,
        _ => {}
    }
    true
}

/// Allinea un candidato, ne calcola le probabilita' dei bit e prova a decodificarlo (`ft8b`); ritorna anche il segnale
/// da tenere per il periodo successivo della stessa parita', se questo candidato ha il motivo giusto sulla passata giusta.
fn try_candidate(ds: &Downsampler, cand: PassCandidate, ctx: &PassContext) -> (Option<Hit>, Capture) {
    let options = ctx.options;
    let ipass = ctx.ipass;
    let c = cand.candidate;
    // la discesa a 200 Hz, con i ritocchi dei bordi per i segnali deboli (`lhighsens`)
    let highsens = c.sync < 1.9 || (matches!(ipass, 2 | 4 | 6) && c.sync < 3.15);
    let mut cd = ds.run(c.freq_hz, highsens);
    let refined: Candidate = sync::refine_ex(&cd, c, SyncKind::for_pass(ipass), cand.cq);
    sync::shift_frequency(&mut cd, -(refined.freq_hz - c.freq_hz));
    let ibest = (f64::from(refined.dt_s + 0.5) * 200.0).round() as i64;

    // lettura a tempo invertito: passate 5 e 7 con piu' cicli, la 2 con uno solo
    let reverse = if ctx.cycles < 2 { ipass == 2 } else { ipass == 5 || ipass == 7 };
    let Some(first) = sync::bit_metrics_ex(&cd, refined.dt_s, MetricsMode { reverse, second: false }) else {
        return (None, Capture::default());
    };

    // controlli di sincronismo prima di LDPC
    let o = &ctx.ap.opts;
    let on_rx = o.rx_freq_hz.is_some_and(|r| (r - refined.freq_hz).abs() < 2.0) && !o.idle;
    let screen = if options.filters {
        filters::sync_screen(&first.s8, refined.dt_s, cand.cq, !on_rx)
    } else if first.nsync >= 7 {
        filters::SyncScreen { pass: true, cq_only: false, skip_plain: false }
    } else {
        filters::SyncScreen { pass: false, cq_only: false, skip_plain: false }
    };
    if !screen.pass {
        return (None, Capture::default());
    }

    // il messaggio di 30 s fa nello stesso punto (`lsd` di `ft8b`): prima della decodifica normale si prova a ritrovarlo (`ft8sd1`)
    let copy = ctx.history.and_then(|h| h.find_copy(options.even_period, c.freq_hz, c.dt_s));
    if let Some(copy) = copy
        && let Some((text, tones)) = crate::copies::ft8sd1(&first.s8, &copy.text, crate::copies::is_cq(&copy.text), &ctx.ap.opts.mycall)
    {
        return (copy_hit(ctx, c, &refined, text, tones, &first.s8, cd, ibest), Capture::default());
    }

    let flags = signal_flags(ctx, &cd, ibest, &first.s8, refined.freq_hz);
    let nweak = if options.subpass || options.swl { 2 } else { 1 };
    let progress = usize::from(o.qso_progress.min(5));

    // sottopassate 3-11: CQ, chiamata diretta a noi e segnale del canale QSO, sommati (in potenza o in ampiezza) con il
    // periodo precedente della stessa parita' se si ripetono li' (`docs/analisi-jtdx-reverse-rea.md` 7.2.4).
    let even = options.even_period;
    let mut nsubpasses = nweak;
    let mut csold_cq = None;
    let mut csold_my = None;
    let mut csold_qso = None;
    if flags.cq_signal {
        nsubpasses = 3;
        if let Some(s) = ctx.history.and_then(|h| h.find_cq(even, refined.freq_hz, refined.dt_s)) {
            nsubpasses = 5;
            csold_cq = Some(s);
        }
    }
    if flags.my_signal {
        nsubpasses = 6;
        if let Some(s) = ctx.history.and_then(|h| h.find_my_call(even, refined.freq_hz, refined.dt_s)) {
            nsubpasses = 8;
            csold_my = Some(s);
        }
    }
    if flags.qso_candidate {
        nsubpasses = 9;
        if let Some(s) = ctx.history.and_then(|h| h.find_qso(even, refined.freq_hz, refined.dt_s)) {
            nsubpasses = 11;
            csold_qso = Some(s);
        }
    }
    // il segnale da proporre per il prossimo periodo della stessa parita': sulla penultima passata per CQ e chiamata
    // diretta, sull'ultima per il canale QSO (`ipass.eq.npass-1` / `ipass.eq.npass` di `ft8b`).
    let mut capture = Capture::default();
    if ipass + 1 == options.passes.max(1) {
        if flags.cq_signal {
            capture.cq = Some(StoredSignal::new(refined.freq_hz, refined.dt_s, &first.cs));
        }
        if flags.my_signal {
            capture.my_call = Some(StoredSignal::new(refined.freq_hz, refined.dt_s, &first.cs));
        }
    }
    if ipass == options.passes.max(1) && flags.qso_candidate {
        capture.qso = Some(StoredSignal::new(refined.freq_hz, refined.dt_s, &first.cs));
    }

    for isubp1 in 1..=nsubpasses {
        if nweak == 1 && isubp1 == 2 {
            continue;
        }
        // le sottopassate del CQ (3, 4, 5) non si provano se il segnale e' anche una chiamata diretta a noi: quella vince
        if isubp1 > 2 && isubp1 < 6 && flags.my_signal {
            continue;
        }
        let (llr, s8): ([[f32; 174]; 4], [[f32; 8]; NN]) = if isubp1 <= 2 {
            let Some(metrics) = (if isubp1 == 1 { Some(first.clone()) } else { sync::bit_metrics_ex(&cd, refined.dt_s, MetricsMode { reverse, second: true }) }) else {
                return (None, capture);
            };
            (metrics.llr, metrics.s8)
        } else if matches!(isubp1, 3 | 6 | 9) {
            // combina la lettura normale e quella a tempo invertito dello stesso periodo: non serve la cronologia
            let source = sync::SymbolSource::PowerSum(&first.cs, &first.csr);
            let Some(llr) = sync::llr_from_source(source, first.srr, false) else { return (None, capture) };
            (llr, first.s8)
        } else {
            // 4/7/10 (potenza) o 5/8/11 (ampiezza): il periodo attuale con quello precedente della stessa parita' (`csold`)
            let old = match isubp1 {
                4 | 5 => csold_cq,
                7 | 8 => csold_my,
                _ => csold_qso,
            };
            let Some(old) = old else { continue };
            let source = if matches!(isubp1, 4 | 7 | 10) {
                sync::SymbolSource::PowerSum(&first.cs, &old.cs)
            } else {
                sync::SymbolSource::AmplitudeSum(&first.cs, &old.cs)
            };
            let Some(llr) = sync::llr_from_source(source, first.srr, false) else { return (None, capture) };
            (llr, first.s8)
        };
        let [llra, llrb, llrc, llrd] = llr;
        let apmag = llra.iter().fold(0.0f32, |m, v| m.max(v.abs())) * 1.01;

        for isubp2 in 1..=31usize {
            let (llrz, mask, aptype);
            if isubp2 < 5 {
                // la quarta lettura sola (`llrd`) esiste solo in SWL (`if(.not.swl .and. isubp2.eq.4) cycle`)
                if screen.cq_only || screen.skip_plain || (isubp2 == 4 && !options.swl) {
                    continue;
                }
                llrz = match isubp2 {
                    // `if(.not.swl .and. ipass.eq.1) llrz=llrd else llrz=llra; if(isubp1.gt.1 .and. ipass.gt.1) llrz=llrd`
                    1 => {
                        if (!options.swl && ipass == 1) || (isubp1 > 1 && ipass > 1) {
                            llrd
                        } else {
                            llra
                        }
                    }
                    2 => {
                        if isubp1 > 1 {
                            llra
                        } else {
                            llrb
                        }
                    }
                    3 => llrc,
                    _ => llrd,
                };
                mask = [false; 174];
                aptype = 0u8;
            } else {
                let t = ctx.ap.aptypes()[progress][isubp2 - 5];
                if t == 0 || !ap_allowed(t, isubp1, isubp2, ctx, &flags, screen.cq_only, refined.freq_hz) {
                    continue;
                }
                let base = [llra, llrb, llrc][ap::llr_index(isubp2, t)];
                let Some((z, m)) = ctx.ap.apply(t, &base, apmag) else { continue };
                llrz = z;
                mask = m;
                aptype = t;
            }

            // BP, e se non basta OSD con la profondita' di questo candidato (`ft8bvar.f90:2153-2168`)
            let ndeep = osd_depth(ctx, &flags, aptype, refined.freq_hz);
            let quality = match ldpc::bp_decode_masked(&llrz, &mask, options.bp_iterations) {
                Some(q) => q,
                None if ndeep > 0 => match ldpc::osd_decode_masked(&llrz, &mask, ndeep) {
                    Some(q) => q,
                    None => continue,
                },
                None => continue,
            };
            if options.filters {
                if !filters::accept_decode(quality.hard_errors, quality.dmin, isubp2 > 2) {
                    continue;
                }
            } else if quality.hard_errors > 45 {
                continue;
            }

            let mut bits77 = [0u8; 77];
            bits77.copy_from_slice(&quality.decoded.message[..77]);
            let (i3, n3) = message::message_type(&bits77);
            // i3=5 (concorso EU VHF) si accetta come in WSJT-X (`ft8bvar.f90`, `i3.gt.5`): prima si scartava
            if i3 > 5 || (i3 == 0 && n3 > 5) {
                continue;
            }
            // il testo si legge su una copia degli hash: se il messaggio poi e' scartato la tabella non si sporca
            capture.used_hashes = true;
            let mut trial = ctx.hashes.clone();
            let Ok(text) = message::unpack(&bits77, &mut trial) else { continue };
            if aptype == 1 && text.starts_with("CQ DE AA00") {
                continue;
            }
            let tones = modem::tones_from_codeword(&quality.decoded.codeword);
            let xsnr = filters::jtdx_snr(&s8, &tones, aptype != 0);
            if options.filters {
                let qual = 1.0 - (quality.hard_errors as f32 + quality.dmin) / 60.0;
                let check = MessageCheck { text: &text, i3, n3, ap_type: aptype, qual, snr: xsnr, dt_s: refined.dt_s };
                if filters::reject_message(&check, &o.mycall, options.call_book.as_deref()) || filters::grid_rejected_ft8(&check, &o.mycall, &o.hiscall, options.grid_check.is_some()) {
                    return (None, capture);
                }
            }

            return (
                Some(Hit {
                    info: DecodedInfo {
                        decoded: Decoded { freq_hz: refined.freq_hz, dt_s: refined.dt_s, snr_db: xsnr.round() as i32, text, hint: aptype != 0 },
                        pass: ipass,
                        ap_type: aptype,
                        hard_errors: quality.hard_errors,
                        dmin: quality.dmin,
                        sync: c.sync,
                    },
                    tones,
                    cd,
                    i0: ibest,
                    hashes: trial,
                }),
                capture,
            );
        }
    }
    // `iqso = 4`: la decodifica normale non ha trovato niente; si prova il seguito del messaggio di 30 s fa
    if let Some(copy) = copy {
        let lcq = crate::copies::is_cq(&copy.text);
        let deep = options.swl || options.subpass || options.low_threshold;
        let (s8, cd, at, ibest) = if deep {
            // `ldeepsync`: si risincronizza sul segnale originale con i toni noti del messaggio atteso
            let mut cd1 = ds.run(c.freq_hz, highsens);
            let rows = crate::copies::known_rows(&copy.text, lcq);
            let (delf, dt) = crate::copies::deep_refine(&cd1, c.dt_s, SyncKind::for_pass(ipass), &rows, lcq);
            sync::shift_frequency(&mut cd1, -delf);
            let Some(m) = sync::bit_metrics_ex(&cd1, dt, MetricsMode { reverse, second: false }) else { return (None, capture) };
            let at = Candidate { freq_hz: c.freq_hz + delf, dt_s: dt, sync: c.sync };
            let ib = (f64::from(dt + 0.5) * 200.0).round() as i64;
            if let Some((text, tones)) = crate::copies::ft8sd1(&m.s8, &copy.text, lcq, &ctx.ap.opts.mycall) {
                return (copy_hit(ctx, c, &at, text, tones, &m.s8, cd1, ib), capture);
            }
            (m.s8, cd1, at, ib)
        } else {
            (first.s8, cd, refined, ibest)
        };
        let found = if lcq { crate::copies::ft8mfcq(&s8, &copy.text) } else { crate::copies::ft8mf1(&s8, &copy.text) };
        if let Some((text, tones)) = found {
            return (copy_hit(ctx, c, &at, text, tones, &s8, cd, ibest), capture);
        }
    }
    (None, capture)
}

/// Il decode trovato dalla ridecodifica del messaggio di 30 s fa ([`crate::copies`]): niente a priori, i toni servono alla sottrazione.
#[allow(clippy::too_many_arguments)]
fn copy_hit(ctx: &PassContext, c: Candidate, at: &Candidate, text: String, tones: [u8; NN], s8: &[[f32; 8]; NN], cd: Vec<Complex>, i0: i64) -> Option<Hit> {
    let xsnr = filters::jtdx_snr(s8, &tones, false);
    Some(Hit {
        info: DecodedInfo {
            decoded: Decoded { freq_hz: at.freq_hz, dt_s: at.dt_s, snr_db: xsnr.round() as i32, text, hint: true },
            pass: ctx.ipass,
            ap_type: 0,
            hard_errors: 0,
            dmin: 0.0,
            sync: c.sync,
        },
        tones,
        cd,
        i0,
        hashes: ctx.hashes.clone(),
    })
}

#[cfg(test)]
mod tests {
    use std::f64::consts::PI;

    use crate::SAMPLE_RATE;

    use super::*;

    /// Generatore pseudocasuale deterministico (nessuna dipendenza): uniforme in (0,1) e gaussiana (Box-Muller).
    struct Rng(u64);
    impl Rng {
        fn uniform(&mut self) -> f64 {
            self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            ((self.0 >> 11) as f64 + 0.5) / (1u64 << 53) as f64
        }
        fn gauss(&mut self) -> f64 {
            (-2.0 * self.uniform().ln()).sqrt() * (2.0 * PI * self.uniform()).cos()
        }
    }

    /// Un periodo con rumore bianco e i messaggi dati (testo, frequenza, dt, SNR in 2500 Hz).
    fn slot(signals: &[(&str, f32, f32, f32)], seed: u64) -> Vec<f32> {
        let mut rng = Rng(seed);
        let sigma = 0.05f64; // ampiezza del rumore (normalizzata)
        let mut audio: Vec<f32> = (0..SLOT_SAMPLES).map(|_| (sigma * rng.gauss()) as f32).collect();
        let mut hashes = CallHashes::default();
        for &(text, freq, dt, snr) in signals {
            let wave = modem::encode_message(text, freq, &mut hashes).expect("messaggio valido");
            // potenza del rumore in 2500 Hz = sigma^2 * 2500 / 6000; l'onda ha picco a, potenza a^2 / 2
            let noise_2500 = sigma * sigma * 2500.0 / 6000.0;
            let amplitude = (2.0 * noise_2500 * 10f64.powf(f64::from(snr) / 10.0)).sqrt() as f32;
            let start = ((f64::from(dt) + 0.5) * SAMPLE_RATE as f64).round() as usize;
            for (i, &w) in wave.iter().enumerate() {
                if let Some(a) = audio.get_mut(start + i) {
                    *a += amplitude * w;
                }
            }
        }
        audio
    }

    fn decode(audio: &[f32]) -> Vec<Decoded> {
        decode_slot(audio, &Options::default(), &mut CallHashes::default())
    }

    #[test]
    fn a_clean_message_is_found_with_frequency_time_and_snr() {
        let audio = slot(&[("CQ K1ABC FN42", 1500.0, 0.3, -8.0)], 1);
        let out = decode(&audio);
        assert_eq!(out.len(), 1, "{out:?}");
        let d = &out[0];
        assert_eq!(d.text, "CQ K1ABC FN42");
        assert!((d.freq_hz - 1500.0).abs() < 1.0, "{}", d.freq_hz);
        assert!((d.dt_s - 0.3).abs() < 0.05, "{}", d.dt_s);
        assert!((d.snr_db - (-8)).abs() <= 3, "snr {}", d.snr_db);
    }

    #[test]
    fn several_signals_are_all_found() {
        let audio = slot(&[("CQ K1ABC FN42", 800.0, 0.1, -6.0), ("IU3ABC K1ABC -15", 1500.0, 0.3, -9.0), ("K1ABC IU3ABC RR73", 2200.0, -0.2, -4.0)], 7);
        let texts: Vec<String> = decode(&audio).into_iter().map(|d| d.text).collect();
        for expected in ["CQ K1ABC FN42", "IU3ABC K1ABC -15", "K1ABC IU3ABC RR73"] {
            assert!(texts.iter().any(|t| t == expected), "manca {expected}: {texts:?}");
        }
        assert_eq!(texts.len(), 3, "nessun falso: {texts:?}");
    }

    #[test]
    fn swl_mode_finds_the_same_signals_and_invents_nothing() {
        let audio = slot(&[("CQ K1ABC FN42", 800.0, 0.1, -6.0), ("IU3ABC K1ABC -15", 1500.0, 0.3, -9.0), ("K1ABC IU3ABC RR73", 2200.0, -0.2, -4.0)], 7);
        let swl = Options { swl: true, ..Options::default() };
        let texts: Vec<String> = decode_slot(&audio, &swl, &mut CallHashes::default()).into_iter().map(|d| d.text).collect();
        for expected in ["CQ K1ABC FN42", "IU3ABC K1ABC -15", "K1ABC IU3ABC RR73"] {
            assert!(texts.iter().any(|t| t == expected), "manca {expected}: {texts:?}");
        }
        assert_eq!(texts.len(), 3, "nessun falso: {texts:?}");
        for seed in [3, 4, 5] {
            let out = decode_slot(&slot(&[], seed), &swl, &mut CallHashes::default());
            assert!(out.is_empty(), "solo rumore, seme {seed}: {out:?}");
        }
        assert!(!Options::default().swl && !Options::depth(3).swl);
    }

    #[test]
    fn a_weak_signal_under_a_strong_one_appears_after_subtraction() {
        // i due toni sono a 12 Hz di distanza: il debole e' coperto dai fianchi del forte finche' questo non viene tolto
        let audio = slot(&[("CQ DX IU3ABC IO91", 1500.0, 0.2, 5.0), ("K1ABC W9XYZ -12", 1512.0, 0.4, -9.0)], 11);
        let texts: Vec<String> = decode(&audio).into_iter().map(|d| d.text).collect();
        assert!(texts.iter().any(|t| t == "CQ DX IU3ABC IO91"), "{texts:?}");
        assert!(texts.iter().any(|t| t == "K1ABC W9XYZ -12"), "{texts:?}");
    }

    #[test]
    fn un_cq_debole_che_si_ripete_decodifica_sommando_due_periodi() {
        let text = "CQ IU3ABC JN45";
        let freq = 1700.0;
        let dt = 0.1;
        // non decodifica da solo, ma due periodi sommati (sottopassate 4/5) si'. Era -23 dB: con la soglia di sincronismo di WSJT-X 3.3 (1,3) un periodo
        // solo lo decodificava gia'; da -23,5 a -25 dB il primo periodo non basta e la somma si' (-24 sta in mezzo)
        let snr = -24.0;
        let mut hashes = CallHashes::default();
        let mut history = PeriodHistory::default();
        let options = Options::default();

        let first_audio = slot(&[(text, freq, dt, snr)], 101);
        let first = decode_slot_with_history(&first_audio, &options, &mut hashes, &mut history);
        assert!(first.is_empty(), "il primo periodo da solo non deve ancora decodificare: {first:?}");

        // stessa parita' (stesso `even_period`), rumore indipendente: lo stesso CQ debole ora si somma col primo
        let second_audio = slot(&[(text, freq, dt, snr)], 202);
        let second = decode_slot_with_history(&second_audio, &options, &mut hashes, &mut history);
        assert!(second.iter().any(|d| d.text == text), "il secondo periodo deve decodificare sommando il primo: {second:?}");
    }

    #[test]
    fn la_cronologia_non_aiuta_la_parita_sbagliata() {
        let text = "CQ IU3ABC JN45";
        let freq = 1750.0;
        let dt = 0.1;
        let snr = -23.0;
        let mut hashes = CallHashes::default();
        let mut history = PeriodHistory::default();
        let options = Options { even_period: true, ..Options::default() };
        let first = decode_slot_with_history(&slot(&[(text, freq, dt, snr)], 303), &options, &mut hashes, &mut history);
        assert!(first.is_empty());

        let odd = Options { even_period: false, ..Options::default() };
        let second = decode_slot_with_history(&slot(&[(text, freq, dt, snr)], 404), &odd, &mut hashes, &mut history);
        assert!(!second.iter().any(|d| d.text == text), "la parita' dispari non deve vedere la cronologia della pari: {second:?}");
    }

    /// L'AGC della radio che "pompa": il guadagno e' 8 volte piu' alto nei primi 0,8 s e negli ultimi 0,8 s del periodo (una trasmissione
    /// forte appena finita o appena cominciata). Con la compensazione (AGCc) quei tratti tornano al livello del resto.
    fn pumped(mut audio: Vec<f32>) -> Vec<f32> {
        for a in &mut audio[..9600] {
            *a *= 8.0;
        }
        for a in &mut audio[161_280..171_650] {
            *a *= 8.0;
        }
        audio
    }

    #[test]
    fn la_compensazione_agc_lascia_stare_un_periodo_uniforme() {
        let mut audio: Vec<f32> = slot(&[], 7).iter().map(|x| x * SCALE).collect();
        let before = audio.clone();
        assert!(!sync::agc_compensate(&mut audio, 200.0, 4000.0), "rumore uniforme: niente da compensare");
        assert_eq!(audio, before);
    }

    #[test]
    fn la_compensazione_agc_riporta_al_livello_i_tratti_pompati() {
        let mut audio: Vec<f32> = pumped(slot(&[], 7)).iter().map(|x| x * SCALE).collect();
        assert!(sync::agc_compensate(&mut audio, 200.0, 4000.0));
        let rms = |r: std::ops::Range<usize>| (audio[r.clone()].iter().map(|x| x * x).sum::<f32>() / r.len() as f32).sqrt();
        let (head, middle, tail) = (rms(0..9600), rms(9600..161_280), rms(161_280..170_880));
        // dopo la compensazione inizio e fine non sono piu' 8 volte (18 dB) sopra il resto
        assert!(head / middle < 3.0 && tail / middle < 3.0, "inizio {head}, centro {middle}, fine {tail}");
    }

    #[test]
    fn con_l_agc_che_pompa_la_compensazione_decodifica() {
        let text = "CQ IU3ABC JN45";
        let audio = pumped(slot(&[(text, 1500.0, 0.1, -17.0)], 31));
        let with = decode_slot(&audio, &Options { agc: true, ..Options::default() }, &mut CallHashes::default());
        assert!(with.iter().any(|d| d.text == text), "con AGCc: {with:?}");
    }

    #[test]
    #[ignore]
    fn esplora_agc() {
        let text = "CQ IU3ABC JN45";
        // a meta' periodo una stazione forte fa scendere il guadagno a un quarto per 5 s
        let mid = |mut a: Vec<f32>| {
            for x in &mut a[60_000..120_000] {
                *x *= 0.25;
            }
            a
        };
        let mut false_hits = 0;
        for seed in 100..130 {
            false_hits += decode_slot(&pumped(slot(&[], seed)), &Options { agc: true, ..Options::default() }, &mut CallHashes::default()).len();
        }
        eprintln!("ESPLORA solo rumore pompato, con AGCc: {false_hits} messaggi su 30 periodi");
        for (name, snr, f) in [("pulito", -21.0, 2), ("pulito", -22.0, 2), ("bordi", -21.0, 0), ("bordi", -22.0, 0), ("meta'", -20.0, 1)] {
            let (mut off, mut on) = (0, 0);
            for seed in 40..60 {
                let clean = slot(&[(text, 1500.0, 0.1, snr)], seed);
                let audio = match f {
                    0 => pumped(clean),
                    1 => mid(clean),
                    _ => clean,
                };
                let hit = |agc: bool| decode_slot(&audio, &Options { agc, ..Options::default() }, &mut CallHashes::default()).iter().any(|d| d.text == text);
                off += usize::from(hit(false));
                on += usize::from(hit(true));
            }
            eprintln!("ESPLORA {name} snr {snr}: senza {off}/20, con {on}/20");
        }
    }

    #[test]
    fn noise_alone_decodes_nothing() {
        for seed in [3, 4, 5] {
            let out = decode(&slot(&[], seed));
            assert!(out.is_empty(), "seme {seed}: {out:?}");
        }
    }

    #[test]
    fn a_short_or_empty_buffer_does_not_panic() {
        assert!(decode(&[]).is_empty());
        assert!(decode(&vec![0.0; 1000]).is_empty());
        assert!(decode(&vec![0.0; SLOT_SAMPLES]).is_empty());
    }

    #[test]
    fn le_profondita_scelgono_le_passate_e_clampano() {
        assert_eq!(Options::depth(1).passes, 3);
        assert_eq!(Options::depth(2).passes, 6);
        assert_eq!(Options::depth(3).passes, 9);
        assert_eq!(Options::depth(0).passes, 3);
        assert_eq!(Options::depth(9).passes, 9);
        assert!(Options::depth(3).low_threshold && !Options::depth(2).low_threshold);
    }

    #[test]
    fn il_segnale_e_il_nominativo_giusto_servono_all_analisi_a_priori_ma_uno_sbagliato_non_inventa_messaggi() {
        // un messaggio per noi a -22 dB con la profondita' normale: con il nominativo giusto l'AP lo recupera piu' spesso
        // di quanto lo faccia senza; con uno sbagliato non deve uscire niente che non sia nel segnale
        let mut con = 0;
        let mut senza = 0;
        let mut sbagliato = 0;
        for seed in 0..6u64 {
            let audio = slot(&[("IU3ABC K1ABC -15", 1200.0 + 100.0 * seed as f32, 0.2, -22.0)], 40 + seed);
            let ap = Options { ap: Some(ApOptions::new("IU3ABC")), ..Options::default() };
            con += decode_slot(&audio, &ap, &mut CallHashes::default()).iter().filter(|d| d.text == "IU3ABC K1ABC -15").count();
            senza += decode(&audio).iter().filter(|d| d.text == "IU3ABC K1ABC -15").count();
            let wrong = Options { ap: Some(ApOptions::new("W1AW")), ..Options::default() };
            let out = decode_slot(&audio, &wrong, &mut CallHashes::default());
            sbagliato += out.iter().filter(|d| d.text.starts_with("W1AW ")).count();
        }
        assert!(con >= senza, "con {con} senza {senza}");
        assert_eq!(sbagliato, 0, "un nominativo sbagliato non deve produrre messaggi per lui");
    }

    #[test]
    fn con_il_dx_non_standard_l_analisi_a_priori_trova_il_seguito_del_qso_in_hash() {
        // `<IU3ABC> PJ4/K1ABC RR73` (tipo 4, hash a 12 bit del proprio nominativo) a -22 dB: la decodifica normale non lo vede, il tipo 14
        // (`<MYCALL> DXCALL RR73`, `ndxnsaptypes(3,4:6)`) si', e la tabella degli hash lo fa leggere con il nominativo
        let text = "<IU3ABC> PJ4/K1ABC RR73";
        let mut con = 0;
        let mut senza = 0;
        for seed in 0..4u64 {
            let f = 1100.0 + 150.0 * seed as f32;
            let audio = slot(&[(text, f, 0.2, -22.0)], 70 + seed);
            let ap = ApOptions {
                hiscall: "PJ4/K1ABC".into(),
                qso_progress: 3,
                idle: false,
                rx_freq_hz: Some(f),
                tx_freq_hz: Some(f),
                ..ApOptions::new("IU3ABC")
            };
            let options = Options { ap: Some(ap), ..Options::default() };
            let out = decode_slot_detailed(&audio, &options, &mut CallHashes::default(), None);
            if let Some(d) = out.iter().find(|d| d.decoded.text == text) {
                assert_eq!(d.ap_type, 14, "{d:?}");
                con += 1;
            }
            senza += decode(&audio).iter().filter(|d| d.text.ends_with("PJ4/K1ABC RR73")).count();
        }
        assert!(con >= 3 && con > senza, "con {con} senza {senza}");
    }

    #[test]
    fn la_semina_degli_hash_scioglie_i_nominativi() {
        let mut hashes = CallHashes::default();
        let added = seed_call_hashes(&mut hashes, ["K1ABC", "IU3ABC", "K1ABC", "AB"]);
        assert_eq!(added, 2);
        assert_eq!(hashes.len(), 2);
    }

    /// Una banda affollata: 30 segnali tra 300 e 2900 Hz, alcuni sovrapposti, da -20 a +5 dB.
    fn crowded(seed: u64) -> Vec<f32> {
        let calls = ["K1ABC", "W9XYZ", "IU3ABC", "DL1ABC", "F4ABC", "EA3ABC", "G4ABC", "JA1ABC", "VK2ABC", "PY2ABC"];
        let mut rng = Rng(seed);
        let mut texts: Vec<String> = Vec::new();
        let mut signals: Vec<(f32, f32, f32)> = Vec::new();
        for i in 0..30 {
            let a = calls[i % calls.len()];
            let b = calls[(i * 7 + 3) % calls.len()];
            let text = match i % 4 {
                0 => format!("CQ {a} FN42"),
                1 => format!("{a} {b} -{:02}", i % 20),
                2 => format!("{a} {b} RR73"),
                _ => format!("{a} {b} R-{:02}", i % 15),
            };
            texts.push(text);
            let freq = 300.0 + (i as f32) * 85.0 + (rng.uniform() as f32) * 20.0;
            let dt = (rng.uniform() as f32) * 0.8 - 0.3;
            let snr = -20.0 + (rng.uniform() as f32) * 25.0;
            signals.push((freq, dt, snr));
        }
        let list: Vec<(&str, f32, f32, f32)> = texts.iter().zip(&signals).map(|(t, &(f, d, s))| (t.as_str(), f, d, s)).collect();
        slot(&list, seed)
    }

    #[test]
    fn il_multithread_da_gli_stessi_decode_del_singolo_thread() {
        for (seed, depth) in [(21u64, 2u8), (22, 3)] {
            let audio = crowded(seed);
            let ap = ApOptions { hiscall: "W9XYZ".into(), hisgrid: "EN37".into(), rx_freq_hz: Some(1200.0), ..ApOptions::new("IU3ABC") };
            let base = Options { ap: Some(ap), ..Options::depth(depth) };
            let one = decode_slot_detailed(&audio, &base, &mut CallHashes::default(), None);
            assert!(one.len() > 15, "{}", one.len());
            for threads in [2usize, 4, 8] {
                let many = decode_slot_detailed(&audio, &Options { threads, ..base.clone() }, &mut CallHashes::default(), None);
                assert_eq!(one, many, "profondita' {depth}, {threads} thread");
            }
        }
    }

    /// Ridecodifica del messaggio di 30 s fa (`crate::copies`): il seguito debole di un QSO e il solo rumore nello stesso punto.
    /// `cargo test --release -p ft8 esplora_copie -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn esplora_copie() {
        for (label, swl) in [("normale", false), ("SWL", true)] {
            for snr in [-20.0f32, -21.0, -22.0, -23.0] {
                let (mut with, mut without, mut false_hits) = (0, 0, 0);
                for seed in 0..20u64 {
                    let even = Options { swl, even_period: true, ..Options::depth(3) };
                    let first = slot(&[("K1ABC W9XYZ -10", 1200.0, 0.1, -8.0)], 5000 + seed);
                    let second = slot(&[("K1ABC W9XYZ RR73", 1200.4, 0.12, snr)], 6000 + seed);
                    let mut hashes = CallHashes::default();
                    let mut history = PeriodHistory::default();
                    decode_slot_with_history(&first, &even, &mut hashes, &mut history);
                    let out = decode_slot_with_history(&second, &even, &mut hashes, &mut history);
                    with += usize::from(out.iter().any(|d| d.text == "K1ABC W9XYZ RR73"));
                    let alone = decode_slot(&second, &even, &mut CallHashes::default());
                    without += usize::from(alone.iter().any(|d| d.text == "K1ABC W9XYZ RR73"));
                    // solo rumore nel punto ricordato: niente deve uscire
                    let mut history = PeriodHistory::default();
                    decode_slot_with_history(&first, &even, &mut CallHashes::default(), &mut history);
                    let noise = slot(&[], 7000 + seed);
                    false_hits += decode_slot_with_history(&noise, &even, &mut CallHashes::default(), &mut history).len();
                }
                println!("{label} SNR {snr}: senza memoria {without}/20, con memoria {with}/20, falsi sul rumore {false_hits}");
            }
        }
    }

    /// Modo SWL su banda fitta (segnali forti con deboli a 25-60 Hz, che restano solo dopo la sottrazione):
    /// `cargo test --release -p ft8 esplora_swl -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn esplora_swl() {
        let calls = ["K1ABC", "W9XYZ", "DL1ABC", "F4ABC", "EA3ABC", "G4ABC", "JA1ABC", "VK2ABC", "PY2ABC", "OH2ABC", "SP3ABC", "ZL1ABC"];
        let (mut total, mut found) = (0usize, 0usize);
        for seed in 0..12u64 {
            let mut rng = Rng(900 + seed);
            let mut texts: Vec<String> = Vec::new();
            let mut signals: Vec<(f32, f32, f32)> = Vec::new();
            for i in 0..24 {
                let a = calls[i % calls.len()];
                let b = calls[(i * 5 + 1) % calls.len()];
                texts.push(if i % 2 == 0 { format!("CQ {a} FN42") } else { format!("{a} {b} -{:02}", 5 + i % 15) });
                // a coppie: uno forte e uno debole vicino
                let base = 300.0 + (i / 2) as f32 * 210.0 + (rng.uniform() as f32) * 10.0;
                let freq = if i % 2 == 0 { base } else { base + 8.0 + (rng.uniform() as f32) * 14.0 };
                let snr = if i % 2 == 0 { 12.0 + (rng.uniform() as f32) * 8.0 } else { -21.0 + (rng.uniform() as f32) * 4.0 };
                signals.push((freq, (rng.uniform() as f32) * 0.6 - 0.2, snr));
            }
            let list: Vec<(&str, f32, f32, f32)> = texts.iter().zip(&signals).map(|(t, &(f, d, s))| (t.as_str(), f, d, s)).collect();
            let audio = slot(&list, 900 + seed);
            let out = decode_slot(&audio, &Options { swl: true, ..Options::depth(3) }, &mut CallHashes::default());
            total += texts.len();
            found += texts.iter().filter(|t| out.iter().any(|d| &d.text == *t)).count();
        }
        println!("SWL: {found}/{total} messaggi");
    }

    /// Tempi su banda affollata (`cargo test --release -p ft8 tempi_multithread -- --ignored --nocapture`).
    #[test]
    #[ignore]
    fn tempi_multithread() {
        let audio = crowded(21);
        for threads in [1usize, 2, 4, 8] {
            let options = Options { threads, ..Options::depth(3) };
            let t = std::time::Instant::now();
            let out = decode_slot(&audio, &options, &mut CallHashes::default());
            println!("{threads} thread: {} decode in {} ms", out.len(), t.elapsed().as_millis());
        }
    }

    fn a8_query() -> A8Query {
        A8Query { mycall: "IU3ABC".into(), dxcall: "K1ABC".into(), dxgrid: "FN42".into(), rx_freq_hz: 1500.0, threads: 4 }
    }

    #[test]
    fn a8_trova_il_corrispondente_sotto_la_soglia_e_non_inventa() {
        let mut found_plain = 0;
        let mut found_a8 = 0;
        let seeds = 501..509u64;
        for seed in seeds.clone() {
            let audio = slot(&[("IU3ABC K1ABC R-17", 1503.0, 0.2, -23.0)], seed);
            let plain = decode_slot_detailed(&audio, &Options::depth(3), &mut CallHashes::default(), None);
            if plain.iter().any(|d| d.decoded.text == "IU3ABC K1ABC R-17") {
                found_plain += 1;
            }
            let with = decode_slot_detailed(&audio, &Options { a8: Some(a8_query()), ..Options::depth(3) }, &mut CallHashes::default(), None);
            for d in &with {
                assert_eq!(d.decoded.text, "IU3ABC K1ABC R-17", "falso: {d:?}");
            }
            if let Some(d) = with.iter().find(|d| d.ap_type == 8) {
                assert!((d.decoded.freq_hz - 1503.0).abs() < 1.0 && (d.decoded.dt_s - 0.2).abs() < 0.05, "{d:?}");
                assert!(d.decoded.hint);
            }
            if with.iter().any(|d| d.decoded.text == "IU3ABC K1ABC R-17") {
                found_a8 += 1;
            }
        }
        assert!(found_a8 > found_plain, "a8 {found_a8} contro {found_plain} su 8");
        // solo rumore: nessun decode
        for seed in 601..611u64 {
            let out = decode_slot_detailed(&slot(&[], seed), &Options { a8: Some(a8_query()), ..Options::depth(3) }, &mut CallHashes::default(), None);
            assert!(out.is_empty(), "rumore, seme {seed}: {out:?}");
        }
    }

    #[test]
    fn a7_trova_il_seguito_del_qso_nel_periodo_dopo_della_stessa_parita() {
        let mut found_a7 = 0;
        for seed in 700..706u64 {
            let mut hashes = CallHashes::default();
            let mut history = PeriodHistory::default();
            let even = Options { period_key: Some(2), ..Options::depth(2) };
            let first = decode_slot_with_history(&slot(&[("K1ABC W9XYZ EN37", 1200.0, 0.1, -5.0)], seed), &even, &mut hashes, &mut history);
            assert_eq!(first.len(), 1);
            let odd = Options { even_period: false, period_key: Some(3), ..Options::depth(2) };
            decode_slot_with_history(&slot(&[], seed + 1000), &odd, &mut hashes, &mut history);
            let third_audio = slot(&[("K1ABC W9XYZ R-19", 1201.0, 0.12, -24.0)], seed + 2000);
            let plain = decode_slot(&third_audio, &Options::depth(2), &mut CallHashes::default());
            let third = decode_slot_detailed(&third_audio, &Options { period_key: Some(4), ..Options::depth(2) }, &mut hashes, Some(&mut history));
            for d in &third {
                assert_eq!(d.decoded.text, "K1ABC W9XYZ R-19", "falso: {d:?}");
            }
            if third.iter().any(|d| d.ap_type == 7) && plain.is_empty() {
                found_a7 += 1;
            }
        }
        assert!(found_a7 >= 3, "a7 ha trovato {found_a7} su 6");

        // il periodo dopo con solo rumore: nessun decode inventato
        for seed in 800..806u64 {
            let mut hashes = CallHashes::default();
            let mut history = PeriodHistory::default();
            decode_slot_with_history(&slot(&[("K1ABC W9XYZ EN37", 1200.0, 0.1, -5.0), ("CQ IU3ABC JN45", 1700.0, 0.0, -8.0)], seed), &Options::depth(2), &mut hashes, &mut history);
            let out = decode_slot_with_history(&slot(&[], seed + 50), &Options::depth(2), &mut hashes, &mut history);
            assert!(out.is_empty(), "rumore, seme {seed}: {out:?}");
        }
    }

    /// Misure di "a7" e "a8" su audio sintetico (`cargo test --release -p ft8 misure_a7_a8 -- --ignored --nocapture`).
    #[test]
    #[ignore]
    fn misure_a7_a8() {
        for snr in [-21.0f32, -22.0, -23.0, -24.0, -25.0] {
            let (mut plain, mut with, mut false_hits) = (0, 0, 0);
            let t = std::time::Instant::now();
            for seed in 0..20u64 {
                let audio = slot(&[("IU3ABC K1ABC R-17", 1503.0, 0.2, snr)], 9000 + seed);
                plain += decode_slot(&audio, &Options::depth(3), &mut CallHashes::default()).iter().filter(|d| d.text == "IU3ABC K1ABC R-17").count();
                let out = decode_slot(&audio, &Options { a8: Some(a8_query()), ..Options::depth(3) }, &mut CallHashes::default());
                with += out.iter().filter(|d| d.text == "IU3ABC K1ABC R-17").count();
                false_hits += out.iter().filter(|d| d.text != "IU3ABC K1ABC R-17").count();
            }
            println!("a8 SNR {snr}: senza {plain}/20, con {with}/20, falsi {false_hits}, {} ms per periodo", t.elapsed().as_millis() / 20);
        }
        let mut noise_false = 0;
        for seed in 0..50u64 {
            noise_false += decode_slot(&slot(&[], 9500 + seed), &Options { a8: Some(a8_query()), ..Options::depth(3) }, &mut CallHashes::default()).len();
        }
        println!("a8 solo rumore: {noise_false} falsi su 50 periodi");

        for snr in [-21.0f32, -22.0, -23.0, -24.0, -25.0] {
            let (mut plain, mut with, mut false_hits) = (0, 0, 0);
            for seed in 0..20u64 {
                let mut hashes = CallHashes::default();
                let mut history = PeriodHistory::default();
                decode_slot_with_history(&slot(&[("K1ABC W9XYZ EN37", 1200.0, 0.1, -5.0)], 7000 + seed), &Options::depth(2), &mut hashes, &mut history);
                let audio = slot(&[("K1ABC W9XYZ R-19", 1201.0, 0.12, snr)], 7100 + seed);
                plain += decode_slot(&audio, &Options::depth(2), &mut CallHashes::default()).iter().filter(|d| d.text == "K1ABC W9XYZ R-19").count();
                let out = decode_slot_with_history(&audio, &Options::depth(2), &mut hashes, &mut history);
                with += out.iter().filter(|d| d.text == "K1ABC W9XYZ R-19").count();
                false_hits += out.iter().filter(|d| d.text != "K1ABC W9XYZ R-19").count();
            }
            println!("a7 SNR {snr}: senza {plain}/20, con {with}/20, falsi {false_hits}");
        }
        let mut noise_false = 0;
        for seed in 0..50u64 {
            let mut hashes = CallHashes::default();
            let mut history = PeriodHistory::default();
            decode_slot_with_history(&crowded(8000 + seed), &Options::depth(2), &mut hashes, &mut history);
            noise_false += decode_slot_with_history(&slot(&[], 8100 + seed), &Options::depth(2), &mut hashes, &mut history).len();
        }
        println!("a7 solo rumore dopo una banda affollata: {noise_false} falsi su 50 periodi");
    }

    #[test]
    fn ritardo_medio_come_wsjtx() {
        // mediane di tre consecutivi, le ultime due ripetute (`decoder_callbacks.f90:381-419`)
        assert_eq!(timing_summary(&[]), (0, 0.0));
        let (n2, s2) = timing_summary(&[0.4, 0.2]);
        assert!(n2 == 2 && (s2 - 0.6).abs() < 1e-6);
        let (n, s) = timing_summary(&[0.1, 0.3, 0.2, 2.0]);
        // terne (0.1,0.3,0.2) -> 0.2, (0.3,0.2,2.0) -> 0.3, poi 0.3 ripetuto due volte
        assert_eq!(n, 4);
        assert!((s - 1.1).abs() < 1e-6, "{s}");
        assert_eq!(timing_summary(&vec![0.5; 250]).0, 200);

        let mut c = DtCenter::default();
        c.update(&[0.4]);
        assert!((c.value() - 0.05).abs() < 1e-6, "{}", c.value());
        // piu' di 5 messaggi: meta' strada tra il valore di prima e la media
        c.update(&[0.6; 8]);
        assert!((c.value() - 0.325).abs() < 1e-6, "{}", c.value());
        // nessun messaggio: si torna a 0 (`ft8_engine_kernel.f90:451-452` e `:478`)
        c.update(&[]);
        assert_eq!(c.value(), 0.0);
        // dopo un cambio di modo, alla seconda decodifica con piu' di 10 messaggi si prende la media del periodo (`:474`)
        c.mode_changed();
        c.update(&[0.3; 12]);
        assert!((c.value() - 0.15).abs() < 1e-6, "{}", c.value());
        c.update(&[0.3; 12]);
        assert!((c.value() - 0.3).abs() < 1e-6, "{}", c.value());
    }

    #[test]
    fn la_ridecodifica_senza_ricezione_in_banda_non_decodifica() {
        let audio = vec![0.0f32; SLOT_SAMPLES];
        let mut hashes = CallHashes::default();
        let mut o = Options::depth(1);
        o.again = true;
        assert!(decode_slot_detailed(&audio, &o, &mut hashes, None).is_empty());
        o.rx_freq_hz = Some(5000.0);
        assert!(decode_slot_detailed(&audio, &o, &mut hashes, None).is_empty());
    }
}
