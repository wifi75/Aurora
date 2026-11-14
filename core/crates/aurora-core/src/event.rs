use serde::{Deserialize, Serialize};

use crate::CoreError;

pub const MAX_SPECTRUM_BINS: usize = 8192;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct StationStatus {
    pub dial_hz: u64,
    pub mode: String,
    pub tx_enabled: bool,
    pub transmitting: bool,
    pub decoding: bool,
    pub de_call: String,
    pub de_grid: String,
    /// Stazione in collegamento (nominativo DX) e suo locatore, se JTDX li riporta.
    #[serde(default)]
    pub dx_call: Option<String>,
    #[serde(default)]
    pub dx_grid: Option<String>,
    /// Rapporto di segnale impostato.
    #[serde(default)]
    pub report: Option<String>,
    /// Frequenze audio di trasmissione e ricezione (Hz).
    #[serde(default)]
    pub tx_df: Option<u32>,
    #[serde(default)]
    pub rx_df: Option<u32>,
    /// Messaggio in trasmissione o pronto.
    #[serde(default)]
    pub tx_message: Option<String>,
    /// Durata del ciclo, in secondi.
    #[serde(default)]
    pub tr_period: Option<u32>,
    /// Continente del proprio nominativo, se `cty.dat` e' caricato.
    #[serde(default)]
    pub own_continent: Option<String>,
    /// Gli strumenti della radio letti dal CAT (S-meter, potenza, ROS, ALC...), se la radio li da'.
    #[serde(default)]
    pub meters: Option<RadioMeters>,
    /// Le stazioni in coda per il prossimo QSO (tasto destro o «Rispondi a tutti»), nell'ordine in cui si risponde.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub queue: Vec<String>,
    /// Gli avvisi di sicurezza della trasmissione in corso (messaggio non rappresentabile, ROS alto, Tx bloccata da un controller esterno).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tx_alerts: Vec<TxAlert>,
    /// Hound: esito della verifica in linea del codice OTP del Fox (`CALL.123456` o `$VERIFY$ CALL 123456`, come `FoxVerifier` di WSJT-X), se c'e' stata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fox_verification: Option<FoxVerification>,
    /// Hound: in questi minuti si riceve un Fox UltraFOX/SuperFox (l'etichetta diventa «Super Hound», come in WSJT-X).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub super_hound: bool,
}

/// L'esito della verifica del Fox: nominativo e se il server ha detto «VERIFIED» (altrimenti «INVALID»).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoxVerification {
    pub call: String,
    pub verified: bool,
}

/// Perche' la trasmissione e' stata fermata o trattenuta.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TxAlertKind {
    /// Un messaggio generato non si impacchetta esattamente nei 77 bit: non e' partito (`detail` = il testo).
    Unrepresentable,
    /// Il ROS letto dalla radio in trasmissione ha superato la soglia: Tx fermata (`value` = il ROS letto, `limit` = la soglia).
    HighSwr,
    /// Un controller UDP (TxInhibit) blocca il PTT (`detail` = chi blocca).
    Inhibited,
    /// Il modo Hound e' stato acceso e Aurora ha sistemato la trasmissione come WSJT-X (`value` = frequenza Tx in Hz, `detail` = `split` se ha
    /// acceso lo Split). Avviso informativo, non ferma niente.
    HoundOn,
    /// Hound automatico: il DX scelto trasmette come Fox di WSJT-X (righe `A RR73; B <DX> -08`) e Hound si e' acceso da solo (`detail` = il DX,
    /// `value` = frequenza Tx in Hz).
    FoxDetected,
    /// Hound spento da solo: il DX risponde con messaggi standard o si e' scelto un altro DX (`detail` = il DX).
    HoundOff,
}

/// Un avviso di sicurezza della trasmissione, da mostrare finche' resta nello stato.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TxAlert {
    pub kind: TxAlertKind,
    #[serde(default)]
    pub detail: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<f32>,
}

