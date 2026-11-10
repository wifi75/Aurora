Elenco dei lavori presi da WSJT-X 3.x: ogni voce si cancella quando e' integrata in Aurora.

# Lavori da WSJT-X 3.x

Ordinati per priorita'. Analisi completa in `docs/analisi-wsjtx-3.md`. Sorgenti di riferimento: `D:\build\analisi-wsjtx\sorgenti`, tag `v3.3.0-beta5` (GPL-3, compatibile con Aurora GPL-3.0-only: citare la provenienza nei commenti e in `docs/licenze-terze-parti.md`).

**UltraFOX (0.11.70)**: le voci SuperFox 1-5 e 7 sono fatte come UltraFOX, compatibile con SuperFox, e sono state tolte, la 6 (verifica OTP del SuperFox) e' fatta. Regola: ogni voce finita si toglie da questo file.

## 16. TCI: verifica dal vivo

- Fatto (2026-11-08, rami `lavoro/tci` e `lavoro/tci2`): backend CAT «TCI» (`rig-control/src/tci.rs`, protocollo letto da WSJT-X 3) con frequenza, modo, PTT, split vero e simulato, misuratori, «Accendi»/«Spegni», audio nei due sensi; ricevitore RX1/RX2 scelto nella scheda Radio (`tci_receiver`, RX2 acceso e rispento come `rx2_enable`); conferma di ogni comando con l'eco come WSJT-X (tempi dei `mysleep`, una ripetizione, poi errore); audio da trasmettere ricampionato se il server non vuole 48 kHz; PWR −/+ e TUNE dei display su `drive` e `tune`. Guida: «SDR con TCI».
- Resta: verifica dal vivo con ExpertSDR3/Thetis (collegamento, RX2, cambio di banda e modo con l'eco, PTT, audio nei due sensi anche con `audio_samplerate` diverso da 48000, misuratori, PWR e TUNE, accensione). Lo split non e' comandato dal motore con nessun CAT (anche `HamlibRig::set_tx_audio_frequency` non ha chiamanti): resta com'e' e nascosto nella scheda Radio. La scelta del VFO A/B non c'e', come in WSJT-X: si ascolta sul VFO A del ricevitore e il VFO B serve allo split.


## 20. Modo Fox e SuperFox trasmettitore

- Fatto: Fox classico (fino a 5 posti affiancati a 60 Hz, `foxgen`; coda dei Hound fino a 10, rapporto -> RR73, tentativi e attese di WSJT-X, un QSO a log per banda) e UltraFOX compatibile con SuperFox (pacchetto da 329 bit, QPC, segnale da 750 Hz) in `engine-native/src/fox.rs`, `ultrafox.rs`, `qpc.rs`, `tx.rs`. Verifica OTP come `foxOTPcode` (`otp.rs`: TOTP HMAC-SHA1 a 6 cifre, passo 30 s, seme base32 di 16 caratteri): nel Fox classico `CALL.123456` ogni N periodi, in UltraFOX nel campo d20 del pacchetto. Zone protette come `FoxGuardBands` (niente Fox a meno di 3 kHz dalle frequenze FT8 standard e nella sottobanda WSPR). UDP `AnnotationInfo` (tipo 16) letto da `udp-bridge` e usato per ordinare i Hound e la coda automatica. Pagina Fox secondo `docs/mockup-fox.html`: numeri con il grafico del ritmo, Hound con doppio ordinamento (SNR, nominativo, locatore, distanza, attesa, Hz, valore esterno) e filtri (SNR, solo nuovi sulla banda), pacchetto in onda con la riga `$VERIFY$`, parametri visibili, seme OTP, Fox log con salvataggio ADIF e Cabrillo. Prove: onde Fox a 5 posti e UltraFOX con OTP decodificate dal decodificatore di Aurora.
- Fatto anche (ramo `lavoro/otp`): seme OTP salvato dal motore in `fox-otp.json` (letto all'avvio, campo `seed` tolto dal pacchetto per l'assistenza); pulsante «Club Log» nel Fox log (`POST /api/v1/fox/clublog`, coda dell'invio diretto); QSO per continente nei numeri (continente da `cty.dat` in `FoxLogView`). Hound: verifica OTP del SuperFox (`$VERIFY$ CALL 123456` a 750 Hz, `parse_superfox_verify`), riga esclusa dall'UDP in `udp-emitter`, etichetta Hound/Super Hound rossa o verde accanto a RX/TX.
- Resta: la prova dal vivo (anche della verifica OTP SuperFox con un Fox vero e del caricamento su Club Log) e il confronto con `jt9.exe` 3.0.2 sui WAV generati.
- WSJT-X: `lib/superfox/sfox_pack.f90`, `sftx_sub.f90`, `sfox_gen_gfsk.f90`, `sfox_wave_gfsk.f90`, `lib/ft8/foxgen.f90`, `widgets/SuperFoxTxPlanner.cpp`, `FoxGuardBands.cpp`, `FoxLogWindow.cpp`, `mainwindow.cpp` (`foxOTPcode`, `sfox_tx`).

## 21. Modo Q65: verifica con WSJT-X

- Fatto (0.11.195 e dopo): Q65 completo nel motore proprio. Periodi 15/30/60/120/300 s e sottomodi A-E in trasmissione e ricezione (`core/crates/jt/src/q65.rs`), ipotesi AP del QSO (CQ, mio nominativo, con il DX, RRR/73/RR73, `q65_ap.f90`), metrica dell'evanescenza rapida con le tabelle gaussiane e lorentziane (`q65fading.rs`, `q65_intrinsics_fastfading`) provata dopo Bessel su sette larghezze B90, media dei periodi della stessa parita'. Periodo e sottomodo scelti accanto al selettore dei modi («Q65-60B»), salvati in `controls.json`; durata dei periodi, avvio e finestra di trasmissione e guardia seguono il Q65 scelto. Pulsanti rapidi dei modi (ex voce 23) completi.
- Fatto anche (2026-11-08): scelta del modello di evanescenza (Lor/Gau) e «Cancella media» accanto a periodo e sottomodo (interruttore `q65_gauss` ricordato in `controls.json`, pulsante `q65_clear_avg`); primo confronto con `jt9.exe` 3.2.0-rc1 su WAV di `q65sim` in `docs/q65-confronto.md` (esempio `core/crates/jt/examples/misura_q65.rs`): pari con Doppler e nei periodi lunghi, indietro di 1-2 dB senza Doppler in 15A/30A/60A, e molto piu' lento.
- Fatto anche (ramo `lavoro/q65b`): decodifica di un periodo rifatta come `q65_decode.f90`/`q65.f90` (spettri a 1/8 di simbolo, `q65_ccf_22` con la soglia statistica dei candidati, frequenza di ricezione provata sempre, `q65_loops` con mezzi bin e sedicesimi di simbolo e le larghezze B90 della profondita' 3, equalizzazione e tetto di `spec64`, `q65_bzap`, `q65_snr`) e la lista dei messaggi del QSO (`q65_set_list`, `q65_ccf_85`, `q65_dec_q3`); il motore passa frequenza di ricezione, locatore del DX e thread. Il vecchio confronto misurava `jt9` con i suoi nominativi di base (K1ABC, W9XYZ, EN37 = il messaggio di prova, cioe' la decodifica da lista): rifatto alla pari, Aurora ora decodifica come `jt9` o meglio in ogni punto, a 0,04-0,9 s per file (`docs/q65-confronto.md`).
- Fatto anche (ramo `lavoro/q65c`): deriva di frequenza di W3SZ (`max_drift` di `q65.f90`: derive da -max a +max in `q65_ccf_22` con la ricerca ristretta a `ntol` dalla frequenza di ricezione, e a 50 la fase 5 di `q65_decode.f90` con la lista sugli spettri raddrizzati `s1w`); ritardo EME (`emedelay`: `lag2` fino a 5,5 s, 4 s nel Q65-15, spettri su tutto il periodo) anche nella ricerca nuova del sincronismo; lista dei contest (`q65_set_list2` con i due valori del 78° bit, chiamanti di `q65_callers.F90` in memoria con scadenza a 24 ore). Interruttori del motore `q65_eme`, `q65_drift` (= `max_drift` 50), `q65_contest`.
- Resta: confronto della media con `jt9`; in contest la lista delle frequenze di sincronismo per Q65-60A (etichetta 900 di `q65_decode.f90`) non portata; prova dal vivo.
- WSJT-X: `lib/qra/q65/` (`q65.c`, `q65.f90`, `q65_ap.f90`, `fadengauss.c`, `fadenlorentz.c`), `lib/q65_decode.f90`.
- Verifica: WAV di `q65sim` per ogni periodo e sottomodo; QSO dal vivo in Q65-60A su 6 m o 2 m.

## 26. QMAP: ricezione panoramica Q65 fino a 90 kHz (parziale)

- Fatto (2026-11-08, rami `lavoro/qmap` e `lavoro/qmap2`): crate `core/crates/qmap`. Sorgente I/Q (`iq.rs`: WAV I/Q a due canali PCM 16/24/32 bit o float, centro letto dal nome `..._144100kHz.wav`; `tci_iq.rs`: collegamento TCI suo, separato da quello del CAT, `iq_samplerate`/`iq_start`/`iq_stop`, centro da `dds:`, blocchi binari di tipo 0 int16/int24/int32/float32). Spettri a mezzo simbolo e candidati (`sync.rs`, da `symspec.f90`, `getcand2.f90`, `q65_sync.f90`): righe ogni 0,15 s a 2,93 Hz, spettro medio normalizzato su 16 tratti, picchi sopra 1,4 dal basso verso l'alto, sincronismo per candidato (22 simboli, 34 ritardi, S/N >= 5). Ricerca (`search.rs`, da `qmapa.f90`, `q65b.f90`): FFT lunga quanto il periodo, ogni candidato riportato a 12 kHz con il tono 0 a 1000 Hz e decodificato da `jt::q65` con il ritardo dell'eco (`eme_delay`, fino a 5,5 s come `emedelay`); nel minuto di 60 s anche i Q65-30 delle due meta' (`bAlso30`, `iseq`, lettera prima), passaggi a 19,5, 30, 49,5 e 58,5 s (`nhsym` 130/200/330/390) con i candidati gia' decodificati saltati e il tempo concesso di `qmapa`; meta' del minuto in trasmissione saltate (`ntx30a/b`); media dei periodi per segnale (periodo, sottomodo, meta', frequenza entro due bin), alimentata anche quando il segnale non e' candidato. Luna e Doppler dell'eco (`astro.rs`, da `astro.f90`, `moondop.f90`, `moon2.f90`): Az/El, «SelfDop» ai MHz interi del centro, frequenza «sked» di ogni messaggio (`fsked`); locatore e nominativo dalle impostazioni della stazione, mai in `qmap.json`. Servizio (`service.rs`): periodo raccolto sull'orologio, passaggi in fila in un thread, configurazione in `qmap.json`. API: `GET/PUT /api/v1/qmap`, `POST /api/v1/qmap/decode` (file), `POST /api/v1/qmap/clear-average`, `POST /api/v1/qmap/tune`. Pagina `#/qmap`: sorgente, periodo, sottomodi, «+30 s», «Media» con il pulsantino per cancellarla, riga della Luna, colonna Sked, segno ×N sui messaggi dalla media; 6 lingue.
- Resta: prototipo grafico da approvare (i controlli nuovi della pagina compresi); click-to-decode su una frequenza scelta (`nagain`, `fselected`, `ntol`); `nfcal` e `offset` di QMAP.
- Verifica dal vivo: con un SDR TCI (ExpertSDR/Thetis) a 96 kHz su 2 m o 6 m, e una registrazione I/Q vera con piu' segnali Q65 (anche Q65-30 nelle due meta' e segnali EME per la media e il Doppler); confronto dei numeri della Luna con la finestra Astro di QMAP.
- WSJT-X: cartella `qmap/` (`libqmap/qmapa.f90`, `getcand2.f90`, `q65_sync.f90`, `q65b.f90`, `symspec.f90`, `astro*.f90`, `moondop.f90`, `moon2.f90`, `mainwindow.cpp`), `Transceiver/TCITransceiver.cpp`.

