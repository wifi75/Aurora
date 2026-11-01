# Analisi di WSJT-X 3.x per Aurora

Stato al 2026-11-01. Confronto tra le novita' di WSJT-X dalla 2.7.0 alla 3.3.0-beta5 e lo stato di Aurora (0.11.53). L'elenco dei lavori che ne derivano e' in `docs/tasks.md`.

## Materiale e licenza

- Sorgenti: clone di `github.com/WSJTX/wsjtx` in `D:\build\analisi-wsjtx\sorgenti`, tag `v3.0.2` (stabile) e `v3.3.0-beta5` (ultima beta, `release-state.txt`: 3.3.0 BETA 5). Note lette: `NEWS` (2.7.0-rc1 ... 3.0.0-rc1), `Release_Notes.txt` (3.2.0-rc1, novita' dopo la 3.0.2), `doc/user_guide/en/new_features.adoc`, `Network/NetworkMessage.hpp`.
- Binari analizzati: WSJT-X 3.0.2 per Windows, `wsjtx.exe` e `jt9.exe` in `D:\build\analisi-wsjtx\binari`.
- Licenza: WSJT-X e' GPL-3 (`COPYING`); il codificatore/decodificatore Q-ary Polar Code di SuperFox (`lib/superfox/qpc/*.c`) e' "GPL-3 o successiva". Aurora e' `GPL-3.0-only` (`core/Cargo.toml`): la logica si puo' portare in Rust citando la provenienza nei commenti e in `docs/licenze-terze-parti.md`.
- Nota sulla storia git: il tag `v3.3.0-beta5` e' un unico commit ("Cut 3.3.0-beta5 from develop") senza storia comune con `v3.0.2`; le differenze si leggono con `git diff v3.0.2 v3.3.0-beta5`, non con `git log`.

## Verifica con Ghidra (REA)

- `jt9.exe` 3.0.2 (sha256 `5ef6c1ff...`): analisi completa. Funzioni SuperFox presenti con i nomi dei sorgenti: `sfrx_sub_` @ `0x14003c7f0`, `qpc_decode2_` @ `0x140198410`, `qpc_sync_` @ `0x140199bb0`, `qpc_likelihoods2_` @ `0x1401995a0`, `qpc_snr_` @ `0x1401999d0`, `sfox_ana_` @ `0x14019b130`, `sfox_demod_` @ `0x14019b700`, `sfox_remove_ft8_` @ `0x14019d700`, `sfox_remove_tone_` @ `0x1401a1be0`, `sfox_unpack_` @ `0x1401a3680`, `qpc_decode` @ `0x140219300`, `qpc_encode` @ `0x140219e00`, `qpc_fwht8..128`, `nhash2` @ `0x140212e20`, `__sfox_mod_MOD_sfox_init` @ `0x1400abc40`. Decompilato `sfrx_sub_`: la catena e' `sfox_init(7,127,50)` -> conversione `iwave` in reale -> `sfox_remove_ft8` -> `sfox_ana` -> `sfox_remove_tone` -> `qpc_decode2` -> se CRC buono `sfox_unpack`, identica a `lib/superfox/sfrx_sub.f90`. Stringa `$VERIFY$ ` @ `0x1402cb854` (emessa da `sfox_unpack`).
- Decodificatore FT8 di `jt9.exe` 3.0.2: `multimode_decoder_` @ `0x140015c60` con 12 regioni OpenMP (`multimode_decoder_._omp_fn.0..11`), `__ft8_decodevar_MOD_decodevar` @ `0x1400ac2d0` (decodificatore multithread `lib/ft8var`, derivato da JTDX), `__ft8_a7_MOD_ft8_a7d` @ `0x14009ee40`, `__ft8_a7_MOD_ft8_a7_save` @ `0x1400a8df0`, `ft8_a8d_` @ `0x1400a9dc0`. Confermato che il rilasciato contiene MTD, a7 e a8.
- `wsjtx.exe` 3.0.2 (64 MB): l'importazione in Ghidra supera il limite di 330 s; non e' stata decompilata. Verifica fatta sui byte del file: presenti `https://www.9dx.cc` (server OTP predefinito), `/check/` (percorso di verifica di `FoxVerifier`), `OTP:%1` e `OTP:000000` (chiave passata al trasmettitore SuperFox), `$VERIFY$ Error: %1`, `Ftol_SF`, `houndcallers`, `Super Hound`, 29 occorrenze di `SuperFox`, 49 di `TCI`. Assenti `InhibitStatus` e `TxInhibit`: i tipi UDP 17 e 18 sono successivi alla 3.0.2 (arrivano con la 3.2/3.3), come dicono le note.
- Non verificato in binario: binari della 3.3.0-beta5 (non scaricati, i sorgenti bastano), la logica di sequenza Hound/SuperHound in `wsjtx.exe` (si fa fede a `widgets/mainwindow.cpp`).

## 1. SuperFox (priorita' massima)

### 1.1 Forma d'onda e decodifica (lato Hound)

WSJT-X (`lib/superfox/`):
- Codice: Q-ary Polar Code (n,k)=(127,50) su GF(128) (`qpc/lib/superfox/qpc`, `qpc/np_qpc.c`, `qpc/qpc_fwht.c`, `qpc/qpc_subs.c`), simboli da 7 bit. 47 simboli di informazione = 329 bit di carico, gli ultimi 3 simboli sono un CRC da 21 bit: `nhash2(xdec, 47, 571) & 0x1FFFFF` (`qpc_decode2.f90`, `qpc/nhash2.c` = `hashlittle` di `lookup3.c`).
- Forma d'onda (`sfox_mod.f90`, `sfox_gen_gfsk.f90`): 128 toni, 127 simboli dati + 24 di sincronismo = 151 simboli, 1024 campioni per simbolo a 12 kHz (11,72 baud, circa 1500 Hz di banda), durata circa 12,9 s (in Tx `1.0+151*1024/12000`). Simboli di sincronismo nelle posizioni `isync = 1,2,4,7,11,16,22,29,37,39,42,43,45,48,52,57,63,70,78,80,83,84,86,89`; tono di sincronismo a 750 Hz nominali.
- Ricezione (`sfrx_sub.f90` -> `sfrx_decode`): sottrazione dei segnali FT8 presenti (`sfox_remove_ft8.f90`), segnale analitico (`sfox_ana.f90`), rimozione di un tono fisso (`sfox_remove_tone.f90`), sincronismo (`qpc_sync.f90`: picco dello spettro in `fsync +- ftol`, poi correlazione sui 24 simboli di sync, lag +-1,5 s, `snrsync`), demodulazione (`sfox_demod.f90`), verosimiglianze (`qpc_likelihoods2.f90`, EsNo 3,16), decodifica con "dither" (`qpc_decode2.f90`: fino a 100 spostamenti in frequenza/tempo `idf/idt`, 4 pesi di smussamento, fino a 5000 perturbazioni casuali con semi fissi `nseed`), scarto se `snr < -16.5` dB (`qpc_snr.f90`).
- Quando: solo FT8, solo con `b_superfox` (Hound o Fox con SuperFox attivo), solo nel periodo pari e a fine periodo (`nzhsym >= 50`), al posto della decodifica FT8 normale di quel periodo (`lib/ft8_engine_kernel.f90` righe 232-236; in 3.0.2 `lib/decoder.f90`). Con SuperHound il decodificatore multithread non si usa per la passata iniziale (`mainwindow.cpp` 5396, 11280).
- Parametri lato interfaccia (`mainwindow.cpp`): Rx tra 200 e 1400 Hz (`on_RxFreqSpinBox_valueChanged`, 12180), FTol al massimo 100 Hz (`on_sbFtol_valueChanged`, 13512, salvato come `Ftol_SF`), entrando in SuperHound l'Rx va a 750 Hz e il valore precedente viene ripristinato all'uscita.

Aurora: non decodifica SuperFox. Il crate `ft8` non ha nulla di QPC; `engine-native` decodifica ogni periodo solo con FT8. **Manca.**

Da fare: nuovo modulo (es. `core/crates/superfox`) con QPC (porting di `qpc/*.c`), CRC `lookup3`, sincronismo, demodulazione e unpack; chiamata da `engine-native` nel periodo pari quando Hound+SuperFox e' attivo. Verifica con file WAV: `lib/superfox/sfoxsim.f90` genera segnali di prova; l'uscita deve coincidere riga per riga con `jt9 -8 ...` della 3.0.2 sugli stessi file.

### 1.2 Formati dei messaggi SuperFox

WSJT-X (`lib/superfox/superfox_qpc_2.txt`, `sfox_unpack.f90`): tipo `i3` negli ultimi 3 bit (327-329), firma/OTP `d20` (307-326), flag "MoreCQs" `q1` (306).
- `i3=0`: Fox c28 + fino a 9 Hound c28; i primi 5 ricevono `RR73`, gli ultimi 4 un rapporto r5 (valore-18 dB, 31 = RR73).
- `i3=1`: Fox con nominativo composto c58 e 8 Hound: documentato ma non trasmesso; la 3.3.0-beta5 lo scarta (`go to 900`), mentre la 3.0.2 lo decodificava come i3=0.
- `i3=2`: Fox + 4 Hound con rapporti + testo libero di 26 caratteri (due t71).
- `i3=3`: `CQ FoxCall Grid` con Fox c58 + griglia g15 + testo libero opzionale.
- Slot vuoti = valore di `QU1RKS` (203514677), ignorati. Ogni Hound diventa una riga separata: `K1ABC VP8PJ RR73`, `W9XYZ VP8PJ -14`; se `q1=1` si mostra anche `CQ VP8PJ`.
- Righe con marcatore `~` e frequenza `fbest-750` (scostamento dal nominale), come in `emit()`.

Aurora: c28/g15/t71 esistono gia' in `core/crates/ft8/src/message.rs` (pack/unpack 77 bit). **Manca** l'unpack del pacchetto da 329 bit e le righe derivate. Da fare: `unpack_superfox(bits) -> Vec<Riga>` riusando le funzioni c28/t71/g15 di `message.rs`.

### 1.3 Verifica del Fox (OTP)

WSJT-X:
- Dalla 2.7.0-rc7 la vecchia firma digitale e' sostituita da un codice OTP a 6 cifre (TOTP con seme da 16 caratteri, `foxOTPcode()` 14979, `OTPGenerator`). SuperFox lo mette nel campo d20 e il decoder emette `$VERIFY$ CALL 123456`; il Fox classico lo manda ogni `OTPinterval` come testo libero `CALL.123456` (nominativo troncato a 6, `mainwindow.cpp` 15604-15609).
- Lato Hound (`processSFoxVerification`, `mainwindow.cpp` 17360-17415): riconosce `^([A-Z0-9]{2,6})\.([0-9]{6})$` e, solo con SuperFox, `^\$VERIFY\$\s+([A-Z0-9/]{2,13})\s+([0-9]{6})$`; se l'OTP non e' `000000` chiede `GET {OTPUrl}/check/{call}/{ISO-time}/{otp}.text` (`Network/FoxVerifier.cpp`, timeout 5 s, URL predefinito `https://www.9dx.cc`, impostazione `OTPUrl`). Esito mostrato come riga `hhmmss   0  0.0 <hz> ~  CALL verified` (o `invalid`); l'etichetta "Super Hound"/"Hound" diventa verde. Le righe OTP si nascondono se "Show OTP messages" e' spento; `000000` = nessuna verifica (prove senza chiave).
- Le righe `$VERIFY$` non vanno al server UDP (`mainwindow.cpp` 13875).

Aurora: **manca** (vale anche per il Hound classico, che Aurora ha). Da fare in `engine-native` (riconoscimento) + `aurora` (richiesta HTTP, `online.rs` ha gia' un client) + interfaccia (etichetta verde, opzione mostra OTP, URL del server). La richiesta va fatta solo con Hound attivo e consenso dell'utente (nessun dato personale: solo nominativo del Fox, ora e codice).

### 1.4 Sequenza SuperHound

WSJT-X (`mainwindow.cpp`):
- Il Hound trasmette FT8 normale, a qualunque frequenza della banda passante del Fox (anche 0-1000 Hz), senza QSY a frequenza bassa per l'ultima trasmissione; il QSO si registra come FT8.
- Si chiama solo dopo aver decodificato il SuperFox, con doppio clic sulla sua riga (dalla 2.7.0-rc7); dal 3.0 e' permesso "Wait and Call" verso il nominativo del SuperFox anche prima della decodifica (`on_dxCallEntry_textChanged` 10414: con SuperHound un DX Call scritto a mano si cancella, salvo doppio clic o Wait & Call).
- `tx1`/`tx3` modificati a mano si cancellano (10053, 10071); la griglia DX si svuota (9939); passando a FT8 in SuperHound si svuotano le finestre di decodifica (11261-11270).
- Nessun `cbRxAll` e nessun limite di decodifica a 1000 Hz con SuperFox (5268, 5405).
- Fine QSO: alla riga `MIOCALL FOX RR73` il QSO e' completo (stessa regola del Hound classico).

Aurora: Hound classico in `qso-native` (`Options::hound`, `sequencer.rs`, `history.rs autoseq_call`) e salti di frequenza in `engine-native/src/tx.rs` (`hound_hop`, logica di JTDX). **In parte**: manca la variante SuperHound (niente salti, niente limite a 1000 Hz, Rx 750 Hz/FTol 100, avvio solo da decode SuperFox o Wait & Call, righe separate per Hound come messaggi `MIOCALL FOX RR73`/rapporto che il sequenziatore deve capire).

### 1.5 Lato Fox e SuperFox trasmettitore

WSJT-X: `sfox_pack.f90` (329 bit, controllo di nominativi/rapporti/testo prima della Tx dalla 3.2), `sftx_sub.f90`, `sfox_gen_gfsk.f90`, `sfox_wave_gfsk.f90`, `qpc_encode`; coda e pianificazione `widgets/SuperFoxTxPlanner.cpp`, `FoxGuardBands.cpp`, `FoxLogWindow.cpp`; Fox classico `lib/ft8/foxgen.f90` (fino a 5 slot); Tx SuperFox sempre a 750 Hz, split disabilitato; ordinamento dei chiamanti via UDP `AnnotationInfo` (tipo 16) ed evidenziazione `HighlightCallsign`.

Aurora: nessun modo Fox. **Manca**, priorita' bassa (servono chiave NCDXF per l'uso reale).

## 2. Decodifica FT8/FT4

| Voce | WSJT-X | Aurora | Stato |
|---|---|---|---|
| Decodificatore multithread (MTD) | `lib/ft8var/*` (dal decoder di JTDX, `ft8_decodevar.f90`; in 3.2/3.3 riscritti `ft8_coarse_sync.f90`, `ft8_mtd_residual.f90`, `ft8_decode_ranges.f90`, `ft8_coarse_mutex.cpp`), scelta del momento di avvio `m_ft8DecoderStart` (hsymStop 48-50, decodifica anticipata a 46). Ghidra: `__ft8_decodevar_MOD_decodevar`, `multimode_decoder_._omp_fn.*` | `ft8/src/decode.rs` porta la logica di JTDX ma su un solo thread (commento in testa al file) | In parte |
| a7: decodifica con i messaggi del periodo precedente della stessa parita' | `lib/ft8/ft8_a7.f90` (`ft8_a7_save`, `ft8_a7d`), chiamata in `lib/ft8_decode.f90` 329-374 | assente (Aurora ha la cronologia tra periodi di JTDX, `ft8/src/history.rs`, che e' un'altra cosa) | Manca |
| a8: "list decoding" sul corrispondente a Rx +-10 Hz quando DX Call e DX Grid sono noti | `lib/ft8/ft8_a8d.f90`, chiamata in `ft8_decode.f90` 381-397 | assente | Manca |
| Rifiuto prima del PTT dei messaggi non rappresentabili (niente troncamenti silenziosi) | 3.2: `pack77_legacy_truncating_fallback_for_state` in `genft8.f90`/`genft4.f90` e controlli in `mainwindow.cpp` | `ft8/src/message.rs` impacchetta; da verificare che un nominativo non rappresentabile blocchi la Tx | Da verificare |
| Stato del codec rientrante (hash dei nominativi per decoder) | `pack77_state`, `decoder_codec_context`, `lib/decoder_engine*.f90` | `CallHashes` gia' passato per riferimento | Gia' presente |
| FT4 | 3.x: solo rifattorizzazione (stato AP nell'oggetto, `ft4_decode.f90`) | crate `ft4` | Gia' presente |
| Riga Decode UDP: campo Mode e' il marcatore (`~` FT8, `+` FT4) | `NetworkMessage.hpp` | `udp-bridge` | Da verificare |

## 3. Sequenza dei QSO e funzioni operative

| Voce | WSJT-X (file) | Aurora | Stato |
|---|---|---|---|
| Wait and Reply / Wait and Call / Wait and Pounce | `mainwindow.cpp` `processWaitReplyCall`, `WaitFeatureContext` | non c'e' | Manca |
| CQ: First / CQ: Max dB considerano ancora i chiamanti dopo un decode filtrato; chi chiama direttamente sostituisce un CQ/QRZ in attesa | 3.2, `mainwindow.cpp` (policy di risposta) | `qso-native/src/score.rs`, «Best caller first» di Decodium | In parte |
| Tx even/1st bloccato indipendente dal modo | 3.2 | `Options::tx_even` | Gia' presente |
| Full Duplex | 3.0 | no | Manca (bassa) |
| Band Hopping FT8/FT4 (sotto-bande o fino a 8 frequenze) | `widgets/mainwindow_bandhopping.cpp` | `rig-control/src/schedule.rs` (programmazione) | In parte |
| Filtri di visualizzazione, Quick Filter per territori | 3.0, `DecodeOutputPlan` | `aurora-core/src/filters.rs`, `ft8/src/filters.rs` | Gia' presente |
| Avvisi sonori su contenuto | 3.0 | avvisi sonori per categoria | Gia' presente |
| Highlight orange/blue con `*` e `!` | 3.2 | wanted list, evidenziazioni | In parte |
| Message System (QSY Monitor/Creator) | 2.7, `widgets/QSYMessage*.cpp` | no | Manca (bassa) |
| Q65 Pileup, Echo, JTTY, MSK144, FST4/W, Q65 | 2.7-3.3 | Aurora: FT8, FT4, JT65, JT9 | Fuori ambito |

## 4. Protocollo UDP

| Tipo | WSJT-X | Aurora (`udp-bridge/src/message.rs`) | Stato |
|---|---|---|---|
| 0-15 | invariati | tutti dichiarati, letti 0-6 e 12; 50/51 di JTDX | Gia' presente |
| 16 `AnnotationInfo` (In): ordinamento dei Hound in modo Fox | 2.7.0-rc7 | assente | Manca (serve solo con Fox) |
| Status: conteggio evidenziazioni/punteggi | 2.7.0-rc7 (note) | no | Manca (bassa) |
| 17 `InhibitStatus` (Out) e 18 `TxInhibit` (In): blocco Tx multi-operatore con lease 100-30000 ms, max 64 controller, OR delle prenotazioni, solo con PTT DTR/RTS e "Accept UDP requests" | 3.2/3.3, `Network/NetworkMessage.hpp` 549-600, `Network/MessageClient.cpp` | assente | Manca |
| Reply: confronto dei campi del Decode, marcatore `~`/`+` | 3.3 (documentazione) | `udp-bridge/src/request.rs` | Da verificare |
| Replay inviato a lotti | 3.2 | n.a. (Aurora non riceve Replay) | n.a. |
| ADIF in LoggedADIF: `adif_ver` 3.1.8 | 3.3 | `logbook/src/adif.rs` scrive 3.1.4 | Manca (bassa) |

## 5. CAT e radio

| Voce | WSJT-X | Aurora (`rig-control`) | Stato |
|---|---|---|---|
| TCI (controllo e audio) | 3.0, `Transceiver/TCITransceiver.cpp` | Hamlib/rigctld e OmniRig | Manca |
| Potenza e SWR in barra di stato, stop se SWR > 2,5 | 3.0 | `meters.rs`, `extras.rs` leggono livelli | In parte (manca lo stop automatico) |
| Mantenere DATA2/DATA3 scelti dall'operatore | 3.2 | `modes.rs` | Da verificare |
| Rig Split mantiene i ruoli VFO in PTT; split sincronizzato al cambio di offset Tx | 3.2 | split in `tx.rs` | Da verificare |
| Cambi di banda bloccati durante Tx/Tune; comandi PTT pendenti scartati dopo un errore della radio | 3.2 | in parte | Da verificare |
| Rimozione dello spostamento automatico 60 m per nominativi UK | 3.2 | n.a. | n.a. |

## 6. Cascata, log e integrazioni

| Voce | WSJT-X | Aurora | Stato |
|---|---|---|---|
| Ridimensionare la cascata conserva la storia; appiattimento sul campo visibile | 3.2, `widgets/plotter.cpp`, `widegraph.cpp`, `WaterfallScale.hpp` | `audio-spectrum`, cascata WebGPU | Da verificare |
| Cascata SuperFox (marcatori Fox/Hound, `setSuperFox`/`setSuperHound`) | `widegraph.cpp` | no | Manca (insieme a SuperFox) |
| PSK Reporter: pacchetti piu' piccoli, data giusta a mezzanotte, banda del periodo | 3.2 | `reporting/src/psk.rs` | Da verificare |
| Spot SuperFox a PSK Reporter | 2.7.0-rc6 | no | Manca (insieme a SuperFox) |
| Cloudlog/Wavelog: verifica connessione, scelta della stazione dal server | 3.2 | `reporting/src/wavelog.rs` | Da verificare |
| eQSL in HTTPS | 3.2 | gia' HTTPS (`reporting/src/eqsl.rs`) | Gia' presente |
| ADIF con scambi di contest (Field Day, RTTY Roundup, seriali, EU VHF) | 3.2 | `logbook` | Da verificare |
| ALL.TXT con banda e frequenza del periodo anche dopo un cambio | 3.2 | `engine-native/src/alltxt.rs` | Da verificare |
| Cancellazione automatica dei WAV dopo 30 giorni | 3.0 | `engine-native/src/wav.rs` | Da verificare |

## 7. Impostazioni nuove rilevanti

`OTPEnabled`, `OTPSeed`, `OTPinterval`, `OTPUrl`, `ShowOTP`, `superFox` (Avanzate), `Ftol_SF`, `Wait_features_enabled`, decoder FT8 multithread e momento di avvio. In Aurora andranno almeno: SuperFox si/no, Mostra OTP, URL del server OTP, verifica in linea si/no.