/// Gli strumenti della radio, ognuno presente solo se il CAT sa leggerlo (se manca, il campo non compare nel JSON: l'interfaccia non accetta `null`). Le unita' sono quelle del pannello della radio.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RadioMeters {
    /// S-meter in punti S (0-9) e oltre in dB sopra S9 (9 + dB/6 non si usa: sono due numeri distinti): `s_points` 0..=9, `s_over_db` 0..=60.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub s_points: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub s_over_db: Option<f32>,
    /// Potenza in uscita, in watt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub power_w: Option<f32>,
    /// Rapporto d'onda stazionaria (1.0 e oltre).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub swr: Option<f32>,
    /// ALC, 0.0-1.0 della scala.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alc: Option<f32>,
    /// Compressione del processore vocale, in dB.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comp_db: Option<f32>,
    /// Corrente dello stadio finale, in ampere.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id_a: Option<f32>,
    /// Tensione di alimentazione, in volt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vdd_v: Option<f32>,
    /// `true` se `s_points`/`s_over_db` non vengono dalla radio ma sono stimati dal livello dell'audio in ingresso (le altre letture, se ci sono, sono vere).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub estimated: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecodeTag {
    Cq,
    /// Paese mai collegato.
    NewDxcc,
    /// Paese gia' collegato, ma non su questa banda.
    NewDxccBand,
    NewZone,
    NewZoneBand,
    NewItu,
    NewItuBand,
    NewGrid,
    NewGridBand,
    NewPx,
    NewPxBand,
    NewCall,
    NewCallBand,
    /// Stazione gia' collegata su questa banda.
    WorkedCall,
    Auto,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DecodedMessage {
    pub id: u64,
    /// Millisecondi dalla mezzanotte UTC.
    pub time_ms: u32,
    pub snr: i32,
    pub delta_time: f32,
    pub freq_hz: u32,
    pub text: String,
    pub tag: Option<DecodeTag>,
    /// Paese di chi trasmette, se riconosciuto.
    #[serde(default)]
    pub country: Option<String>,
    /// Continente di chi trasmette (AF, AN, AS, EU, NA, OC, SA), se il paese e' noto.
    #[serde(default)]
    pub continent: Option<String>,
    /// Data dell'ultimo invio a LoTW (`AAAA-MM-GG`) di chi trasmette, se compare nell'elenco pubblico dell'ARRL; l'interfaccia decide da quanti mesi conta.
    #[serde(default)]
    pub lotw_last_upload: Option<String>,
    /// Decodifica «ipotizzata» (con informazioni a priori, il marcatore `*` di JTDX): meno sicura delle altre. Non va ai servizi pubblici di spotting e
    /// nel flusso UDP porta il bit di bassa fiducia (`low_confidence`).
    #[serde(default)]
    pub hint: bool,
    /// La novita' di `tag` (nuova griglia o nuovo paese, anche per banda) c'e' solo perche' la griglia o il paese sono gia' stati lavorati ma non
    /// confermati, con la scelta «non confermato = nuovo» accesa: l'interfaccia la colora piu' chiara, come GridTracker (issue #40).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub unconfirmed: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct QsoRecord {
    pub call: String,
    pub grid: String,
    pub band: String,
    pub mode: String,
    pub freq_hz: u64,
    pub rst_sent: String,
    pub rst_rcvd: String,
    pub time_ms: u32,
    /// Inizio del QSO (millisecondi UTC dall'epoca UNIX): `QSO_DATE` e `TIME_ON` del log. 0 = non noto (si usa `time_ms` con la data di oggi).
    #[serde(default)]
    pub time_on_unix_ms: u64,
    /// Fine del QSO come la scrive JTDX (istante dell'invio di RR73/73 + 14 s in FT8, + 7 s in FT4): `QSO_DATE_OFF` e `TIME_OFF`. 0 = non noto.
    #[serde(default)]
    pub time_off_unix_ms: u64,
    /// Nome dell'operatore (campo `NAME` del log; si scrive nella finestra di conferma del log).
    #[serde(default)]
    pub name: String,
    /// Citta' del corrispondente (campo `QTH`), dalla ricerca QRZ.com/HamQTH se accesa; vuota altrimenti.
    #[serde(default)]
    pub qth: String,
    /// Potenza in watt scritta nel QSO (`TX_PWR`); vuota = quella della stazione.
    #[serde(default)]
    pub tx_power: String,
    /// Commento (campo `COMMENT`): rapporti e distanza se `dBtoComments` e `distanceToComments` sono accesi, piu' quello scritto a mano.
    #[serde(default)]
    pub comment: String,
    /// Messaggio per eQSL (`QSLMSG`): serve solo all'invio a eQSL, non entra nel log.
    #[serde(default)]
    pub eqsl_comment: String,
    /// Scambio di contest inviato e ricevuto (`579 WI`, `2A EMA`, `579 0013`), come `xSent`/`xRcvd` di WSJT-X: nel log ADIF diventa
    /// `STX`/`SRX`, `STX_STRING`/`SRX_STRING` o i campi di Field Day e RTTY Roundup (`logbook::adif::contest_fields`). Vuoti fuori dai contest.
    #[serde(default)]
    pub exch_sent: String,
    #[serde(default)]
    pub exch_rcvd: String,
    /// Stato o provincia del corrispondente (`STATE` dell'ADIF), dalla ricerca QRZ.com/HamQTH; vuoto se non si sa.
    #[serde(default)]
    pub state: String,
}

