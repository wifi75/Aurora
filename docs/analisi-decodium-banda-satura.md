# Decodium in banda satura: cosa fa e cosa fare in Aurora

Analisi del 2026-10-25, in sola lettura. Nessun file di Aurora e' stato toccato, nulla e' stato eseguito.

## Fonti e metodo

- **Sorgente pubblico** (GPL 3): <https://github.com/iu8lmc/Decodium-4.0-Core-Shannon>, versione 1.0.705, commit `3d4b0b0`, clonato in una cartella temporanea fuori
  dal repository e trattato come dato non fidato. Se non diversamente detto, `file:riga` si riferisce a quel sorgente; `DecodiumBridge.cpp` e'
  `src/bridge/DecodiumBridge.cpp` (61 mila righe) e `widgets/mainwindow.cpp` e' il vecchio codice figlio di WSJT-X.
- **Copia installata** (`...\Programs\Decodium`, versione 1.0.711, non avviata): confrontati i file QML con quelli del sorgente (differiscono
  cinque file, uno solo e' rilevante: vedi «Differenza tra sorgente e copia installata»), le chiavi del file delle impostazioni e le stringhe
  dell'eseguibile. La decompilazione con REA + Ghidra e' stata tentata una volta e non e' riuscita (vedi «Pesi del punteggio: non recuperati»).
- **JTDX**: `docs/logiche-jtdx.md`, `JTDX.ini` (solo i nomi e i valori delle chiavi di logica), le stringhe di `jtdx.exe` e il manuale incluso.
- **Aurora**: `core/crates/qso-engine` (`brain.rs`, `ghost.rs`, `runner.rs`), `engine-sim`, `aurora-core`, `engine-jtdx`.

Dove Decodium ha piu' di un percorso, quello letto qui e' il **nativo** (predefinito su Windows). Il vecchio percorso `widgets/mainwindow.cpp`
(`usingLegacyBackendForTx`, `DecodiumBridge.cpp:24540`) e' il predefinito solo su macOS e serve a Fox/Hound.

## In breve

1. **Decodium non cerca una frequenza di trasmissione libera.** Non esiste «Auto Tx freq» ne' una ricerca nello spettro: nel sorgente, nel
   registro modifiche, nelle chiavi delle impostazioni della copia installata e nelle stringhe dell'eseguibile non c'e' nulla del genere. La
   frequenza Tx (1500 Hz di base) la sceglie l'operatore, con la casella «Hold Tx Freq» per bloccarla. Le sole eccezioni sono la spaziatura
   dei flussi paralleli del Multi Answer Mode (60 Hz) e un salto casuale di +-25 Hz del solo modo FT2.
2. **Auto Call** (risposta automatica a un CQ) e' semplice: scarta i CQ delle stazioni gia' collegate su questa banda, applica i filtri
   dell'operatore e sceglie tra i rimasti in base a un'unica regola a scelta (ultimo decodificato, segnale piu' forte, piu' lontano). Si ferma
   dopo 5 QSO per sessione. Non guarda il paese nuovo, non ha il tail-ending, non guarda la direzione del CQ.
3. **Quando nessuno risponde** si limita a ripetere e a contare: tetto «Caller retries» (10), watchdog (6 minuti o 3 periodi), per il Direct
   Call 10 trasmissioni a vuoto o 90 s. **Non cambia frequenza, non cambia periodo, non si accorge che il DX sta lavorando un altro**: ignora i
   messaggi della stazione non attiva.
4. **Il vero lavoro in banda affollata e' sul lato di chi chiama CQ**: coda FIFO di 20 chiamanti, Multi Answer Mode (fino a 10 flussi
   simultanei sulla frequenza di ciascun chiamante, 3 di base) e, solo nella copia installata, «Best caller first» (punteggio opzionale).
5. **JTDX fa gia' meglio di Decodium su alcune cose** (contatori per stazione, «ferma se risponde a un altro», CQ direzionale, AutoSeq 6/7 che
   chiama i 73) ma tutte spente o a livello 3 nella configurazione attuale; e **non cerca neppure lui una frequenza libera**.
6. **Aurora ha gia' il pezzo piu' difficile** (priorita' tra CQ per rarita' o segnale, filtro dei falsi, tetti) ma **non guarda cosa succede
   dopo la risposta**: non conta le trasmissioni, non sa se il DX parla con altri, non sceglie la frequenza. E' li' che stanno le regole da
   aggiungere (ultima sezione).

## 1. Frequenza di trasmissione

