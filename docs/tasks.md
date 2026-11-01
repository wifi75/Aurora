Elenco dei lavori presi da WSJT-X 3.x: ogni voce si cancella quando e' integrata in Aurora.

# Lavori da WSJT-X 3.x

Ordinati per priorita'. Analisi completa in `docs/analisi-wsjtx-3.md`. Sorgenti di riferimento: `D:\build\analisi-wsjtx\sorgenti`, tag `v3.3.0-beta5` (GPL-3, compatibile con Aurora GPL-3.0-only: citare la provenienza nei commenti e in `docs/licenze-terze-parti.md`).


## 1. SuperFox: codice QPC (127,50) su GF(128)

- Cosa fare: portare in Rust codifica e decodifica del Q-ary Polar Code e il CRC a 21 bit (`lookup3` `hashlittle`, seme 571, maschera `0x1FFFFF` sui primi 47 simboli).
- Aurora: nuovo crate `core/crates/superfox` (`qpc.rs`, `crc.rs`).
- WSJT-X: `lib/superfox/qpc/np_qpc.c`, `lib/superfox/qpc`, `qpc_fwht.c`, `qpc_subs.c`, `nhash2.c`, `lookup3.c`, `lib/superfox/qpc_decode2.f90` (controllo CRC).
- Verifica: test di andata e ritorno codifica/decodifica su messaggi casuali; CRC confrontato con valori calcolati da `nhash2` della 3.3.0-beta5 compilato a parte.

## 2. SuperFox: ricezione (sincronismo, demodulazione, decodifica)

- Cosa fare: catena `sfox_remove_ft8` -> `sfox_ana` -> `sfox_remove_tone` -> `qpc_sync` (24 simboli di sync, picco in `fsync +- ftol`) -> `sfox_demod` -> `qpc_likelihoods2` -> decodifica con dither (frequenza/tempo, smussamento, perturbazioni con semi fissi) -> scarto sotto -16,5 dB.
- Aurora: `core/crates/superfox` (`sync.rs`, `demod.rs`, `decode.rs`); riuso della sottrazione FT8 di `core/crates/ft8/src/subtract.rs`.
- WSJT-X: `lib/superfox/sfrx_sub.f90`, `sfox_mod.f90`, `qpc_sync.f90`, `sfox_demod.f90`, `qpc_likelihoods2.f90`, `qpc_snr.f90`, `qpc_decode2.f90`, `sfox_remove_ft8.f90`, `sfox_remove_tone.f90`, `sfox_ana.f90`, `twkfreq2.f90`.
- Verifica: file WAV generati con `sfoxsim` (da compilare dai sorgenti) a SNR diversi; Aurora deve decodificare almeno quanto `jt9.exe` 3.0.2 sugli stessi file e con lo stesso SNR, DT e frequenza.

## 3. SuperFox: unpack dei 329 bit e righe di decodifica

- Cosa fare: tipi `i3` 0, 2, 3 (1 si scarta come nella beta5), slot `QU1RKS` ignorati, una riga per Hound (`K1ABC FOX RR73`, `W9XYZ FOX -14`), `CQ FOX` se il bit q1 e' acceso, testo libero da 26 caratteri, riga `$VERIFY$ FOX 123456`; frequenza mostrata `fbest-750`, marcatore `~`.
- Aurora: `core/crates/superfox/src/message.rs`, riuso di c28/g15/t71 da `core/crates/ft8/src/message.rs`.
- WSJT-X: `lib/superfox/sfox_unpack.f90`, `superfox_qpc_2.txt`, `sfox_pack.f90` (per i test).
- Verifica: test con pacchetti costruiti a mano per ogni tipo e confronto con l'uscita di `sfox_unpack`.

## 4. SuperFox nel motore: Hound con SuperFox attivo