/// Un QSO concluso che aspetta di essere registrato.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PendingQso {
    pub id: u64,
    /// `true`: la finestra di conferma si apre da sola (`PromptToLog`); `false`: il QSO resta in attesa del pulsante «Registra QSO».
    pub prompt: bool,
    /// I campi come li proporrebbe JTDX; chi conferma puo' cambiarli.
    pub record: QsoRecord,
    /// Con eQSL acceso la finestra mostra anche il messaggio per eQSL.
    #[serde(default)]
    pub eqsl: bool,
}

/// Una stazione della lista dei desiderati e' stata vista, in un messaggio decodificato o in uno spot del cluster.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WantedAlert {
    pub call: String,
    /// Perche' e' desiderata: `call`, `prefix`, `country` o `grid`.
    pub reason: String,
    /// Dove e' stata vista: `decode` o `cluster`.
    pub source: String,
    /// Il messaggio decodificato o il commento dello spot.
    pub text: String,
    pub freq_hz: Option<u64>,
    pub band: Option<String>,
    pub snr: Option<i32>,
    pub country: Option<String>,
    pub grid: Option<String>,
    /// Quando (millisecondi Unix).
    pub time_ms: u64,
}

/// E' uscita una versione di Aurora piu' nuova di quella in uso.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UpdateNotice {
    pub version: String,
    pub tag: String,
    pub notes: String,
    pub url: String,
}

/// Giudizio sul livello dell'audio in ingresso.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LevelState {
    /// Ingresso muto da molto: cavo, dispositivo o volume.
    Silent,
    /// Troppo debole per decodificare bene.
    Low,
    Good,
    /// Alto ma non ancora saturo.
    High,
    /// Saturo da qualche secondo: abbassare il volume della scheda o della radio.
    Overdrive,
}