| Domanda | Cosa fa Decodium | Dove |
|---|---|---|
| Cerca una zona libera nello spettro? | **No.** Nessuna funzione legge lo spettro per scegliere la Tx. | ricerche su tutto il sorgente, QML compreso |
| «Auto Tx freq»? | **Non esiste** (ne' nel sorgente 1.0.705 ne' nelle chiavi delle impostazioni della 1.0.711). | `Decodium3.ini`: solo `txFrequency`, `HoldTxFreq` |
| Frequenza di partenza | `txFrequency` = 1500 Hz; se salvata sotto 100 Hz torna a 1500. Limite 0-5000 (3000 al caricamento). | `DecodiumBridge.cpp:42985`, `setTxFrequency` `:19494` |
| Chi la sposta | L'operatore: clic sinistro sulla cascata, **doppio clic su una riga**, UDP `setup_tx` con offset. «Hold Tx Freq» impedisce gli ultimi due. | `DecodeWindow.qml:1400`, `DecodiumBridge.cpp:35405-35415` |
| Doppio clic su un CQ | Tx e Rx vanno sulla frequenza del CQ (cioe' **tutti i chiamanti si sovrappongono sulla stessa frequenza**, come in JTDX/WSJT-X) a meno di Hold. | `DecodeWindow.qml:1399-1403` |
| Auto Call | Imposta solo la frequenza **Rx** su quella del CQ (`setRxFrequency`); **la Tx resta dov'era**. | `processDecodeDoubleClick` `DecodiumBridge.cpp:37578` |
| Spaziatura minima | Solo nel Multi Answer Mode: i CQ paralleli partono dalla Tx e salgono di **60 Hz** (poi scendono), saltando tutto cio' che dista meno di **50 Hz** da una frequenza gia' in uso; nessun controllo sull'occupazione reale della banda. | `mamPlanCqFrequencies` `:25782` |
| Evitare i segnali vicini | Mai sul segnale ricevuto: nel MAM ogni risposta resta **sulla frequenza del chiamante** («meglio aspettare quel canale che spostare una stazione»); se il canale e' occupato a meno di 50 Hz il chiamante va in coda. | `mamIngestDecode` `:26240-26294`, `mamPromoteFromQueue` `:26513-26531` |
| Dopo una collisione | **Niente**: la Tx non cambia. Si ripete lo stesso messaggio fino ai tetti (sezione 3). | `checkAndStartPeriodicTx` `:41190-41330` |
| Salti casuali | FT2 asincrono, durante il CQ e senza Hold: +-25 Hz a caso. Fox: 300-599 Hz, Hound: 1000-2999 Hz (protocollo DXpedition, vecchio percorso). | `DecodiumBridge.cpp:28033`, `mainwindow.cpp:17055-17095` |
| Split della radio | La Tx audio e' tenuta tra 1500 e 2000 Hz spostando il VFO di Tx a multipli di 500 Hz (`(tx/500)*500 - 1500`): e' la «Fake It» di WSJT-X, non una ricerca di spazio libero. | `catSplitXitHzForTxFrequency` `:17922` |

**Dal lato decodifica** Decodium ha un recupero dei segnali in grappoli fitti (`Detector/FtxFt8Stage4.cpp:8927`, passata tardiva se vicino a un
candidato ce ne sono 3 o piu' entro 45 Hz): riguarda il motore proprio di decodifica di Aurora, non le regole di chiamata.

**Conclusione:** per la frequenza libera non c'e' nulla da imitare. E' una funzione nuova, che Aurora puo' fare meglio di Decodium perche'
ha la cascata (`SpectrumRow`) e i decode con la frequenza di ognuno (sezione «Cosa fare in Aurora», regola A).

## 2. Chi chiamare quando ci sono molti CQ

### Auto Call (`tryAutoCallFromRows`, `DecodiumBridge.cpp:40136`)

Parte solo se: Auto Call e' armato, modo FT8/FT4/FT2, nessun Direct Call, nessuna trasmissione/accordo, **nessun DX impostato, nessun QSO in corso**,
contatore sotto il tetto. Poi, per ogni riga decodificata **del ciclo**:

1. deve essere `CQ`, `CQ <modificatore>` o `QRZ` (**niente 73 e niente chiamate ad altri: nessun tail-ending**);
2. passa il filtro semantico (`shouldAcceptDecodedMessage`) e **non e' il proprio nominativo**;
3. **scartato se il nominativo e' gia' nel log su questa banda** (`m_worked.callByBand`: «WSJT-Z Auto Call forces new call on band»);
4. scartato se lavorato/registrato negli ultimi **90 s** (`isRecentAutoCqWorkedOrLoggedDuplicate`, `kRecentDuplicateLogWindowSeconds`);
5. passa i filtri dell'operatore: lista nera, lista bianca, «sempre passa», **territori esclusi** (paese, prefisso, continente);
6. si calcola la distanza (dal locatore proprio e dal locatore del CQ).

Tra i candidati rimasti sceglie con una regola sola (`autoCallPriority`, `CallFeature/AutoCallPriority`):

| Valore | Regola | Note |
|---|---|---|
| 0 (predefinito) | **Ultimo decodificato**: il piu' recente per ora, a parita' l'ultimo della lista | e' un'arbitrarieta', non una priorita' |
| 1 | **Segnale piu' forte** (SNR piu' alto) | a parita' vince l'ultimo della lista |
| 2 | **Piu' lontano** (distanza piu' grande); senza locatori ripiega sul piu' recente | |

Poi esegue lo stesso percorso del doppio clic (`processDecodeDoubleClick`): imposta DX, genera i messaggi, regola il periodo **opposto** a
quello del DX (`replyTxPeriodForClickedDecode`, `:37585`), accende Abilita Tx e avvia il controllo dello slot. Tetto: `AutoCallMaxQsos`
(5; 0 = senza limite; massimo 999), conta i QSO **completati** (`noteAutoCallQsoCompleted`, `:40334`).