- Cosa fare: opzione «SuperFox» accanto a Hound; con tutte e due accese, nel periodo pari e a fine periodo si chiama il decodificatore SuperFox al posto dell'FT8; Rx forzata a 750 Hz (ripristino all'uscita), Rx limitata a 200-1400 Hz, FTol al massimo 100 Hz; niente limite di decodifica a 1000 Hz.
- Aurora: `core/crates/engine-native/src/lib.rs` (scelta del decodificatore e parita'), `engine-native/src/rig.rs` (interruttore `superfox`), `rig-control/src/controls.rs`, `app/src/components/RxControls.tsx`, stringhe in `app/src/i18n`.
- WSJT-X: `lib/ft8_engine_kernel.f90` (righe 232-236), `widgets/mainwindow.cpp` (5262-5271, 5396-5409, 11261-11280, 12180-12189, 13512-13515).
- Verifica: riproduzione di un WAV SuperFox dal motore proprio; le righe compaiono nella pagina Stazione; prova dal vivo del proprietario durante una DXpedition SuperFox.

## 5. Sequenza SuperHound

- Cosa fare: avvio del QSO solo con doppio clic su una riga SuperFox o con «Wait and Call» verso il nominativo del Fox; niente salti di frequenza (`hound_hop` spento), Tx a frequenza libera; il sequenziatore riconosce `MIOCALL FOX -14` (rapporto: si manda R+rapporto) e `MIOCALL FOX RR73` (registra e ferma); il QSO si registra in FT8; tx1/tx3 scritti a mano e griglia DX si svuotano.
- Aurora: `core/crates/qso-native/src/sequencer.rs`, `types.rs` (`Options::superhound`), `history.rs`, `core/crates/engine-native/src/tx.rs`.
- WSJT-X: `widgets/mainwindow.cpp` (`on_dxCallEntry_textChanged` 10414, `on_tx1_editingFinished` 10053, 7714-7735, 9939), `widgets/HoundTransmissionPolicy*`.
- Verifica: test in `core/crates/qso-native/tests/` con una sequenza SuperFox simulata (chiamata, rapporto, RR73, log); prova dal vivo.

## 6. Verifica del Fox con OTP: parte SuperFox

- Fatto (Hound classico): riconoscimento di `CALL.123456`, righe nascoste con «Mostra i messaggi OTP» spento o codice `000000`, richiesta `GET {url}/check/{call}/{ora ISO}/{codice}.text` (5 s) solo con verifica in linea accesa e codice diverso da `000000`, esito nella pagina Stazione e riga «CALL verified/invalid» (`engine-native/src/features.rs`, `online-info/src/foxcheck.rs`, `aurora/src/lib.rs` `wire_fox_verifier`).
- Manca: il formato SuperFox `$VERIFY$ CALL 123456` (hz 750) e il filtro `$VERIFY$` verso il server UDP (`udp-bridge/src/emit.rs`): dipende dal ricevitore SuperFox (voci 1-3); l'etichetta Hound/SuperHound verde dell'interfaccia.
- WSJT-X: `widgets/mainwindow.cpp` `processSFoxVerification` (17360-17415), `handleVerifyMsg`.

## 7. Cascata e spot per SuperFox

- Cosa fare: in SuperHound segnare sulla cascata la banda del SuperFox (circa 1500 Hz attorno a 750 Hz di sync) e la tolleranza; mandare a PSK Reporter gli spot dei decode SuperFox; righe SuperFox in ALL.TXT come WSJT-X.
- Aurora: cascata in `app/src`, `core/crates/reporting/src/psk.rs`, `core/crates/engine-native/src/alltxt.rs`.
- WSJT-X: `widgets/widegraph.cpp`, `widgets/plotter.cpp` (`setSuperFox`, `setSuperHound`), note 2.7.0-rc6.
- Verifica: confronto visivo con WSJT-X sullo stesso WAV; spot visibili su PSK Reporter.

## 10. Decodifica FT8 multithread: misura su banda satura

- Fatto: decodifica multithread (`mtd`) e momento di avvio della decodifica (`FT8DecoderStart`: Presto 13,82 s, Normale 14,11 s, Tardi 14,40 s, come `m_ft8DecoderStart`/`fixStop` di WSJT-X) in `engine-native/src/features.rs` e `slots.rs`.
- Manca: misurare il tempo su un periodo vero di banda satura (non ci sono WAV nel repository); le decodifiche anticipate a due/tre passate di WSJT-X (`m_earlyDecode`, `m_earlyDecode2` a 46-47) non sono portate.

## 12. Wait and Pounce

- Fatto: Wait and Reply e Wait and Call (`engine-native/src/tx.rs` `wait_features`, `features.rs`, pulsante `wait_call`, voce `WaitFeaturesEnabled`).
- Manca: Wait and Pounce (risposta automatica a un CQ secondo il criterio scelto, «Max Dist», «Max dB», «Min dB», con i filtri e senza doppioni): da coordinare con l'Automazione di Aurora e con i filtri dei decode (altro lavoro).
- WSJT-X: `widgets/mainwindow.cpp` `updateRespondTarget`, `AutoRespondPolicy.hpp`, `AutoRespondScoring.hpp`.

## 16. TCI

- Cosa fare: controllo radio e audio via TCI (WebSocket) per radio SDR (Expert, SunSDR, ANAN con Thetis).
- Aurora: `core/crates/rig-control` (nuovo `tci.rs`), `audio-spectrum` (ingresso audio TCI), `tx-native` (uscita audio).
- WSJT-X: `Transceiver/TCITransceiver.cpp/.hpp`.
- Verifica: con un server TCI di prova; dal vivo con una SDR.

## 17. Band Hopping FT8/FT4: rifiniture

- Fatto: `aurora/src/band_hop.rs` (elenco «FT8 20m», «FT4 40m» e fino a 8 frequenze in kHz, salto ogni due minuti, saltato con Abilita Tx acceso o in trasmissione), voci `BandHopping` e `BandHopList` nella scheda Generali.
- Fatto anche: clic destro sulla barra delle bande per la voce successiva (`POST /api/v1/commands/band-hop-next`).
- Manca: MSK144 (modo non presente).
- WSJT-X: `widgets/mainwindow_bandhopping.cpp`.

## 18. Controlli dalla 3.2 su PSK Reporter, ALL.TXT, Wavelog, ADIF (parziale)

- Fatto (2026-11-01, ramo `wsjtx-cat`): pacchetti PSK Reporter entro 952 byte; ora dello spot = periodo del messaggio con la data giusta a mezzanotte; messaggi dei periodi prima di un cambio di banda/modo segnalati con la banda di allora; prova Wavelog/Cloudlog con permesso di scrittura (`api/auth`) ed elenco delle stazioni lette dal server nel messaggio della prova; `ADIF_VER` 3.1.8 (export ed eQSL); funzione `logbook::adif::contest_fields` con le regole di `AdifQso.cpp`.
- Resta: ALL.TXT con la banda del periodo dopo un cambio (`engine-native/src/alltxt.rs`, lasciato a chi lavora sul motore); scambi di contest nei record ADIF (servono i campi dello scambio in `QsoRecord`, riempiti dal motore QSO, poi `contest_fields` nei record).
- WSJT-X: note 3.2 «Networking, logging, and integrations», `logbook/AdifQso.cpp`.
- Verifica: test unitari sui casi di mezzanotte e cambio banda.

## 20. Modo Fox e SuperFox trasmettitore

- Cosa fare: Fox classico (fino a 5 slot, `foxgen`) e SuperFox (pack 329 bit, QPC, GFSK a 750 Hz), coda dei Hound con ordinamento UDP `AnnotationInfo` (tipo 16) e OTP con seme TOTP.
- Aurora: `core/crates/superfox`, `tx-native`, `engine-native/src/tx.rs`, `udp-bridge/src/message.rs`.
- WSJT-X: `lib/superfox/sfox_pack.f90`, `sftx_sub.f90`, `sfox_gen_gfsk.f90`, `sfox_wave_gfsk.f90`, `lib/ft8/foxgen.f90`, `widgets/SuperFoxTxPlanner.cpp`, `FoxGuardBands.cpp`, `FoxLogWindow.cpp`, `mainwindow.cpp` (`foxOTPcode`, `sfox_tx`).
- Verifica: il segnale generato si decodifica con `jt9.exe` 3.0.2 e con il decodificatore di Aurora.
- Interfaccia approvata dal proprietario il 2026-11-01: `docs/mockup-fox.html` (contatori con grafico del ritmo, Hound con doppio ordinamento e filtri, posti del SuperFox con avanzamento e anteprima del pacchetto, parametri visibili, coda, Fox log con ADIF, Cabrillo e ClubLog). Limiti di WSJT-X da superare in `docs/analisi-wsjtx-3.md`.

## 21. Modo Q65

- Cosa fare: aggiungere Q65 (sottomodi A-E, periodi 15/30/60/120/300 s) con codifica, modulazione e decodifica, compresa la media dei messaggi e la decodifica con le ipotesi del QSO (AP).
- Aurora: nuovo crate `core/crates/q65`; scelta del modo in `engine-native`, `qso-native`, `app/src/components/ModeSwitch.tsx`.
- WSJT-X: `lib/qra/q65/` (`q65.c`, `q65.f90`, `q65_decode.f90`, `q65_sync.f90`), `lib/q65params.f90`, `widgets/mainwindow.cpp` (sezioni Q65).
- Verifica: WAV generati con `q65sim` a vari SNR; Aurora decodifica almeno quanto `jt9.exe`.

## 22. Modo MSK144 (meteor scatter)

- Cosa fare: aggiungere MSK144 (periodi 15/30 s, messaggi brevi «Sh»), decodifica dei ping brevi a blocchi nel periodo, sequenza dei QSO del meteor scatter.
- Aurora: nuovo crate `core/crates/msk144`; `engine-native`, `qso-native`, `ModeSwitch.tsx`.
- WSJT-X: `lib/msk144*.f90`, `lib/mskrtd.f90`, `lib/msk144decodeframe.f90`, `widgets/mainwindow.cpp` (MSK144).
- Verifica: WAV con ping simulati (`msk144sim`), confronto con `jt9.exe`.

## 23. Pulsanti rapidi dei modi

- Cosa fare: estendere i pulsanti di scelta del modo della pagina Stazione a Q65 e MSK144 quando ci saranno (oggi FT8, FT4, JT65, JT9).
- Aurora: `app/src/components/ModeSwitch.tsx`, `core/crates/rig-control/src/table.rs` (frequenze per modo).
- WSJT-X: `widgets/mainwindow.ui` (pulsanti dei modi della 3.0).
- Verifica: cambio modo dal pulsante con radio collegata.


## 26. QMAP: ricezione panoramica Q65 fino a 90 kHz

- Cosa fare: dopo il Q65 (voce 21), un pannello che decodifica tutta una fetta di banda larga fino a 90 kHz e con un clic imposta frequenza e parametri del QSO. Serve un ricevitore SDR a banda larga (flusso I/Q), non la sola scheda audio.
- Aurora: nuovo ingresso I/Q in `core/crates/audio-*`, decodifica Q65 su piu' canali, pagina nell'interfaccia.
- WSJT-X: cartella `qmap/`.
- Verifica: registrazione I/Q di prova con piu' segnali Q65.

