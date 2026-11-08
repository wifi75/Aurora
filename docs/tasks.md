Elenco dei lavori presi da WSJT-X 3.x: ogni voce si cancella quando e' integrata in Aurora.

# Lavori da WSJT-X 3.x

Ordinati per priorita'. Analisi completa in `docs/analisi-wsjtx-3.md`. Sorgenti di riferimento: `D:\build\analisi-wsjtx\sorgenti`, tag `v3.3.0-beta5` (GPL-3, compatibile con Aurora GPL-3.0-only: citare la provenienza nei commenti e in `docs/licenze-terze-parti.md`).

**UltraFOX (0.11.70)**: le voci SuperFox 1-5 e 7 sono fatte come UltraFOX, compatibile con SuperFox, e sono state tolte; della 6 resta solo la parte qui sotto. Regola: ogni voce finita si toglie da questo file.

## 6. Verifica del Fox con OTP: parte SuperFox

- Fatto (Hound classico): riconoscimento di `CALL.123456`, righe nascoste con «Mostra i messaggi OTP» spento o codice `000000`, richiesta `GET {url}/check/{call}/{ora ISO}/{codice}.text` (5 s) solo con verifica in linea accesa e codice diverso da `000000`, esito nella pagina Stazione e riga «CALL verified/invalid» (`engine-native/src/features.rs`, `online-info/src/foxcheck.rs`, `aurora/src/lib.rs` `wire_fox_verifier`).
- Manca: il formato SuperFox `$VERIFY$ CALL 123456` (hz 750) e il filtro `$VERIFY$` verso il server UDP (`udp-bridge/src/emit.rs`): dipende dal ricevitore SuperFox (voci 1-3); l'etichetta Hound/SuperHound verde dell'interfaccia.
- WSJT-X: `widgets/mainwindow.cpp` `processSFoxVerification` (17360-17415), `handleVerifyMsg`.

## 10. Decodifica FT8 multithread: misura su banda satura

- Fatto: decodifica multithread (`mtd`) e momento di avvio della decodifica (`FT8DecoderStart`: Presto 13,82 s, Normale 14,11 s, Tardi 14,40 s, come `m_ft8DecoderStart`/`fixStop` di WSJT-X) in `engine-native/src/features.rs` e `slots.rs`.
- Fatto (2026-11-08): misura su 5 periodi veri della stazione con 42-46 messaggi (10 m), `ft8/examples/misura_wav.rs`: profondita' 2 in 0,7-0,85 s, profondita' 3 in 1,7 s (8 thread) - 2,8 s (1 thread), 192 messaggi contro 167. Non serve dividere la banda in fette su piu' processori.
- Manca: le decodifiche anticipate a due/tre passate di WSJT-X (`m_earlyDecode`, `m_earlyDecode2` a 46-47): una prima decodifica a circa 12 s sul periodo non finito, poi il completamento a fine periodo.

## 16. TCI

- Cosa fare: controllo radio e audio via TCI (WebSocket) per radio SDR (Expert, SunSDR, ANAN con Thetis).
- Aurora: `core/crates/rig-control` (nuovo `tci.rs`), `audio-spectrum` (ingresso audio TCI), `tx-native` (uscita audio).
- WSJT-X: `Transceiver/TCITransceiver.cpp/.hpp`.
- Verifica: con un server TCI di prova; dal vivo con una SDR.

## 18. Controlli dalla 3.2 su PSK Reporter, ALL.TXT, Wavelog, ADIF (parziale)

- Fatto (2026-11-01, ramo `wsjtx-cat`): pacchetti PSK Reporter entro 952 byte; ora dello spot = periodo del messaggio con la data giusta a mezzanotte; messaggi dei periodi prima di un cambio di banda/modo segnalati con la banda di allora; prova Wavelog/Cloudlog con permesso di scrittura (`api/auth`) ed elenco delle stazioni lette dal server nel messaggio della prova; `ADIF_VER` 3.1.8 (export ed eQSL); funzione `logbook::adif::contest_fields` con le regole di `AdifQso.cpp`.
- Resta: tendina della stazione Wavelog nelle Impostazioni (oggi le stazioni del server compaiono nel messaggio della prova); lo scambio di contest inviato (`exch_sent`) resta vuoto finche' il motore non fa la sequenza dei contest (lo scambio ricevuto e i campi ADIF sono fatti).
- WSJT-X: note 3.2 «Networking, logging, and integrations», `logbook/AdifQso.cpp`.
- Verifica: test unitari sui casi di mezzanotte e cambio banda.