/// Misura del livello dell'audio in ingresso, una ogni quarto di secondo.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct AudioLevel {
    /// Picco assoluto, 0-1.
    pub peak: f32,
    /// Valore efficace, 0-1.
    pub rms: f32,
    /// Quota di campioni a fondo scala, 0-1.
    pub clipped: f32,
    pub state: LevelState,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    Status(StationStatus),
    Decode(DecodedMessage),
    QsoLogged(QsoRecord),
    /// Il motore ha concluso un QSO: non e' ancora nel log. Chi decide se e come registrarlo (`PromptToLog`, `AutoQSOLogging`) lo trasforma in `QsoLogged`.
    QsoCompleted(QsoRecord),
    /// Un QSO aspetta la conferma o la registrazione a mano (finestra di registrazione del log).
    QsoPending(PendingQso),
    /// Il QSO in attesa e' stato registrato o scartato: la finestra si chiude.
    QsoPendingDone { id: u64 },
    TxHalted,
    Wanted(WantedAlert),
    UpdateAvailable(UpdateNotice),
    AudioLevel(AudioLevel),
    /// La banda della radio e' cambiata (da qualunque causa): il motore ha gia' fermato la trasmissione e azzerato la stazione DX;
    /// l'interfaccia svuota attivita' di banda, CQ e novita' e i campi del chiamante. `epoch` cresce a ogni cambio.
    BandChanged { band: Option<String>, dial_hz: u64, epoch: u64 },
    /// Un programma collegato via UDP (Highlight Callsign, tipo 13) chiede di colorare le righe di un nominativo. Colori `#rrggbb`;
    /// entrambi `None` = togli l'evidenziazione. Con `last_only` si colora solo la riga piu' recente con quel nominativo.
    HighlightCallsign { call: String, background: Option<String>, foreground: Option<String>, last_only: bool },
    /// Un programma collegato via UDP (Clear, tipo 3) chiede di svuotare le liste dei messaggi, come «Cancella».
    /// `window`: 0 attivita' di banda, 1 frequenza Rx, 2 entrambe.
    ClearDecodes { window: u8 },
    /// Lo stato della Fox (modo Fox di WSJT-X): chi chiama, la coda, i QSO in corso e i messaggi del periodo.
    Fox(FoxSnapshot),
    /// Un problema da mostrare sullo schermo con una scheda di avviso: `id` dice quale (`audio_loss`, `no_radio`, `omnirig`), `detail` i dettagli.
    Notice { id: String, detail: String },
    /// SSTV (crate `mode-sstv`): righe dell'immagine in arrivo, immagine finita, stato della ricezione e della trasmissione.
    Sstv(SstvEvent),
    /// Un avvenimento dei modi testuali (PSK31/PSK63, RTTY).
    Digi(DigiEvent),
    /// Un periodo WSPR o FST4W ricevuto dal vivo e decodificato (anche senza messaggi): serve solo all'invio facoltativo a WSPRnet, la pagina
    /// non lo riceve.
    WsprCycle(WsprCycle),
}

/// Un periodo WSPR o FST4W decodificato, con quello che WSJT-X manda a WSPRnet (`uploadWSPRSpots`, `widgets/mainwindow.cpp:14353-14377`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WsprCycle {
    /// `WSPR` o `FST4W`.
    pub mode: String,
    /// Durata del periodo T/R in secondi (120 in WSPR; 120, 300, 900 o 1800 in FST4W).
    pub period_s: u32,
    /// Inizio del periodo, millisecondi Unix.
    pub slot_start_unix_ms: u64,
    /// Frequenza della radio durante il periodo, Hz (0 = non nota: niente invio).
    pub dial_hz: u64,
    /// Frequenza audio di trasmissione (la `tqrg` del radiofaro), Hz.
    pub tx_df: u32,
    /// «Tx Pct» se Abilita Tx e' acceso, altrimenti 0 (`autoButton->isChecked () ? sbTxPercent->value () : 0`).
    pub tx_pct: u8,
    /// Potenza dichiarata dal radiofaro, dBm.
    pub dbm: u8,
    pub spots: Vec<WsprSpot>,
}

/// Un messaggio WSPR o FST4W ricevuto.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WsprSpot {
    pub snr: i32,
    pub dt_s: f32,
    /// Frequenza audio del segnale, Hz (in WSPR il centro, in FST4W quella del messaggio come la scrive `jt9`).
    pub audio_hz: f64,
    /// Deriva in Hz (sempre 0 in FST4W, come WSJT-X).
    pub drift_hz: i32,
    pub message: String,
}