**Cosa non c'e'**: paese/zona/banda nuovi come priorita', forza del segnale come soglia minima (si sceglie il piu' forte ma non c'e' un
minimo), pesi combinati, tentativi per stazione (un CQ da cui non si e' ottenuto nulla si riprova al ciclo dopo se non e' nel log),
direzione del CQ (un «CQ NA» viene chiamato anche dall'Europa), tail-ending, attesa.

### Direct Call e DX-watch (`startTargetCall`, `:21451`)

L'operatore scrive il nominativo del DX; Decodium lo chiama con la sequenza automatica. **DX-watch** («Armed», spento di base) **non trasmette**
finche' non decodifica il bersaglio (come mittente di un CQ o di qualunque messaggio) e allora avvia la chiamata
(`tryArmedWatchSeesTarget`, `:21711`). Il periodo si sceglie 1o, 2o o **alternato** (di base alternato: serve quando non si e' ancora
sentito il DX). Non c'e' priorita' tra DX diversi: il bersaglio e' uno.

### Chi risponde a noi quando siamo noi a chiamare CQ

- **Un QSO per volta** (percorso a stazione singola): in `autoSequenceStep` vince il **primo messaggio diretto a noi** nell'ordine della lista dei
  decode; gli altri chiamanti (messaggio che non e' un 73) finiscono in coda (`:41865-41884`).
- **Coda** `m_callerQueue`: voci «nominativo frequenza SNR», **FIFO di 20** (`FOX_QUEUE_MAX`), senza duplicati, escludendo il chiamante attivo e
  i recenti (90 s); l'ordine si cambia solo a mano (`moveCallerQueueItem`). Il prossimo viene preso da `processNextInQueue` (`:57418`): parte
  direttamente da Tx2 con il rapporto misurato; coda vuota = torna al CQ.
- **Multi Answer Mode** (opzionale, spento): fino a 10 QSO insieme (3 di base), ognuno con il suo flusso sulla **frequenza del chiamante**,
  messaggio Tx per ogni flusso, tutti nello stesso periodo (`mamDispatchPeriod`, `:26594`). Uno slot si libera per QSO chiuso, per «piu' di
  `maxRetry` ripetizioni» o per «silenzio da piu' di 6 periodi»; uno slot caduto con scambio avviato resta recuperabile 120 s se arriva il 73
  (`mamPruneSlots`, `:26354`). Non validato in aria per FT2 (avviso in interfaccia).

## 3. Se la chiamata non va

| Situazione | Reazione di Decodium |
|---|---|
| **Nessuna risposta a Tx1/Tx2** | ripete lo stesso messaggio nel prossimo periodo utile; al tetto **«Caller retries»** (10 invii totali per passo, 1-99) si ferma: azzera DX, spegne la trasmissione (o passa al prossimo in coda / torna al CQ se AutoCQ). Solo in AutoCQ/Multi Answer ricorda anche la stazione come abbandonata per 90 s; in Auto Call e nei QSO a mano **non la ricorda**, quindi lo stesso CQ puo' essere scelto di nuovo (per quanto letto). Il tetto non vale per Tx3-Tx5 (chiusura) e, se il watchdog e' impostato, cede a lui. `:41190-41330` |
| **Tx4/Tx5 senza risposta** | tetto di chiusura per modo: FT8 3, FT4 4, FT2 4 (1-8); poi **registra comunque** il QSO e chiude, perche' il rapporto e' gia' stato scambiato (`deferredSignoffRetryCapForMode`, `Sequencer/QsoSequencerRules.cpp:12`). |
| **Altri rispondono allo stesso DX** | **nessun rilevamento**. In `autoSequenceStep` i messaggi della stazione non attiva sono ignorati («ignore directed decode from non-active partner», `:41974`). Si continua fino ai tetti. |
| **Il DX risponde a un altro** | idem: non viene visto. (JTDX ha «ferma Tx se risponde a un altro», spenta.) |
| **Cambio periodo pari/dispari** | Il periodo si imposta una volta, **opposto a quello del DX**, alla scelta. Nel percorso nativo non c'e' cambio automatico in FT8/FT4 (solo il «phase-lock breaker» di FT2, `:40880-41100`). Nel vecchio percorso, dopo **10 CQ senza risposta**, se «Alt 1/2» e' attivo inverte il periodo (`mainwindow.cpp:17800-17817`); il Direct Call alterna. |
| **Cambio frequenza** | nessuno (sezione 1). |
| **Passaggio al CQ** | solo in AutoCQ: dopo il tetto dei tentativi, o a coda vuota, riparte il Tx6 (`autoCqCanRestartTx6`). Con AutoCall o Direct Call ci si ferma. |
| **Direct Call** | conta le trasmissioni **senza aver rivisto il bersaglio** (`missedTargetTx`, tetto 10, 0 = illimitato); ogni volta che lo decodifica il contatore riparte da zero. Scade dopo 90 s (10-600) se non e' arrivata alcuna risposta e il watchdog non e' impostato. Con «Pausa» (0-300 s) spegne la Tx tra un tentativo e l'altro. Con DX-watch + «Re-arm» torna in ascolto fino a **3 volte**, poi ferma e chiede l'intervento dell'operatore (`engageManualTxHold`). `:21566-21863` |
| **Stazione appena lavorata** | per 30 s (cooldown) un suo 73/RR73 ripetuto non riapre un QSO; per 90 s non viene richiamata in AutoCQ/Auto Call (`kQsoCooldownWindowMs`, `kRecentDuplicateLogWindowSeconds`). |
| **Ripresa dopo Halt** | opzione «Resume QSO on reply» (2 minuti): se il partner risponde dopo lo stop, il QSO riprende (`engageManualTxHold`, `:38414`). |
| **Wait & Pounce** | con il CQ armato, un CQ di un'altra stazione che passa i filtri e non e' recente fa partire la risposta (`tryStartWaitPounceFromEntry`, `:37138`), non piu' di una volta al minuto per lo stesso messaggio. |

## 4. Limiti e protezioni

| Limite | Valore di base | Note | Dove |
|---|---|---|---|
| QSO per sessione di Auto Call | 5 (0 = illimitato, max 999) | conta i QSO completati, azzerabile a mano | `:21279`, `:42903` |
| «Caller retries» | 10 invii per passo (1-99) | solo Tx1/Tx2 (e non in chiusura) | `:7817`, `:42864` |
| Ripetizioni di 73/RR73 | FT8 3, FT4 4, FT2 4 (1-8) | «boost» opzionale per partner debole | `QsoSequencerRules.cpp:12` |
| Watchdog Tx | modo 1 = **6 minuti**, modo 2 = **3 periodi**, modo 0 = spento | il modo a tempo misura la durata della sessione di trasmissione (dal primo armarsi di Abilita Tx/AutoCQ), **non si azzera quando il partner risponde**; il modo a periodi si azzera a ogni messaggio del partner. All'allarme: Halt, spegne la Tx, opzione «logga il QSO se lo scambio e' completo». | `:51739-51850` |
| AutoCQ | cicli 0 (illimitati), pausa 0 s, «N CQ poi M periodi di ascolto» spento | conta i CQ **completati**; solo CQ puro | `Sequencer/AutoCqCallPolicy.hpp`, `:37769` |
| Coda chiamanti | 20 | | `DecodiumBridge.h:4286` |
| Multi Answer Mode | 3 flussi (1-10); CQ paralleli sugli slot liberi acceso | opzionale e spento | `:25747` |
| Duplicati | 90 s (stazione abbandonata/lavorata), 30 s (cooldown di chiusura) | per nominativo+banda+modo | `:4938`, `:4943` |
| Direct Call | 10 trasmissioni a vuoto, 90 s, re-arm 3 | vedi sopra | `:42893-42899` |
| Protezione SWR | blocca la Tx oltre 2,5 (1,5-5); impedisce di armare Auto Call; un'interruzione non conta come QSO | serve il sensore (con OmniRig non c'e') | `:23744` |
| Salute dell'ingresso audio | sospende il **riavvio del CQ** se l'audio e' «non sano» da 15 s o piu' | solo in AutoCQ puro | `:40504` |
| Blocco generale della Tx | una Tx automatica parte solo se non c'e' blocco manuale, non siamo in avvio, e il Tx6 solo con AutoCQ/MAM acceso | `txDispatchGuardAllows` | `:20535` |
| Filtri sui falsi decode | forma del nominativo, conferma per i segnali sotto -23 dB, discordanza paese/locatore | sezione 6 | `:6827`, `:7287` |

**Non esiste un tetto di trasmissioni di fila** diverso dal watchdog; **non esiste un tetto al giorno**.

## 5. JTDX per gli stessi casi

Da `docs/logiche-jtdx.md` e da `JTDX.ini` (nostra configurazione). JTDX e' la fonte della verita' per la logica dei QSO: **Aurora legge o comanda, non ricalcola**.

| Caso | JTDX | Valore attuale |
|---|---|---|
| Frequenza Tx libera | **Non c'e'.** Doppio clic su un CQ = Tx e Rx sulla frequenza del CQ; su un messaggio per noi la Tx resta; Ctrl+doppio clic sposta entrambe; «Lock Tx=Rx» le lega (dal manuale incluso, `jtdx-manuale.txt`). Nessuna stringa di ricerca di spazio libero in `jtdx.exe`. | `TxFreq` 1084, `LockTxFreq=false` |
| Scelta tra piu' CQ | AutoSeq **4 e oltre**: priorita' per nominativi, prefissi, paesi, griglie desiderati; «chiama prima i nominativi con priorita' alta dei nuovi» solo da 4 in su. AutoSeq **6/7**: cerca CQ o 73 validi tra i decodificati e li chiama, altrimenti chiama CQ. | AutoSeq 3, `CallPriorityCQ=false` |
| Rinuncia | quattro contatori (rispondere a un CQ, rispondere a una chiamata, R-rapporto, RR73): al valore massimo **chiama CQ** (livelli 1-3) | `SeqAnswerCQCount=false` (**spento, quindi senza limite**), `SeqAnswerInCallCount=false`; `SeqSentRReportCount` e `SeqSentRR7373Count` accesi, massimo 5 |
| Watchdog | `TxWatchdogTimer` = minuti senza risposta | 6 |
| Il DX risponde a un altro | `SeqHaltTxReplyOther`: ferma la Tx | **spento** |
| CQ direzionale | `StrictDirectionalCQ`; `CQdirection` e' il modificatore dei propri CQ | spento; `DX` |
| Gia' lavorati | `AnswerWorkedB4` («Risp B4») risponde anche a chi si e' gia' lavorato | **acceso**: in banda satura e' controproducente |
| Un solo QSO | `SingleShotQSO` («1 QSO») | spento |
| Periodo | `TxMinuteButton` impostato da JTDX al doppio clic, opposto al DX | |
| Ripetizione senza risposta | ripete il messaggio corrente (osservato in `ALL.TXT`) | |

Quindi con la configurazione attuale JTDX **richiama la stessa stazione senza limite fino al watchdog (6 minuti, circa 12 trasmissioni)** e **non
si accorge se il DX lavora un altro**. Il limite oggi sta solo nel tetto di Aurora (`max_attempts_per_call`), che pero' conta le **risposte
mandate**, non le trasmissioni.

## 6. Cosa ha gia' Aurora (`core/crates/qso-engine`)

| Cosa | Dove | Stato |
|---|---|---|
| Risponde solo a CQ con etichetta di novita' (paese, paese su banda, zona, griglia) o alla lista dei desiderati; i CQ comuni sono **rifiutati dalla validazione** delle regole | `brain.rs` `Rules::is_valid` | c'e' |
| Priorita' tra piu' CQ **nello stesso ciclo**: finestra di 350 ms, si risponde a **uno solo**: rarita' (desiderati 5 > paese 4 > paese su banda 3 > zona 2 > griglia 1) poi SNR, oppure SNR poi rarita' | `on_batch`, `Candidate::better_than`, `runner.rs` | c'e' |
| Segnale minimo (-18 dB), esclusioni, tentativi per stazione (2), un QSO attivo per volta (60 s o fino al log), tetti (6 di fila, 40 al giorno), dead-man 120 s, STOP spegne, prova a vuoto | `brain.rs` | c'e' |
| Filtro dei falsi: «?», forma del nominativo, sotto -23 dB solo se gia' sentito in sessione | `ghost.rs` | c'e' (piu' semplice di Decodium) |
| Conta delle trasmissioni, stato del QSO dopo la risposta, DX che parla con altri, cambio di frequenza o periodo | | **manca** |
| Distanza, direzione del CQ, tail-ending | | **manca** |
| Tentativi per stazione che **scadono** | `attempts` si azzera solo al QSO registrato: una stazione provata due volte non e' piu' chiamata fino al riavvio | **manca** (da sostituire con una pausa) |
| Il cervello legge solo `Decode`, `QsoLogged`, `TxHalted`, `Status.dial_hz` | `runner.rs` `handle_event` | per le nuove regole va esteso a tutto `Status` (trasmissione, `dx_call`, `tx_df`, `tx_message`) |

Differenze di **filtro dei falsi** da prendere da Decodium (`DecodiumBridge.cpp:6827-7100`, `:7287`):

- a -23 dB o meno una stazione e' accettata se gia' sentita **forte** (>= -20 dB) **oppure ripetuta in un altro ciclo entro 10 minuti**: il falso
  da CRC non ripete lo stesso nominativo (misura loro: un DX debole era stato scartato 190 volte); Aurora oggi accetta chi e' stato sentito a
  qualunque livello;
- chiamata diretta a noi con locatore a piu' di **5000 km** dal paese del nominativo e segnale <= -18 dB o «?»: scartata;
- chiamata a noi, segnale <= -14 dB e distanza > 8000 km: scartata se non e' il QSO in corso.

## Differenza tra sorgente e copia installata

La copia installata (1.0.711) ha, rispetto al sorgente pubblico (1.0.705), **una sola funzione che riguarda le chiamate**: «Best caller first»
(`callerScoring`, interruttore in Impostazioni > Sequenza automatica, **spento di base**). Testo dell'aiuto nel QML installato:

> Quando piu' stazioni ti chiamano (AutoCQ / multi-risposta), risponde alla migliore invece che alla prima arrivata. Il punteggio favorisce i segnali
> forti, i nuovi DXCC (di piu' se nuovi su qualsiasi banda, di meno se nuovi solo su questa) e le stazioni che hanno atteso di piu'. Le stazioni gia'
> lavorate su questa banda scendono. Un ordine impostato a mano nella coda non viene mai cambiato.

(Parafrasi mia dal testo inglese `SettingsTab3.qml`.) Nell'eseguibile restano i soli nomi `callerScoring`, `setCallerScoring` e i messaggi di
registro `CallerScoring (%1): %2` e `CallerScoring (slot): %1`: **i pesi non sono nel sorgente pubblico.**

### Pesi del punteggio: non recuperati

Per leggere i pesi serviva decompilare `decodium.exe` (48 MB, Qt/C++) con REA + Ghidra, partendo dalla stringa `CallerScoring (%1): %2`. Il tentativo
(un solo Ghidra alla volta, analisi sulla copia installata, sola lettura) e' **fallito**: con il limite predefinito di REA l'importazione e' scaduta
dopo 330 s; con 40 minuti di limite Ghidra e' andato in `OutOfMemoryError` dopo circa 9 minuti. Servirebbe aumentare la memoria di Ghidra
(`MAXMEM` in `support/launch.properties`) e probabilmente disattivare gli analizzatori piu' pesanti: lavoro lungo per un dettaglio che qui non
serve (i pesi di Aurora si tarano comunque in simulazione). **Quindi di «Best caller first» si conosce solo la descrizione a parole** (segnale
forte, DXCC nuovo piu' o meno a seconda della banda, attesa, gia' lavorato su banda penalizzato, ordine manuale rispettato), che e' quella usata
nella regola E.

Non c'e' invece in 1.0.711 nulla sulla frequenza Tx: stesse chiavi di impostazione, stesse proprieta' esposte al QML (le sole tre nuove sono
`callerScoring`, `setCallerScoring`, `looksLikeWeakComputer`).

## Cosa fare in Aurora

Principi: (1) **JTDX decide il QSO, Aurora decide chi chiamare, quando smettere e dove trasmettere**; (2) tutto spento di base e in prova a
vuoto, come ora; (3) logica pura in `qso-engine`, con il tempo passato dal chiamante, cosi' ogni regola si prova senza radio; (4) ogni azione
sulla radio passa dai comandi che gia' esistono (`Reply`, `HaltTx`, `SetTxFrequency`, `CallCq`), che restano dietro `AURORA_ALLOW_TX`.

Legenda: difficolta' **S** (qualche ora), **M** (un giorno), **L** (piu' giorni). Colonna *Motore*: **JTDX** = si puo' fare ora, **proprio** = richiede il
motore proprio (crate `ft8`, trasmissione propria).