## 20. Modo Fox e SuperFox trasmettitore

- Cosa fare: Fox classico (fino a 5 slot, `foxgen`) e SuperFox (pack 329 bit, QPC, GFSK a 750 Hz), coda dei Hound con ordinamento UDP `AnnotationInfo` (tipo 16) e OTP con seme TOTP.
- Aurora: `core/crates/superfox`, `tx-native`, `engine-native/src/tx.rs`, `udp-bridge/src/message.rs`.
- WSJT-X: `lib/superfox/sfox_pack.f90`, `sftx_sub.f90`, `sfox_gen_gfsk.f90`, `sfox_wave_gfsk.f90`, `lib/ft8/foxgen.f90`, `widgets/SuperFoxTxPlanner.cpp`, `FoxGuardBands.cpp`, `FoxLogWindow.cpp`, `mainwindow.cpp` (`foxOTPcode`, `sfox_tx`).
- Verifica: il segnale generato si decodifica con `jt9.exe` 3.0.2 e con il decodificatore di Aurora.
- Interfaccia approvata dal proprietario il 2026-11-01: `docs/mockup-fox.html` (contatori con grafico del ritmo, Hound con doppio ordinamento e filtri, posti del SuperFox con avanzamento e anteprima del pacchetto, parametri visibili, coda, Fox log con ADIF, Cabrillo e ClubLog). Limiti di WSJT-X da superare in `docs/analisi-wsjtx-3.md`.

## 21. Modo Q65: verifica con WSJT-X

- Fatto (0.11.195 e dopo): Q65 completo nel motore proprio. Periodi 15/30/60/120/300 s e sottomodi A-E in trasmissione e ricezione (`core/crates/jt/src/q65.rs`), ipotesi AP del QSO (CQ, mio nominativo, con il DX, RRR/73/RR73, `q65_ap.f90`), metrica dell'evanescenza rapida con le tabelle gaussiane e lorentziane (`q65fading.rs`, `q65_intrinsics_fastfading`) provata dopo Bessel su sette larghezze B90, media dei periodi della stessa parita'. Periodo e sottomodo scelti accanto al selettore dei modi («Q65-60B»), salvati in `controls.json`; durata dei periodi, avvio e finestra di trasmissione e guardia seguono il Q65 scelto. Pulsanti rapidi dei modi (ex voce 23) completi.
- Resta: il confronto con `jt9.exe` su WAV generati con `q65sim` a vari SNR e Doppler (Aurora deve decodificare almeno quanto WSJT-X); la scelta del modello di evanescenza nell'interfaccia (oggi sempre lorentziano, come WSJT-X) e il pulsante «Clear Avg» (la media si svuota da sola dopo una decodifica e al cambio di modo o periodo).
- WSJT-X: `lib/qra/q65/` (`q65.c`, `q65.f90`, `q65_ap.f90`, `fadengauss.c`, `fadenlorentz.c`), `lib/q65_decode.f90`.
- Verifica: WAV di `q65sim` per ogni periodo e sottomodo; QSO dal vivo in Q65-60A su 6 m o 2 m.

## 26. QMAP: ricezione panoramica Q65 fino a 90 kHz

- Cosa fare: dopo il Q65 (voce 21), un pannello che decodifica tutta una fetta di banda larga fino a 90 kHz e con un clic imposta frequenza e parametri del QSO. Serve un ricevitore SDR a banda larga (flusso I/Q), non la sola scheda audio.
- Aurora: nuovo ingresso I/Q in `core/crates/audio-*`, decodifica Q65 su piu' canali, pagina nell'interfaccia.
- WSJT-X: cartella `qmap/`.
- Verifica: registrazione I/Q di prova con piu' segnali Q65.