/// Un avvenimento dell'SSTV. `kind`: `rx_start` (modo, larghezza, altezza), `rx_rows` (righe nuove da `first_row`, `rows` righe, `rgb`
/// in base64, 3 byte per pixel), `rx_done` (immagine salvata `image_id`), `rx_lost`, `rx_fskid` (nominativo `call` dall'FSK ID, messo
/// all'immagine `image_id`), `rx_status` (livello, sincronismo, ascolto),
/// `tx` (stato e avanzamento della trasmissione), `band` (i segnali di tutta la banda in `signals`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SstvEvent {
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_row: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rows: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rgb: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub progress: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sync: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset_hz: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level_db: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub listening: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Il nominativo letto dall'FSK ID (`rx_fskid`, e `rx_done` se arrivato prima che l'immagine si chiudesse).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub call: Option<String>,
}

/// Un avvenimento dei modi testuali. `kind`: `rx_text` (caratteri nuovi in `text`), `rx_status` (ascolto, livello, modo, frequenza),
/// `tx` (stato e avanzamento della trasmissione), `band` (i segnali di tutta la banda in `signals`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DigiEvent {
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// `rx_text`: le posizioni (in caratteri di `text`) dei caratteri incerti, che la finestra mostra in chiaro (RTTY).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weak: Option<Vec<u32>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub progress: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level_db: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub freq_hz: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub listening: Option<bool>,
    /// Lo squelch lascia passare il segnale (`Sq *` nella barra di stato).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub squelch_open: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// `band`: i segnali di tutta la banda audio (CW, PSK) che si stanno decodificando, al massimo due volte al secondo.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signals: Option<Vec<DigiSignal>>,
    /// `rx_text` di una finestra RX in piu': il suo numero (senza: la ricezione principale).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rx_id: Option<u32>,
}

/// Un segnale di tutta la banda audio (evento `band` dei modi testuali): frequenza, misure, ultimo testo e nominativi sentiti.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DigiSignal {
    pub id: u32,
    pub freq_hz: f32,
    /// CW: la velocita' capita.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wpm: Option<f32>,
    pub snr_db: f32,
    /// La coda del testo ricevuto (nell'evento gli ultimi caratteri; tutto lo storico di 60 s dalla rotta `GET /api/v1/digi/band`).
    pub text: String,
    /// Le posizioni (in caratteri di `text`) dei caratteri incerti, da mostrare in grigio.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unsure: Vec<u32>,
    /// I nominativi sentiti, dal piu' probabile.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub calls: Vec<DigiSignalCall>,
    /// La stazione chiama CQ (o TEST, QRZ).
    pub cq: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub report: Option<String>,
}

/// Un nominativo sentito in un segnale di tutta la banda.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DigiSignalCall {
    pub call: String,
    /// `cq` (chiama), `de` (trasmette) o `other` (nominato da altri).
    pub role: String,
    /// Sentito piu' volte con caratteri sicuri.
    pub verified: bool,
    /// Il paese DXCC (da `cty.dat`), se si sa.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    /// Non lo si sente da oltre 75 s: resta nell'elenco, ma la sua etichetta sulla cascata si toglie.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub stale: bool,
    /// Non c'e' piu': non lo si sente da 3 minuti, o un'altra stazione ha preso il suo posto da oltre 60 s. L'elenco non lo mostra.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub gone: bool,
}

/// Una riga della cascata: viaggia su un canale proprio, non e' un `Event`.
#[derive(Clone, Debug, PartialEq)]
pub struct SpectrumRow {
    pub seq: u64,
    pub f_min_hz: u32,
    pub f_max_hz: u32,
    pub bins: Vec<u8>,
}

