# QMAP — ricezione panoramica Q65

QMAP (come nella cartella `qmap/` di WSJT-X 3) ascolta in una volta sola una fetta di banda larga fino a 90 kHz e decodifica tutti i
segnali Q65 che trova. Serve su 6 m, 2 m e nell'EME, dove le stazioni Q65 sono sparse su molte frequenze.

## Chi puo' usarlo

Solo chi ha un ricevitore **SDR che manda il flusso I/Q** via TCI, oppure un file WAV I/Q registrato. Con una radio che ha solo
l'uscita audio (FT-2000, IC-7300, FT-991 e simili) QMAP non ha niente da leggere: per il Q65 normale basta il selettore dei modi.

## Radio adatte

- **SunSDR2** (Pro, DX, QS) e **ColibriNANO** con ExpertSDR2/ExpertSDR3 (server TCI integrato);
- **ANAN** (Apache Labs) e **Hermes-Lite 2** con Thetis (server TCI integrato);
- qualunque altro SDR il cui programma ha un server TCI con il flusso I/Q (`iq_start`);
- in alternativa un **file WAV I/Q** a due canali (16/24/32 bit o float), con il centro nel nome (`..._144100kHz.wav`).

## Come si configura

1. Impostazioni → SDR → «Pagina QMAP nel menu» (spenta di base): compare la voce QMAP.
2. Nel programma della SDR: server TCI acceso (di solito porta 40001), flusso I/Q a 96 kHz, SDR sintonizzata al centro della fetta.
3. Pagina QMAP → «Sorgente I/Q»: «SDR via TCI» con l'indirizzo (per esempio `127.0.0.1:40001`) oppure «File WAV I/Q» con il percorso;
   periodo e sottomodo (per esempio 60 s, A); «Applica».
4. Opzioni: «+30 s» cerca anche i Q65-30 dentro il minuto, «Media» somma i periodi per i segnali deboli («×» la cancella),
   «Evanescenza (EME)» usa il modello dell'evanescenza.
5. «QSO» su un messaggio porta la radio su quel segnale in Q65 con periodo e sottomodo giusti. Per l'EME la riga della Luna dice
   azimut, elevazione e Doppler; la colonna Sked la frequenza da usare.

Codice: `core/crates/qmap` (ricerca, sincronismo, media, Luna), `app/src/pages/QmapPage.tsx`, API `GET/PUT /api/v1/qmap`.
