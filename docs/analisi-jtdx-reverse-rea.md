# JTDX 2.2.159 letto con i sorgenti pubblici e con REA

Analisi del 2026-10-25, in sola lettura. `jtdx.exe` e `jtdxjt9.exe` installati non sono mai stati avviati ne' modificati (REA ne analizza una copia
nella cartella temporanea). Scopo: capire a fondo le **logiche di JTDX** (sequenza dei QSO, scelta di chi chiamare, filtri sui falsi decode,
parametri del decoder) per il crate `qso-native` e per il crate `ft8`, e confrontarle con `docs/logiche-jtdx.md`. Il primo riferimento sono i
sorgenti GPL 3; REA e' servito a verificare che l'eseguibile 2.2.159 coincida con quei sorgenti e a leggere le costanti dove contava.

## In breve

1. **Il sorgente pubblico coincide con l'eseguibile** nei punti controllati con REA. Il ramo `master` di `jtdx-project/jtdx` e' la 2.2.159 finale
   (`Versions.cmake`: 2.2.159, release; ultimo commit `2a0e2be`, 2022-03-01). In `jtdx.exe`: `QsoHistory::autoseq` e `::message` (stesse soglie, stesso
   `switch` sugli stati, `86100` e `300` s), `MainWindow::process_Auto` (bit dei flag `0x80/0x40/0x100/0x10/0x20`, contatori), `guiUpdate` (durate `13,64` e
   `6,04`, soglia `99,0`), `readFromStdout` (bande di guardia `51/84/176/16/67`), `Configuration::read_settings` (valori e limiti dei contatori, watchdog),
   le priorita' di `DisplayText::displayDecodedText`. In `jtdxjt9.exe` (il decoder): `ft8_decode::decode` (`syncmin` 1,5 / 1,225 / 1,1, finestra `+-62`/`+-86`
   passi, `lhighsens` a 1,9 e 3,15). Dove la verifica non c'e' lo dico; le costanti di `ft8b`, `sync8`, `ft8s` e FT4 sono nella sezione 5.4. L'installazione e' del 28/7/2023 mentre l'ultimo commit pubblico e' del 1/3/2022: il binario potrebbe contenere modifiche non pubbliche, per questo i controlli sull'eseguibile.
2. **Il cervello della sequenza non e' `processMessage`** (che e' solo il doppio clic): e' la classe `QsoHistory` (`logbook` di Arvo ES1JA, 17 stati) alimentata da
   `DisplayText::displayDecodedText` (cio' che si riceve) e `displayTransmittedText` (cio' che si trasmette), e letta da `MainWindow::process_Auto`. Tabella
   completa delle transizioni nella sezione 2.
3. **Non esistono i livelli AutoSeq 4 e 5.** I livelli sono **0, 1, 2, 3, 6, 7**: `CallMode` 0-3 (nessuna scelta, prima chiamata, meta' tra una cosa e l'altra,
   fine decodifica) con o senza «Priorita' di chiamata e ricerca CQ» (6 = 2 con ricerca, 7 = 3 con ricerca). La differenza tra i livelli e' **quando** gira
   `process_Auto` (sezione 2.6). `docs/logiche-jtdx.md` sbagliava a descrivere 4 e 5.
4. **Il QSO si registra quando JTDX comincia a trasmettere RR73 o 73, non quando riceve il 73.** Verificato su `ALL.TXT`: «Transmitting ... RR73» alle 04:42:45.048,
   «QSO logged» alle 04:42:45.054 (6 ms dopo). `docs/logiche-jtdx.md` diceva il contrario.
5. **Il limite di avvio della trasmissione e' la durata del messaggio, non 2 s**: una trasmissione puo' partire in qualunque istante dei primi **13,64 s (FT8)** o
   **6,04 s (FT4)** del periodo; l'avvio tardivo trasmette **la coda** della forma d'onda dall'istante giusto (il modulatore salta i campioni gia' trascorsi). E se
   AutoSeq cambia messaggio dopo l'avvio c'e' una **ritrasmissione** («Retransmitting» in `ALL.TXT`: 14 casi nel mese, da 1,2 fino a 13,3 s dall'inizio del periodo in FT8, 4,1 s in FT4).
   Chiude la voce «Da verificare» di `docs/logiche-jtdx.md`. Il valore di `qso-native` (2 s / 1 s) e' troppo corto.