| ID | Regola | Origine | Diff. | Motore |
|---|---|---|---|---|
| A | **Tx in zona libera** | nuova (nessuno la fa) | M | JTDX (con una prova) |
| B | **Rinuncia dopo N trasmissioni** | JTDX contatori; Decodium «Caller retries» | M | JTDX |
| C | **Il DX lavora un altro** | JTDX `SeqHaltTxReplyOther`; Decodium non la ha | M | JTDX |
| D | **Pausa per stazione** (al posto dei tentativi eterni) | Decodium 90 s / 30 s | S | JTDX |
| E | **Priorita' a punteggio** (rarita' + segnale + distanza) | Decodium Auto Call 0/1/2 e «Best caller first» | S | JTDX |
| F | **CQ direzionale** | JTDX `StrictDirectionalCQ` | S | JTDX |
| G | **Falsi: conferma per ripetizione e per distanza** | Decodium `looksLikeGhostDecode` | S-M | JTDX |
| H | **Auto Call su CQ comuni, con tetto** | Decodium Auto Call | S | JTDX |
| I | **Tail-ending** (chiamare chi manda 73) | JTDX AutoSeq 6/7 | M | JTDX |
| J | Coda chiamanti e scelta di chi rispondere quando **siamo noi a fare CQ** | Decodium coda FIFO + «Best caller first» | M | JTDX solo scelta del `Reply`; vero parallelismo = proprio |
| K | **Cadenza del CQ** (N CQ poi M periodi di ascolto, cambio periodo dopo N CQ a vuoto) | Decodium AutoCQ burst, «Alt 1/2» | S | JTDX (`CallCq`) |
| L | **Risposta a piu' chiamanti insieme (Multi Answer)**, CQ paralleli sugli slot liberi | Decodium MAM | L | **proprio** |