## 27. Pagina Contest

- Piano completo in `docs/piano-contest.md`. Prototipo approvato (`anteprime/contest-prototipo.html`).
- Fatto (2026-11-08, ramo `lavoro/contest`, fasi 0-6 del piano): crate `core/crates/contest` (regole TOML di 17 contest in `contest/regole` con controllo dei campi, calendario, punti, doppi, moltiplicatori, punteggio, valore di una stazione, rate, ore usate, previsione, log del contest in `<dati>/contest/log-<id>-<anno>.json`, export Cabrillo 3.0/EDI/ADIF con controllo); sequenza di contest in `qso-native::contest` (attivita' di WSJT-X 3.0.1, scambio ricevuto tradotto in rapporto per la sequenza, seriale, Fox/Hound spenti in contest); API `/api/v1/contest/*`; pagina `#/contest` con impostazioni per contest e ritorno a quelle normali; Guida «Contest».
- Fatto (2026-11-09, ramo `lavoro/contest-scambi`): liste ufficiali in `contest/src/liste.rs` (sezioni ARRL/RAC, stati USA, province canadesi e olandesi) per controllare lo scambio ricevuto (nota nel log, niente moltiplicatore se fuori lista); moltiplicatori `dxcc` da cty.dat (FT8/FT4 DX Contest dell'European FT8 Club); EDI con righe `ERROR` per i seriali persi, `PSect` SINGLE/MULTI, `SAnte` e controllo di entrambi.
- Fatto (2026-11-09, ramo `lavoro/contest-consiglio`): consiglio di banda e modo (rate stimato e moltiplicatori nuovi per banda dalle decodifiche, FT4 se il pile-up e' fitto, «Porta la radio su», cambi di banda nell'ora se il regolamento li limita) e mappa dei moltiplicatori per banda (lavorati, nuovi in ascolto, sentiti; elenco per sezioni/paesi; clic = chiama), API `/api/v1/contest/consiglio` e `/api/v1/contest/mappa`.
- Fatto (2026-11-09, ramo `lavoro/contest-allena`): linguette «In gara / Allenamento / Analisi»; analisi dopo la gara dal log del contest attivo o di un'edizione salvata (`/api/v1/contest/analisi`: totali, QSO ora per ora con l'ora migliore e la peggiore, per banda rate migliore e tempo perso, consiglio); simulatore di pile-up solo nella pagina (`app/src/lib/contestSim.ts`: niente motore, radio o log, punteggio a parte).
- Resta:
  - verifica dal vivo di ogni attivita' (soprattutto EU VHF con i due hash, Field Day e Roundup) e del reinvio di RR73;
  - avvio umano vero (oggi c'e' solo il watchdog: un CQ che si ferma dopo N minuti senza intervento e la risposta «con un clic» del piano 5.2);
  - fasi 7-12 del piano: dati di PSK Reporter sulla mappa dei moltiplicatori, coda «chi chiamare dopo» della fase 8, ponte N1MM, simulatore dentro il motore (oggi solo nella pagina, periodi FT8 di 15 s e punteggio semplificato), scheda per il telefono, multi-op in rete;
  - date e valori segnati «da verificare» nei file di regole (FT Challenge, ARRL Jan/Sep VHF, RSGB FT4, PACCdigi, EU VHF).


## 29. FT-991(A) Link (SP4UBW)

- Fatto (0.11.203): nessun errore durante il cambio di banda, DATA-USB rimandato a ogni cambio di banda, frequenze dei 70 cm.
- Resta: prova dal vivo con FTLink 4.16 e lo S-meter `SM0;`.

# Richieste degli utenti (2026-11-09)


# Progetti grossi (decisi dal proprietario, da fare dopo che Aurora e' stabile)

Ognuno e' un progetto a se', da fare a fasi, con un piano scritto e un prototipo grafico da approvare prima del codice.

## P1. Satelliti: orbite, passaggi e correzione Doppler (richiesta di UN7MBV, RS-44)

- Fase 1, solo da guardare: dati delle orbite (TLE da Celestrak, aggiornati ogni giorno), calcolo SGP4 (libreria Rust `sgp4`, licenza MIT da verificare), passaggi dal locatore, pagina Satelliti con elenco dei passaggi, azimut ed elevazione in tempo reale, scelta del satellite.
- Fase 2: correzione Doppler sulla radio con Hamlib (VFO di trasmissione in 2 m e di ricezione in 70 cm, aggiornati ogni secondo; transponder invertente come RS-44), per i satelliti lineari piu' usati.
- Fase 3: full duplex vero per le radio che lo fanno (IC-9700, IC-910, TS-2000, FT-847) e rotore con `rotctld` di Hamlib.
- Prova: serve un tester con radio da satellite (UN7MBV ha un FT-991A: split su due bande, non full duplex).

## P2. Altri modi: RTTY, PSK, CW, SSTV

- Il proprietario ha gia' il materiale da cui riprendere le tabelle per codifica e decodifica in trasmissione e ricezione: prima di cominciare va controllata la licenza di ogni fonte (deve essere compatibile con la GPL-3 di Aurora).
- Ordine proposto: PSK31/PSK63 e RTTY (testo, simili per interfaccia), poi CW (decodifica e trasmissione), poi SSTV (immagini, interfaccia a parte).
- Per ognuno: motore di modulazione e demodulazione in `core/crates`, finestra di ricezione testo (o immagine), macro di trasmissione, log come per FT8, Guida e Novita' nelle sei lingue.

## 28. MSK144 con periodo di 30 s (issue #27, UN7MBV)

- Oggi MSK144 usa solo periodi di 15 s (`engine-native`: `Mode::Msk144 => 15`, `mskrt.rs`, `rig.rs` «TX 00/30»). Da aggiungere la scelta 15/30 s (30 s TX, 30 s RX, molto usata per il meteor scatter in VHF), come per Q65: motore, trasmissione (quadro ripetuto per tutto il periodo), decodifica in tempo reale, selettore accanto al modo.
- Quando e' fatta: commento nella issue #27 (UN7MBV riceve la notifica) e chiusura.

## P3. Rotori d'antenna

- Il proprietario ha gia' tutto il materiale per la gestione dei rotori: prima di cominciare va visto cosa contiene (protocolli, modelli, codice) e controllata la licenza (compatibile con la GPL-3 di Aurora).
- Base proposta: `rotctld` di Hamlib (gia' dentro Aurora per il CAT), che comanda centinaia di rotori; pulsante «Punta» sulla stazione scelta (azimut dal locatore), sulla Mappa DX e sui passaggi dei satelliti (P1, fase 3).
- Da fare come gli altri progetti grossi: piano scritto e prototipo grafico da approvare prima del codice.

## P4. Menu nuovo: i modi in alto (scelto dal proprietario il 2026-11-10)

- Prototipo approvato: `docs/prototipo-menu-modi/index.html` (aprire nel browser; i pulsanti funzionano). Costruito sulla Stazione di oggi.
- In alto, al posto di Stazione · Fox · Contest · Statistiche · Mappa DX · Log · Propagazione · Impostazioni · Guida · Novita': i **modi** (FT8, VHF, CW, RTTY, PSK, SSTV, Satelliti) come pillole con il pallino del colore del modo; quelli non ancora fatti in grigio.
- A destra, con lo stesso aspetto dei pulsanti di oggi (pillola scura con il pallino): le pagine del **modo scelto** (FT8: Fox · Contest · Statistiche; CW/RTTY: Macro · Contest; Satelliti: Doppler · Mappa), pallino del colore del modo, aperta = viola come oggi; poi le pagine comuni **Log · Mappa DX · Propagazione** (pallino blu) e il menu **⚙ ▾** con Impostazioni, Guida e Novita' (il menu resta dentro lo schermo).
- Niente seconda riga. Un clic su una pagina aperta (o sul modo) torna alla Stazione del modo.
- Orario tolto dalla testata; volume, cursore e tema (🌙/☀) nella barra in fondo, dove la barra del periodo diventa piu' corta (circa un terzo).
- Ogni modo ha la sua pagina e il suo motore; DX Cluster, Bande aperte, Comandi, CQ/DXpedition, Log, radio e rotore sono in comune.
- Da fare a passi: prima la testata nuova con FT8 e VHF (le pagine di oggi spostate, nessuna funzione persa, telefono compreso), poi i modi nuovi uno alla volta (P1, P2). Guida, Novita' e i testi nelle sei lingue.