6. **Contatori di rinuncia**: valori di fabbrica 2 / 3 / 5 / 4, tutti spenti; contano le trasmissioni dello stesso messaggio; il contatore «AnswerCQ» non vale per
   le stazioni volute (priorita' 2-4 e 17-19) ne' per i nuovi DXCC (20+); un cooldown `m_counter` impedisce di cercare altri CQ per un po' dopo una rinuncia. Sezione 2.5.
7. **Scelta di chi chiamare**: tre blocchi in ordine (risposte al mio CQ nell'ultimo periodo; risposte al mio CQ nei 5 minuti; CQ degli altri con AutoSeq 6/7), priorita'
   numeriche da 0 a 31, con regole di sorpasso che favoriscono chi mi ha chiamato direttamente. Sezione 3.
8. **I filtri di visualizzazione alimentano l'automazione**: una riga che i filtri nascondono **non entra in `QsoHistory`** (e se «filtra UDP» e' acceso non esce nemmeno
   via UDP). «Bypassa» li annulla tutti. Sezione 3.5.
9. **In FT8 `ndepth` (Rapida/Media/Profonda) non conta nulla**: il decoder FT8 usa solo cicli (3/6/9 passate), sensibilita' (soglie basse, seconda lettura), ricerca
   sul canale del QSO e soglie fisse; `ndepth` e `nft4depth` servono a JT65/JT9/T10 e a FT4. Sezione 5. Il crate `ft8` di Aurora mappa invece «profondita'» su passate,
   OSD e AP: da rivedere.
10. **I sei pallini «Next» sono disattivati con AutoSeq acceso**: `on_AutoSeqButton_clicked(true)` chiama `enableTab1TXRB(false)`. Aurora li comanda: con AutoSeq acceso non rispondono
    (per cambiare messaggio a mano servono i pulsanti «Now» o spegnere AutoSeq).

## Fonti e metodo

| Fonte | Cosa e' | Come e' citata |
|---|---|---|
| `C:\JTDX64\159\bin\jtdx.exe` | PE x86-64, 26 262 483 byte, SHA-256 `5cce71a64f4a73bc7bdba75d9bc5f86887780f0a8b802676e0e3fa772fb7182b`, base `0x400000`, **con i simboli** (nomi C++ mangled: 6 143 procedure) | `REA 0x...` (indirizzo virtuale) |
| `C:\JTDX64\159\bin\jtdxjt9.exe` | il decoder (Fortran), 2 301 598 byte, base `0x400000`, **con i simboli** (`ft8b_`, `sync8_`, `__ft8_decode_MOD_decode`, ...) | `REA9 0x...` |
| Sorgenti `jtdx-project/jtdx`, ramo `master`, commit `2a0e2be` (2022-03-01, 2.2.159) | `mainwindow.cpp` (8 078 righe), `qsohistory.cpp`, `displaytext.cpp`, `decodedtext.cpp`, `Modulator.cpp`, `Configuration.cpp`, `lib/*.f90` (`ft8_decode`, `ft8b`, `sync8`, `chkfalse8`, `chkflscall`, `ft8s`, `ft4_decode`, `osd174_91`...) | `src file:riga` |
| `ALL.TXT` e `JTDX.ini` dell'installazione | prove sul comportamento reale (solo lettura; nessun nominativo riportato) | `ALL.TXT` |
| `docs/logiche-jtdx.md`, `docs/fase0-rapporto.md`, `docs/analisi-decodium-reverse-rea.md` | stato delle conoscenze prima di questa analisi; ricetta REA | - |

**REA/Ghidra**: stessa ricetta di `analisi-decodium-reverse-rea.md` (10 GB di heap, 11 analizzatori spenti, sessione persistente con lo script Node della sezione 1.3 di
quel documento, una richiesta alla volta). Il progetto Ghidra di `jtdx.exe` citato nella consegna **non c'era** piu' nella cartella temporanea (nessuna cartella
`rea-ghidra-*`): e' stato ricreato. Tempi misurati: importazione e analisi di `jtdx.exe` in circa **80 s** (la prima chiamata, `binary_overview`), di `jtdxjt9.exe`
in circa **60 s** (i binari sono molto piu' piccoli di Decodium e la presenza dei simboli aiuta); decompilazione di una funzione normale 1-20 s; `Configuration::read_settings`
(`REA 0x50b240`, funzione enorme) **230 s**. I nomi di funzione sono quelli veri (simboli), quindi qui non servono ipotesi sui nomi. Dopo l'analisi di `jtdx.exe` la sessione e' stata
chiusa e se ne e' aperta una sola per `jtdxjt9.exe` (una analisi pesante alla volta).

Cosa **non** e' stato confrontato nell'eseguibile: le sottoprocedure dei filtri `chk*` e di `ft8s`/`ft8sd`/`ft8mf` oltre a quanto dichiarato nella sezione 5; tutto
`MainWindow` fuori dalle funzioni citate. Per le parti non verificate nell'eseguibile vale il sorgente, che nei punti controllati
coincide.

## 1. Come e' fatto il sistema (chi fa cosa)

| Pezzo | Dove | Cosa fa |
|---|---|---|
| `jtdxjt9.exe` | processo a parte (memoria condivisa, `.lock`) | decodifica un periodo e scrive righe `hhmmss snr dt freq ~ testo` (`~` FT8, `:` FT4) piu' la riga finale `<DecodeFinished> ... <ncand> <avexdt>` |
| `MainWindow::readFromStdout` (`REA 0x494220`) | `src mainwindow.cpp:3686` | per ogni riga: la mostra (`displayDecodedText`), aggiorna Rx, decide se avanzare con `process_Auto`, decide se fermare Tx, manda il decode via UDP (`postDecode`); alla riga finale: guardia di frequenza, `process_Auto`, indicatore di ritardo |
| `DisplayText::displayDecodedText` (`REA 0x4eb170`) | `src displaytext.cpp:222` | **classifica** il messaggio (CQ, a me, tra altri, 73), calcola la **priorita'** (nuovo DXCC...) e, se la riga e' visibile, la passa a `QsoHistory::message` |
| `DisplayText::displayTransmittedText` (`REA 0x4f2810`) | `src displaytext.cpp:946` | classifica cio' che **si trasmette** (Tx1-Tx6 -> SCALL, SREPORT, SRREPORT, SRR/SRR73, S73, SCQ) e lo passa a `QsoHistory::message` |
| `QsoHistory` (`REA 0x4d8260` `autoseq`, `0x4dbbf0` `message`, `0x4d5060` `reset_count`, `0x4dda60` `calllist`, `0x4dd7d0` `owndata`, `0x4dea10` `blacklist`, `0x4dded0` `log_data`, `0x4d5fe0` `rx`) | `src qsohistory.cpp` | una tabella per stazione (chiave: nominativo di base) con stato, contatori, priorita', ultima frequenza; `autoseq` risponde «a che punto siamo con questa stazione» oppure «chi chiamare» |
| `MainWindow::process_Auto` (`REA 0x47d3e0`) | `src mainwindow.cpp:3439` | legge lo stato, applica i contatori di rinuncia, sceglie il prossimo Tx (`txb1..txb6`), sceglie il prossimo DX, ferma Tx a fine QSO |
| `MainWindow::processMessage` (`REA 0x490db0`) | `src mainwindow.cpp:4945` | doppio clic o `Reply` UDP: sceglie periodo e primo messaggio |
| `MainWindow::genStdMsgs` (`REA 0x474800`) | `src mainwindow.cpp:5343` | scrive i sei testi Tx1-Tx6 (stazioni standard o con `<...>`) |
| `MainWindow::guiUpdate` (`REA 0x4c4350`) | `src mainwindow.cpp:4094` | ogni 100 ms: finestra di trasmissione, avvio PTT, generazione dei toni, registrazione al log, watchdog |
| `Modulator` | `src Modulator.cpp` | emette l'audio dalla posizione giusta anche se si parte in ritardo |

`m_QSOProgress` (CALLING, REPLYING, REPORT, ROGER_REPORT, ROGERS, SIGNOFF = 0..5) e `m_nlasttx` (0 = Tx fermato, 1..6) seguono il messaggio scelto e vanno al
decoder (`nQSOProgress`, `nlasttx`): **lo stato del QSO sceglie quali analisi a priori (AP) prova il decoder** (sezione 5.3). Lo stato e il decoder sono accoppiati.

## 2. La sequenza dei QSO

### 2.1 I 17 stati di `QsoHistory`

`Status {NONE, RFIN, RCQ, SCQ, RCALL, SCALL, RREPORT, SREPORT, RRREPORT, SRREPORT, RRR, SRR, RRR73, SRR73, R73, S73, FIN}`, numerati 0..16. Il prefisso `R` e' «ricevuto dal DX», `S` e'
«inviato da noi».

| Stato | Valore | Quando |
|---|---|---|
| NONE | 0 | messaggio fra altre stazioni con locatore (serve solo a ricordare locatore e distanza) |
| RFIN | 1 | altra stazione che ha appena concluso (RR73/73 fra altri, con la spunta «marker RR73»): potrebbe richiamare CQ |
| RCQ | 2 | il DX chiama CQ (o `DE ...`, `QRZ`) |
| SCQ | 3 | abbiamo mandato CQ (Tx6) |
| RCALL | 4 | il DX ci chiama: `MIA DX locatore` o `MIA DX` |
| SCALL | 5 | abbiamo mandato Tx1 (`DX MIA locatore`) |
| RREPORT | 6 | il DX manda il rapporto: `MIA DX -07` |
| SREPORT | 7 | abbiamo mandato Tx2 (`DX MIA -07`) |
| RRREPORT | 8 | il DX manda R+rapporto: `MIA DX R-07` |
| SRREPORT | 9 | abbiamo mandato Tx3 (`DX MIA R-07`) |
| RRR | 10 | il DX manda `RRR` |
| SRR | 11 | abbiamo mandato Tx4 con `RRR` |
| RRR73 | 12 | il DX manda `RR73` |
| SRR73 | 13 | abbiamo mandato Tx4 con `RR73` |
| R73 | 14 | il DX manda `73` |
| S73 | 15 | abbiamo mandato Tx5 (`DX MIA 73`) |
| FIN | 16 | scambio concluso da tutte e due le parti |

### 2.2 Come si classifica un messaggio (cosa riceve `QsoHistory`)

**Ricevuto** (`displayDecodedText`, `src displaytext.cpp:270-432`). `CQersCall` estrae il chiamante da `CQ`, `DE`, `QRZ` (con modificatore: `CQ DX`, `CQ EU`, `CQ 123`; i modificatori
`CQ`, `LP`, `POTA`, `SOTA`, `IOTA` valgono come CQ semplice, `908` e' `JA`, `ASIA` e' `AS`, `USA` e' `K`; un modificatore piu' lungo di due caratteri annulla il CQ):

| Caso | Stato registrato | Parametro |
|---|---|---|
| CQ o `QRZ` (anche con modificatore) | RCQ (`std_type` 1) | locatore |
| `DE <call> ...` **fuori** da +-10 Hz dalla frequenza Rx | nessuno stato (come un messaggio fra altri, `std_type` 3) | - |
| `DE <call> ...` entro 10 Hz dalla frequenza Rx, senza DX scelto, con locatore | RCALL | locatore |
| `DE <call> ...` con DX scelto e stato > SCQ: rapporto / `R`rapporto / `RRR` / `RR73` / `73` | RREPORT, RRREPORT, RRR, RRR73, R73 | rapporto |
| prima parola = mio nominativo di base, con locatore o solo due parole | RCALL | locatore |
| prima parola = mio nominativo, terzo campo `+nn`/`-nn` | RREPORT | rapporto |
| come sopra con `Rnn` | RRREPORT | rapporto |
| prima parola = mio nominativo e ` RRR` / `RR73` / ` 73` | RRR, RRR73, R73 | - |
| prima parola = mio nominativo, altro | RCALL | - |
| messaggio non standard con `73`, DX scelto, mio stato >= RRR | R73 | - |
| messaggio fra altri con `73`/`RR73`, spunta «marker RR73» accesa | RFIN (`std_type` 4) | - |
| messaggio fra altri con locatore (`std_type` 3) | NONE (solo locatore e distanza), e `rx()` se e' del DX scelto | locatore |

Il rapporto passato a `QsoHistory` e' l'SNR della riga (`DecodedText::report`: `+nn` / `-nn`, limitato a -50..+49).
Il **colore** e la **priorita'** si calcolano nello stesso punto (sezione 3.2). **Solo le righe visibili** (`show_line`) arrivano a `QsoHistory::message`; i messaggi diretti a noi (`std_type` 2)
non sono mai nascosti dai filtri.

**Inviato** (`displayTransmittedText`, `src displaytext.cpp:970-1055`, a ogni inizio trasmissione e a ogni ritrasmissione), dal testo del messaggio `P0 P1 P2`:

| Testo | Stato registrato |
|---|---|
| `CQ ...` (anche `CQ DX call locatore`) | SCQ (con il modificatore come `tyyp`) |
| `P2` di 4 caratteri: `RR73` | SRR73 |
| `P2` di 4 caratteri che inizia per `R+`/`R-` | SRREPORT |
| `P2` di 4 caratteri (locatore) | SCALL |
| `P2` di 3 caratteri: `RRR` | SRR |
| `P2` di 3 caratteri `+nn`/`-nn` | SREPORT (con `tyyp` = «S» se «Salta Tx1» e' acceso) |
| `73`, `TNX`, `TKS`, `TU` come ultima parola, o messaggio `DX 73` | S73 |
| due parole: `MIA DX`, senza 73 | SCALL |

### 2.3 Transizioni di `QsoHistory::message` (`REA 0x4dbbf0`, `src qsohistory.cpp:395-736`)

La stazione e' identificata dal **nominativo di base**. Un evento viene **ignorato** se non e' piu' recente di quello registrato, a meno che avanzi lo stato (`time >= t.time` oppure
`time == 0` oppure `stato >= t.status` oppure `stato == RREPORT`).

**Passi comuni**:

1. `t.time = time`; se la tabella non ha ancora un orario di CQ, `_CQ.time = time`.
2. **Evento inviato** (SCALL, SREPORT, SRREPORT, SRR, SRR73, S73): se la stazione era NONE e si invia SCALL, oppure SREPORT con «Salta Tx1» (`tyyp == "S"`), si **finge**
   un CQ ricevuto (`t.status = RCQ`, `srx_c = RCQ`: partenza manuale o per `Reply`). Poi **`count`**: se lo stato inviato e' uguale a quello inviato la volta prima e la frequenza e' la stessa,
   `count += 1`, altrimenti `count = 1`; `_CQ.count = 0`. Se invece e' un evento ricevuto con stato > RCQ e diverso dall'ultimo ricevuto, `count = 0` (una risposta nuova azzera il conteggio).
3. Con RCQ, RCALL o un nominativo piu' lungo (`/P`, prefisso) si adotta il nominativo completo.

**Per stato** (S = stato in tabella, `srx_c`/`srx_p` = ultimo e penultimo stato ricevuto, `stx_c` = ultimo inviato, `r_rep` = rapporto che il DX ci ha mandato, `s_rep` = rapporto che
abbiamo mandato/che manderemmo):

| Evento | Si aggiorna lo stato se | Effetto |
|---|---|---|
| **NONE** | sempre | memorizza locatore e distanza |
| **RFIN** | `S <= SCQ`, o `S == SCALL`, o (`S == SREPORT` e `srx_p < RREPORT`), o (`S == RCALL` e orario diverso), o (`SCQ < S < SRR73` e l'ultimo contatto e' vecchio di piu' di 300 s) | `S = RFIN`, priorita' (la piu' alta), `b_time = time`, `s_rep = SNR`, `rx = freq`; `as_active = true` |
| **RCQ** | stesse condizioni di RFIN | `S = RCQ`, locatore, distanza, `tyyp` (direzione del CQ), continente, prefisso, `b_time`, `s_rep`, `rx`; `as_active = true` |
| **RCALL** | sempre | `S = RCALL`, **priorita' sovrascritta**, locatore, continente, prefisso, `b_time = time`, `s_rep = SNR`, `rx`; `as_active = true` |
| **SCALL** | sempre | `S = SCALL`, `b_time`, `tx = freq` |
| **RREPORT** | `S < FIN` | `s_rep = SNR`; **se `S == SREPORT` diventa RRREPORT** (un rapporto in risposta al nostro rapporto vale R+rapporto), altrimenti RREPORT; `r_rep = rapporto ricevuto`; `b_time` se `S < SCQ` o non abbiamo ancora inviato; `rx`; `as_active` |
| **SREPORT** | `S <= SREPORT` | `s_rep = rapporto inviato`, `S = SREPORT`, `tx`, `b_time` se `S < SCQ` o «Salta Tx1» |
| **RRREPORT** | `S < FIN` **e** `stx_c >= SREPORT` (abbiamo gia' mandato un rapporto) | `S = RRREPORT`, `r_rep`, `rx`; `as_active`. Un R+rapporto senza un nostro rapporto **non avanza** |
| **SRREPORT** | `S <= SRREPORT` | `s_rep`, `S = SRREPORT`, `tx` |
| **RRR** | `r_rep` e `s_rep` non vuoti e `S < FIN` | `S = RRR`, `rx` |
| **SRR** | `S <= SRR` | `S = SRR`, `tx` |
| **RRR73** | `r_rep` e `s_rep` non vuoti e `S >= SREPORT` | se `S == SRR73` (gia' chiuso da noi) o (`srx_p != RRR73` e `S == FIN`): **FIN**, priorita' 0; altrimenti `S = RRR73`; `rx` |
| **SRR73** | sempre | se `S` e' RRR73, R73 o FIN: **FIN** (priorita' 0); altrimenti `S = SRR73`; `tx` |
| **R73** | come RRR73 | se `S` e' SRR73, S73 o FIN: **FIN**; altrimenti `S = R73`; `rx` |
| **S73** | sempre | se `S` e' RRR73, R73 o FIN: **FIN**; altrimenti `S = S73`; `tx` |

Conseguenze: **RR73 o 73 ricevuti senza un rapporto ricevuto (`r_rep`) non fanno niente** (non si chiude un QSO mai scambiato); il **FIN** arriva solo quando le due chiusure si sono incrociate
(la nostra e la sua), in qualunque ordine; `RRR` ricevuto porta a Tx5 (73) e **non** a FIN (serve poi il 73 del DX o la nostra chiusura).

**CQ nostro** (SCQ): `_CQ.call`, `_CQ.tyyp` (il modificatore del nostro CQ), `_CQ.time = _CQ.b_time = time`, `_CQ.tx = freq`, `_CQ.count += 1`.

### 2.4 `process_Auto`: stato -> messaggio (`REA 0x47d3e0`, `src mainwindow.cpp:3439-3684`)

Con un DX scelto, `autoseq(DX)` restituisce lo stato `t.status`, la priorita', `count`, locatore (`t.grid`), rapporto (`t.s_rep`), frequenza Rx (`t.rx`), frequenza Tx (`t.tx`) e `b_time`. `process_Auto`:
imposta la frequenza **Rx** su `t.rx` (JTDX segue il DX se si sposta; la frequenza Tx **non** viene toccata, la riga e' commentata), se il rapporto e' cambiato lo mette in `rptSpinBox`
e **rigenera i sei messaggi** (`genStdMsgs`) — per questo con «Salta Tx1» il Tx2 ripetuto porta ogni volta l'**SNR piu' recente** del DX (se lo stato e' SREPORT lo ritrasforma in RCQ
con «Salta Tx1», in RREPORT senza) — e poi sceglie il messaggio:

| Stato letto | Messaggio da cui si trasmette | Altro |
|---|---|---|
| RFIN, RCQ | Tx2 con «Salta Tx1», altrimenti Tx1 | - |
| RCALL | Tx2 | azzera il watchdog (se Enable Tx, AutoSeq acceso, livello > 0); se «filtro automatico» acceso, filtro di banda |
| RREPORT | Tx3 | azzera il watchdog, filtro automatico |
| RRREPORT | Tx4 | - |
| RRR | Tx5 | - |
| RRR73 | Tx5 (con Hound: registra e ferma) | - |
| R73 | Tx5 | - |
| SRR73 | niente; se non «1 QSO», non Autolog e il DX e' gia' registrato: **ferma Tx** («SRR73, none received») | - |
| S73 | **ferma Tx** («S73, none received») | - |
| FIN | **ferma Tx** (motivo: 1 QSO, Autolog, non registrato, non titolare della frequenza) | con Autolog o `clear_DX` anche libera il DX |

Il pallino `txrb` e' sempre quello scelto da `on_txbN_clicked` (m_ntx = N, m_QSOProgress, m_nlasttx; **se si sta gia' trasmettendo `m_restart = true`**: la trasmissione riparte).

**Cosa cambia con Autolog.** Prima della tabella, se **non** si e' in Hound e **non** c'e' «1 QSO» e **non** c'e' l'Autolog, e lo stato e' SRR73 o >= S73, il DX e' gia' registrato, la frequenza Tx non e' bloccata e
(il DX ha parlato sulla nostra frequenza di CQ, `tx == 1`, o la sua frequenza dista piu' della banda di guardia dalla nostra Tx), il DX viene **liberato** (`clearDX`) e il blocco di scelta sotto scatta
subito: si passa al chiamante successivo o al CQ senza fermarsi. **Con Autolog acceso (come qui, `AutoQSOLogging=true`) questo ramo non esiste**: alla fine del QSO si arriva a S73/FIN e JTDX **ferma la trasmissione**;
chi vuole continuare deve riarmarla (qui lo fa Aurora con `Reply`). Nei `ALL.TXT` del mese dopo il «QSO logged» si vede l'RR73 ripetuto (si aspetta il 73, stato SRR73, nessun arresto) e poi il chiamante successivo; non si distingue da fuori se una ripartenza dopo un arresto sia dovuta a un `Reply` di Aurora (alcune partenze sono tardive di 3-4 s, e ci sono righe «Retransmitting»). Questo ramo e' letto nel sorgente, non provato in esecuzione.

### 2.5 Contatori di rinuncia, `count`, «1 QSO», reply other

Il numero `count` e' quello di `QsoHistory` (sezione 2.3, passo 2): **quante volte di fila** si e' trasmesso lo stesso tipo di messaggio sulla stessa frequenza; se si sta trasmettendo (`m_transmitting`) `process_Auto` lo
riduce di uno. Un contatore con valore N **scatta quando `N <= count`**, cioe' dopo N trasmissioni: **la (N+1)-esima non parte**. Le impostazioni (`Configuration::read_settings`, `REA 0x50b240`,
verificate nell'eseguibile; la riga «predefinito» e' il valore se la chiave manca, «ripiego» quello usato se e' fuori da 1..5):

| Contatore | Chiave | Predefinito | Ripiego | Acceso di base | Stati in cui conta | Cosa fa quando scatta |
|---|---|---|---|---|---|---|
| AnswerCQ | `SeqAnswerCQCounterValue` / `SeqAnswerCQCount` | 2 | 2 | no | RCQ, SCALL, SREPORT con «Salta Tx1» (e non Hound) | libera il DX; imposta `m_counter = N` (cooldown), annota `calllist(DX, SNR, orario)` (non lo richiamera' prima di 5 minuti salvo SNR migliore); poi sceglie il prossimo |
| AnswerInCall | `SeqAnswerInCallCounterValue` / `SeqAnswerInCallCount` | 3 | 2 | no | RCALL, SREPORT senza «Salta Tx1» | libera il DX; `calllist`; **non** fa partire la ricerca CQ subito (`counters2 = false`) |
| SentRReport | `SeqSentRReportCounterValue` / `SeqSentRReportCount` | 5 | 3 | no | RREPORT, SRREPORT | libera il DX; `counters2 = false` |
| SentRR7373 | `SeqSentRR7373CounterValue` / `SeqSentRR7373Count` | 4 | 2 | no | RRR, RRR73, R73, SRR73, S73 | libera il DX; `counters2 = false` |
| `SeqHaltTxReplyOther` | stessa chiave | vero | - | **si'** | vedi sotto | ferma Tx se il DX risponde a un altro |

Qui (`JTDX.ini`): valori 5/5/5/5, AnswerCQ e AnswerInCall **spenti**, SentRReport e SentRR7373 **accesi**, `SeqHaltTxReplyOther=false`.

**Eccezioni e dettagli** (dal sorgente; le soglie e le condizioni di `process_Auto` e `readFromStdout` sono state controllate nell'eseguibile):

- **AnswerCQ non vale per le stazioni volute**: scatta solo se `(priorita' > 4 e < 17) o priorita' < 2` oppure se «direzione CQ rigorosa» (`StrictDirectionalCQ`) e' accesa. Le stazioni con
  priorita' 2-4 (paese, prefisso/locatore, nominativo voluti) e 17-19 (stessi, gia' nuovi) e i nuovi DXCC (20-31) **non vengono mai abbandonati dal contatore**: JTDX continua a chiamarli finche'
  non risponde, il watchdog scade o si ferma a mano.
- **`m_reply_other`**: a ogni inizio decodifica `m_reply_me`, `m_reply_other` e `m_reply_CQ73` si azzerano. Una riga del DX scelto: se contiene il mio nominativo -> `reply_me`; se contiene `73` o inizia per `CQ`
  -> `reply_CQ73`; altrimenti (se non c'e' gia' `reply_CQ73`) -> **`reply_other`** (il DX sta lavorando un altro). In `process_Auto` `reply_other` libera il DX negli stati RCQ/SCALL/SREPORT(Salta Tx1) **solo se il contatore AnswerCQ e'
  acceso** e negli stati RCALL/SREPORT **solo se AnswerInCall e' acceso**. Indipendentemente dai contatori, `readFromStdout` (riga del DX, non a me, senza `73`/`RR73`/`RRR`, non CQ) **ferma Tx** se Enable Tx e' acceso,
  non c'e' `reply_me`, non e' Hound e (la sua frequenza dista meno di `m_nguardfreq` dalla nostra Tx **oppure** `SeqHaltTxReplyOther` e' acceso).
- **Banda di guardia** `m_nguardfreq` (`REA 0x494220`, impostata a ogni `<DecodeFinished>` secondo il modo di Tx): **FT8 51 Hz, FT4 84, JT65 176, JT9 16, T10 67**.
- **CQ diretto ad altri**: in `autoseq(DX)`, se il CQ del DX ha un modificatore di **due** caratteri che non e' il mio continente, ne' i primi due caratteri del **mio nominativo** (`_CQ.call` e' il nominativo con cui
  chiamo CQ), ne' il mio prefisso (eccezione: `DX` vale se il continente del DX non e' il mio), oppure di **un** carattere diverso dal primo del mio nominativo e dal mio prefisso, e (la direzione non e' rigorosa
  oppure priorita' < 20 e stato diverso da RCQ), allora **`count = 1`**: con valore 1 il contatore scatta subito, con valori piu' alti non scatta mai. Letto dal codice; il perche' non e' dichiarato.
- **«1 QSO»** (`m_singleshot`): se un contatore libera il DX, impedisce la scelta del prossimo (`counters = false`) e **ferma Tx** («m_singleshot, counter triggered»); a FIN ferma Tx; in `readFromStdout` fa avanzare subito anche con 73/RR73.
- **Contatore e «Salta Tx1»**: se il DX e' libero per `skipTx1` e il motivo di arresto finisce con «counter triggered», `autoStopTx` cancella la stazione da `QsoHistory` (`remove`): al prossimo tentativo si riparte da capo.
- **Cooldown `m_counter`**: ogni `process_Auto` senza DX (e senza ricerca CQ permessa) lo decrementa; dopo una rinuncia AnswerCQ vale N, quindi per N periodi non si cercano altri CQ (si chiama CQ).

### 2.6 Quando gira `process_Auto`: i livelli AutoSeq

Etichette del pulsante (`REA 0x4ac7f0`, stringhe `AutoSeq0`, `1`, `2`, `6`, `3`, `7`; **non** ci sono 4 e 5): `CallMode` 0 = `AutoSeq0` (menu «Call None»), 1 = `AutoSeq1` («Call First»), 2 = `AutoSeq2`,
3 = `AutoSeq3`; con «Priorita' di chiamata e ricerca CQ» (`CallPriorityCQ`) 2 diventa **6** e 3 diventa **7**. Questa opzione si puo' accendere solo con 2 o 3. Impostazione di fabbrica `CallMode = 3`.

`process_Auto` e' una sola funzione, che parte in momenti diversi:

| Momento | Quando scatta |
|---|---|
| a **fine decodifica** (`<DecodeFinished>`) | sempre, se AutoSeq e' acceso e non e' gia' stata eseguita nel periodo (`m_processAuto_done`, azzerato a ogni inizio decodifica) |
| a **ogni riga del DX scelto indirizzata a noi** | subito (senza aspettare la fine della decodifica) se non e' un `73`/`RR73`; per `73`/`RR73` solo se lo stato in tabella e' RRREPORT con la casella RRR e il messaggio contiene 73, oppure lo stato e' RREPORT e il messaggio contiene RR73, oppure con livello 0, «1 QSO» o Hound; per i livelli 0-1 un `73`/`RR73` mette `m_callFirst73` |
| a ogni riga indirizzata a noi **da un altro** con **nessun DX scelto** | subito, **solo livello 1** (prima chiamata: si prende il primo che ci chiama) |
| all'**inizio della trasmissione** (`guiUpdate`, finestra Tx aperta, PTT non ancora alzato) | livello 2/6: sempre; livelli 0-1: solo se `m_callFirst73` o `m_status >= RRR73` |

Quindi: **livello 0** non sceglie mai chi chiamare (richiede `CallMode != 0` per il blocco di scelta) e avanza subito sui messaggi del DX; **livello 1** sceglie il primo che chiama; **livello 2/6** decide
a inizio periodo di trasmissione con quanto e' stato decodificato **fino a quel momento** (le passate del decoder escono man mano, quindi puo' mancare qualcosa); **livello 3/7** aspetta la **fine della decodifica**: la
trasmissione parte col messaggio vecchio e **riparte** (`m_restart`, «Retransmitting») appena `process_Auto` cambia messaggio. Lo dice anche il suggerimento di JTDX (`REA 0x13c1ee8`): con ritardo di decodifica
0..0,4 s si puo' usare AutoSeq 3/7 («il cambio del messaggio Tx scatta abbastanza presto»), con ritardo > 0,4 s meglio 2/6 perche' la risposta partirebbe parzialmente danneggiata.

Il **livello 6/7** (ricerca CQ): nel blocco di scelta, `CallPriorityCQ` acceso, Tx=Rx non bloccato, nessun cooldown (`m_counter == 0`) e **il prossimo periodo di trasmissione e' quello che segue il periodo decodificato**
(`m_txFirst != (nmod != 0)`, cioe' le stazioni che hanno chiamato sono nel periodo a cui possiamo rispondere) e non e' stato appena escluso un contatore (`counters2`) -> flag `time = 1`, che attiva anche la ricerca fra i CQ altrui. Se non c'e' niente di valido e non si sta
gia' chiamando, `on_txb6_clicked` (CQ). **AutoSeq non cambia il periodo di trasmissione**: lo cambiano solo `processMessage` (doppio clic o `Reply`) e il pulsante `TxMinuteButton`.

### 2.7 Watchdog

`TxWatchdogTimer` (minuti, predefinito 6, ripiego 6 se > 99; 0 = spento). Il contatore `m_idleMinutes` sale di uno a ogni **minuto** UTC (`on_the_minute`) **solo se l'ultima trasmissione e' finita da non piu' di
32 s (FT8), 16 s (FT4) o 134 s (altri)**: i minuti di sola ricezione non contano. Quando `m_idleMinutes >= watchdog` e non si trasmette: `txwatchdog(true)` -> spegne Enable Tx, annulla `m_bTxTime`, etichetta «Tx watchdog expired».
**Si azzera** (`txwatchdog(false)`): a ogni doppio clic; in `process_Auto` negli stati RCALL e RREPORT se Enable Tx, AutoSeq acceso e livello > 0; a ogni inizio trasmissione in cui il **testo trasmesso e' diverso dall'ultimo**
(`m_curMsgTx != m_msgSent0`). Quindi ripetere lo stesso messaggio non azzera, un messaggio nuovo si'.

### 2.8 Chiusura, registrazione al log

In `guiUpdate`, a ogni nuovo messaggio trasmesso: `is_73 = (m_QSOProgress >= ROGER_REPORT o Tx4/Tx5) e il testo contiene "73" o "RR73"` (in FT8 `m_currentMessageType` e' sempre 0; **`RRR` non conta**).
`m_sentFirst73 = is_73 e l'ultimo DX registrato e' diverso e (il messaggio precedente non era un 73 o non era a questo DX)`. Se `m_sentFirst73` e (`PromptToLog` o `AutoQSOLogging`) e non e' una prova di accordo:
**registra subito** (`logQSOTimer`), salvo che sia gia' stato registrato lo stesso nominativo meno di `2 * periodo` secondi fa (guardia contro i doppioni; 30 s in FT8). L'ora di fine QSO e' `ora + 14 s` (FT8), `+ 7 s` (FT4).
Con la casella RRR (Tx4 = `RRR`) il QSO si registra al Tx5 (73), non al Tx4. Il log `wsjtx_log.adi` cambia e `acceptQSO2` manda `QsoLogged` e `LoggedADIF` via UDP (vedi `docs/fase0-rapporto.md`).

### 2.9 La finestra di trasmissione e l'avvio tardivo (`guiUpdate`, `REA 0x4c4350`; `src Modulator.cpp:60-100`)

- Durata del messaggio usata: **FT8 13,64 s** (`1 + 79 * 1920 / 12000`), **FT4 6,04 s**, JT65 47,81 s, JT9 e T10 49,96 s. La finestra e' `[inizio, inizio + durata)` del periodo di Tx (nel periodo dispari se `m_txFirst` e' falso,
  spostata di un periodo). `m_bTxTime` e' vero in quella finestra.
- Il test `fTR < 99,0` (con `fTR` frazione del periodo, da 0 a 1) e' **sempre vero**: commento del codice «Allow late starts». **Una trasmissione parte in qualunque momento della finestra**, se Enable Tx e' acceso e (`g_iptt == 0`).
- Il `Modulator` calcola `mstr` = millisecondi dall'inizio nominale del periodo; ritardo di avvio nominale **500 ms** (FT8 e FT4; 1000 ms negli altri modi); se `mstr <= 500` parte da `m_ic = 0` con `(500 - mstr)` ms di silenzio, altrimenti
  parte da `m_ic = (mstr - 500) * 48` campioni (48 kHz): **trasmette la coda del messaggio** da dove sarebbe, non da capo. In FT8 un avvio oltre ~1,6 s dall'inizio del periodo (0,5 s di ritardo nominale piu' i 7 simboli, 1,12 s, del primo gruppo di Costas) fa perdere al ricevente il primo gruppo di sincronismo (restano gli altri due).
- `m_restart` (da `on_txbN_clicked` durante la trasmissione, da `processMessage`, da `process_Auto`): rigenera i toni e **ricomincia**, sempre con lo stesso calcolo (riga `Retransmitting` in `ALL.TXT`).
- Dati reali (`ALL.TXT`, ottobre, FT8): 265 trasmissioni nei primi 0-1 s, poi fino a 12 s (2, 2, 3, 5, 1, 2, 6, 1, 1, 2, 3, 1 per ciascun secondo da 1 a 12): coerente con la finestra di 13,64 s.
  Righe `Retransmitting`: 14 nel mese, in FT8 a 1,2 / 2,8 / 2,9 / 3,5 / 4,5 / 6,9 / 7,3 / 8,3 / 8,8 / 10,9 / 11,7 / 11,9 / 13,3 s dall'inizio del periodo, in FT4 a 4,1 s.
- Mentre si trasmette il decoder tiene conto (`ltxing`: Enable Tx acceso e trasmissione nei 26 s precedenti: soglie di sincronismo piu' alte, sezione 5.3).

### 2.10 Altre regole utili

- **Doppio clic / `Reply`** (`processMessage`, `REA 0x490db0`): il periodo di Tx diventa **opposto** a quello della riga cliccata (`nmod = secondi % (2 * periodo)`; `nmod != 0` -> `m_txFirst = true`); cambiarlo con una trasmissione in
  corso la ferma. Poi: se `m_lockTxFreq` o Ctrl, frequenza Tx = quella della riga; frequenza Rx = quella della riga; se il nominativo e' diverso dal DX scelto si azzera il locatore, `reset_count(nuovo)`, nuovo `dxCall`; rapporto = SNR della riga;
  `genStdMsgs`; poi il messaggio: se la riga contiene **il mio nominativo**: terzo campo non locatore e `RRR`/`RR73`/`73` -> **Tx5**; `R...` -> **Tx4**; numero -50..49 -> **Tx3**; altro (locatore) -> **Tx2**; se non contiene il mio nominativo (un CQ): Tx2 con «Salta Tx1», altrimenti Tx1.
  Con «AutoTX» (`m_autoTx`) e senza Alt accende Enable Tx (con Alt: lo spegne, `haltTx`).
- **«Salta Tx1»** (`m_skipTx1`): se il proprio nominativo non e' standard (`!m_bMyCallStd`) viene disattivato (`toggle_skipTx1`); in Hound e' spento; cambia Tx1 e Tx2 nella partenza e il rapporto del Tx2 si aggiorna a ogni periodo (2.4).
- **Casella RRR**: cambia solo il testo del Tx4 (`RRR`/`RR73`) e quando si registra (2.8).
- **Tx5 e AutoSeq**: se «Tx5 spegne AutoSeq» (`m_Tx5setAutoSeqOff`) e Tx5 e' il messaggio in uso, AutoSeq si spegne e si riaccende alla generazione dei messaggi successiva (`m_wasAutoSeq`).
- **Tasto destro su «Canc DX»**: senza Ctrl cancella la stazione da `QsoHistory` (`remove`) e libera il DX; con Ctrl la mette in **lista nera** (`blacklist`: non verra' piu' scelta).
- **`TxMinuteButton`**: testo «TX 00/30» (acceso) e «TX 15/45» (spento) in FT8; in FT4 «TX 00» (acceso) e «**TX 7.5**» (spento) (il documento precedente scriveva solo «TX 00»); in modi a minuto «TX Even»/«TX Odd»; in WSPR «N/A».
  Se Enable Tx e AutoSeq sono accesi e il periodo cambia, il DX viene liberato.
- **I messaggi standard** (`genStdMsgs`, `REA 0x474800`): per stazioni standard `DX MIA locatore4`, `DX MIA -07`, `DX MIA R-07`, `DX MIA RR73|RRR`, `DX MIA 73`, `CQ [dir] MIA locatore4`; con un nominativo non standard
  o composto compaiono i `<...>` (`<DX> MIA`, `DX <MIA> ...`) come nel sorgente; il rapporto e' `%+2.2d` dell'SNR. `qso-native` (`stdmsg.rs`) li ha gia' scritti.
- **Hound** (modo DXpedition): ramo a parte (rapporto R+ tre volte, salti di frequenza); non descritto qui.

## 3. Come sceglie chi chiamare

### 3.1 Il periodo di trasmissione

- **Solo `processMessage` e `TxMinuteButton` lo cambiano** (sezione 2.10). La regola: si trasmette nel periodo **opposto** a quello della riga cliccata o del `Reply`. `nmod = secondi % (2 * periodo)`; se `nmod != 0`
  (messaggio ricevuto a :15/:45 in FT8) allora `m_txFirst = true` (TX 00/30), se `nmod == 0` allora `m_txFirst = false` (TX 15/45). Non si cambia se la riga e' una nostra trasmissione (`Tx`).
- **AutoSeq non lo cambia.** Con AutoSeq 6/7 la ricerca nei CQ degli altri e' permessa **solo** quando il prossimo periodo di trasmissione e' quello che segue il periodo decodificato (sezione 2.6): JTDX non va a
  cercare CQ in un periodo che non potrebbe rispondere.
- Un periodo diverso da quello con cui sono stati generati i messaggi (`m_txGenerated`) con Enable Tx e AutoSeq accesi libera il DX.

### 3.2 Le priorita' (`displayDecodedText`, `REA 0x4eb170`, `src displaytext.cpp:433-840`)

Ogni riga con un nominativo riceve una **priorita' intera**. Parte da 0; **1** se la stazione e' un utente LoTW (`logBook.getLOTW`) [valore base per LoTW, poi sostituito dai livelli sotto]. Poi si controlla **in quest'ordine, il primo che vale
vince** (solo per i criteri che l'utente ha acceso nelle evidenziazioni; «band» vale per «nuovo su questa banda» e, con l'opzione, «banda + modo»); il valore piu' alto e' per LoTW:

| Criterio (nuovo...) | Senza LoTW | Con LoTW |
|---|---|---|
| zona CQ | 30 | 31 |
| zona CQ su questa banda | 28 | 29 |
| zona ITU | 26 | 27 |
| zona ITU su questa banda | 24 | 25 |
| DXCC | 22 | 23 |
| DXCC su questa banda | 20 | 21 |
| locatore (4 caratteri) | 15 | 16 |
| locatore su questa banda | 13 | 14 |
| prefisso | 11 | 12 |
| prefisso su questa banda | 9 | 10 |
| nominativo | 7 | 8 |
| nominativo su questa banda | 5 | 6 |

Poi le liste dei **desiderati** (nominativo di base o completo, prefisso, locatore a 4 caratteri, paese = prefisso principale): se la priorita' e' **< 5**: nominativo voluto -> **4**, prefisso o locatore voluto -> **3**, paese voluto -> **2**;
se invece e' **< 20** (cioe' nuovo di rango 5-19): nominativo -> **19**, prefisso/locatore -> **18**, paese -> **17**. I «nuovi» da 20 a 31 non vengono sostituiti dai desiderati.
Stringhe di debug in `ALL.TXT` (con «scrivi debug»): priorita' > 27 «New CQZ», > 23 «New ITUZ», > 19 «New DXCC», 19 o 4 «Wanted Call», 18 o 3 «Wanted Prefix», 17 o 2 «Wanted Country»; stato > RREPORT: «Resume interrupted QSO».
Nota: la priorita' vale per le righe di tipo CQ, a me o di chiusura; per le righe fra altri (`std_type 3`) solo se «mostra potenziali» e' acceso. Il **continente** e il **paese** vengono da `cty.dat` (`logBook.getDXCC`); senza continente noto la stazione non
viene mai scelta. Verificato nell'eseguibile (`REA 0x4eb170`): 20/21 (DXCC su banda), 22/23 (DXCC), 26/27 (zona ITU), 28/29 e 30/31 (zona CQ); gli altri valori (24/25 e i livelli bassi) non sono stati riletti uno a uno.

### 3.3 `QsoHistory::autoseq` senza nominativo: la scelta (`REA 0x4d8260`, `src qsohistory.cpp:165-326`)

Chiamata da `process_Auto` quando non c'e' un DX scelto, con i flag `time` (3.4) e **solo se dall'ultima volta e' arrivato un messaggio utile** (`as_active`: RFIN, RCQ, RCALL, RREPORT o RRREPORT). Tre blocchi, in ordine; il primo che sceglie qualcuno ferma la ricerca.
Una stazione e' **candidata** se non e' in lista nera, ha un continente noto e (`tt.time` come indicato) ha lo stato richiesto.

**Blocco A: risposte al mio CQ nell'ultimo periodo.** `tt.time == max_r_time` (ha parlato nell'ultimo periodo decodificato); stato RCALL, RREPORT, RRREPORT, RRR o RRR73; **oppure** (RCQ o RFIN) con priorita' 17-19 (desiderato) e nessuno scelto che mi abbia chiamato.
**Blocco B: risposte nei 5 minuti.** Come A, ma `tt.time` entro 300 s da `_CQ.time` (orario del mio ultimo CQ; l'aritmetica e' senza segno, quindi chi ha parlato prima del mio CQ non conta).
**Blocco C: i CQ degli altri** (solo se `algo & 1`, cioe' AutoSeq 6/7 con le condizioni di 2.6). `tt.time == max_r_time`, stato RCQ (o RFIN con priorita' > 0), e **non gia' chiamato di recente**: la stazione non e' in `calllist` oppure il suo SNR attuale e' maggiore di quello
al momento della rinuncia, oppure sono passati piu' di 300 s dal suo ultimo contatto (`calllist` tiene il rapporto e l'orario dell'ultima rinuncia; e' una finestra di 5 minuti).

Regole di confronto (A e B):

1. **Soglia**: la priorita' di partenza e' `a_init` = **4** (solo candidati con priorita' > 4: nuovi o voluti) oppure **-1** (chiunque) se il flag 128 e' acceso (nessuna evidenziazione di «nuovo» attiva, oppure «Risp B4» acceso); per il blocco C `b_init` = **4**, o **-1** (flag 64:
   nessuna evidenziazione o «Chiama gia' lavorati»), o **8** (flag 256, «Chiama prima i nuovi nominativi»).
2. Si **sostituisce** il candidato corrente se `priorita' > corrente`, oppure (dopo aver superato la soglia) se il nuovo e' RCALL/RREPORT/RRREPORT e il corrente non lo era (**chi mi ha chiamato direttamente batte chi ha solo mandato RRR/RR73/CQ**, qualunque priorita'),
   oppure a **parita' di priorita'** per SNR (il piu' alto; con il flag 16 «priorita' al rapporto» si preferisce chi ha mandato il rapporto) oppure, con il flag 32, per distanza (il piu' lontano).
3. **Chi non ho ancora risposto** (`tt.time == tt.b_time`: l'ultimo evento e' l'inizio) **prevale** su chi sto gia' seguendo (il «candidato fresco» riporta la soglia a `a_init`; una volta preso un fresco, contano solo i freschi).
4. **Direzione del mio CQ**: se il mio CQ ha un modificatore (`CQ EU`, `CQ DX`, `CQ JA`...), il chiamante deve essere di quel continente, o avere quel prefisso, o (con `DX`) non essere del mio continente; con «direzione CQ rigorosa» passano anche i
   desiderati (priorita' 17-19 o 2-4).
5. Per il blocco C anche la **direzione del suo CQ** deve includermi: nessun modificatore, o modificatore piu' lungo di due caratteri, o il mio continente, o l'inizio del mio nominativo, o il mio prefisso, o `DX` se non siamo dello stesso continente.
6. L'ordine di visita (le chiavi di una tabella `QHash`) **non e' definito**: a parita' completa il risultato dipende dall'ordine interno della tabella (non dall'ordine di arrivo).

### 3.4 I flag `time` (`REA 0x47d3e0`)

| Bit | Valore | Si accende se | Effetto |
|---|---|---|---|
| 0 | 1 | AutoSeq 6/7 e le condizioni di 2.6 | attiva il blocco C |
| 4 | 16 | «Priorita' al rapporto» (`m_rprtPriority`) | a parita', preferisce chi ha mandato il rapporto (RREPORT) e poi l'SNR |
| 5 | 32 | «Distanza massima» (`m_maxDistance`; esclude il precedente) | a parita', il piu' lontano (locatore -> distanza in km) |
| 6 | 64 | nessuna evidenziazione di nuovo attiva (DXCC, locatore, prefisso, nominativo) **oppure** «Chiama gia' lavorati» | `b_init = -1` |
| 7 | 128 | nessuna evidenziazione attiva **oppure** «Risp B4» (`AnswerWorkedB4`) | `a_init = -1` |
| 8 | 256 | altrimenti (flag 64 spento), se «Chiama prima i nuovi nominativi» | `b_init = 8` |

Con **le evidenziazioni di nuovo spente** JTDX risponde a chiunque (a_init = b_init = -1). Con **una qualunque accesa** e «Risp B4» spenta, risponde **solo** a stazioni con priorita' > 4: chi chiama ed e' gia' lavorato (priorita' 0) **viene ignorato**.
Qui (`JTDX.ini`): `AnswerWorkedB4=true`, `CallWorkedB4=false`, `CallHigherNewCall=false`, `ReportMessagePriority=false`, `MaxDistance=false`, `CallMode=3`, `CallPriorityCQ=false`.

### 3.5 I filtri di visualizzazione e l'automazione

`displayDecodedText` nasconde una riga (`show_line = false`) con: «non mostrare i gia' lavorati» (`displayWorkedDontShow_`, esclusi i messaggi a me), continenti nascosti, paesi, nominativi nascosti, «filtro mio continente»,
«nascondi testo libero non standard» (`hidefree_`), «mostra solo CQ» (`showcq_`), «solo CQ e RR73» (`showcqrrr73_`), «solo CQ e 73» (`showcq73_`) (le ultime tre solo fuori da +-10 Hz dalla frequenza Rx); i **desiderati** riaccendono la riga
(salvo nominativo nascosto). `bypassButton` («Bypassa»: `m_bypassAllFilters`) li annulla tutti. **Solo le righe visibili** passano a `QsoHistory::message` (`src displaytext.cpp:925-928`), e con `filterUDP` («applica i filtri alla trasmissione UDP»)
solo quelle visibili escono via UDP (`postDecode`, `src mainwindow.cpp:3952`). Con `prevent_spotting_false` non escono quelle marcate `?` (`isWrong`: nel sorgente il decoder FT8 non produce quel carattere, lo usano i modi vecchi).
**Conseguenza per Aurora**: i filtri di JTDX cambiano chi AutoSeq vede (e chi Aurora riceve); per un'automazione prevedibile conviene tenere «Bypassa» acceso o i filtri spenti, oppure decidere la scelta in Aurora.

### 3.6 Il database dei nominativi e dei locatori

- `cty.dat` (`share\jtdx\cty.dat`): continente, paese, zone.
- `ALLCALL7.TXT` (`share\jtdx\`, **100 515** nominativi spottati da pskreporter in FT4/FT8 dal 2023-06-29; copia installata del 30/6/2024, il file del repository e' del 28/2/2022; `cwfilter` lo carica in 16 tabelle per iniziale; se una tabella e'
  vuota (`ldbvalid` falso, messaggio «ALLCALL7.TXT is too short or broken?») **ogni controllo `searchcalls` passa**): serve ai filtri sui falsi (sezione 4).
- `grid_data.bin`, `state_data.bin`: dati dei locatori e delle zone (`JTDX\`).

## 4. I filtri sui falsi decode

Tutti stanno nel **decoder** (`jtdxjt9.exe`): `ft8b` scarta un candidato con `nbadcrc = 1` e ritorno; `chkfalse8`, `chkspecial8`, `chkflscall`, `chkgrid`, `chklong8`, `filtersfree`, `searchcalls` sono le sue funzioni di appoggio (`REA9 0x4eb150`, `0x505520`,
`0x4f2eb0`, `0x4f2ff0`, `0x4b2130`; `chklong8` e `filtersfree` non cercate). Il decoder FT4 ne usa solo una parte. L'interfaccia ne aggiunge altri (3.5). Nulla di cio' e' una soglia unica: e' una **catena di controlli**, ognuno con una storia di falsi decode osservati (i commenti del sorgente
riportano i casi, con SNR e testo).

### 4.1 Gli stadi, nell'ordine (FT8, `lib/ft8b.f90`, `REA9 0x438420`)

| # | Stadio | Condizione per **scartare** (`nbadcrc = 1`) | Dove |
|---|---|---|---|
| 1 | Candidato (`sync8`) | `sync < syncmin` della passata (1,5; con sensibilita' bassa 1,225 / 1,5 / 1,1 a rotazione); sul canale di Rx (+-3 Hz) soglia 1,1; ritardo (DT) fuori da `jpeak` -49..76 passi, cioe' da -2,0 a +3,0 s (SWL -74..101); doppioni a meno di 4 Hz e 0,1 s tengono il migliore | `sync8.f90` |
| 2 | Sincronismo duro | `nsync` = numero dei 21 simboli di Costas per cui il tono piu' forte e' quello giusto: **scarto se `nsync < 7`**; eccezione per i candidati «CQ» (`lcqcand`): con `nsync` 4, 5 o 6 si prosegue solo se `nsync + nsync2 >= 12 / 12 / 11` oppure se `rscq` (somiglianza ai simboli del `CQ`) e' >= 6,6 / 6,1 / 5,6; in quel caso si prova **solo** l'AP «CQ» (`lapcqonly`) | `ft8b.f90:356-383` |
| 3 | Punteggio di sincronismo | `nsyncscore` (quanti simboli di sincronismo hanno il tono giusto sopra la media degli altri sette) e `scoreratio` (rapporto medio), a tre gruppi di 7; tre regioni di ritardo `rrxdt` (da -0,5 a 2,13 s; < -0,5; > 2,13). Nella regione centrale: scarto se `nsyncscore < 8`, o `< 10` con `scoreratio < 5,5`, o `< 11` con `scoreratio < 3,63`; con `nsyncscore` = 11, 12, 13 e `scoreratio` sotto 5,37 / 4,6 / 4,4 si scarta solo se anche i punteggi dei singoli gruppi sono bassi; con `nsyncscorew` (punteggio sui 7 toni della matrice di Costas) < 3, = 3, = 4, = 5 si scarta salvo che un gruppo abbia `scoreratio` > 13,8 / 15 / 10 (oppure `nsyncscore` > 17 per 5). Le due regioni laterali hanno tabelle piu' corte (6, 8, 9 e 5,5 / 6,6 / 6,0). **Solo se il candidato e' a >= 2 Hz dalla frequenza Rx** o se `stophint` (nessun QSO in corso) | `ft8b.f90:414-504` |
| 4 | Correzione del codice | BP (max 30 iterazioni, uscita anticipata se i controlli non migliorano per 5 giri dopo il decimo con piu' di 15 parita' violate); se fallisce OSD (`ndeep` 3, 4 o 5 secondo i casi, sezione 5.3). **Scarto** se `nharderrors < 0`, o `nharderrors + dmin >= 60`, o (sottopasso `isubp2 > 2` e `nharderrors > 39`), o codeword tutta a zero | `ft8b.f90:1488-1500` |
| 5 | Decodifica a 77 bit | `i3 > 4` o (`i3 = 0` e `n3 > 5`); testo non decifrabile (`unpack77`); AP tipo 1 che produce `CQ DE AA00` | `ft8b.f90:1613-1650` |
| 6 | **Plausibilita' del testo** | per decode deboli o con AP: `qual = 1 - (nharderrors + dmin) / 60 < 0,39`, o SNR < -20,5, o `rxdt` fuori da -0,5..1,9 s, o tipo AP 1, 2, 3, 11, 21, 40, 41 (i tipi 35/36, ricerca del DX, hanno un controllo a parte: se il primo nominativo finisce in `/R` deve essere in `ALLCALL7`): i tipi AP 4-6 (MIA SUO RRR/73/RR73) non vengono controllati; se il messaggio contiene il **mio** o **suo** nominativo di base (per i tipi AP diversi da 2, 3, 11, 21, 40, 41) si salta; altrimenti `chkspecial8` (messaggi DXpedition) o **`chkfalse8`** (4.2) | `ft8b.f90:1658-1685` |
| 7 | Altri controlli di forma | `CQ` senza AP con locatore: `chkgrid` sul nominativo; messaggi con ` R ` (`i3 = 1/2`); messaggi contest `i3 = 0, n3 = 3/4` («ARRL Field Day»: i nominativi non USA/Canada devono mandare `DX`; con SNR < -19 o `rxdt` fuori -0,5..1,0 anche `chkflscall`); `/R` (`i3 = 1`); messaggi con `<...>` e `RRR`/`RR73`/`73` in testa (non ammessi); `<...> call grid` debole (SNR < -18 o `rxdt` fuori -0,5..1,0: `chkgrid` e `chkflscall`); `call <...> grid`: `chkflscall('CQ', call)`; RTTY `TU;` (si traduce in `DE call TU` dopo `chkflscall`); AP tipi 35/36 (cerca il primo nominativo in `ALLCALL7`); `CQ call/R grid` (`i3 = 4`); `i3 = 2` senza `/P` | `ft8b.f90:1700-1925` |
| 8 | Duplicati e forma finale | `ft8_decode`: stesso testo di uno gia' trovato e (SNR non maggiore e a meno di 45 Hz) oppure (SNR maggiore, a meno di 45 Hz e piu' di un thread); con «nascondi doppi FT8» (`lhideft8dupes`): SNR non maggiore **o** (maggiore e a meno di 45 Hz). `ft8b`: stesse prime due parole di un decode `ft8sd` gia' mandato; `lhidehash` (nasconde i `<...>` oltre la sesta colonna); i decode `ft8s`/`ft8sd` che contengono il **proprio** nominativo come secondo (non dovrebbero esistere) | `ft8_decode.f90:239-345`, `ft8b.f90:1745-1900` |

Per FT4 (`ft4_decode.f90`): `smax < 1,2` (sincronismo 4D), `nsync_qual < 20` (di 32 bit di sincronismo), `f1` fuori da 10..4990 Hz, `message = ""`, e dopo la decodifica: messaggi `TU;` con SNR < -15 -> `chkflscall`; AP tipo 1 con SNR < -15 -> `chkfalse8`;
messaggi con ` R ` -> `chkflscall`.

### 4.2 `chkfalse8` (`lib/chkfalse8.f90`, `REA9 0x4eb150`)

Riceve il messaggio, `i3`, `n3`, il tipo AP e se il primo nominativo e' un hash `<...>`. Controlla, nell'ordine:

1. **Messaggi `CQ`**: se `i3 = 4` (nominativo non standard): senza `/` il nominativo non deve avere spazi interni e non deve finire con una cifra; con `/` si usa `chkflscall('CQ', call)`; nominativo di 10 o piu' caratteri -> `chklong8`; di 6 caratteri senza `/` con cifre
   in posizioni 3 (`001000`), 1+3 (`101000`) o 2+3 (`011000`) (cioe' un nominativo **standard** in un messaggio non standard: protocollo violato). `i3 = 1` e AP tipo 1: `chkgrid(call, grid)`. `CQ nnn call/R grid`: `chkgrid` e `chkflscall('MYCALL', call)`. `CQ call grid` senza AP: `chkgrid`;
   con `/R` anche `chkflscall`. Con `CQ`, la funzione ritorna dopo i controlli.
2. **AP tipo 2** (`MIA ??? ???`): il secondo nominativo senza `/R` passa per `callsign_q` (forma plausibile); se c'e' ` R ` il locatore deve esistere e avere forma valida, altrimenti scarto; se non c'e' `R` e il terzo campo ha 4 caratteri deve essere un locatore;
   poi `chkgrid`. **Non** si usa `chkflscall` qui («troppo rischioso scartare un decode giusto se il nominativo manca in ALLCALL7»).
3. **AP tipo 40** (`<MIA> ??? ???`): come il 2, senza `/R`.
4. **AP tipo 3, 11, 21, 41** (`MIA SUO ???`): se il messaggio e' proprio `MIA SUO grid` con `grid` diverso dal locatore noto di SUO, `chkgrid` sul locatore (in Hound qualunque ` R ` e' scarto, `RR73` passa).
5. **Messaggi con ` R `, `/R `, `/P ` (`i3` 1..3)**: `chkflscall(call_a, call_b)` salvo che il secondo sia il DX scelto; con `/` si isolano i nominativi base e si ripete se non sono il proprio e il DX.
6. `<...>` come primo nominativo e `i3 = 1`: controllo `chkgrid` sul secondo nominativo, se non e' il DX.
7. **Due nominativi e basta** (`i3 = 1/3` senza ` R ` e senza `/`, o `i3 = 0, n3 = 3`): `chkflscall(call_a, call_b)`, eccetto quando il primo e' il **proprio** (chiamata a me).
8. `i3 = 4` con `<...>` (messaggi con nominativo non standard): forma del nominativo (spazi, ultima cifra, `/` finale), `chklong8`, controllo dei nominativi di 11 caratteri con `/` (che parte e' il prefisso), `chkflscall`.
9. `i3 = 3` `TU; a b ...` -> `chkflscall(a, b)`.
10. **Testo libero** (`i3 = 0, n3 = 0`): `/.` nel testo -> scarto; `filtersfree` (elenco di forme tipiche dei falsi in testo libero). `CQ_...` o `^` nel testo -> scarto.

### 4.3 `chkflscall`, `searchcalls`, `chkgrid`

- **`chkflscall(a, b)`** (`REA9 0x4f2eb0`): se `a` inizia per `<.` non controlla. Parte da `falsedec = vero`; lo toglie solo se `searchcalls` trova il nominativo in `ALLCALL7`: se `a` e' `MYCALL` o `CQ` si cerca solo `b`; se `b` inizia per `<` si cerca solo `a`;
  altrimenti `searchcalls(a, b)`, che passa se **almeno uno** dei due e' nell'elenco (ritorna al primo trovato): con due nominativi basta che uno dei due sia noto.
- **`searchcalls`** (`REA9 0x4b2130`): se la base di dati non e' valida (`ldbvalid`), passa. Un nominativo di piu' di 7 caratteri senza secondo nominativo passa; un secondo di piu' di 7 si ignora. `TU;` si salta. Cerca nella tabella dell'iniziale (16 tabelle, 100 515 voci, nominativi a 7 caratteri al massimo).
- **`chkgrid(call, grid)`** (`REA9 0x4f2ff0`, 3 392 righe): per ogni **campo** (prima lettera del locatore) e **quadrato** (prime quattro lettere/cifre) un elenco di prefissi di nominativo compatibili; restituisce `lgvalid` (il locatore esiste per quel
  campo), `lchkcall` (conviene controllare il prefisso) e `lwrongcall` (il prefisso non puo' stare in quel locatore). E' una tabella geografica scritta a mano, senza dati: non c'e' nulla da caricare.
- `callsign_q`, `call_q` (file `include`): forma plausibile del nominativo (prefisso, cifra, suffisso).

### 4.4 I filtri della sequenza che somigliano a filtri sui falsi

- **`ft8s`** (decoder del canale del QSO) rifiuta i propri risultati se i rapporti di potenza non tornano (`spratio` fuori da 0,6..1,25, `soratio` > 1,29, `spnratiom` > 2,3 con `sporatiom`/`spratiom` > 1,35); sezione 5.5.
- **`lrepliedother`**: se tra i decode c'e' un messaggio del DX (non a noi, non CQ), un decode `ft8s` dello stesso periodo viene scartato (il DX parla con un altro).
- **`lqsomsgdcd`**: appena decodificato il messaggio del QSO (`MIA SUO ...`) i tipi AP 3-6 e il `ft8s` degli altri thread si spengono (risparmio di CPU e meno falsi).

## 5. I parametri della decodifica

### 5.1 Dalle impostazioni ai parametri (`MainWindow::decode`, `REA 0x4bbad0`, `src mainwindow.cpp:3281-3437`)

| Impostazione (`JTDX.ini`) | Predefinito (qui) | Parametro | Effetto |
|---|---|---|---|
| `NDepth` 1 Rapida, 2 Media, 3 Profonda | 3 (3) | `ndepth` | **non usato da FT8 ne' da FT4**: arriva solo ai decoder JT65, JT9 e T10 (`decoder.f90:2407, 2434, 2458`) |
| `NFT4Depth` 1 veloce, 2 media, 3 profonda | 3 (3) | `nft4depth` | profondita' FT4 (5.6) |
| `NFT8Cycles` 1..3 | 1 (1) | `nft8cycles` | passate: **3, 6 o 9** (5.2); `NFT8SWLCycles` lo stesso in modo SWL |
| `FT8Sensitivity` 0 minima, 1 «soglie basse», 2 «sottopassate» | 1 (1) | `lft8lowth` = (`!= 0`); `lft8subpass` = (`== 2`) | soglia di sincronismo bassa a rotazione; seconda lettura dei simboli per ogni candidato |
| `NFT8QSORXfreqSensitivity` 1 bassa, 2 media, 3 alta | 2 (2) | `nft8rxfsens` (Hound: sempre 1) | soglie di `ft8s` (5.5) |
| `NCandidateListThinning` 5..100 | 100 (100) | `ncandthin` | **diradamento dei candidati** (100 = nessuno): vedi 5.2 |
| `FT8threads` 0 = automatico, 1..24 | 0 (0) | `nmt` | thread (5.2) |
| `FT8DecoderEarlyStart` | vero alla prima esecuzione (vero) | `learlystart` | il decoder parte al blocco audio 48 invece del 49 (circa 14,4 s invece di 14,7 s dall'inizio del periodo; in SWL il 51, circa 15,3 s) |
| `FT8WideDXCallSearch` | vero (vero) | `lwidedxcsearch` | cerca il DX scelto (AP tipo 31, 35, 36) anche fuori dal canale Rx |
| `HideFT8Dupes`, `HideFTContestMessages`, `HideTelemetryMessages`, `HideMsgsWith2ndCallAsHash` | - (vero, falso, falso, falso) | `lhideft8dupes`, `lhidetest`, `lhidetelemetry`, `lhidehash` | nascondono (non decodificano diversamente) |
| Pulsante «Filtro» | spento | `nfilter` | banda +-60 Hz attorno a `nfqso` (+-290 in Hound), thread <= 8; l'AutoSeq con «filtro automatico» lo accende da solo |
| Decodifica a richiesta sulla frequenza Rx (`freezeDecode`, doppio clic sulla cascata) | - | `nagainfil` | banda +-25 Hz, thread <= 4, OSD d'ordine 5 |
| «Mitiga AGC» / pulsante «Sync» | spenti | `nagcc`, `lforcesync` | con `nagcc` `sync8` usa una metrica a rapporti (tono / somma degli altri) e `agccft8` compensa l'AGC (`agccft8` non letta); con `lforcesync` anche un ritardo forzato (`forcedt`) |
| SWL | spento | `nswl` | finestra di ricerca piu' ampia (+-86 passi), `syncmin` come con le soglie basse, la terza passata di un ciclo da 3 sottrae, `nweak` sempre 2, il sottopasso 4 senza AP abilitato |

`nQSOProgress` e `nlasttx` (sezione 1), `lmycallstd`, `lhiscallstd`, `lapmyc`, `lskiptx1`, `ltxing`, `hiscall`/`hisgrid` completano gli ingressi. `ltxing` e' vero se Enable Tx e' acceso e l'ultima trasmissione e' stata
meno di **26 s** fa. `lapmyc` (permette l'AP con il **proprio** nominativo) vale 1 dal secondo 10-11 di ogni trasmissione e torna a 0 al primo minuto in cui sono passati **120 s** dall'ultima (`on_the_minute`: in pratica 2-3 minuti).

### 5.2 Thread, banda, passate e candidati (`decoder.f90`, `ft8_decode.f90`, `sync8.f90`; `REA9 0x42b3c0`, `0x4c9470`)

- **Thread**: automatico = 1 con 1 processore; `processori - 1` da 2 a 4; `- 2` da 5 a 8; `- 3` da 9 a 15; `- 4` da 16 a 20; `- 5` da 21 a 29; **24** da 30 in su. Con un numero scelto (1..24) si usa quello, al massimo i processori. La banda `[nfa, nfb]` viene divisa in **fette
  uguali**, una per thread; **ogni thread esegue da solo tutte le sue passate e sottrazioni sulla propria fetta** (OpenMP annidato); il thread la cui banda contiene `nfqso` (`lqsothread`) fa anche i decoder del QSO. Quindi **il risultato dipende dal numero di thread**
  (i segnali a cavallo di una fetta non vengono sottratti dall'altro thread). Un solo thread = il comportamento che ha oggi il crate `ft8` di Aurora.
- **Passate** (`npass`): `nft8cycles` 1 -> 3, 2 -> 6, 3 -> 9. Nel ciclo 2 (passate 4-6) l'audio e' **spostato di mezzo campione** (`dd8(i) = (dd8(i) + dd8(i+1)) / 2`, dopo le sottrazioni; con 9 passate se ne tiene una copia: anche un lieve passa-basso) e nel ciclo 3 (passate 7-9) di
  mezzo campione dall'altra parte (`(dd8m(i-1) + dd8m(i)) / 2`, dalla copia). I thread si aspettano (`omp barrier`) alle passate 4 e 7.

| Passata | Spettro per `sync8` | `syncmin` normale | con `lft8lowth` o SWL | Sottrazione |
|---|---|---|---|---|
| 1, 4, 7 | ampiezza `sqrt(re^2 + im^2)` | 1,5 | **1,225** | si' |
| 2, 5, 8 | potenza `re^2 + im^2` | 1,5 | 1,5 | si' |
| 3, 6, 9 | `abs(re) + abs(im)` | 1,5 | **1,1** | **no** se `ipass > 5` o (`ipass == 3` con `npass == 3`, non SWL) |

  Verificato nell'eseguibile (`REA9 0x42b3c0`): costanti 1,5 (`0x3fc00000`), 1,225 (`0x3f9ccccd`), 1,1 (`0x3f8ccccd`), `avexdt * 25 +- 62` e `+- 86`, 1,9 e 3,15.
  La ricerca fine di `sync8d` ha raggruppamenti **diversi** dalle passate di `sync8`: ampiezza (modulo) nelle passate 1, 5, 9; potenza nelle 2, 6, 7; modulo `|re| + |im|` della media di coppie di toni adiacenti nelle 3, 4, 8 (`sync8d.f90`; il crate `ft8` lo ha gia', `SyncKind::for_pass`).
- **Alta sensibilita'** (`lhighsens`) per il candidato: `sync < 1,9` oppure (passate 2, 4, 6 e `sync < 3,15`). Non e' una soglia di scarto: e' passato a `ft8_downsample` (non letta) per la scelta del filtro.
- **Finestra di ritardo**: `jzb = -62 + avexdt * 25`, `jzt = 62 + avexdt * 25` passi da 0,04 s (cioe' +-2,48 s attorno al ritardo medio delle stazioni sentite, `avexdt`); in SWL +-86 passi (+-3,44 s).
- **`sync8`**: spettrogramma a passo 0,04 s (`NSTEP`), bin da 3,125 Hz (`NFFT1`), finestra di un simbolo con una coda di 201 campioni (16,75 ms) su ogni lato (`windowx`, primo e ultimo campione x1,9); per ogni (frequenza, ritardo) somma i toni di Costas sui tre gruppi (e il
  motivo del `CQ` nei simboli 8-16) rispetto alla media dei restanti; il punteggio e' il massimo fra «tre gruppi» e «ultimi due» (`sync_abc`, `sync_bc`); per ogni frequenza si tiene il ritardo migliore; poi si divide per il valore al **40% della classifica** (`red / base`).
  Candidati: `red >= syncmin` (1,1 entro +-3 Hz da `nfqso`); `jpeak` in -49..76 (SWL -74..101); al massimo **450** (lista di 460); eliminazione dei vicini (< 4 Hz, 3 in SWL, e < 0,1 s: si tiene il migliore, salvo +-3 Hz da `nfqso`); **in cima** i candidati entro
  +-3 Hz da `nfqso` (soglia 1,1); due candidati **virtuali** a `nfqso` con ritardo +5,0 e -5,0 s (per i decoder del QSO). Il bit `candidate(4)` > 1 segna i candidati «tipo CQ» (`redcq`).
- **Diradamento** (`ncandthin < 100`): ordina per `sync / (|dt - dtcenter| + 1)` (con il denominatore al quadrato nelle passate 2, 5, 8) invece che per `sync`, e **tiene solo una frazione** dei candidati (oltre a quelli del canale Rx): `ncandthin / 100` (triplicato dal filtro Rx), moltiplicato
  nelle passate 1, 4, 7 per 1,27, nelle 2, 5, 8 elevato al quadrato (se > 0,79; altrimenti x0,79), nelle 3, 6, 9 per 5 (sempre con tetto 1). Serve solo a ridurre la CPU.
- **Dopo ogni decodifica** (`ft8b`): il segnale viene **sottratto** (`subtractft8`) con ritardo e frequenza rifiniti (`peakup`), e la frequenza va in `freqsub` (200 al massimo) per non ricercarla.

### 5.3 `ft8b`: il tentativo su un candidato (`lib/ft8b.f90`, `REA9 0x438420`)

1. **Quattro modi di ricerca** (`nqso`): 1 = normale; 2 = secondo tentativo **stretto** (+-1,25 Hz in passi di 0,25 Hz, audio `cd2`) se il candidato e' entro 2,5 Hz da `nfqso`, nessuna decodifica del QSO e Tx tra 1 e 4, e il ritardo e' a meno di 0,18 s da quello dell'ultimo messaggio del DX (o non c'e' un ultimo messaggio);
   i due candidati **virtuali** (ritardo +-5 s) usano al posto del ritardo quello **gia' noto del DX** (`lastrxmsg`, o la storia `calldteven`/`calldtodd`: modo 2 o 3; il 3 solo con `nft8rxfsens >= 3`) per sentire un segnale molto in ritardo o in anticipo; 4 = `ft8sd`: lo stesso
   segnale e lo stesso messaggio visti **nel periodo precedente della stessa parita'** (`evencopy`/`oddcopy`: CQ e messaggi standard con frequenza e ritardo, fino a 130).
2. **Ritardo e frequenza fini**: +-8 campioni a 200 Hz (+-0,04 s) con `sync8d`; frequenza +-2,5 Hz in 0,5 Hz (largo) o +-1,25 Hz in 0,25 Hz (stretto, per `nfqso`).
3. **Sincronismo**: `syncav` = media dei rapporti tono/rumore dei 21 simboli di Costas; se < 2,5 si gonfiano di 1,9 il primo e l'ultimo campione di ogni simbolo, con un fattore di equilibrio; si divide ogni tono la cui energia complessiva supera di 1,5 volte
   quella del tono piu' debole (`sp`).
4. **Filtri di sincronismo**: sezione 4.1, stadi 2 e 3.
5. **Sottopassate** (`nsubpasses`): `nweak` = 2 se `lft8subpass`, SWL, `dfqso < 2 Hz` o trasmissione sul tuo `nftx`, altrimenti 1 (la seconda lettura usa i simboli a tempo invertito). Se il segnale ha il motivo del **CQ** (`s256` col massimo nel bin 5 o `rscq > 3,1`)
   **3** (**5** se nel periodo precedente c'era lo stesso segnale entro 2 Hz e 0,05 s: si **sommano** i simboli dei due periodi); se ha il motivo del **mio nominativo** (`lmycsignal`, AP attivo) **6** (**8** con memoria); se e' il segnale del QSO (`lqsocandave`) **9** (**11**).
   Le sottopassate 3, 6, 9 combinano la lettura normale e quella a tempo invertito; **4, 7, 10 mediano in potenza** e **5, 8, 11 in ampiezza** i simboli del periodo precedente (`csold`) con quelli attuali: per CQ e chiamate a noi JTDX **somma gli ultimi due periodi**.
6. **LLR** (`bmeta`, `bmetb`, `bmetc`, `bmetd`): metriche da 1, 2 e 3 simboli coerenti (`nsym` 1..3) e la quarta normalizzata per bit; ciascuna divisa per la propria radice della media dei quadrati e moltiplicata per **2,83**; `apmag = max|llr| * 1,01`.
   Se `srr` (rapporto del sincronismo) < 2,5 (circa -19 dB): il primo sottopasso porta `s2` al quadrato (tra 2,3 e 2,5) o lo rimodella con `1 + 8 s^2 - 0,12 s^4` (sotto 2,3; `(s + 5,82)^2` se `s >= 5,77`), gli altri sottopassi usano `(0,5 s)^3`.
7. **31 sottopassate di decodifica** (`isubp2`): 1-4 senza AP (con `llra`, `llrb`, `llrc`, `llrd`; la 4 solo in SWL; richiedono `syncavemax >= 1,8`, o 1,9 se `ltxing` e il candidato e' a >= 3 Hz da `nfqso`; non si fanno se `lapcqonly` o `lskipnotap`);
   5-31 con **analisi a priori** (AP), a gruppi di tre (con `llrc`, `llrb`, `llra`) per tipo:

| Gruppo | `isubp2` | Tipo AP (stazione propria e DX standard) |
|---|---|---|
| G1 | 5-7 | 3 `MIA SUO ???` (58 bit + 3 di `i3`) |
| G2 | 8-10 | 6 `MIA SUO RR73` (77 bit) |
| G3 | 11-13 | 5 `MIA SUO 73` |
| G4 | 14-16 | 4 `MIA SUO RRR` |
| G5 | 17-19 | 2 `MIA ??? ???` (29+3 bit) |
| G6 | 20-22 | 1 `CQ ??? ???` (29+3 bit) |
| G7 | 23-25 | 31 `CQ SUO locatore` (cerca il DX scelto) |
| G8 | 26-28 | 36 `??? SUO RR73` (29+19 bit) |
| G9 | 29-31 | 35 `??? SUO 73` |

   Quali gruppi sono attivi dipende dallo stato (`naptypes(nQSOProgress, ...)`): **Tx6 (CQ, 0)**: G5, G6, G7, G8, G9; **Tx1 e Tx2 (1, 2)**: G1, G6, G7, G8, G9; **Tx3 (3)**: G1-G4 e G7-G9; **Tx4 (4)**: G1-G5 e G7-G9; **Tx5 (5)**: G5-G9.
   Per nominativi non standard ci sono le tabelle gemelle (`nmycnsaptypes`: tipi 40-44; `ndxnsaptypes`: 11-14, 31, 35, 36; Hound: `nhaptypes`: 21-24, 31, 36, 111). Le condizioni che escludono un tipo sono molte e sono nel sorgente (`ft8b.f90:981-1210`); le principali: i tipi con `MIA` solo se `lapmyc`;
   i tipi 3-6 solo se non e' gia' stato decodificato il messaggio del QSO (`lqsomsgdcd`), il candidato e' entro `napwid` dal canale Rx o Tx (**5 Hz** sotto 30 MHz, 15 fino a 100 MHz, 50 oltre) e il segnale ha il motivo atteso (`lqsosigtype3`, `lqso73`, `lqsorr73`, `lqsorrr`: 6 su 19 simboli, o piu', con il tono giusto);
   31/35/36 solo senza QSO in corso (`stophint`), con Enable Tx spento e il DX scelto diverso dall'ultimo registrato (`lenabledxcsearch`), e il motivo del CQ o della chiusura; i tipi 3-30 solo con un QSO in corso (non `stophint`); il tipo 1 solo dopo un controllo del rapporto `scqnr` (>= 1,0 nel gruppo 20, >= 1,2 con `lft8lowth` o `lft8subpass` o >= 1,3 senza, nel 21) o con `lcqsignal`; il tipo 2 analogo con `smycnr`.
8. **BP e OSD** (`bpdecode174_91`: **30 iterazioni** al massimo, uscita anticipata se per 5 giri dopo il decimo non migliora il numero delle parita' violate e ne restano piu' di 15). Se BP fallisce, **OSD**: `ndeep = 3` di base; **4** per i segnali con il motivo del QSO o del mio nominativo (`lqsosig`, `lmycsignal`) entro `napwid` dal canale Rx o dal Tx (con `lapmyc`), fuori dalla decodifica a doppio clic, **e** con AP (`iaptype` diverso da 0) e senza QSO attivo (con un QSO in corso e un DX scelto torna 3: «lascia lavorare `ft8s`»); **4** anche per `ldxcsig` con `stophint` entro `napwid`; sempre 3 in Hound; **5** nella decodifica a doppio clic. Mai 1 o 2. OSD (`osd174_91`):
   `ndeep` 1: ordine 1, 0 preprocessi; 2: ordine 1 con `npre1`; 3: ordine 1 con `npre1` e `npre2` (`ntau` 14, `nt` 40, `ntheta` 12); 4: ordine 2, `npre1` (`ntau` 19); 5: ordine 2, `npre1` e `npre2`.
9. Dopo la decodifica: sezioni 4.1 (stadi 5-8) e la stima dell'SNR: `xsnr = 10 log10(media(xsig/xnoi) - 1) - 26,5` (con `xsig` il tono giusto al quadrato e `xnoi` la media degli altri sette), correzioni sopra 7, 30, 40, 49 dB e sotto -17 dB (`xsnr - (1 + 1,4/(23 + xsnr))^2 + 1,2`),
   minimo -23 senza AP e -24 con AP; per `ft8s`/`ft8sd` minimo -26. Il ritardo scritto e' `xdt - 0,5`. (`jtdx_snr` di `ft8/src/filters.rs` e' gia' questa formula.)

### 5.4 Cosa e' verificato nell'eseguibile (`jtdxjt9.exe`)

Decompilate con REA da `jtdxjt9.exe` (una sessione, una richiesta alla volta): `__ft8_decode_MOD_decode` (`0x42b3c0`, 17 s), `ft8b_` (`0x438420`, **19 672 righe, 1 000 s** di decompilazione, 4 MB), `sync8_` (`0x4c9470`, 3 670 righe), `ft8s_` (`0x4562d0`, 4 657 righe),
`__ft4_decode_MOD_decode` (`0x423930`, 8 042 righe), `chkfalse8_` (`0x4eb150`, 17 192 righe) e `chkflscall_` (`0x4f2eb0`, con `searchcalls` in linea). Ho cercato nel testo le **costanti del sorgente** e controllato i confronti dove la struttura era leggibile. Esito:

| Valore del sorgente | Nell'eseguibile |
|---|---|
| `syncmin` 1,5 / 1,225 / 1,1; finestra `avexdt * 25 +- 62` e `+- 86`; `lhighsens` 1,9 e 3,15 | trovati in `0x42b3c0` (`0x3fc00000`, `0x3f9ccccd`, `0x3f8ccccd`) |
| `nsync` 4 / 5 / 6 con `nsync2` < 8 / 7 / 5 (cioe' `nsync + nsync2 < 12 / 12 / 11`) e `rscq` < 6,6 / 6,1 / 5,6 | trovati negli stessi confronti (`0x438420`, righe 15560-15575 del testo) |
| `nsyncscore` < 8; < 10 con 5,5; < 11 con 3,63; = 11 con 5,37 (e 4,2 sui gruppi); = 12 con 4,6 (4,0); = 13 con 4,4; `scoreratio` > 13,8 e 10,0 | trovati negli stessi confronti |
| `qual = 1 - (nharderrors + dmin) / 60 < 0,39`, SNR < -20,5 | trovati (`1,0 - (... ) / 60,0`, `0,39 <=`, `-20,5`) |
| rimodellamento di `s2` con 5,77 e 5,82; offset SNR -26,5; fattore 2,83 | trovati (2,83 in 131 punti) |
| `sync8`: percentile 0,4; soglia 1,1; 450 e 460 candidati; passo 0,04; bin 3,125; distanza 4,0 Hz; diradamento x1,27, x0,79 | trovati in `0x4c9470` |
| `ft8s`: 3,55; 4,0; 3,0 (soglie per sensibilita'); 0,77; 2,6; 3,43; 2,63; 2,45; 1,29; 1,25; 0,6; 2,3; 1,35; 1,5 | trovati in `0x4562d0` |
| FT4: 1,2 (sincronismo); 40 iterazioni; 20 bit su 32; 4990 Hz; 666,67; -14,8; -21 | trovati in `0x423930` |
| `chkfalse8` chiama `chkflscall` 9 volte, `chkgrid` 7, `chklong8` 3, `filtersfree` 1 | conteggio nell'eseguibile (nel testo decompilato) |

Non riletti nell'eseguibile: le tabelle `naptypes`, le condizioni singole dei tipi AP, i controlli di forma del testo (4.2) uno a uno, `ft8sd*`, `ft8mf*`, `ft8_downsample`, `agccft8`, `chkgrid`.
Cio' basta per dire che **il decoder installato e' quello del sorgente 2.2.159** nei punti che contano per i filtri e per i parametri; i confronti riga per riga di tutto `ft8b` (2 034 righe di sorgente) non sono stati fatti.

### 5.5 `ft8s`: il decoder del canale del QSO (`lib/ft8s.f90`, `REA9 0x4562d0`)

Non e' LDPC: **confronta i toni demodulati con i 56 messaggi possibili** della risposta (`msg(1..56)` = `MIA SUO` + `-01..-26` (1-26), `R-01..R-26` (27-52), `AA00` (53, sostituito dall'ultimo messaggio ricevuto se era il locatore), `RRR` (54), `RR73` (55), `73` (56)) e ne sceglie uno se la correlazione e' abbastanza chiara. Si attiva solo con DX scelto e Tx non fermato ne' in CQ (`nlasttx` diverso da 0 e 6),
nessun `stophint`, candidato a < 2 Hz da `nfqso`, QSO non ancora decodificato, tre candidati: il normale e i due **virtuali** (+-5 s). I messaggi ammessi dipendono dallo stato: `nlasttx = 1` (abbiamo mandato Tx1): solo i rapporti (1-26); se l'ultimo ricevuto era `MIA SUO locatore`: gli R-rapporti e il locatore ripetuto (27-53); se abbiamo mandato Tx2 senza risposta: gli R-rapporti (27-52); se l'ultimo era `R+rapporto`: R-rapporti, locatore (poi escluso), `RRR`, `RR73`, `73` (27-56);
se l'ultimo era `RRR/RR73/73`: solo 54-56; se l'ultimo era un semplice rapporto, niente R-rapporti. Con `nlasttx = 1` o rapporto senza risposta `nft8rxfslow` e' forzato a 1 (la meno sensibile). Soglie: `ntresh1` 26 (29 se il segnale e' forte, `srr > 7`), `ntresh2` 38 (41); `ncrcpaty` (corrispondenze sui simboli di parita') almeno 8; con `srr > 3` servono almeno 5 toni giusti sul mio nominativo e 10 sulla base; poi `thresh = (qual + 10) * (u1 - 0.6)`
(`u1` e `u2` i migliori punteggi, `qual = 100 (u1 - u2)`): si accetta se `thresh > 4,0` con `nft8rxfsens` = 1, `> 3,55` con 2, `> 3,0` con 3 (e `qual > 2,6`, `u1 > 0,77`), oppure per le varie corrispondenze progressive (`nmatch` > 22, 26, 38, 44, 47, 50... con `ncrcpaty` > 10, 14, 19...). Alla fine rifiuta se il rapporto di potenza sincronismo/parita'
(`spratio`) e' fuori da 0,6..1,25, o se `soratio` > 1,29 quando si e' risposto a un CQ o con un rapporto. **Lettura parziale**: le soglie del sorgente sono state ritrovate nell'eseguibile (5.4); la logica di `ft8s` non e' stata confrontata riga per riga.

### 5.6 FT4 (`lib/ft4_decode.f90`, `REA9 0x423930`)

- `nft4depth` 1: **una** passata, senza sottrazione e senza OSD, tre passate di BP (`npasses = 3`); 2: **tre** cicli di sottrazione, solo BP; 3: tre cicli, BP + OSD (`ndeep = 3`). `max_iterations = 40`, `syncmin = 1,2`, `maxcand = 100`; ai cicli 2 e 3 si entra solo se il ciclo precedente ha decodificato qualcosa.
- Candidati: `getcandidates4` (`REA9 0x46ad60`, picchi nello spettro medio); per ciascuno `ft4_downsample` a 667 Hz, ricerca in ritardo su **tre segmenti** (da -1,0 a +1,4 s, da -1,2 a +1,7 in SWL; con `avexdt` noto finestre di 490 campioni, 623 in SWL, centrate su `avexdt`) e in frequenza +-12 Hz in passi di 3, poi +-4 in passi di 1 (`sync4d`); si scarta se `smax < 1,2`
  (o se nei segmenti 2 e 3 e' minore del primo), `f1` fuori da 10..4990 Hz, bit di sincronismo corretti < 20 su 32.
- LLR a tre metriche (`llra`, `llrb`, `llrc`) x 2,83; AP: tipi 1-6 con `nappasses(0..5)` = 2, 2, 2, 2, 2, 3 passate extra e `naptypes` per stato (Tx6: 1, 2; Tx1-Tx2: 2, 3; Tx3-Tx4: 3, 6; Tx5: 3, 1, 2); `napwid = 50 Hz`; con `stophint` solo il tipo 1.
- SNR: `xsnr = 10 log10(snr) - 14,8` con il rapporto del candidato, minimo -21; ritardo `ibest / 666,67 - 0,5`.

## 6. Confronto con `docs/logiche-jtdx.md`

Segno: **C** confermato (nel sorgente e, dove detto, nell'eseguibile), **N** nuovo (non scritto prima), **S** sbagliato (da correggere).

| Punto di `logiche-jtdx.md` | Esito | Dettaglio |
|---|---|---|
| Principio: JTDX e' la verita', Aurora legge | C (invariato) | la logica vera e' in `QsoHistory` + `process_Auto` + `displayDecodedText`; il crate `qso-native` la riscrive (non la legge) |
| Tx1-Tx6: testi e uso | C | `genStdMsgs`; Tx5 e' un campo con macro (suffisso ` ^`) |
| Pallino «Next» (`txrb`) e pulsante «Now» (`txb`) | C, **N** | «Now» riavvia la trasmissione in corso (`m_restart`); **N**: con **AutoSeq acceso i sei `txrb` sono disattivati** (`enableTab1TXRB(false)`) |
| `AutoSeqButton`: «n e' il livello 0-7» | **S** | i livelli sono **0, 1, 2, 3, 6, 7**; 4 e 5 non esistono (etichette `AutoSeq0/1/2/6/3/7`) |
| «AutoSeq 4 e oltre: scelta automatica con le priorita' dei desiderati» | **S** | le liste dei desiderati valgono da AutoSeq **1 a 7** (suggerimento ufficiale: «AutoSeq 1..7», non «AutoSeq0»); cio' che cambia fra i livelli e' **quando** gira `process_Auto` (2.6) e la ricerca nei CQ (solo 6/7) |
| «chiama prima i nominativi con priorita' alta dei nuovi: solo da 4 in su» | **S** | e' `CallHigherNewCall` (soglia `b_init = 8`) e conta solo nella ricerca CQ (AutoSeq 6/7) |
| «AutoSeq 1..3: quando un contatore raggiunge il suo valore, JTDX chiama CQ» | C | dopo che un contatore libera il DX, `process_Auto` sceglie il successivo o chiama CQ (`on_txb6_clicked`) |
| «AutoSeq 6/7: cerca un CQ o un 73 valido e li chiama; se non ne trova, chiama CQ; si ferma se «1 QSO»» | C, **N** | «73 valido» = RFIN con priorita' > 0 (2.2); regole complete in 3.3; la ricerca vale solo se il prossimo periodo Tx e' quello che segue il periodo decodificato |
| Quattro contatori (AnswerCQ, AnswerInCall, SentRReport, SentRR7373) e valori `JTDX.ini` | C, **N** | valori di fabbrica 2/3/5/4 e tutti spenti; contano **trasmissioni** (la (N+1)-esima non parte); eccezioni per i desiderati, `reply_other`, CQ diretti (2.5) |
| `SeqHaltTxReplyOther`, `StrictDirectionalCQ`, `CallHigherNewCall` | C, **N** | il primo ferma Tx se il DX parla con un altro **entro la banda di guardia** (FT8 51 Hz, FT4 84) o **sempre** se acceso; di fabbrica e' acceso, qui spento |
| «1 QSO» «dopo un QSO finito spegne Abilita Tx» | C, **N** | spegne anche quando un contatore libera il DX («counter triggered») e fa avanzare subito anche con 73/RR73 |
| «Risp B4» «risponde anche a chi si e' gia' lavorato» | C, **N** | conta solo se almeno una evidenziazione di «nuovo» e' accesa; senza evidenziazioni JTDX risponde a tutti comunque (3.4) |
| «Bypassa» «salta i filtri del testo» | C, **N** | i filtri decidono anche **cosa vede `QsoHistory`** e cosa esce via UDP con `filterUDP` (3.5) |
| Casella RRR | C | cambia il testo del Tx4 e il momento della registrazione (2.8) |
| `TxMinuteButton`: pari/dispari, testi | C, **S** | il testo con il pulsante spento in **FT4 e' «TX 7.5»**, non «TX 00» soltanto; a minuto «TX Even»/«TX Odd» |
| Regola del periodo (opposto, impostato dal doppio clic) | C | + `nmod` e che AutoSeq non lo cambia |
| «Una trasmissione parte all'inizio del periodo scelto, o subito se conviene» | **S**, **N** | parte in qualunque istante dei primi **13,64 s (FT8) / 6,04 s (FT4)** del periodo; l'avvio tardivo trasmette la **coda** del messaggio; `Retransmitting` se cambia il messaggio dopo l'avvio (2.9) |
| «Limite di avvio per FT8 e FT4 non documentato» (Da verificare) | risolto | 13,64 / 6,04 s (durata del messaggio), `fTR < 99` sempre vero |
| Doppio clic: Tx1 o Tx2, Abilita Tx col «doppio clic imposta Abilita Tx» | C, **N** | il messaggio scelto dipende dal testo della riga (Tx2, Tx3, Tx4 o Tx5, 2.10); `AutoTX` = impostazione `QuickCall` (il doppio clic accende Enable Tx), non «trasmissione automatica» in generale |
| Sequenza osservata: ripetizione senza risposta, avanzamento con la risposta | C | con «Salta Tx1» il Tx2 si rigenera a ogni periodo con l'SNR piu' recente |
| **«Il QSO si chiude e si registra quando arriva RR73/RRR/73»** | **S** | **si registra quando JTDX comincia a trasmettere RR73 o 73** (`m_sentFirst73`); il FIN richiede anche la chiusura del DX (2.3, 2.8); verificato su `ALL.TXT` (6 ms) |
| `TxWatchdogTimer` «minuti senza risposta» | C, **N** | conta minuti solo se si e' trasmesso negli ultimi 32/16/134 s; si azzera con un messaggio **nuovo** o con RCALL/RREPORT (2.7) |
| `AutoQSOLogging` «registra da solo a QSO chiuso» | **S**, **N** | registra all'**invio** del primo 73/RR73; con Autolog acceso JTDX **ferma Tx** a S73/FIN (2.4) |
| `NDepth` «profondita' di decodifica» | **S** per FT8/FT4 | `ndepth` non conta in FT8 ne' in FT4; la profondita' FT8 e' `NFT8Cycles` e `FT8Sensitivity` (5.1) |
| `AcceptUDPReplyMessages` e `Reply` | C (`fase0-rapporto.md`) | `replyToUDP` -> `processMessage`: stesso percorso del doppio clic |
| Tabella dei controlli (153 elementi) | non toccata | non verificata contro il sorgente |

Nuove informazioni di rilievo non scritte prima: stati di `QsoHistory` (2.1-2.3); priorita' 0-31 (3.2); i tre blocchi di scelta e i flag (3.3-3.4); AnswerCQ non vale per i desiderati; il cooldown; la finestra di avvio tardivo e la ritrasmissione; il watchdog a minuti; la guardia di frequenza;
i filtri di visualizzazione che nascondono righe a `QsoHistory`; `ALLCALL7.TXT` (100 515 nominativi); FT8 senza `ndepth`; thread e fette di banda; la cronologia dei periodi e le sottopassate che sommano due periodi (5.3); `ft8s` (5.5).

## 7. Cosa fare in Aurora

Difficolta': **S** poche ore, **M** un giorno, **L** piu' giorni.

### 7.1 Per `core/crates/qso-native` (sequenza dei QSO, gia' scritto)

Il crate coincide gia' con JTDX su: registrazione all'invio di RR73/73 (e `RRR` al Tx5), contatori come numero di trasmissioni, Tx2 con l'SNR piu' recente, `reply_to` come `processMessage`, watchdog con azzeramento a messaggio nuovo. Cio' che cambia:

| # | Cosa | Dove | Diff. | Note |
|---|---|---|---|---|
| 1 | **Finestra di avvio = durata del messaggio** (13,64 s FT8, 6,04 s FT4) invece di 2 s / 1 s, e il `Tx` deve dire **da quale istante** del periodo parte; chi trasmette emette la **coda** della forma d'onda (500 ms di ritardo nominale; `m_ic = (mstr - 500) * 48`) | `types.rs` (`late_limit_s`, `Tx`), `sequencer.rs` (`next_tx`), `tx-native/scheduler.rs` (`max_late` 1 s, massimo 3 s: oggi **rifiuta** il ritardo) | M | l'avvio piu' tardi dei primi 1,6 s perde il primo gruppo di Costas ma funziona; serve una prova su aria con il proprietario |
| 2 | **Ritrasmissione**: poter sostituire il `Tx` dello **stesso periodo** quando la decisione cambia dopo l'avvio (equivalente di AutoSeq 3/7: si decide a fine decodifica, con la trasmissione gia' partita) | `sequencer.rs` (`next_tx` ritorna la copia in cache per lo stesso periodo), `tx-native` (riavvio dell'audio in corso) | M | senza questo il motore nativo decide solo prima dell'avvio (equivalente di AutoSeq 2/6) |
| 3 | **Valori di fabbrica dei contatori** (2/3/5/4, spenti) e **semantica completa**: AnswerCQ non vale per desiderati e nuovi DXCC; `reply_other` libera il DX solo con contatore acceso; CQ diretto altrui = `count 1`; cooldown `m_counter` dopo una rinuncia (per N periodi non si cercano altri CQ) | `types.rs` (`RepeatLimits`), `sequencer.rs` (`limit_for`) | S | oggi default 5/5/5/3 |
| 4 | **Banda di guardia** FT8 51 Hz, FT4 84: se il DX parla con un altro entro la banda dalla nostra Tx, **fermare la trasmissione** (indipendente dai contatori); con `SeqHaltTxReplyOther` sempre | `sequencer.rs` (`abandon_if_dx_answers_other`: oggi solo se `!dx.contacted` e non chiusura) | S | JTDX ferma Tx, non solo abbandona |
| 5 | **Transizioni di `QsoHistory`**: RR73/73/RRR ignorati senza `r_rep`; FIN solo quando le due chiusure si sono incrociate; un R+rapporto senza un nostro rapporto gia' inviato: JTDX **non avanza** (resta sul messaggio corrente, ad esempio Tx1), `qso-native` passa a Tx3 | `sequencer.rs` (`handle_dx_message`) | S | scegliere deliberatamente: Aurora puo' tenere il suo comportamento (valido sul protocollo), ma va dichiarato |
| 6 | **Scelta del chiamante come JTDX**: blocchi A, B, C, priorita' 0-31 (richiede le evidenziazioni «nuovo» dal registro e dal `cty.dat`), soglie `a_init`/`b_init`, regole di sorpasso (chi chiama direttamente batte chi manda RRR/CQ; il «fresco» batte chi si sta seguendo), `calllist` (5 minuti o SNR migliore), lista nera, direzione del CQ | `sequencer.rs` (`collect_callers`, `start_next_caller`, coda con TTL e tetto 20 di Decodium), `logbook`, `dxcc` | L | in alternativa tenere la coda attuale e il punteggio di Decodium (`analisi-decodium-reverse-rea.md`, sezione 2) come strategia scelta dall'utente; JTDX non ha coda a tempo |
| 7 | **Livelli AutoSeq**: oggi `auto_seq: bool`; servono `CallMode` 0-3 e «ricerca CQ» (6/7) con le regole di quando decidere (2.6) e il blocco C solo nel periodo che segue il periodo decodificato | `types.rs`, `sequencer.rs` | M | |
| 8 | **Watchdog a minuti**: conta solo i minuti con trasmissione recente (32 s FT8, 16 s FT4), si azzera a messaggio nuovo o a RCALL/RREPORT; oggi si misura il tempo reale dall'ultimo progresso | `sequencer.rs` | S | |
| 9 | **Fine QSO**: con Autolog JTDX ferma Tx a S73/FIN; il nativo deve avere una scelta esplicita «continua con CQ/prossimo chiamante» (e per `engine-jtdx` va ricordato che chi continua e' Aurora con `Reply`) | `sequencer.rs` (`end_qso`) | S | |
| 10 | **Frequenze**: Rx segue la frequenza del DX; Tx non viene spostata da AutoSeq (solo `LockTxFreq`/clic) | `sequencer.rs` | S | |
| 11 | **`bypass_tx1`**: in JTDX non esiste (il pulsante «Bypassa» e' il filtro dei testi): rinominare o togliere; il comportamento giusto e' il punto 1 | `types.rs` | S | |
| 12 | Doppio clic e **periodo**: opposto alla riga, AutoSeq non lo cambia, un cambio di periodo con Enable Tx e AutoSeq libera il DX | `sequencer.rs` (`reply_to`, `set_tx_even`) | S | |
| 13 | **Interfaccia verso il decoder**: passare allo `ft8` lo stato del QSO (`nQSOProgress`, `nlasttx`, DX, locatore, ultimo messaggio ricevuto e il suo ritardo, `lapmyc`, `ltxing`, `stophint`): serve alle analisi a priori e a `ft8s` | nuovo tipo condiviso (es. `QsoHint`) tra `qso-native` e `ft8` | M | vedi 7.2, punti 3-4 |
| 14 | Prove: i casi di 2.3 (una riga per transizione), 2.5 (contatori), 3.3 (confronti), 2.9 (avvio tardivo) come test del crate | `tests/scenari.rs` | M | |

### 7.2 Per `core/crates/ft8` (decoder)

Il crate e' gia' un porting fedele (sync8 con le tre forme, sync8d con i suoi raggruppamenti, `syncmin` a rotazione, soglie del sincronismo e tabelle dei filtri: i numeri di 4.1 sono quelli di `jtdxjt9.exe`; nel crate ho ritrovato a campione 3,63, 5,37 e 13,8 in `filters.rs`). Le differenze:

| # | Cosa | Dove | Diff. | Note |
|---|---|---|---|---|
| 1 | **Profondita'**: separare `cicli` (1..3 -> 3/6/9 passate) e `sensibilita'` (0, 1 soglie basse, 2 sottopassate) come in JTDX (di fabbrica 1 ciclo e sensibilita' 1); togliere «profondita' 1 = OSD d'ordine 2 e niente AP» (JTDX non usa mai `ndeep < 3` in FT8) e il legame «profondita' 3 = soglia bassa + sottopassate + 9 passate» | `decode.rs` (`Options::depth`) | S | oggi 3 livelli che mescolano due comandi |
| 2 | **OSD**: `ndeep` 3 di base, 4 per i segnali del QSO/del proprio nominativo, 5 nella decodifica a doppio clic; tabelle `ntau`, `npre1`, `npre2` | `ldpc.rs`, `decode.rs` | S | |
| 3 | **Cronologia dei periodi**: `evencopy`/`oddcopy` (fino a 130 messaggi con frequenza e ritardo, per parita'), `calldteven`/`calldtodd` (150 ritardi per nominativo), `lastrxmsg`; e le **sottopassate 3-11** che sommano in potenza e in ampiezza i simboli del periodo precedente per CQ, chiamate a noi e segnale del QSO (`csold`) | `decode.rs`, `sync.rs`, `ap.rs` | M | gia' elencate come mancanti in `ap.rs`; e' dove JTDX guadagna sui segnali deboli che si ripetono |
| 4 | **`ft8s`** (decoder del canale del QSO: 56 messaggi, soglie in 5.5) e **candidati virtuali** a +-5 s, modi `nqso` 2 e 3 | nuovo `qso_decoder.rs` | M-L | richiede il `QsoHint` (7.1 punto 13) |
| 5 | **`ft8sd`, `ft8sd1`, `ft8mf1`, `ft8mfcq`, `tone8`, `genft8sd`**: ridecodifica con il messaggio del periodo precedente | nuovo | L | non letti in dettaglio; 700 righe di Fortran |
| 6 | **Nominativi non standard e Hound**: AP 11-14, 40-44 (e 21-24, 111) con le tabelle di `tone8`/`tone8myc` | `ap.rs` | M-L | Hound anche fuori ambito |
| 7 | **Thread**: JTDX divide la banda in fette e ogni thread fa tutte le sue passate; con 1 thread il crate coincide. Per parallelizzare scegliere se imitare le fette (risultato dipendente dai thread) o fare altro | `decode.rs` | M | non e' un requisito di parita' |
| 8 | `ALLCALL7.TXT`: oggi `call_book` e' opzionale; con elenco incompleto si scartano stazioni vere. Usare la copia installata (100 515 nominativi, 30/6/2024) e un modo di aggiornarla | `filters.rs`, installazione | S | file nella cartella `share\jtdx` |
| 9 | `ncandthin`, `ndtcenter` (diradamento dei candidati), `lagcc` (AGC), `nfilter`/`nagainfil` (banda ristretta: +-60 / +-25 Hz, OSD 5) | `sync.rs`, `decode.rs` | S-M | utili per CPU debole e per il doppio clic |
| 10 | **Tempi**: JTDX avvia il decoder a ~14,7 s (14,4 con early start) e vive della finestra fino al successivo periodo; il decoder nativo deve consegnare entro quel margine per far valere i livelli 3/7 (7.1 punto 2) | `engine-native` | M | |
| 11 | **FT4**: i punti verificati (soglia 1,2, 40 iterazioni BP, 20/32 bit di sincronismo, tre cicli) coincidono con `ft4/src/decode.rs`; restano le AP per stato (`naptypes` 4 colonne) | `ft4/src/decode.rs` | S | da controllare |

**Priorita' consigliata**: (qso-native 1, 2, 3, 4) e (ft8 1, 2) prima; poi (qso-native 6, 7, 13) con (ft8 3, 4) insieme, perche' si vincolano; il resto dopo le registrazioni.

## 8. Cosa e' ancora ignoto

- **Esecuzione reale** di FIN/S73 con Autolog (`autoStopTx`): letta nel sorgente, non provata in aria; nei log reali non si distingue da un `Reply` di Aurora.
- Costanti dell'eseguibile non rilette: i livelli bassi delle priorita' (da 5 a 16), la logica di `ft8s` (le soglie si', 5.4), `ft8sd`, `ft8mf1`, `ft8mfcq`, `ft8_downsample` (uso di `lhighsens`), `agccft8`, `chklong8`, `filtersfree`, `callsign_q`/`call_q`, `chkgrid` (3 392 righe). Per queste vale il sorgente.
- Il motivo di `count = 1` per i CQ diretti altrui.
- Il comportamento di **Hound** (DXpedition) e dei modi JT65/JT9/T10.
- La CPU: i tempi di decodifica reali per profondita' non sono stati misurati (nessun avvio di JTDX).
- Gli effetti **misurati** (parita' decodifiche, guadagno delle sottopassate): nessun dato; servono registrazioni.

## 9. Indice degli indirizzi

`jtdx.exe` (base `0x400000`):

| Indirizzo | Funzione |
|---|---|
| `0x47d3e0` | `MainWindow::process_Auto` |
| `0x490db0` | `MainWindow::processMessage` |
| `0x474800` | `MainWindow::genStdMsgs` |
| `0x4c4350` | `MainWindow::guiUpdate` |
| `0x494220` | `MainWindow::readFromStdout` |
| `0x4bbad0` | `MainWindow::decode` |
| `0x478b70` | `MainWindow::txwatchdog` |
| `0x473f90` | `MainWindow::autoStopTx` |
| `0x4bd4e0` | `MainWindow::haltTx` |
| `0x4be1b0` | `MainWindow::clearDX` |
| `0x47c050`, `0x4a56c0`, `0x4ad430`, `0x47b130`, `0x4ac620`, `0x489560`, `0x4a8ce0`, `0x4d6e30` | `band_changed`, `displayDialFrequency`, `handle_transceiver_update`, `switch_mode`, `on_stopTxButton_clicked`, `clearDXfields`, `on_EraseButton_clicked`, `QsoHistory::init` (sezione 10) |
| `0x4ae8b0` | `MainWindow::on_TxMinuteButton_clicked` |
| `0x4ac7f0` | `MainWindow::on_AutoSeqButton_clicked` |
| `0x4a0b50`, `0x4a1060` | `doubleClickOnCall`, `doubleClickOnCall2` |
| `0x4d8260` | `QsoHistory::autoseq` |
| `0x4dbbf0` | `QsoHistory::message` |
| `0x4d5060`, `0x4dda60`, `0x4dd7d0`, `0x4dea10`, `0x4dded0`, `0x4d5fe0`, `0x4d70a0` | `reset_count`, `calllist`, `owndata`, `blacklist`, `log_data`, `rx`, `remove` |
| `0x4eb170` | `DisplayText::displayDecodedText` |
| `0x4f2810` | `DisplayText::displayTransmittedText` |
| `0x50b240` | `Configuration::impl::read_settings` |
| `0x51b890` | `Configuration::impl::write_settings` |
| `0x13c1ee8` | suggerimento «Avg/Lag» (testo sui livelli AutoSeq 2/6 e 3/7) |

`jtdxjt9.exe` (base `0x400000`):

| Indirizzo | Funzione |
|---|---|
| `0x42b3c0` | `__ft8_decode_MOD_decode` |
| `0x438420` | `ft8b_` |
| `0x4c9470`, `0x4ce840` | `sync8_`, `sync8d_` |
| `0x4eb150`, `0x505520` | `chkfalse8_`, `chkspecial8_` |
| `0x4f2eb0`, `0x4f2ff0`, `0x4b2130` | `chkflscall_`, `chkgrid_`, `searchcalls_` |
| `0x4562d0` | `ft8s_` |
| `0x45c960`, `0x4615b0`, `0x468b10` | `ft8sd_`, `ft8sd1_`, `genft8sd_` |
| `0x453960`, `0x455340` | `ft8mf1_`, `ft8mfcq_` |
| `0x484a30`, `0x4e9560` | `osd174_91_`, `bpdecode174_91_` |
| `0x423930` | `__ft4_decode_MOD_decode` |
| `0x46ad60`, `0x4c1a30` | `getcandidates4_`, `sync4d_` |
| `0x4024e0` | `jt9a_` |

## 10. Cambio di banda (REA, 2026-10-25)

Richiesta del proprietario: al cambio di banda si interrompe sempre la trasmissione, si svuota l'attivita' di banda e si cancella la stazione chiamante (Stazione DX).
Letto nei sorgenti (`mainwindow.cpp`, ramo `master`) e verificato in `jtdx.exe` 2.2.159 con REA (pseudocodice di ogni funzione sotto; i nomi sono i simboli veri).

### 10.1 Le funzioni (`REA 0x...`)

| Indirizzo | Funzione | Cosa fa al cambio di banda |
|---|---|---|
| `0x47c050` | `MainWindow::band_changed(Frequency)` | punto d'arrivo di ogni cambio **comandato** (combo banda, cambio modo, scheduler, doppio clic su una frequenza) |
| `0x4af1f0` | `on_bandComboBox_activated(int)` | imposta `m_bandEdited = true` e chiama `band_changed` (scrive «Band changed from bandComboBox» in `ALL.TXT`) |
| `0x47b130` | `switch_mode(Mode)` | se il modo e' cambiato: `QsoHistory::init()` (`REA 0x4d6e30`), poi `best_working_frequency` e `on_bandComboBox_activated` (quindi il cambio modo che sposta la frequenza passa da `band_changed`) |
| `0x4a56c0` | `displayDialFrequency()` | punto d'arrivo del cambio **dalla radio** (`handle_transceiver_update`, `REA 0x4ad430`, aggiorna `m_freqNominal` e poi chiama questa): se la banda e' cambiata (e non e' l'avvio) fa `QsoHistory::init()`; con l'opzione «Erase windows at band change» (`m_autoEraseBC`, predefinito **true**: `QSettings("EraseWindowsAtBandChange", QVariant(true))` in `readSettings`, `REA 0x47fa20`) svuota le due finestre e chiama `clearDX` |
| `0x4bd4e0` | `haltTx(QString)` | scrive l'evento e chiama `on_stopTxButton_clicked` |
| `0x4ac620` | `on_stopTxButton_clicked()` | `stop_tuning` se si sta accordando; `enableTx_mode(false)` (spegne «Abilita Tx», `m_enableTx`; «AutoTX», `m_autoTx`, e' un'altra cosa e non si tocca: sezione 11.8); `m_nlasttx = 0`; `m_haltTrans` se il PTT e' attivo; azzera `m_bTxTime`, `m_tx_when_ready`, `m_transmitting`, `m_restart`, `m_txNext`; con «Salta Tx1» e trasmissione in corso toglie il DX da `QsoHistory` |
| `0x48a230`, `0x4af3c0` | `enableTx_mode(bool)`, `on_enableTxButton_clicked(bool)` | spegne il pulsante e riallinea le variabili di Tx |
| `0x4be1b0` | `MainWindow::clearDX(QString)` (= pulsante «Erase» del DX, `on_ClearDxButton_clicked`, `REA 0x4acbf0`) | `clearDXfields` (`0x489560`: svuota nominativo e locatore del DX, azzera il nome), `genStdMsgs("")` (messaggi Tx rigenerati senza DX), `RxFreqSpinBox = TxFreqSpinBox`, seleziona Tx6 (CQ) o Tx7 (messaggio libero), `m_QSOProgress = CALLING`, `m_nlasttx = 6`, azzera il pulsante di spot DX |
| `0x4a8ce0` | `on_EraseButton_clicked()` | svuota la finestra Band Activity (`decodedTextBrowser`), il testo del QSO e `MessageClient::clear_decodes` (UDP); Rx Frequency solo col doppio clic entro 500 ms |
| `0x4d6e30` | `QsoHistory::init()` | scarta le tabelle dei QSO per nominativo e dei contatori (stato per stazione, rinunce): la «memoria» della sequenza |

### 10.2 Cosa succede, in ordine

1. **Cambio comandato** (`band_changed`, pseudocodice `REA 0x47c050`, righe `m_bandEdited`, `0x2b54` = `m_transmitting`, `0x31f` = `m_autoEraseBC`):
   a. «QSY cieco» (la nuova frequenza fa uscire dalla cascata la frequenza Tx): `haltTx("band changed ")` **solo se** si sta trasmettendo o il PTT e' attivo (`g_iptt == 1`), poi `enableTx_mode(false)`. **Non** e' incondizionato: un salto che lascia la frequenza Tx dentro la cascata non ferma niente. Con i salti di banda la frequenza e' sempre fuori dalla cascata, quindi in pratica si ferma.
   b. se cambia la banda (`Bands::find(nuova) != Bands::find(ultima monitorata)`) **o** il modo: `clearDX(...)` **sempre**, poi, con `m_autoEraseBC`, `decodedTextBrowser->clear()` e `decodedTextBrowser2->clear()` (Band Activity e Rx Frequency).
   c. se la nuova e la vecchia frequenza distano piu' di 1 MHz (`REA`: `1000000 < |diff|`): `QsoHistory::init()`, di nuovo `clearDX` e, se non l'ha gia' fatto, svuotamento delle finestre; poi la potenza per banda (`pwrBandTxMemory`).
2. **Cambio dalla radio** (`displayDialFrequency`, `REA 0x4a56c0`): nessun `haltTx` ne' `enableTx_mode`. Se la banda letta e' diversa da `m_lastBand` e dalla banda del combo, e non e' l'avvio: svuota le due finestre e `clearDX` (con l'opzione), `QsoHistory::init()` (sempre), potenza per banda. Nota: quindi con la radio che cambia banda a mano JTDX **non** ferma una trasmissione in corso (lo fa solo il PTT che cade: `handle_transceiver_update` fa `haltTx("Halt Tx from rig detected ")` se il PTT della radio si spegne mentre si trasmetteva).
3. **Cambio di modo** (`switch_mode`): `QsoHistory::init()` quando il modo cambia e, se la frequenza di lavoro del nuovo modo e' su un'altra banda, `on_bandComboBox_activated` -> `band_changed` (punto 1, con `clearDX` perche' `m_oldmode != m_mode`).
4. **Doppio clic su uno spot del cluster**: JTDX non ha un cluster; nel mondo reale la frequenza arriva dal CAT, cioe' dal punto 2. In Aurora e' il comando `/api/v1/commands/frequency`.
5. **Non** vengono toccati dal cambio di banda: l'AutoSeq (`m_autoseq` resta com'e'), la cache degli hash dei nominativi del decoder, la lista dei QSO di sessione per il log (`QsoHistory::init` azzera solo la memoria della sequenza), la lista dei messaggi gia' trasmessi.

### 10.3 Differenza voluta in Aurora

JTDX ferma la trasmissione al cambio di banda solo nel ramo «QSY cieco» (a) e solo se il cambio e' comandato dal programma. Il proprietario chiede di **fermare sempre**, da qualunque causa (pagina, radio, spot, cambio modo che sposta la frequenza): Aurora e' quindi piu' severo di JTDX nella sola parte di Halt e uguale per il resto (`clearDX` + `QsoHistory::init` + finestre svuotate). Le soglie: cambio di banda = `band_of(dial)` diverso dal precedente (con il precedente noto); uno spostamento di pochi Hz o kHz dentro la banda non fa niente.

## 11. Verifica con REA del 2026-10-25

Controllo sull'eseguibile `jtdx.exe` 2.2.159 (REA/Ghidra, pseudocodice delle funzioni; i nomi sono i simboli veri) di cio' che il motore proprio aveva copiato dai soli sorgenti.
Esito per punto: **coincide**, **corretto** (il binario faceva altro e il codice e' stato cambiato, con test) o **non verificabile**. Campi di `MainWindow` letti nel pseudocodice: `+0x311` `m_txFirst`,
`+0x312` `m_txGenerated`, `+0x314` `m_enableTx`, `+0x360` `m_lockTxFreq`, `+0x361` `m_skipTx1`, `+0x368` `m_autoTx`, `+0x369` `m_autoseq`, `+0x264` `m_ntx`, `+0x2b54` `m_transmitting`.

### 11.1 Salta Tx1 (`on_skipTx1_clicked`, `REA 0x4a1950`; `on_skipGrid_clicked`, `0x4a54a0`) - coincide
Imposta `m_skipTx1` e il gemello `skipGrid`. Acceso: se Tx1 e' il messaggio scelto passa a Tx2 (`m_ntx = m_QSOProgress = m_nlasttx = 2`, pallino Tx2, e `m_restart = true` se si trasmette) e,
se il campo del messaggio generato e' uguale a Tx1, lo porta a Tx2. Spento: il simmetrico da Tx2 a Tx1. Il motore proprio lo fa (`Sequencer::set_options`). **Differenza dichiarata**: JTDX, spegnendo «Salta Tx1» su Tx2,
torna a Tx1 anche se il DX ci aveva gia' scritto (con AutoSeq acceso il periodo dopo `process_Auto` rimette il messaggio giusto dallo stato); Aurora non lo fa per un QSO avviato (`!contacted`) perche' non ricalcola il
messaggio dallo stato a ogni periodo. Non implementato: il riavvio della trasmissione in corso (`m_restart`).

### 11.2 Pulsante periodo Tx (`on_TxMinuteButton_clicked`, `REA 0x4ae8b0`; `setMinButton`, `0x486f70`) - coincide
`m_txFirst = checked`; se si trasmette scrive «Tx halted: period changed via TX period button»; se `m_txGenerated != checked` **e** Abilita Tx **e** AutoSeq sono accesi, `clearDX(" cleared, Tx Minute button clicked")`.
Le etichette: «TX 00/30» / «TX 15/45» (FT8), «TX 00» / «TX 7.5» (FT4), «TX Even»/«TX Odd» (modi a minuto). Il motore proprio ha lo stesso comportamento (`Sequencer::set_tx_even`, `NativeRig::controls_and_tx`).

### 11.3 Doppio clic e periodo di risposta (`doubleClickOnCall` `0x4a0b50`, `doubleClickOnCall2` `0x4a1060`, `processMessage` `0x490db0`) - coincide, un punto corretto
`doubleClickOnCall` azzera il watchdog (`txwatchdog(false)`), prende la riga selezionata dalla finestra di destra o di sinistra (`doubleClickOnCall2` mette solo un flag, `+0x32a`, e richiama la prima) e chiama `processMessage`.
In `processMessage` il periodo e' `nmod = fmod(timeInSeconds, 2 * periodo)`: se `m_txFirst != (nmod != 0)` e la riga non e' una nostra trasmissione si pone `m_txFirst = (nmod != 0)` e si preme `TxMinuteButton` se serve
(quindi risposta nel periodo **opposto**: riga a :15/:45 -> TX 00/30). Il primo messaggio: se il messaggio non e' diretto a noi, Tx1, o Tx2 se `m_skipTx1` e non Hound (`m_ntx = 2`, `m_QSOProgress = 2`); se contiene
il nostro nominativo Tx2/Tx3/Tx4/Tx5 secondo il contenuto. Il motore proprio lo fa (`Sequencer::reply_to`).
**Corretto**: alla fine del doppio clic, con `m_autoTx` acceso e senza Alt, JTDX preme «Abilita Tx» (se non e' gia' acceso); con `m_autoTx` spento no. Il motore proprio accendeva sempre Abilita Tx: ora il doppio clic
lo accende solo con AutoTX acceso (sezione 11.8).

### 11.4 Gen.Msgs (`genStdMsgs`, `REA 0x474800`; `on_genStdMsgsPushButton_clicked`, `0x4b7690`) - coincide, un dettaglio corretto
In cima `m_txGenerated = m_txFirst` (`+0x312 = +0x311`, **prima** del controllo sul DX); senza DX svuota Tx1-Tx5 e il messaggio generato e ritorna; con il DX scrive i sei messaggi e rimette il messaggio di
partenza: con `m_skipTx1` (e non Hound) `m_ntx = m_nlasttx = 2`, altrimenti 1. **Corretto**: `Sequencer::regen_messages` impostava `tx_generated` solo con un DX; ora lo fa sempre (il caso senza DX ha effetto solo
sul successivo cambio di periodo, per questo non c'e' un test dedicato).

### 11.5 Rx->Tx, Tx->Rx e Split (`on_pbR2T_clicked` `0x49d600`, `on_pbT2R_clicked` `0x49d640`, `on_pbTxLock_clicked` `0x4a4b30`) - coincide, un valore iniziale corretto
`pbR2T`: `TxFreqSpinBox.setValue(RxFreqSpinBox.value())` solo se la casella Tx e' abilitata; `pbT2R` il contrario. `pbTxLock`: `m_lockTxFreq = checked` (+0x360), scritte «Lockd Tx=Rx» (acceso) / «Tx/Rx Split» (spento),
`WideGraph::setLockTxFreq`, e se acceso richiama `on_pbR2T_clicked`. Quindi **acceso = bloccato (Tx = Rx), spento = Split**; il motore proprio usa l'id `split` con il senso della scritta (acceso = Split) e spegnerlo
porta Tx = Rx come JTDX. Le sorgenti aggiungono che con il blocco acceso Rx trascina Tx e Tx trascina Rx (`on_RxFreqSpinBox_valueChanged`, `on_TxFreqSpinBox_valueChanged`): fatto.
**Corretto**: di fabbrica `LockTxFreq` e' **falso** (`MainWindow::readSettings`, `REA 0x47fa20`: `value("LockTxFreq", false)`), cioe' Split **acceso**; il motore proprio partiva con Split spento. Ora parte acceso.
Nota per `engine-jtdx` (non toccato): li' `split` e' mappato su `pbTxLock` senza inversione, quindi pigiare «Split» accenderebbe il blocco.

### 11.6 Messaggio libero a 13 caratteri e ripetizione (`pack77` `0x4244f0`, `packtext77` `0x41c300` / `0x41af00`, `on_rbFreeText_clicked` `0x4a7e70`, `guiUpdate` `0x4c4350`) - 13 caratteri coincide, ripetizione corretta
`pack77` prova i tipi standard e, se nessuno va, chiama `packtext77` con **13 caratteri** (`param_3 = 13`, `adjustr`, alfabeto di 42 simboli): il testo libero e' tagliato ai primi 13. Coincide. Il campo accetta fino a 37 caratteri
(`character*37`). Il messaggio libero e' `m_ntx = 8` (`on_rbFreeText_clicked`: `m_ntx = m_nlasttx = 8`, `m_QSOProgress = SIGNOFF`, riavvio se si trasmette); in `guiUpdate` l'unica scrittura di `m_ntx` e' il ripiego a 7
(`REA 0x4c4350`, riga 1235, a Abilita Tx spento dopo il primo 73): **il messaggio libero non viene tolto dopo l'invio e si ripete a ogni periodo di trasmissione** finche' Abilita Tx e' acceso. **Corretto**: il motore proprio lo
mandava una volta sola; ora lo ripete (nella parita' scelta) fino a «Ferma Tx», spegnimento di Abilita Tx, scelta di un altro messaggio, doppio clic, CQ o Gen.Msgs (test `a_free_text_repeats_every_period_of_the_chosen_parity_until_halted`).
Non implementato: con un testo libero che non e' un 73 (`isAutoSeq73`) JTDX spegne AutoSeq (`m_FTsetAutoSeqOff`) e lo riaccende dopo. Scelta di Aurora, non verificabile come comportamento di JTDX: «Invia» accende anche Abilita Tx
(in JTDX il comando `send` dell'UDP preme solo il pallino del messaggio libero).

### 11.7 Livello di uscita, cursore Pwr (`on_outAttenuation_valueChanged` `0x4b69c0`, `setupUi` `0x579570`, `readSettings` `0x47fa20`, `band_changed` `0x47c050`) - coincide, memoria per banda aggiunta
- Cursore: `QSlider` **verticale** (orientamento 2), minimo 0, massimo 450 (`0x1c2`), **non invertito** (`setInvertedAppearance(false)`), valore iniziale della finestra **1**. L'attenuazione e' `(450 - valore) / 10` dB
  (`0x1c2 - param_2`), cioe' valore 450 = 0 dB (massima potenza), valore 0 = 45 dB; in un `QSlider` verticale non invertito il minimo sta in basso e il massimo in alto: **0 dB in alto** (potenza massima), 45 dB in basso. CORREZIONE del 2026-10-25: una versione precedente di questa nota diceva «0 dB in basso» per un errore di lettura; il `JTDX.ini` del proprietario ha `OutAttenuation=450` (cursore in alto) e il sorgente (`on_outAttenuation_valueChanged`: `areversed = 450 - a`) lo conferma.
  `JTDX.ini` del proprietario, `OutAttenuation=450`). In Aurora l'attenuazione e' in decimi di dB e **coincide col valore del cursore di JTDX** (`attenuazione = 450 - valore`, quindi 225 e' 225).
- Valore di fabbrica: `OutAttenuation` letto solo se in 0..450, predefinito **225** (`0xe1`), cioe' 22,5 dB. **Corretto**: il motore proprio partiva da 0 dB; ora 225.
- Il caso `value == 1` (`band_changed`, riga 138 del pseudocodice; `mainwindow.cpp:6587`): il cursore nasce a 1 come segnaposto; alla **prima** `band_changed`, se vale 1 e `m_outAttenuation != 1`, si imposta il valore salvato
  (`m_outAttenuation`, 225 di fabbrica) e `m_outAttenuation = 1` perche' non si ripeta. E' un artificio dell'avvio: in Aurora basta partire dal valore iniziale.
- Memoria per banda (`m_pwrBandTxMemory`, `QHash<banda+modo, valore>`, nella `band_changed` e in `displayDialFrequency`): con l'opzione `pwrBandTxMemory` (di fabbrica **falso**: `Configuration::read_settings`, `REA 0x50b240`, campo
  `+0x9c0`; nel `JTDX.ini` del proprietario `pwrBandTxMemory=false`) e non in accordatura, se la banda cambia (oltre 1 MHz) **e la chiave c'e'** il cursore prende il valore ricordato (con `m_PwrBandSetOK = false` per non
  riscriverlo), **altrimenti** si ricorda il valore attuale per quella chiave. Ogni cambio del cursore lo riscrive per la banda e il modo correnti (`on_outAttenuation_valueChanged`, solo con `m_PwrBandSetOK`).
  **Aggiunto** nel motore proprio (`NativeControls::track_band` e `set_attenuation`, chiamate da `engine-native/src/lib.rs` a ogni lettura della banda): sempre attiva, come chiesto (JTDX la ha spenta di fabbrica), con chiave
  banda + modo; la prima banda vista ricorda il valore attuale. Non persiste tra un avvio e l'altro (JTDX la salva in `pwrBandTxMemory` delle impostazioni). Test: `the_output_level_is_remembered_per_band_and_restored_on_a_band_change`.

### 11.8 «AutoTX» e «Abilita Tx» sono due cose diverse - corretto
| | **Abilita Tx** (`enableTxButton`, `m_enableTx` `+0x314`) | **AutoTX** (`AutoTxButton`, `m_autoTx` `+0x368`, chiave `QuickCall`) |
|---|---|---|
| Cosa e' | il consenso a trasmettere: la finestra `m_bTxTime` e l'avvio del PTT in `guiUpdate` richiedono `m_transmitting`, `m_enableTx` o `m_tune` | solo «il doppio clic accende Abilita Tx» (e `Reply` UDP, che passa da `doubleClickOnCall2`) |
| Funzione (`REA`) | `on_enableTxButton_clicked` `0x4af3c0`: imposta `m_enableTx`; spento e senza trasmissione azzera `m_bTxTime`, `m_tx_when_ready`, `m_restart`, `m_txNext` | `on_AutoTxButton_clicked` `0x4aa4d0`: imposta **solo** `m_autoTx` (e lo stile del pulsante) |
| Stato iniziale | spento (`m_enableTx{false}`) | **acceso** (`value("QuickCall", true)`, `readSettings` `0x47fa20`) |
| Chi lo cambia | l'utente, il doppio clic con AutoTX, Halt Tx, watchdog, QSY cieco, fine QSO | solo l'utente |

- **Avvio al periodo giusto**: Abilita Tx non parte da solo a inizio periodo: `m_bTxTime = (t2p >= tx1) && (t2p < tx2)` (finestra dei primi 13,64 s) e `guiUpdate` fa partire la trasmissione appena la finestra e' aperta, anche in ritardo (2.9).
  AutoTX non partecipa: serve soltanto al doppio clic: `if (m_autoTx && !alt) { se Abilita Tx non e' acceso, click }`; con Alt e Abilita Tx acceso, `haltTx("ALT modifier is used at double click")`.
- **Halt Tx e Esc** (`haltTx` `0x4bd4e0` -> `on_stopTxButton_clicked` `0x4ac620`; `keyPressEvent`: Esc = `haltTx("TX halted via Escape button")`, Alt+H = `on_stopTxButton_clicked`): spegne **solo** Abilita Tx
  (`enableTx_mode(false)` se `!m_tuneup`) e azzera `m_btxok`, `m_nlasttx = 0`, `m_bTxTime`, `m_transmitting`, `m_restart`, `m_txNext`, spegne il filtro automatico; **non tocca `m_autoTx` (+0x368) ne' AutoSeq (+0x369)**.
- **AutoSeq**: nessuna dipendenza di stato; `on_AutoSeqButton_clicked` cambia solo lo stile (rosso) di AutoTX quando AutoSeq e' acceso e AutoTX spento, per il caso «Abilita Tx a mano sotto AutoSeq». `m_enableTx && m_autoseq` insieme
  contano per `clearDX` al cambio periodo (11.2), per il watchdog e per `autoStopTx`.
- **Fine QSO**: con AutoSeq, `autoStopTx` -> `haltTx` (2.4); senza AutoSeq, se «Disable Tx after sending 73» e' acceso (`m_disable_TX_on_73`) un timer di 100 ms chiama `enableTxButton_off` (`enableTx_mode(false)`) dopo il 73 inviato.
  AutoTX non c'entra. Dopo il log, `clear_DX` libera il DX con un timer **solo se AutoTX e AutoSeq sono entrambi spenti** (`mainwindow.cpp:6069`).
- **Altri spegnimenti di Abilita Tx**: watchdog (`txwatchdog`: «Tx watchdog expired»), frequenza Tx in una sottobanda WSPR, cambio di impostazioni audio, QSY cieco; mai di AutoTX.
- **`tx_status_label`**: solo barra di stato: «Tx: <messaggio>» giallo mentre si trasmette (`m_transmitting`), «Tx: TUNE», verde quando e' vuoto/in attesa, «Tx watchdog expired» rosso; non mostra AutoTX.

**Corretto** nel motore proprio: `NativeRig::set_control` mandava `auto_tx` e `enable_tx` entrambi a `TxCommand::Enable`, `snapshot` mostrava `auto_tx` uguale a `enabled` e AutoTX partiva spento. Ora: `TxCommand::AutoTx(bool)` distinto
(solo `TxDriver::auto_tx`, che decide se il doppio clic / `Reply` accende Abilita Tx; `Sequencer::reply_to_with`), interruttore `auto_tx` a se' (di fabbrica acceso), `enable_tx` = stato della sequenza; `TxView.enabled` (cioe' `tx_enabled`
verso la UI) resta Abilita Tx e lo schema non cambia. Test: `auto_tx_and_enable_tx_are_separate_commands_and_separate_states`, `the_double_click_enables_tx_only_with_auto_tx_on`, `the_initial_state_is_the_one_of_jtdx`.
**Differenze lasciate in sospeso e come si sono chiuse (2026-10-25, con il binario)**

| Differenza | Cosa dice il binario | Decisione |
|---|---|---|
| Esc spegne anche AutoTX | `keyPressEvent` -> `haltTx` (`0x4bd4e0`) -> `on_stopTxButton_clicked` (`0x4ac620`): azzera `m_enableTx`, `m_btxok`, `m_bTxTime`, `m_transmitting`, `m_restart`, `m_txNext`; **non scrive `m_autoTx` (+0x368)** | **Adattato**: `useEscapeStop` manda solo Halt e `enable_tx` spento, mai `auto_tx`. Il testo del suggerimento `tip.gen.escStop` (sei lingue) non nomina piu' AutoTX. Test aggiornati |
| «Invia» del messaggio libero accende Abilita Tx | `on_rbFreeText_clicked` (`0x4a7e70`) e `genStdMsgs` non toccano `m_enableTx`; il consenso a trasmettere e' solo `enableTxButton` | **Adattato**: `FreeText { send }` sceglie il messaggio e lo pubblica, **non** accende Abilita Tx; `tip.tx.free` lo dice. I test del messaggio libero premono `enable_tx` come l'utente |
| «CQ» accende Abilita Tx | `on_txb6_clicked` sceglie Tx6 senza accendere `m_enableTx` | **Resta diverso, voluto**: il pulsante CQ della pagina e' un'azione in un solo gesto (scegli il CQ e parti, come il tasto «CQ» dei programmi concorrenti). Si puo' spegnere subito con Esc. Non e' un errore di logica: il periodo e il messaggio sono quelli di JTDX |
| AutoSeq con testo libero diverso da 73 | in `guiUpdate` (`0x4c4350`) un messaggio libero che non e' «73» spegne AutoSeq (`m_autoseq = false`) e il pulsante; il ritorno e' manuale | **Adattato**: `Sequencer::note_free_text` spegne AutoSeq per un testo libero diverso da `73` (`is_autoseq_73`) e `auto_seq_effective` lo pubblica; il testo libero viene pianificato una volta sola per periodo. Test in `regole_jtdx.rs` e in `engine-native/src/tx.rs` |
| `split` mappato su `pbTxLock` senza inversione | `on_pbTxLock_clicked` (`0x4a4b30`): `pbTxLock` **acceso** = Tx bloccata sulla Rx, cioe' Split **spento** | **Corretto**: `rig-control/src/controls.rs` ha `Control.inverted`; lettura e comando invertono lo stato (`split` = non `pbTxLock`). Test `split_is_the_opposite_of_the_tx_lock_button` |

## Registrazione dei QSO (REA, 2026-10-25)

Richiesta urgente del proprietario: con il motore proprio (`AURORA_ENGINE=native`, senza JTDX) i QSO registrati non arrivavano piu' a Log4OM2. Letto nei sorgenti (`mainwindow.cpp`, `logqso.cpp`,
`qsohistory.cpp`, `Logbook/adif.cpp`, ramo `master`) e verificato in `jtdx.exe` 2.2.159 con REA (pseudocodice; nomi di funzione = simboli veri). I sorgenti coincidono con l'eseguibile in tutti i punti sotto.

### Perche' Log4OM2 non riceveva piu' niente

Con JTDX, a mandare il record a Log4OM2 era JTDX stesso (secondo server UDP, `127.0.0.1:2237` su questo PC); Aurora ne era solo spettatrice e il suo esportatore non aveva destinatari. Con il motore proprio
JTDX non c'e' piu' e **nessuno** mandava il QSO a Log4OM2: l'evento `QsoLogged` arrivava solo al log SQLite interno di Aurora. In piu' il record che l'esportatore sapeva fare non era quello di JTDX
(campi mancanti, `MODE` di FT4, data presa dall'ora di arrivo dell'evento, fine QSO assente, nessun nominativo/locatore propri).

### 1. Quando JTDX registra (`MainWindow::guiUpdate`, `REA 0x4c4350`; `on_logQSOButton_clicked`, `REA 0x4ab440`; `acceptQSO2`, `REA 0x472db0`)

- **Momento**: quando **comincia a trasmettere** un messaggio che e' un 73 (`message_is_73`: contiene «73» o «RR73»; `RRR` no), a patto che `m_QSOProgress >= ROGER_REPORT` (o `m_ntx` valga 4 o 5), che l'ultimo DX registrato sia diverso
  e che il messaggio precedente non fosse gia' un 73 per questo DX. Confermato nel binario (`REA 0x4c4350`, righe 1090-1150: `m_sentFirst73`, `prompt_to_log() || autolog()`, `!m_tune`, `QTimer::start(0)` su `logQSOTimer`).
  Non conta se il DX ha mandato il suo 73: nessun controllo sul messaggio ricevuto. Il rapporto ricevuto **non e' obbligatorio**: `log_data` (`REA 0x4dded0`) restituisce lo stato e i rapporti della cronologia, e se mancano si usano `m_rptSent`/`m_rptRcvd` (anche vuoti).
- **Guardia contro i doppioni**: `m_lastloggedcall != m_hisCall` **oppure** sono passati piu' di `int(periodo) * 2000` ms (30 s FT8, 14 s FT4) dall'ultima registrazione (`m_lastloggedtime`). Nel ramo Hound (`RRR73`, process_Auto) il tetto e' `periodo * 7000` ms.
- **Con «Prompt me to log QSO»** (`PromptToLog`) si apre la finestra di conferma, con `AutoQSOLogging` (`autolog()`) il dialogo si accetta da solo (`initLogQSO(..., autologging)` chiama `accept()` subito). Su questo PC: `PromptToLog=false`, `AutoQSOLogging=true`.
- **Orari**: `QSO_DATE_OFF`/`TIME_OFF` = ora attuale (UTC) **+ 14 s** in FT8, **+ 7 s** in FT4, **+ 50 s** negli altri modi (`on_logQSOButton_clicked`, costanti `0xe`, `7`, `0x32` nel binario). `QSO_DATE`/`TIME_ON` = ora di inizio dalla cronologia (`QsoHistory::log_data` -> `b_time`, secondi del giorno
  del primo messaggio del QSO, cioe' l'inizio del periodo), con un giorno in meno se supera la fine. Verifica su `wsjtx_log.adi` reale: inizio `164645`, fine `164729` (periodo di trasmissione `164715` + 14); FT4 `162500` / `162522` (`162515` + 7).
- **Dopo la registrazione** (`acceptQSO2`): `m_lastloggedcall`/`m_lastloggedtime`, riga `QSO logged: <nominativo>` in `ALL.TXT`, etichetta «Logd», `countQSOs`. Il DX viene azzerato da `logClearDXTimer` (periodo meno secondi trascorsi, in ms) **solo** con «Clear DX» e se non c'e' AutoTX/AutoSeq
  (`!m_autoTx && !m_autoseq`): con AutoSeq acceso la sequenza passa da sola al chiamante dopo.

### 2. Cosa manda JTDX (`MainWindow::acceptQSO2`, `REA 0x472db0`)

In quest'ordine, per ogni QSO registrato:
1. `LogBook::addAsWorked` (lista dei lavorati in memoria).
2. `MessageClient::qso_logged` (`REA 0x569b60`): messaggio **WSJT-X tipo 5** al **primo** server UDP (qui `127.0.0.1:2334`), campi: ora di fine, nominativo, locatore, frequenza (u64), modo, rapporto inviato, rapporto ricevuto, potenza, commento, nome, ora di inizio, nominativo operatore, nominativo stazione, locatore proprio.
3. Se `EnableUDP1adifSending`: `MessageClient::logged_ADIF` (`REA 0x56a6d0`): messaggio **tipo 12** allo stesso server, con il testo `"\n<adif_ver:5>3.1.0\n<programid:4>JTDX\n<EOH>\n"` + il record ADIF.
4. Se `EnableUDP2adifBroadcast` e secondo server valido: **un datagramma UDP con il solo record ADIF (`myadif2`)** a `UDP2Server:UDP2ServerPort` (qui `127.0.0.1:2237` = Log4OM2). Un solo invio per QSO.
5. eQSL (se acceso), `ALL.TXT`.
Il file `wsjtx_log.adi` e il `wsjtx.log` li scrive il dialogo del log (`LogQSO::accept`, `REA 0x60be60`) **prima** di emettere `acceptQSO`; la connessione TCP (`EnableTCPConnection`, comando `Log`) e' un altro percorso, spento qui.

### 3. Il record ADIF (`LogQSO::accept`, `REA 0x60be60`)

Datagramma per Log4OM2 (`myadif2`), campi nomi in **maiuscolo**, separati da **uno spazio**, `.trimmed()` (nessun «a capo», nessuno spazio finale), senza intestazione, chiuso da `<EOR>`. Ordine verificato nel binario:
`BAND`, `STATION_CALLSIGN`, `MY_GRIDSQUARE` (8 caratteri se il locatore ne ha almeno 8, intero se almeno 4, altrimenti assente), `CALL`, `FREQ` (MHz con **6 decimali**, `QString::number(f/1e6,'f',6)`, frequenza di dial + frequenza Tx audio),
`MODE` (per FT4: `<MODE:4>MFSK <SUBMODE:3>FT4`), `QSO_DATE`, `TIME_ON`, `QSO_DATE_OFF`, `TIME_OFF`, `RST_SENT`, `RST_RCVD` (**sempre**, anche vuoti: `<RST_RCVD:0>`), `TX_PWR` (solo se non vuota), `GRIDSQUARE` (solo se il locatore del DX ha piu' di 3 caratteri),
`NAME` e `COMMENT` (**`COMMENT`, non `COMMENTS`**; solo se non vuoti), per eQSL `EQSL_QSL_SENT:1>Y <EQSL_QSLSDATE:8>`. Niente `OPERATOR`, niente `SUBMODE` per FT8, niente `TIME_OFF` diverso da quello sopra.
Esempio (inventato): `<BAND:3>20m <STATION_CALLSIGN:6>IU3ABC <MY_GRIDSQUARE:6>IO91wm <CALL:5>K1ABC <FREQ:9>14.075084 <MODE:3>FT8 <QSO_DATE:8>20261024 <TIME_ON:6>164645 <QSO_DATE_OFF:8>20261024 <TIME_OFF:6>164729 <RST_SENT:3>-16 <RST_RCVD:3>-09 <GRIDSQUARE:4>FN42 <EOR>`.

### 4. I file (`ADIF::addQSOToFile`, `REA 0x5ec2c0`; `LogQSO::accept`)

- `wsjtx_log.adi` nella cartella dati di JTDX: aperto in aggiunta e testo (CR LF); se vuoto, prima riga `WSJT-X ADIF Export<eoh>`. Riga: `call`, `gridsquare` (**anche vuoto**, `<gridsquare:0>`), `mode` (+ `submode`), `rst_sent`, `rst_rcvd`, `qso_date`, `time_on`, `qso_date_off`, `time_off`, `band`, `freq`, `station_callsign`,
  `my_gridsquare` (intero), `tx_pwr`, `comment`, `name` (gli ultimi tre solo se non vuoti), `eqsl_qsl_sent`, `<eor>`; **nomi in minuscolo, ordine diverso dal datagramma**.
- `wsjtx.log` (CSV: `aaaa-mm-gg,hh:mm:ss,` inizio, fine, nominativo, locatore, frequenza, modo, rapporti, potenza, commento, nome).
- `ALL.TXT`: riga `QSO logged: <nominativo>` (e `Transmitting ... RR73` sei millisecondi prima, per l'ordine).

### 5. Cosa non coincideva in Aurora e cosa e' stato corretto

| Punto | Prima | Ora |
|---|---|---|
| Destinatario Log4OM2 con il motore proprio | nessuno | **secondo server UDP di JTDX** letto da `JTDX.ini` (`EnableUDP2adifBroadcast`, `UDP2Server`, `UDP2ServerPort`) in `desktop/src-tauri/src/settings.rs::udp2_from_ini`; con motore JTDX niente (lo fa lui: niente doppioni). `AURORA_EXPORT_UDP` dall'ambiente vince |
| Record ADIF | `CALL, GRIDSQUARE, BAND, MODE, FREQ, RST_*, QSO_DATE, TIME_ON`, a campi senza spazi, con «a capo», data = ora in cui arrivava l'evento | `logbook::adif::jtdx_record`: stessi campi, ordine, spazi, `MFSK`+`SUBMODE FT4`, `QSO_DATE_OFF`/`TIME_OFF`, `STATION_CALLSIGN`, `MY_GRIDSQUARE`, `TX_PWR`, `<EOR>` senza «a capo»; test su record campione |
| Orari | solo ora del giorno di fine, data di oggi | `QsoRecord::time_on_unix_ms` / `time_off_unix_ms`: inizio = primo periodo del QSO, fine = trasmissione del 73 + 14 s (FT8) o + 7 s (FT4), come JTDX; due date a cavallo della mezzanotte |
| File | `aurora_log` non esisteva; formato generico | `aurora_log.adi` nella cartella dei dati con il motore proprio (`Target::File`, `jtdx_file_record`): intestazione `WSJT-X ADIF Export<eoh>`, CR LF, campi minuscoli come `wsjtx_log.adi`; `AURORA_EXPORT_FILE` aggiunge un secondo file. Non si tocca mai il file di JTDX |
| Lavorati | solo `AURORA_WORKED_ADIF` (file di JTDX) | legge anche `aurora_log.adi`; nel motore proprio un QSO appena registrato non e' piu' una novita' (`record_qso`) |
| Doppioni | `Logbook::add` a ogni evento | guardia come JTDX nell'esportatore: stesso nominativo entro 2 periodi (30 s FT8, 14 s FT4) = non e' un altro QSO; un solo datagramma per QSO |
| QSO senza rapporto ricevuto | non si registrava mai | si registra quando si comincia a trasmettere RR73/73 (`Tx4` non-RRR / `Tx5`), con `RST_RCVD` vuoto, come JTDX (in ricezione serve ancora il rapporto) |
| Banda sconosciuta | record muto | avviso nel registro (radio non letta) |

Test: `logbook` (`adif::tests`, record UDP e riga del file su un campione, FT4, mezzanotte, locatore corto/lungo, rapporto vuoto), `qso-export` (datagramma esatto su un socket locale, un solo invio, guardia doppioni, file con una sola intestazione),
`engine-native/tests/qso_to_log.rs` (un QSO chiuso dalla sequenza con `TxDriver` + finto trasmettitore -> `Exporter` -> datagramma UDP e riga di file), `aurora` (`export_targets`), `desktop` (`settings`, il secondo server UDP diventa destinatario solo con il motore proprio), `qso-native/tests/scenari.rs` (73 senza rapporto).

### 6. Differenze volute e cosa resta

- **Momento in ricezione**: se il QSO si chiude perche' il DX manda RR73/RRR, la sequenza di Aurora lo registra gia' alla ricezione (JTDX aspetta di cominciare a trasmettere il 73). Il QSO e' lo stesso, registrato una volta sola, e `TIME_OFF` e' calcolato come se il 73 partisse nel periodo dopo
  (+14 s): il record e' identico a quello di JTDX. Se la trasmissione venisse spenta prima del 73, Aurora registra comunque (JTDX no).
- **`NAME` e `COMMENT`** non ci sono (Aurora non ha la rubrica ne' le opzioni «distanza/rapporti nei commenti»); `TX_PWR` solo con `AURORA_TX_POWER` (watt). Niente eQSL, niente `wsjtx.log` (CSV), niente riga `QSO logged` in un `ALL.TXT` (Aurora ha il suo registro).
- **GridTracker** (QSO Logged tipo 5, Logged ADIF tipo 12, Heartbeat, Status, Decode): fatto, vedi la sezione «Flusso UDP WSJT-X verso GridTracker e Log4OM (REA, 2026-10-25)» (crate `udp-emitter`).
- Prima del primo QSO, se la radio non e' ancora stata letta (`dial_hz == 0`), banda e frequenza del record sono vuote: l'avviso nel registro lo segnala; non si inventa una frequenza.

## Flusso UDP WSJT-X verso GridTracker e Log4OM (REA, 2026-10-25)

Richiesta urgente del proprietario: con il motore proprio «il call in chiamata non arriva piu' al log di Log4OM2». Il percorso, quando c'era JTDX: JTDX parla il protocollo UDP di WSJT-X al suo **primo** server UDP (qui `127.0.0.1:2334` = hub di Aurora),
l'hub inoltra gli stessi datagrammi a GridTracker2 (`AURORA_UDP_FORWARD`, `127.0.0.1:2333`), GridTracker inoltra a Log4OM2, che cosi' mostra il nominativo in chiamata (e gli spot). Con `AURORA_ENGINE=native` nessun pacchetto WSJT-X partiva piu' verso la 2333:
ne' Heartbeat, ne' Status, ne' Decode, ne' QSO Logged. Letto nei sorgenti (`MessageClient.cpp`, `NetworkMessage.hpp`, `mainwindow.cpp`, `Configuration.cpp`, ramo `master`) e verificato in `jtdx.exe` 2.2.159 con REA (pseudocodice dei simboli veri
`MessageClient::...`, `MainWindow::statusUpdate`, `MainWindow::postDecode`). I sorgenti coincidono con l'eseguibile in tutti i punti sotto.

### 1. Indirizzi (REA)

| Funzione | `REA` | Note |
|---|---|---|
| `MessageClient::MessageClient` | `0x56e800` | costruttore: schema iniziale **2** (`impl+0x40 = 2`), `QTimer` del Heartbeat a **15000 ms**, `bind` su porta effimera (0), `QHostInfo::lookupHost` del server |
| `MessageClient::impl::host_info_results` | `0x56d1b0` | a risoluzione finita: primo indirizzo, se non e' bloccato; manda **subito un Heartbeat** e poi svuota la coda dei messaggi accodati prima |
| `MessageClient::impl::heartbeat` | `0x56dd80` | tipo 0; scrive lo schema massimo (**3**, costante `Builder::schema_number` con Qt >= 5.4) e la **versione**; non passa dal filtro dei doppioni |
| `MessageClient::status_update` | `0x56abc0` | tipo 1 |
| `MessageClient::decode` | `0x56e420` | tipo 2 |
| `MessageClient::clear_decodes` | `0x56aa20` | tipo 3, solo l'intestazione; chiamanti: `0x4a8ce0` (`on_EraseButton_clicked`) e `0x49af50` (clic destro su Erase) |
| `MessageClient::qso_logged` | `0x569b60` | tipo 5 |
| `MessageClient::logged_ADIF` | `0x56a6d0` | tipo 12 |
| `MessageClient::impl::closedown` | `0x56dc70` | tipo 6; lo chiama il distruttore (`0x56df00`/`0x56e130`) |
| `MessageClient::impl::send_message` | `0x56c0d0` (`.part.28` `0x56c220`) | filtro dei doppioni e coda |
| `MessageClient::impl::parse_message` | `0x56c360` | legge i comandi di ritorno |
| `MainWindow::statusUpdate` | `0x64d4c0` | raccoglie i campi dello Status dalla finestra; chiamanti `0x474610` (`decodeBusy`), `0x478b70` (`txwatchdog`) e altri 11 |
| `MainWindow::postDecode` | `0x477540` | spezza la riga del decoder e chiama `MessageClient::decode`; chiamanti `0x494220` (`readFromStdout`) e `0x48af80` (replay) |
| `MainWindow::replyToUDP` | `0x477860` | comando Reply (e Status forzato) |

### 2. Formato esatto

Tutto big-endian, `QDataStream` di Qt. Intestazione di ogni datagramma: `magic u32 = 0xadbccbda`, `schema u32`, `tipo u32`, `id` (stringa Qt: `u32` lunghezza + byte UTF-8, `0xFFFFFFFF` = nulla). Lo schema **parte da 2** e sale a quello del server (massimo 3) quando
arriva un datagramma con **lo stesso id** e schema piu' alto (`parse_message`); la versione di `QDataStream` cambia solo per il `QDateTime` (Qt 5.2/5.4), che JTDX scrive in UTC: `giorno giuliano i64`, `millisecondi del giorno u32`, `tipo di fuso u8 = 1`.
Il Builder rifiuta schemi oltre il 3.

**Identificativo di istanza**: `QApplication::applicationName()`, impostato in `main()` a **`JTDX`** (`a.setApplicationName("JTDX")`; con `--rig-name nome` diventa `JTDX - nome`, e il file delle impostazioni `JTDX - nome.ini`). **Non** `WSJT-X`. Su questo PC il file e' `JTDX.ini`, quindi l'id e' `JTDX`.
I datagrammi di ritorno si leggono solo se portano lo stesso id (`id_ == in.id()`): per questo i comandi devono riconoscere `JTDX`.

| Tipo | Campi, nell'ordine |
|---|---|
| 0 Heartbeat | `schema massimo u32` (3), `versione` (stringa, `2.2.159`). **Niente revisione** (WSJT-X la manda) |
| 1 Status | `frequenza di dial u64`, `modo` (`FT8`/`FT4`), `DX call`, `rapporto` (testo del `rptSpinBox`, es. `-15`), `modo Tx` (uguale al modo), `Tx abilitato bool`, `trasmette bool`, `decodifica bool` (`m_decoderBusy`), `Rx DF i32`, `Tx DF i32`, `mio nominativo`, `mio locatore`, `locatore DX`, `watchdog Tx bool`, `sottomodo` (**nullo**: `QChar submode {0}`), `fast_mode bool` (sempre `false`), e **un solo byte finale `tx_first`** (`m_txFirst`: il periodo di trasmissione e' il primo, quello pari). |
| 2 Decode | `nuovo bool`, `ora u32` (`QTime`: millisecondi dalla mezzanotte; l'ora e' quella del periodo, `hhmmss` in FT8/FT4), `SNR i32`, `dt` (`float` scritto come **double**), `frequenza audio u32`, `carattere del modo` (**`~` per FT8, `:` per FT4**, come in `ALL.TXT`), `messaggio`, `bassa confidenza bool`, `off air bool` (`m_diskData`: file WAV) |
| 3 Clear | solo intestazione |
| 5 QSO Logged | `ora di fine QDateTime`, `nominativo DX`, `locatore DX`, `frequenza u64` (**dial + frequenza Tx audio**: `m_freqNominal + TxFreqSpinBox`), `modo`, `rapporto inviato`, `rapporto ricevuto`, `potenza Tx`, `commento`, `nome`, `ora di inizio QDateTime`, `nominativo operatore`, `nominativo stazione`, `locatore proprio` (operatore e stazione sono il nominativo di `MyCall`) |
| 6 Close | solo intestazione |
| 12 Logged ADIF | una sola stringa: `"\n<adif_ver:5>3.1.0\n<programid:4>JTDX\n<EOH>\n"` + il record ADIF del QSO (`myadif2`, lo stesso che va a Log4OM2, sezione precedente) |

**FT8 e FT4 non cambiano il formato.** JTDX **non manda** i campi che WSJT-X aggiunge dopo `fast_mode` (`special_op_mode`, `frequency_tolerance`, `tr_period`, `config_name`, `tx_message`): c'e' un solo byte, `tx_first`, che il decoder di `udp-bridge` legge
come `special_op_mode`. Cambiano solo `modo`/`modo Tx` (`FT8` o `FT4`) e il carattere del modo nel Decode (`~`/`:`); la durata del periodo non viaggia. Un client WSJT-X che leggesse quel byte come «modalita' operativa speciale» vedrebbe `1` (NA VHF)
quando JTDX trasmette per primo: e' il comportamento reale di JTDX e Aurora lo ripete identico.

### 3. Quando JTDX manda cosa

- **Heartbeat**: appena il server e' risolto (`host_info_results`) e poi **ogni 15 s** (`QTimer`, `NetworkMessage::pulse`). Se `UDPServerPort` e' 0 o il server e' vuoto non parte nulla (nemmeno il Heartbeat). Un indirizzo bloccato (`add_blocked_destination`) si scarta.
- **Filtro dei doppioni** (`send_message`): un datagramma **uguale byte per byte all'ultimo mandato** (di qualunque tipo: Status, Decode, Clear, QSO Logged, ADIF) **non si rimanda**; Heartbeat e Close non passano da li'. Uno Status identico al precedente non esce; ma se tra due Status uguali e' uscito un Decode, il secondo esce di nuovo.
  Il flag `force` (`status_update(..., force = true)`) lo salta: lo mette il comando Reply (`replyToUDP`) quando il decode indicato contiene il DX call corrente (interazione JTAlert/Log4OM: «il call in chiamata»).
- **Status** (`MainWindow::statusUpdate`, `REA 0x64d4c0`): 13 chiamanti nell'eseguibile (`procedure_callers`); nei sorgenti: cambio di DX call (`on_dxCallEntry_textChanged`), locatore DX (`on_dxGridEntry_textChanged`), rapporto (`on_rptSpinBox_valueChanged`), Rx e Tx DF (`on_RxFreqSpinBox_valueChanged`, `on_TxFreqSpinBox_valueChanged`),
  Abilita Tx (`on_enableTxButton_clicked`), modo (`commonActions`, `on_pbTxMode_clicked`), frequenza o modo letti dalla radio (`handle_transceiver_update`), inizio trasmissione (`guiUpdate`), fine (`stopTx`, `stopTx2`), inizio e fine della decodifica (`decodeBusy`, `0x474610`),
  watchdog (`txwatchdog`, `0x478b70`), e `statusChanged` (che scrive anche `wsjtx_status.txt`). Sono molte chiamate, ma per il filtro dei doppioni escono solo gli Status diversi dall'ultimo datagramma.
- **Decode**: uno per ogni riga che `readFromStdout` (`REA 0x494220`) mostra, con `nuovo = true`, dopo averla elaborata; filtri: con `preventFalseUDPspots` non escono le righe marcate `?` (`isWrong`); con `ApplyFiltersToUDPmessages` (`filterUDP`) solo quelle che i filtri di visualizzazione lasciano vedere.
  `postDecode` (`REA 0x477540`) spezza la riga: ora (`hhmmss`, o `hhmm` se la prima colonna e' corta), SNR, dt, frequenza, carattere del modo, **messaggio = 23 caratteri dalla colonna 23** (21 con `hhmm`), bassa confidenza se il carattere dopo il messaggio e' `*` o `^`. Il **Replay** (`replayDecodes`, `REA 0x48af80`)
  rimanda tutta la finestra Band Activity con `nuovo = false` e poi lo Status; il Replay si accetta **anche con `AcceptUDPRequests` spento**.
- **Clear**: solo quando l'operatore preme **Erase** (`on_EraseButton_clicked`, `REA 0x4a8ce0`; e il clic destro sul pulsante, `REA 0x49af50`) e **una volta all'avvio** (il costruttore chiama `on_EraseButton_clicked`: il Clear resta in coda finche' il server non e' risolto, quindi parte subito dopo il primo Heartbeat). **Non** parte al cambio di banda
  (`band_changed` svuota solo le finestre di testo) e non parte in WSPR.
- **QSO Logged** e **Logged ADIF**: alla registrazione (`acceptQSO2`, sezione precedente): QSO Logged sempre, Logged ADIF solo con `EnableUDP1adifSending`.
- **Close**: alla distruzione del client (chiusura di JTDX).

### 4. I server UDP e le opzioni di `JTDX.ini`

| Chiave | Predefinito | Cosa fa |
|---|---|---|
| `UDPServer`, `UDPServerPort` | `127.0.0.1`, `2237` | **primo** server: l'unico a ricevere il protocollo WSJT-X (Heartbeat, Status, Decode, Clear, QSO Logged, Logged ADIF, Close) e a mandare i comandi di ritorno; ammesso anche un indirizzo multicast. Qui `127.0.0.1:2334` (Aurora) |
| `EnableUDP1adifSending` | `false` | dopo il QSO Logged manda anche il Logged ADIF al primo server |
| `UDP2Server`, `UDP2ServerPort`, `EnableUDP2adifBroadcast` | `127.0.0.1`, `2333`, `false` | **secondo** server: un `QUdpSocket` nuovo a ogni QSO manda **un solo datagramma con il record ADIF grezzo**, senza protocollo WSJT-X; nessun Heartbeat, nessuno Status. Qui `127.0.0.1:2237` = Log4OM2 «WSJT-X DIRECT» |
| `AcceptUDPRequests` | `false` | senza, i comandi HaltTx, FreeText, SetTxFreq, TriggerCQ e Reply si ignorano (il Replay no) |
| `AcceptUDPReplyMessages` | `1` | quali messaggi un Reply puo' far rispondere: 1 solo `CQ `/`CQDX `/`QRZ`, 2 anche i `73`/`RR73`, 3 qualsiasi (`m_acceptUDP`, menu «Accept UDP»); fuori da 1-3 torna a 1 |
| `ApplyFiltersToUDPmessages`, `preventFalseUDPspots` | `false`, `true` | filtri dei Decode (sopra) |
| `udpWindowToFront`, `udpWindowRestore` | `false` | dopo un Reply la finestra di JTDX passa in primo piano o si ripristina |

Piu' destinatari: JTDX ne ha **uno solo** (il primo server). A fare «piu' destinatari» era l'hub di Aurora (`AURORA_UDP_FORWARD`): riceve da JTDX e rimanda identico a ognuno.

### 5. Comandi di ritorno (`parse_message`, `REA 0x56c360`; gestori in `MainWindow`)

Il client riceve sulla **stessa porta effimera** da cui scrive (`bind()`), quindi GridTracker risponde all'indirizzo sorgente del flusso. Leggibili: Reply (4), Replay (7), HaltTx (8: `solo Abilita Tx`), FreeText (9: testo, invio), SetTxDeltaFreq (50: `u32`), TriggerCQ (51: direzione, invio).
Con `AcceptUDPRequests` spento passa solo il Replay. Reply: `replyToUDP` ricostruisce la riga dai campi (ora, SNR, dt, frequenza, modo, messaggio), la cerca nella finestra Band Activity e, se la trova e il messaggio e' accettato da `AcceptUDPReplyMessages`, chiama `processMessage` come un doppio clic.
HaltTx con `solo Abilita Tx` spegne solo Abilita Tx, altrimenti `haltTx`; SetTxFreq vale solo tra l'inizio e la fine della cascata; TriggerCQ con invio preme CQ (`txb6`).

### 6. Cosa fa ora Aurora con il motore proprio

| Punto | Prima | Ora |
|---|---|---|
| Datagrammi verso GridTracker | nessuno | crate `udp-emitter` (`core/crates/udp-emitter`), avviato da `aurora::run` con `AURORA_ENGINE=native` quando `AURORA_UDP_FORWARD` non e' vuoto e `AURORA_UDP_EMIT` non e' 0: stessi datagrammi di JTDX a tutti i destinatari di `AURORA_UDP_FORWARD`. Con `AURORA_ENGINE=jtdx` non parte (l'hub inoltra gia' i pacchetti di JTDX: niente doppioni) |
| Encoder | solo comandi | `udp-bridge::emit`: `encode_heartbeat`, `encode_status`, `encode_decode`, `encode_clear`, `encode_close`, `encode_qso_logged`, `encode_logged_adif`; il decoder legge anche la coda del QSO Logged (`QsoExtra`) e il tipo 12 (`Message::LoggedAdif`); `udp-bridge::request` legge i comandi di ritorno |
| Identificativo | - | `JTDX` (versione `2.2.159`, schema 2 che sale a quello del client) |
| Heartbeat | - | alla partenza e ogni 15 s, seguito subito dal Clear |
| Status | - | da `Event::Status` (il motore proprio lo pubblica ogni secondo): campi come in JTDX; **DX call e locatore DX** sono quelli scelti da Aurora (doppio clic, Canc DX, cambio banda: il motore proprio azzera il DX); uno identico al precedente non si rimanda; byte finale `tx_first` dall'interruttore del periodo di Tx |
| Decode | - | da `Event::Decode`: `nuovo = true`, `ora = time_ms` (inizio periodo), `snr`, `dt`, `frequenza`, carattere del modo dal modo dell'ultimo Status (`~` FT8, `:` FT4), testo del messaggio |
| QSO Logged / ADIF | - | da `Event::QsoLogged`: stessi campi (ora di inizio e di fine come nel record ADIF, nominativo e locatore propri da `MyCall`/`MyGrid` di `JTDX.ini`), guardia contro i doppioni (stesso nominativo entro 2 periodi); Logged ADIF con `AURORA_UDP_ADIF=1` |
| Comandi di ritorno | persi | l'emettitore ascolta sulla stessa porta da cui scrive: Reply (cerca il messaggio tra gli ultimi 300 decode per ora, frequenza e testo), HaltTx, FreeText, SetTxFreq e TriggerCQ diventano `Command` per il motore proprio; solo con `AURORA_UDP_ACCEPT=1`, il Reply filtrato da `AURORA_UDP_ACCEPT_REPLY`; il Replay e' sempre accettato |
| Close | - | alla chiusura di Aurora |
| Opzioni da `JTDX.ini` | - | `desktop/src-tauri/src/settings.rs::udp1_options_from_ini` imposta `AURORA_UDP_ACCEPT`, `AURORA_UDP_ACCEPT_REPLY`, `AURORA_UDP_ADIF`; `AURORA_UDP_FORWARD=127.0.0.1:2333` era gia' rilevato |

Test: `udp-bridge/tests/emit.rs` (ogni encoder si rilegge con `decode`/`decode_request`: Heartbeat senza revisione, Status con il solo byte `tx_first` e con la coda WSJT-X, stringa nulla e vuota, Decode, Clear, Close, QSO Logged con le due date, Logged ADIF con l'intestazione, comandi di ritorno),
`udp-emitter/tests/flow.rs` (bus con eventi finti e socket locale: Heartbeat e Clear, Status con il DX scelto e Canc DX, Status identico non rimandato, Decode FT8 e FT4, QSO Logged e ADIF e doppione, Reply/HaltTx/FreeText che arrivano come `Command`, comandi ignorati senza `AcceptUDPRequests`, Replay, Close),
`desktop` (`settings`: opzioni del primo server UDP).

### 7. Differenze volute e cosa resta

- `decodifica` nello Status di Aurora e' «in ascolto» (monitor acceso): JTDX lo alza solo mentre decodifica (pochi secondi a fine periodo).
- `rapporto` nello Status: il motore proprio non pubblica ancora il rapporto da inviare (`rptSpinBox`), va vuoto. Il DX, il locatore e la frequenza ci sono.
- `bassa confidenza` e `off air` nei Decode: sempre `false` (il bus non porta la confidenza del decoder).
- Il Clear non parte quando l'operatore svuota l'attivita' nell'interfaccia di Aurora (non c'e' un evento): parte solo all'avvio.
- `NAME` e `COMMENT` mancano nel QSO Logged e nell'ADIF, come per il secondo server.
- Comandi: «HaltTx solo Abilita Tx» ferma comunque la trasmissione (Aurora ha un solo STOP); TriggerCQ non cambia il periodo di Tx.
- Non provato dal vivo con GridTracker2 e Log4OM2: serve una prova del proprietario (nominativo in chiamata nella finestra di Log4OM2, Reply da GridTracker).