Le cinque piu' importanti per la banda satura sono **A, B, C, E+D, F+G**. Il resto si fa dopo.

### Stato al 2026-10-25 (motore proprio, `core/crates/qso-native` e `engine-native`)

| ID | Stato | Dove |
|---|---|---|
| A | **realizzata come funzione nuova**, spenta di base (`AURORA_FREE_TX=1`): solo con Split acceso, a inizio QSO e dopo due Tx senza risposta, mai a QSO avviato | `freefreq.rs`, `TxDriver::set_free_tx` |
| B | **realizzata** come i contatori di JTDX (le trasmissioni dello stesso messaggio) | `process_auto`, `RepeatLimits` |
| C | **realizzata**: Tx fermo se il DX parla con altri nella banda di guardia (FT8 51 Hz, FT4 84) o sempre con `SeqHaltTxReplyOther`; contatore che libera il DX | `readFromStdout` -> `halt_tx_reason` |
| D | **realizzata** (`m_counter` e `calllist`, 5 minuti salvo SNR migliore) | `history.rs` |
| E | **realizzata**: priorita' 0-31 di JTDX oppure «Best caller first» con i pesi del binario 1.0.711 (segnale [-24, 10] -> 0..30, attesa fino a 90 s -> 0..20, DXCC nuovo +40 / su banda +25, gia' lavorato -35 / altrove -12, fissato 1000) | `score.rs`, `Options.caller_selection` |
| F | **realizzata** (CQ direzionale, `StrictDirectionalCQ`) | `history.rs` |
| G | non realizzata | |
| H | non realizzata | |
| I | **realizzata** (AutoSeq 6/7: i CQ degli altri, anche chi manda 73) | blocco C di `autoseq_pick` |
| J | **realizzata** (scelta tra piu' chiamanti ai nostri CQ, blocchi A e B) | `autoseq_pick` / `pick_scored` |
| K | non realizzata | |
| L | non realizzata | |

Il cambio di banda e di modo azzera il QSO come in JTDX; l'avvio tardivo (13,64 s in FT8, 6,04 s in FT4) e la ritrasmissione sono in `tx-native`.

### A. Tx in zona libera (`tx_free`)

- **Perche'**: in un doppio clic su un CQ JTDX porta la Tx **sulla frequenza del CQ**: con 20 stazioni che chiamano lo stesso DX si
  sovrappongono tutte a quel tono. Decodium non lo evita. Spostare la Tx in uno spazio libero (come fa chi sa fare) e' il vantaggio piu' grande.
- **Dati**: `SpectrumRow` (1024 bin, 0-3000 Hz, ~2,9 Hz per bin) e i `Decode` con `freq_hz`, tenuti per gli ultimi `window_cycles` cicli.
  Un segnale FT8 occupa da `freq` a `freq+50 Hz` (JTDX riporta il tono piu' basso), FT4 da `freq` a `freq+83 Hz`.
- **Condizione di scatto** (tutte): regola accesa; sta per partire un QSO nostro (subito dopo il `Reply`, prima che JTDX trasmetta) **oppure** e'
  stato fatto un tentativo senza risposta e la Tx attuale cade entro la distanza minima da un segnale recente; **il QSO non e' ancora avanzato**
  (nessun messaggio del DX per noi). Mai a QSO avviato (cambiare Tx a meta' fa perdere l'a-priori del decodificatore del DX).
- **Azione**: calcolare la frequenza migliore e mandare `SetTxFrequency { hz }` (gia' disponibile, `POST /api/v1/commands/tx-frequency`).
  Migliore = massimo spazio libero attorno al tono (distanza minima dal segnale piu' vicino), tra `min_hz` e `max_hz`, scartando le posizioni
  sotto `guard_hz`; a parita' la piu' vicina alla Tx attuale (meno salti) e dentro la fascia piatta `sweet_lo..sweet_hi`. Se non c'e' spazio
  (nessun candidato sopra `guard_hz`) **non si sposta** e lo scrive nel registro.
- **Parametri** (valori di partenza):

  | Parametro | Valore | Significato |
  |---|---|---|
  | `enabled` | falso | interruttore |
  | `min_hz` / `max_hz` | 300 / 2700 | limite della Tx (la rotta accetta 200-3000) |
  | `guard_hz` | FT8 60, FT4 90 | distanza minima tra toni: larghezza del segnale + 10 Hz (Decodium nel MAM: 60 e 50) |
  | `window_cycles` | 4 | per quanti cicli un segnale conta come presente (60 s in FT8) |
  | `sweet_lo` / `sweet_hi` | 800 / 2200 | fascia preferita (audio piu' piatto della radio) |
  | `move_after_unanswered` | 2 | trasmissioni senza risposta prima di cambiare frequenza (0 = mai, solo alla partenza) |
  | `min_move_hz` | 40 | non spostare per meno |
  | `hold_in_qso` | vero | mai a QSO avviato |

- **Limite da dire**: durante la nostra trasmissione non si riceve, quindi lo spettro del periodo in cui trasmettiamo (quello in cui chiamano gli
  altri) si ricostruisce dai cicli in cui eravamo in ascolto di entrambi i periodi e **decade**; la scelta e' una stima, non una misura.
  Si tengono due mappe (periodo pari e dispari) e si da' il doppio del peso a quella del nostro periodo di Tx quando c'e'.
- **Con JTDX**: possibile ora (`SetTxFrequency` esiste e usa il messaggio UDP 50 «SetTxDeltaFreq», solo JTDX, trovato con REA: `docs/fase0-rapporto.md`). **Da provare con la radio** (la fa il
  proprietario): che JTDX accetti il cambio tra la fine della decodifica e l'inizio del periodo, che non lo riporti sulla frequenza del CQ al
  primo messaggio del DX, e se serve accendere «Hold Tx Freq» (`pbTxLock` e' Tx=Rx; la casella Hold non e' nella tabella dei controlli: da censire).
- **Con il motore proprio**: la frequenza e' un parametro del messaggio, si decide per ogni trasmissione e non c'e' nulla da provare.
- **Interfaccia**: nella pagina Automazioni una scheda «Frequenza di trasmissione» con l'interruttore, i due limiti, la distanza minima, una
  barra che mostra sulla cascata la zona scelta e il motivo nell'elenco delle decisioni («Tx 1840 Hz: libera da 112 Hz da ogni segnale»).

### B. Rinuncia dopo N trasmissioni (`give_up_after_tx`)

- **Condizione**: `Status.transmitting` torna a falso (fine di una trasmissione) con `dx_call` = la stazione che abbiamo chiamato e il messaggio
  di JTDX e' ancora Tx1/Tx2 (stesso testo, nessun avanzamento; si legge da `Status.tx_message` o dal pallino «Next» letto dalla finestra): si incrementa `unanswered_tx`. Si azzera se arriva un messaggio del DX per noi.
- **Azione** al raggiungimento di `max_unanswered_tx`: `HaltTx` (JTDX smette), registro «X: nessuna risposta dopo N trasmissioni», pausa per la
  stazione (regola D), poi **al ciclo successivo** il normale arbitraggio sceglie il prossimo CQ. L'automazione resta accesa; il tetto «risposte
  di fila senza QSO» conta il tentativo.
- **Parametri**: `max_unanswered_tx` = **4** in FT8 e **6** in FT4 (periodi piu' brevi), intervallo 1-12; `0` = lascia fare a JTDX.
  Perche' 4: con JTDX a contatori spenti il limite e' il watchdog di 6 minuti (12 trasmissioni); Decodium ne permette 10. In banda satura
  le stazioni che rispondono lo fanno nei primi 2-3 tentativi.
- **Con JTDX**: si fa ora, solo con eventi che Aurora gia' riceve (`Status`). Alternativa non dinamica: scrivere i contatori `SeqAnswerCQCount`
  nel `JTDX.ini` (a JTDX chiuso): non serve.
- **Interfaccia**: numero «Rinuncia dopo ... trasmissioni a vuoto» nella scheda delle regole, accanto a «tentativi per stazione».

### C. Il DX lavora un altro (`yield_to_other`)

- **Condizione**: `Status.dx_call` = X; arriva un `Decode` il cui mittente e' X e il cui destinatario **non siamo noi e non e' un CQ**
  (X sta rispondendo a un'altra stazione), e il nostro QSO con X **non e' avanzato** (Tx1/Tx2).
- **Azione** dopo `others_cycles` cicli consecutivi: come la regola B (`HaltTx`, pausa breve, prossimo candidato). Se il messaggio di X a
  quell'altro e' un 73/RR73, il QSO di X e' finito: si **mantiene** la chiamata (e' il momento giusto per essere chiamati) e il contatore si azzera.
- **Parametri**: `enabled` falso; `others_cycles` = 2 (1-5); `pause_min` = 5 (regola D, piu' corta: X tornera' libero).
- **Con JTDX**: si fa ora. In JTDX esiste `SeqHaltTxReplyOther` ma agisce sul solo caso «il DX risponde ad altri durante il mio QSO»; farlo in
  Aurora da' il motivo scritto e il passaggio al candidato successivo, che JTDX non fa.
- **Interfaccia**: casella «Lascia perdere il DX se risponde ad altri» con il numero di cicli.

### D. Pausa per stazione (`cooldown`)

- Al posto di `attempts` eterno: dopo una rinuncia (B, C) o dopo `max_attempts_per_call` risposte, la stazione non si richiama per
  `cooldown_min` minuti, poi i tentativi si azzerano. Dopo un QSO registrato: nessuna richiamata per quella stazione sulla stessa banda e modo
  per `worked_pause_min` (il log e i filtri sulle etichette gia' lo coprono; qui e' la rete di sicurezza di 90 s di Decodium).
- **Parametri**: `cooldown_min` = 15 (1-120); `worked_pause_min` = 2.
- **Con JTDX**: si fa ora (solo il cervello). Difficolta' S.
- **Interfaccia**: un solo campo «Non richiamare la stessa stazione per ... minuti».

### E. Priorita' a punteggio (`prefer = score`)

- Il cervello oggi ordina per (rarita', SNR) o (SNR, rarita'). Aggiungere `Prefer::Score`:
  `punteggio = 100 * rarita + clamp(snr, -24, +10) + bonus_distanza + bonus_attesa`, con
  `bonus_distanza` = distanza in migliaia di km (0-20) se `Prefer::Distance` e' attivo (Decodium: «piu' lontano»), e un piccolo bonus per le
  stazioni **che hanno aspettato di piu'** (sentite in piu' cicli senza ricevere risposta da noi). **Valori e pesi sono una proposta**: quelli di
  Decodium non si conoscono, c'e' solo la descrizione a parole (sezione «Pesi del punteggio: non recuperati»); si tarano in simulazione.
- `Prefer::Distance` come terza scelta semplice: serve la distanza dal locatore del CQ (nel messaggio) e dal nostro (`Status.de_grid`); **nel codice non c'e' ancora una funzione di distanza tra locatori**: da scrivere (formula del cerchio massimo, con prova su coppie note).
- **Parametri**: `prefer` = rarita' (invariato); nuove voci `score` e `distance`; `weights` modificabili solo da file.
- **Con JTDX**: si fa ora (S).
- **Interfaccia**: la tendina «Se ce ne sono piu'» ha due voci in piu'.

### F. CQ direzionale (`respect_cq_direction`)

- **Condizione**: il CQ ha un modificatore (`CQ DX`, `CQ EU`, `CQ NA`, `CQ JA`, ...). Se e' un continente/zona e **non e' il nostro**
  (`Status.own_continent`), il CQ e' scartato con il motivo; «CQ DX» vale solo per chi e' in un continente diverso da quello del mittente (serve
  `DecodedMessage.continent` del mittente, che c'e').
- Si applica **a tutti**, anche ai paesi nuovi, a meno di `strict_direction = falso` (come `StrictDirectionalCQ` di JTDX: «rispetta la
  direzione anche per un nuovo DXCC»).
- **Parametri**: `respect_cq_direction` = vero, `strict_direction` = vero.
- **Con JTDX**: si fa ora (S). **Interfaccia**: due caselle.

### G. Falsi: conferma per ripetizione e per distanza (`ghost`)

- Estendere `ghost::suspicion`: sotto -23 dB passa solo se gia' sentito **a >= -20 dB** o **visto in un altro ciclo entro 10 minuti**; scarto per
  locatore a piu' di 5000 km dal paese quando il segnale e' <= -18 dB o il messaggio e' «?». Serve in particolare dove tanti CQ deboli
  possono nascondere falsi del decodificatore (anche del motore proprio, che oggi non decodifica come JTDX).
- **Parametri**: `weak_snr` -23 (esiste), `repeat_window_min` 10, `max_grid_mismatch_km` 5000. **Con JTDX**: si fa ora (S-M, serve la distanza).

### H. Auto Call su CQ comuni (`auto_call`)

- La validazione di `Rules` oggi **vieta** di rispondere a ogni CQ (`DecodeTag::Cq`). Aggiungere, separata e spenta, `auto_call`: risponde anche ai
  CQ **non lavorati su questa banda** (etichette `NewCall`/`NewCallBand`/`NewPx*` o senza `WorkedCall`), con **tetto per sessione**
  (`auto_call_max_qso` = 5, mai 0) e con tutte le salvaguardie che esistono (segnale minimo, falsi, un QSO per volta, tetti). Scelta tra i CQ con
  la regola E.
- **Parametri**: `enabled` falso; `auto_call_max_qso` 5 (1-50); si spegne da solo al tetto come fa il tetto di fila.
- **Con JTDX**: si fa ora (S). Decisione da prendere col proprietario: e' una trasmissione a tappeto, per questo il tetto e' obbligatorio.

### I. Tail-ending (`tail_end`)

- Chiamare chi manda un **73/RR73** a un'altra stazione (e' appena finito un QSO: e' libero). Decodium **non** lo fa; JTDX con AutoSeq 6/7 si'.
  Condizione: messaggio di chiusura tra X e Y non nostro, X o Y desiderati; azione: `Reply` sul messaggio (la rotta di Reply di JTDX accetta
  anche i 73 con `AcceptUDPReplyMessages` 2 o 3; oggi e' 1 = solo CQ: va cambiato **nelle impostazioni di JTDX**).
- **Parametri**: `enabled` falso; solo per i desiderati (`reply_to_wanted`). **Con JTDX**: si fa ora (M), con l'avviso sul valore di `AcceptUDPReplyMessages`.

### J. Chi rispondere quando facciamo noi CQ

- Aurora non ha ancora un'automazione di CQ; JTDX risponde ai chiamanti con le sue regole. In futuro, se l'automazione fara' CQ, la coda e la
  scelta si prendono da Decodium: **FIFO di 20**, esclusione di chi e' stato lavorato/abbandonato da meno di 90 s, e scelta a punteggio (regola E)
  dei chiamanti dello stesso ciclo. Con JTDX si puo' pilotare solo il `Reply` su un chiamante alla volta.

### K. Cadenza del CQ

- `cq_burst`: N CQ completati poi M periodi di ascolto (Decodium `AutoCqBurstCalls`/`AutoCqListenCycles`, 0/0 = continuo); dopo 10 CQ senza risposta, se
  `alternate_period` e' acceso, inverte il periodo di Tx (vecchio percorso di Decodium). Utile in banda satura per ascoltare invece di
  occupare. Si fa con `CallCq` ogni periodo scelto. **S**, JTDX. Non e' prioritaria finche' Aurora non fa CQ da sola.

### L. Multi Answer Mode e CQ paralleli

- Richiede di trasmettere piu' flussi FT8 nello stesso periodo con un modulatore proprio: **solo motore proprio** (crate `ft8`, modulo `modem`
  somma piu' toni). Spaziatura minima 60 Hz, frequenza = quella di ciascun chiamante, coda se il canale e' occupato a meno di 50 Hz, abbandono
  dopo `maxRetry` o 6 periodi di silenzio. Decodium stesso lo segna come non validato in aria per FT2. **L**; non farlo ora.

## Come provarle in simulazione (`engine-sim`), senza radio

**Livello 1: prove unitarie** (come `core/crates/qso-engine/tests/brain.rs`; il tempo lo passa il test). Tutta la logica sta in funzioni pure; ogni
regola ha prove che non toccano il motore:

| Regola | Prove (esempi) |
|---|---|
| A | spettro/elenco con 25 segnali: la scelta cade nel varco piu' largo; stessa scelta a parita' di dati; nessun varco = nessuna mossa; mai a QSO avviato; rispetta `min_hz`/`max_hz`; FT4 usa 90 Hz |
| B | 4 fine-trasmissione con `dx_call` = X e stesso Tx1: `HaltTx` + registro; un messaggio di X per noi azzera; `0` non fa nulla |
| C | decode X -> Y (non nostro) per 2 cicli: stop; X -> Y con RR73: nessuno stop; X -> noi: nessuno stop |
| D | dopo la rinuncia nessuna richiamata per `cooldown_min`; dopo il tempo si richiama; QSO registrato azzera |
| E/F/G | tra 5 CQ nello stesso ciclo vince il punteggio atteso; «CQ EU» scartato da fuori Europa; falso debole isolato scartato, lo stesso ripetuto in un altro ciclo passa |
| H/I | tetto di 5 QSO spegne; i 73 vanno solo ai desiderati |

**Livello 2: prove del task** (come `tests/runner.rs`: `Rig` con orologio finto, `Bus`, eventi `Status` e `Decode` iniettati, comandi letti dal canale):
iniettare `Status{transmitting: true->false, dx_call}` e controllare che esca `Command::HaltTx` e poi `Command::Reply` verso il candidato giusto.

**Livello 3: `engine-sim` come banda satura.** Oggi `engine-sim` genera 5-12 messaggi casuali per ciclo e il `Reply` accende solo `transmitting`. Per le
regole servono tre aggiunte, tutte deterministiche dal seme:

1. **Scenario «banda satura»** (variabile `AURORA_SIM_SCENARIO=saturated`, o `SimConfig.scenario`): 30-60 CQ per ciclo con paesi e segnali
   distribuiti, 20+ tracce nello spettro, qualche «DX» desiderato; comunque riproducibile.
2. **Il simulatore ascolta `SetTxFrequency`**: aggiorna `Status.tx_df` e disegna la propria traccia nello spettro durante la trasmissione.
   Regola semplice per la risposta del DX: probabilita' di rispondere **alta se la nostra Tx dista piu' di 60 Hz da ogni chiamante, bassa se
   cade su uno**; questo rende misurabile il guadagno della regola A.
3. **DX scritto**: dopo un `Reply` il DX simulato risponde dopo K tentativi, non risponde mai, oppure risponde a un altro (C), come da
   `SimConfig`; se completa lo scambio emette `QsoLogged`.

Misure da riportare nell'esito: QSO completati per ora simulata, trasmissioni a vuoto, tempo medio fino al QSO, con la regola accesa e spenta,
a parita' di seme. Si avvia anche dal vivo senza radio con `AURORA_ENGINE=sim` e l'automazione **in prova a vuoto**, per vedere il registro delle decisioni
(`GET /api/v1/automation`).

**Ordine consigliato**: D e B (sbloccano tutto, usano solo `Status`), poi C, E+F+G, poi A (prima solo la funzione pura e il suo test, poi il
collegamento a `SetTxFrequency` e la prova con JTDX alla radio, che fa il proprietario), poi H e I.

## Da verificare e limiti di questa analisi

- Decodium e' un programma grande: il percorso **nativo** e' stato letto per intero nelle parti citate; quello **legacy** (`mainwindow.cpp`, 34 mila
  righe) solo dove serviva. Le righe indicate sono del commit `3d4b0b0` e cambiano a ogni versione.
- Il watchdog a tempo non si azzera alla risposta del partner **per quanto ho letto**: non l'ho provato; il modo a periodi si'.
- Nel percorso nativo `alt12Enabled` e' memorizzato e passato al vecchio percorso, ma **non ho trovato** dove il nativo lo usi per invertire il periodo.
- Non e' stato misurato nulla in aria: le soglie di Aurora (4 trasmissioni, 60 Hz, 15 minuti) sono proposte da tarare con la simulazione e poi con la radio.
- Da censire in JTDX: la casella «Hold Tx Freq» (identificativo, effetto del `Reply` sulla Tx) e l'effetto di `SetTxFrequency` a periodo avviato.
- La licenza di Decodium e' GPL 3, come quella di Aurora; qui si sono **riscritte le idee**, nessun codice e' stato copiato.