impl SpectrumRow {
    pub fn validate(&self) -> Result<(), CoreError> {
        if self.bins.is_empty() {
            return Err(CoreError::InvalidSpectrum("nessun bin"));
        }
        if self.bins.len() > MAX_SPECTRUM_BINS {
            return Err(CoreError::InvalidSpectrum("troppi bin"));
        }
        if self.f_min_hz >= self.f_max_hz {
            return Err(CoreError::InvalidSpectrum("intervallo di frequenza invertito"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_meters_are_left_out_of_the_json_not_null() {
        let m = RadioMeters { s_points: Some(7.0), s_over_db: Some(0.0), ..RadioMeters::default() };
        let json = serde_json::to_value(&m).expect("json");
        assert_eq!(json, serde_json::json!({ "s_points": 7.0, "s_over_db": 0.0 }));
        let back: RadioMeters = serde_json::from_value(json).expect("lettura");
        assert_eq!(back, m);
    }

    #[test]
    fn estimated_flag_is_only_serialised_when_true() {
        let m = RadioMeters { s_points: Some(4.0), estimated: true, ..RadioMeters::default() };
        let json = serde_json::to_value(&m).expect("json");
        assert_eq!(json, serde_json::json!({ "s_points": 4.0, "estimated": true }));
        assert_eq!(serde_json::from_value::<RadioMeters>(json).expect("lettura"), m);
    }
}

/// Un Hound che chiama la Fox, come lo mostra la pagina Fox.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoxCallerView {
    pub call: String,
    pub grid: String,
    pub snr: i32,
    pub freq_hz: u32,
    pub age: u32,
    /// Valore di ordinamento dato da un programma esterno (UDP `AnnotationInfo`), 0 se non c'e'.
    #[serde(default)]
    pub order: u32,
}

/// Un QSO registrato dalla Fox in questa sessione.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoxLogView {
    pub call: String,
    pub grid: String,
    pub band: String,
    pub mode: String,
    /// Frequenza di trasmissione (Hz, sintonia + audio).
    pub freq_hz: u64,
    pub sent: String,
    pub rcvd: String,
    /// Ora del QSO (ms dal 1970, UTC).
    pub time_ms: u64,
    /// Continente della stazione (`cty.dat`: AF, AN, AS, EU, NA, OC, SA), vuoto se ignoto: per la ripartizione nei numeri della pagina Fox.
    #[serde(default)]
    pub continent: String,
}

/// I parametri fissi della Fox (quelli di WSJT-X), mostrati nella pagina.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoxParams {
    /// Tentativi per Hound (`maxStrikes`).
    pub strikes: i64,
    /// Periodi di attesa prima di lasciar perdere (`maxFoxWait`).
    pub wait: i64,
    /// Un CQ almeno ogni tanti periodi con «More CQs» (`foxCQtime`).
    pub cq_every: i64,
    /// Posti in coda (`MAX_HOUNDS_IN_QUEUE`).
    pub queue_max: usize,
}

/// Un QSO della Fox in corso.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoxQsoView {
    pub call: String,
    pub sent: String,
    pub rcvd: Option<String>,
    pub tries: i64,
    pub rr73_sent: bool,
}

/// Lo stato della Fox per l'interfaccia.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoxSnapshot {
    pub enabled: bool,
    pub slots: usize,
    pub more_cqs: bool,
    pub cq: String,
    pub auto_queue: bool,
    /// UltraFOX acceso.
    #[serde(default)]
    pub ultra: bool,
    pub callers: Vec<FoxCallerView>,
    pub queue: Vec<FoxCallerView>,
    pub in_progress: Vec<FoxQsoView>,
    /// I messaggi dell'ultima trasmissione, uno per posto.
    pub tx: Vec<String>,
    pub periods: i64,
    pub logged: u64,
    /// Verifica OTP accesa (seme valido) e ogni quanti periodi il Fox classico la manda.
    #[serde(default)]
    pub otp: bool,
    #[serde(default)]
    pub otp_interval: u32,
    /// L'ultimo codice OTP mandato.
    #[serde(default)]
    pub otp_code: Option<String>,
    /// La sintonia cade in una zona protetta (`FoxGuardBands`): la Fox non trasmette. Testo come «FT8 14074000» o «WSPR 14095600».
    #[serde(default)]
    pub guard: Option<String>,
    /// Il Fox log della sessione (gli ultimi 500, il piu' recente in fondo).
    #[serde(default)]
    pub log: Vec<FoxLogView>,
    #[serde(default)]
    pub params: FoxParams,
}