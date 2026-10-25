# Logiche di JTDX e come le segue Aurora

Analisi fatta il 2026-10-24 su JTDX 2.2.159 in esecuzione (finestra, `JTDX.ini`, `ALL.TXT`, flusso UDP) e sul manuale incluso
nell'installazione (`share\doc\JTDX\wsjtx-main-2.2.159.html`, guida di WSJT-X). Serve a non indovinare piu': ogni volta che Aurora
ha sbagliato era perche' **ricostruiva** una decisione che JTDX prende e mostra.

## Principio

**JTDX e' la fonte della verita'. Aurora legge, non ricalcola.** La logica di un QSO (quale messaggio mandare, in quale periodo,
quando passare al successivo, quando registrare) sta in JTDX e cambia con le sue impostazioni. Copiarla in Aurora vuol dire avere
due versioni che divergono. Quindi:

1. se JTDX espone il dato (UDP, finestra, file), Aurora lo **legge**;
2. se si deve cambiare, Aurora **comanda JTDX** come farebbe l'utente (UDP o pulsanti) e poi rilegge quello che JTDX ha fatto;
3. solo se JTDX non espone nulla, Aurora ricostruisce, e lo scrive qui come eccezione.

Prima di toccare una logica di QSO, di trasmissione o di selezione dei messaggi, leggere questo file.

> **Aggiornamento del 2026-10-25**: la sezione in fondo a questo file («Aggiornamento del 2026-10-25: reverse di JTDX 2.2.159») corregge alcuni punti (livelli AutoSeq, momento della registrazione, limite di avvio, `NDepth`) e ne aggiunge molti; ha la precedenza sul testo sottostante.

## Come si parla con JTDX

| Canale | Cosa passa | Note |
|---|---|---|
| UDP (hub `127.0.0.1:2334`) | JTDX → Aurora: `Heartbeat`, `Status`, `Decode`, `Clear`, `QsoLogged`, `LoggedADIF`. Aurora → JTDX: `Reply`, `HaltTx`, `FreeText`, `TriggerCQ`, `Replay`, frequenza Tx | `Status` e `Decode` arrivano a **fine periodo** e a ogni cambio di stato, non su richiesta: dopo un riavvio di Aurora il primo puo' arrivare 13-15 s dopo (vedi «Dopo un riavvio») |
| Finestra (accessibilita' di Windows, UI Automation) | stato e pulsanti: tutti i controlli della tabella sotto | circa 0,6-1,1 s a lettura; i comandi si danno premendo i pulsanti (`Invoke`), mai con `Toggle()` |
| OmniRig | frequenza della radio (`FreqA`, split) | `Freq` non e' scrivibile sulla FT-2000 |
| `JTDX.ini` | impostazioni (chiave=valore) | si legge e si modifica con JTDX chiuso, con copia di sicurezza |
| `ALL.TXT`, `wsjtx_log.adi` | tutti i messaggi e i QSO | solo lettura, utile per verificare |

## Come si legge lo stato (tabella dei controlli)

Identificativi (`AutomationId`, la parte finale) dei controlli di JTDX e come li usa Aurora. «Letto» = Aurora ne mostra lo stato;
«Comandato» = Aurora lo preme; «Mai» = deciso di non toccarlo.

| Gruppo | Controllo (id) | Cosa e' | Aurora |
|---|---|---|---|
| Messaggi | `tx1`..`tx6` | i sei testi Tx, come li ha generati JTDX (rapporto vero, CQ come scritto) | letto (`GET /controls` → `tx.messages`) |
| Messaggi | `txrb1`..`txrb6` | **pallino «Next»**: il messaggio da cui JTDX trasmette; avanza da solo con AutoSeq | letto (`tx.selected`) e comandato (`POST /commands/tx-message`, selezione) |
| Messaggi | `txb1`..`txb6` | pulsanti «Now»: cambiano subito il messaggio, anche durante la trasmissione (riavviando la sequenza) | **non usati**: potrebbero tagliare una trasmissione; si usa il pallino |
| Messaggi | `tx5` (combo) | messaggio 5 con le macro | letto come testo |
| Messaggi | `genStdMsgsPushButton` «Gen.Msgs» | rigenera i sei messaggi standard | comandato (`gen_msgs`) |
| QSO | `AutoSeqButton` «AutoSeq n» | sequenza automatica dei messaggi; n e' il livello 0-7 | letto e comandato; il testo (con il livello) e' quello di JTDX |
| QSO | `skipTx1` «Salta Tx1» | rispondendo a un CQ si parte da Tx2 invece di Tx1 | letto e comandato; la scheda barra la riga 1 |
| QSO | `singleQSOButton` «1 QSO» | dopo un QSO finito spegne Abilita Tx | letto e comandato |
| QSO | `AnsB4Button` «Risp B4» | risponde anche a chi si e' gia' lavorato | letto e comandato |
| QSO | `bypassButton` «Bypassa» | salta i filtri del testo | letto e comandato |
| QSO | `rrr1CheckBox` «RRR» | usa RRR al posto di RR73 nel Tx4 | letto e comandato |
| QSO | `TxMinuteButton` «TX 00/30» / «TX 15/45» | **periodo di trasmissione**: pari o dispari; il testo cambia con lo stato e col modo (in FT4 «TX 00») | letto e comandato; Aurora mostra **il testo di JTDX** |
| Trasmissione | `enableTxButton` «Abilita Tx» | arma la trasmissione automatica | letto e comandato (solo con la trasmissione aperta) |
| Trasmissione | `AutoTxButton` «AutoTX» | trasmissione automatica | letto e comandato (solo con la trasmissione aperta) |
| Trasmissione | `stopTxButton` «Ferma Tx» | ferma e disabilita | via UDP `HaltTx` (sempre permesso) |
| Trasmissione | `tuneButton` «Accordo» | portante per l'accordatore | **Mai** |
| Frequenze | `TxFreqSpinBox`, `RxFreqSpinBox` | frequenze audio Tx e Rx | letto (`tx_df`, `rx_df`), comandato (barre sulla cascata) |
| Frequenze | `pbR2T` ▲, `pbT2R` ▼, `pbTxLock` | copia Rx→Tx, Tx→Rx, Tx=Rx | comandato |
| DX | `dxCallEntry`, `dxGridEntry` | stazione in collegamento | letto; impostato col doppio clic (`Reply`) |
| DX | `ClearDxButton` «Canc DX» | libera la stazione | comandato (anche con doppio Esc) |
| DX | `wantedCall`, `wantedGrid`, `wantedCountry`, `cbShowWanted` «Ricerca» | ricerca di una stazione | non usato (comando `show_wanted` disponibile) |
| DX | `cbShowSpot` «Spt» | manda lo spot al cluster/rete | **Mai**: Aurora non manda spot ne' messaggi pubblici |
| Report | `rptSpinBox` «Rapporto -12» | rapporto da mandare | letto (`report`) |
| Modo | `ft8Button`, `ft4Button`, `jtButton`, `HoundButton` | modo e Hound | letto e comandato |
| Banda | `pb160`..`pb2`, `bandComboBox` | banda | letta (frequenza) e comandata (OmniRig) |
| Ricezione | `monitorButton`, `swlButton`, `syncButton`, `AGCcButton`, `filterButton` | interruttori | letti e comandati |
| Ricezione | `DecodeButton`, `EraseButton`, `stopButton` | pulsanti | comandati |
| Livello | `outAttenuation` (0-450 decimi di dB) | cursore Pwr | letto e comandato |
| Log | `logQSOButton` «QSO a Log» | registra il QSO | **Mai**: il log e' di JTDX/Log4OM2 |
| Stato | barra di stato, prima etichetta | «Ricezione» (verde), «Tx» (giallo), «Accordo» (rosso); il testo trasmesso | letta all'avvio; poi vale lo `Status` UDP |
| Orologio | `labUTC`, `labDialFreq` | ora UTC e frequenza | frequenza letta all'avvio |

Il catalogo completo (153 elementi, con stato) si rigenera con una lettura della finestra; gli identificativi stanno anche in
`core/crates/rig-control/src/controls.rs` e `snapshot.rs`.

## Logica della sequenza di trasmissione

### I sei messaggi

| N. | Messaggio | Quando |
|---|---|---|
| Tx1 | `DX ME GRIGLIA` | risposta a un CQ (saltato con «Salta Tx1») |
| Tx2 | `DX ME -07` | rapporto |
| Tx3 | `DX ME R-07` | rapporto con R |
| Tx4 | `DX ME RR73` (o `RRR` con la casella RRR) | chiusura |
| Tx5 | `DX ME 73` | saluto (e' il campo con le macro) |
| Tx6 | `CQ [DX] ME GRIGLIA` | chiamata CQ |

### Il pallino «Next» e il pulsante «Now» (dal manuale)

- «Next» (`txrb`) sceglie il messaggio **da cui parte la prossima trasmissione**.
- «Now» (`txb`) cambia il messaggio **subito**, anche durante la trasmissione; il manuale avverte che a meta' trasmissione riduce
  la probabilita' di decodifica, accettabile nei primi 10-15 s.
- Con AutoSeq il pallino avanza da solo in base a cio' che la stazione chiamata manda; l'utente puo' sempre sceglierlo a mano e
  **JTDX riparte da quella scelta**.

Aurora: la riga evidenziata e' sempre il pallino selezionato di JTDX. **Forte** mentre JTDX trasmette (barra di stato su «Tx»),
**col solo bordo** mentre aspetta il suo turno. Un clic su una riga seleziona quel messaggio in JTDX.

### Sequenza osservata (ALL.TXT, FT4, 2026-10-24)

```
16:24:37  RX  CQ LZ2KSQ KN23                 la stazione chiama CQ
16:24:47  TX  LZ2KSQ IU3ABC -12              rispondiamo col rapporto (Salta Tx1 acceso: si parte da Tx2)
16:25:00  TX  LZ2KSQ IU3ABC -12              nessuna risposta: JTDX ripete lo stesso messaggio
16:25:07  RX  IU3ABC LZ2KSQ R-10             la stazione manda R+rapporto
16:25:15  TX  LZ2KSQ IU3ABC RR73             JTDX passa a Tx4 (chiusura)
16:25:22  RX  IU3ABC LZ2KSQ 73               la stazione chiude: QSO fatto, si registra
```

Regole che se ne ricavano (comportamento di AutoSeq; non va riscritto in Aurora):

- senza risposta JTDX **ripete** il messaggio corrente;
- una risposta a quel messaggio fa passare al successivo; da `R-rapporto` si va a Tx4;
- il QSO si chiude e si registra quando arriva RR73/RRR/73 (`AutoQSOLogging=true`, `PromptToLog=false`);
- dopo il QSO, con «1 QSO», Abilita Tx si spegne.

### AutoSeq: il numero e i contatori (dai testi del programma e da `JTDX.ini`)

Il numero in «AutoSeq n» (qui 3) e' il **livello della sequenza automatica**, da 0 a 7. Dai suggerimenti che JTDX stesso contiene:

- **AutoSeq 0**: sequenza dei messaggi ma **senza scelta automatica** delle stazioni volute (lista dei desiderati non usata);
- **AutoSeq 1..3**: quando un contatore (sotto) raggiunge il suo valore, JTDX **chiama CQ**;
- **AutoSeq 4 e oltre**: scelta automatica delle stazioni in base alle priorita' scelte (nominativi, prefissi, paesi, griglie desiderati;
  l'opzione «chiama prima i nominativi con priorita' alta dei nuovi» e' solo da 4 in su);
- **AutoSeq 6/7**: cerca tra i messaggi decodificati un CQ o un 73 valido e li chiama; se non ne trova, chiama CQ. Si ferma (Halt Tx) se
  «1 QSO» e' acceso.

I livelli 4 e 5 non sono descritti nei testi che ho trovato oltre a quanto scritto qui: non si deduce altro.

I **quattro contatori** (scheda AutoSeq di JTDX; ognuno ha un interruttore e un valore massimo) fermano o fanno ripartire la sequenza
quando una situazione si ripete troppe volte:

| Contatore | Quando conta | Qui |
|---|---|---|
| AnswerCQ | ho risposto a un CQ e non risponde nessuno | spento, massimo 5 |
| AnswerInCall | ho chiamato CQ, ho ricevuto una chiamata e ho risposto | spento, massimo 5 |
| SentRReport | ho mandato il rapporto rispondendo a un rapporto in arrivo | **acceso**, massimo 5 |
| SentRR7373 | ho mandato RR73 (73) rispondendo a un RRR (RR73) in arrivo | **acceso**, massimo 5 |

Altre opzioni della stessa scheda: `SingleShotQSO` (il «1 QSO»), `SeqHaltTxReplyOther` (ferma Tx se la stazione chiamata risponde a un altro),
`StrictDirectionalCQ` (rispetta la direzione del CQ anche per un nuovo DXCC), `CallHigherNewCall`, liste `WantedCall/Prefix/Country/Grid`.

### La casella RRR

Dal suggerimento di JTDX: **senza spunta il Tx4 e' `RR73`, con la spunta e' `RRR`** (`RRR\RR73` nel file delle impostazioni; qui false, quindi RR73).
Con RRR la stazione risponde 73 e il QSO si chiude col suo 73; con RR73 si chiude gia' alla ricezione.

### Il periodo (pari/dispari)

- `TxMinuteButton` acceso = periodo **pari**; JTDX lo imposta da solo rispondendo a una stazione (opposto al suo periodo), come dice il
  manuale: «selezione automatica al doppio clic su una riga decodificata».
- Il testo del pulsante lo scrive JTDX e cambia col modo: FT8 «TX 00/30» (acceso) e «TX 15/45» (spento); FT4 «TX 00». Aurora mostra
  quel testo e non ne inventa altri.
- Una trasmissione parte all'inizio del periodo scelto, o subito se conviene; il manuale di WSJT-X dice che nei modi a minuto non parte
  oltre i 24 s. Il limite per FT8 e FT4 non e' documentato qui (vedi «Da verificare»).

### Doppio clic su una riga decodificata

1. JTDX imposta la stazione DX (nominativo, griglia), la frequenza Rx e il periodo;
2. genera i sei messaggi e seleziona il primo: Tx1, oppure Tx2 con «Salta Tx1»;
3. se «Il doppio clic imposta Abilita Tx» e' attivo, accende Abilita Tx.

Aurora: invia `Reply` (UDP) e, se la trasmissione e' aperta, accende Abilita Tx **in parallelo** (ogni attimo conta per non perdere il
periodo). Quanto JTDX accetta come `Reply` dipende da `AcceptUDPReplyMessages` (1 = solo CQ, 2 = CQ e 73, 3 = qualsiasi): Aurora avvisa
se un messaggio verra' ignorato.

### Impostazioni di JTDX che governano la logica (`JTDX.ini`)

| Chiave | Valore qui | Effetto |
|---|---|---|
| `AutoSequence` | true | AutoSeq acceso |
| `SkipTx1` | true | si parte da Tx2 |
| `TxWatchdogTimer` | 6 | minuti senza risposta dopo i quali JTDX smette di trasmettere |
| `AutoQSOLogging` | true | registra da solo a QSO chiuso |
| `PromptToLog` | false | nessuna finestra di conferma |
| `ClearCallGrid`, `ClearWCallAtLog` | true | libera DX e griglia dopo il log |
| `AcceptUDPRequests` | true | accetta i comandi UDP |
| `AcceptUDPReplyMessages` | 1 | i `Reply` valgono solo per i CQ |
| `LockTxFreq` | false | Tx=Rx |
| `TxDelayInt` | 100 | ritardo PTT (ms) |
| `NDepth` | 3 | profondita' di decodifica |
| `EnableHoundMode` | false | modo Hound |

## Cosa Aurora non fa mai (decisioni)

- non preme «Accordo» (portante) ne' «QSO a Log» (il log resta a JTDX e Log4OM2);
- non trasmette con la trasmissione chiusa (`AURORA_ALLOW_TX=0`): `Reply`, `TriggerCQ`, Abilita Tx e AutoTX la richiedono, `HaltTx` e lo
  spegnimento no;
- non cambia banda, frequenza o impostazioni di JTDX mentre si trasmette;
- non preme i pulsanti «Now» (`txb`): selezionare il pallino non interrompe nulla, «Now» si'.

## Dopo un riavvio di Aurora

JTDX continua a girare, quindi tutto il suo stato e' ancora valido; Aurora deve solo ritrovarlo in fretta:

- **stato**: letto dalla finestra all'avvio (meno di un secondo) invece di aspettare il primo `Status` UDP (13-15 s); il primo `Status`
  vero lo sostituisce;
- **indirizzo e identificativo UDP di JTDX**: ricordati in `data/jtdx-peer.txt`, cosi' `HaltTx` e i comandi partono subito;
- **messaggi decodificati**: restano quelli del periodo in corso (nessun `Replay`: non devono ricomparire stazioni vecchie).

## Errori passati e la regola che ne viene

| Errore | Causa | Regola |
|---|---|---|
| La scheda indicava Tx1 con «Salta Tx1» acceso | prevedeva la fase del QSO dai decodificati | leggere il pallino selezionato di JTDX |
| Riga sbagliata con un rapporto diverso | confronto esatto col modello | usare i testi veri di JTDX, e il tipo di messaggio solo come ripiego |
| «Tx 00/15» in FT4 | etichetta dedotta | leggere il testo del pulsante |
| Cambio banda ignorato dalla radio | si scriveva `Freq`, non scrivibile | scrivere `FreqA` e attendere che la radio la prenda |
| Pulsanti di banda che tornavano indietro | l'ultimo stato UDP, vecchio, sovrascriveva la scelta | tenere la scelta finche' JTDX non la conferma |
| Dati lenti dopo un riavvio | si aspettava il primo pacchetto UDP | leggere lo stato dalla finestra, ricordare l'indirizzo di JTDX |
| Frequenza illeggibile all'avvio | la console rovinava lo spazio non divisibile | uscita delle letture in UTF-8 |

## Da verificare / lacune note

- **Tempo massimo di avvio dopo l'inizio del periodo** (misurato su `ALL.TXT`, 346 trasmissioni di ottobre): una trasmissione regolare e'
  registrata a **0,03-0,15 s** dal bordo del periodo (FT8: mediana 0,09 s; FT4: mediana 0,06 s, 95% sotto 0,12 s). Ci sono pero' righe con
  scarti di 2-12 s (28 su 272 in FT8, 3 su 74 in FT4): non si capisce dal log se sono avvii tardivi o se la riga e' scritta al momento
  della decisione. **Il limite oltre il quale JTDX non parte piu' non risulta**: serve una prova (Abilita Tx a periodo avviato), che fa il
  proprietario;
- casella `rrr1CheckBox` e numero di AutoSeq: risolti sopra; restano i livelli 4 e 5 non descritti;
- gestione di Hound (modo DXpedition) e delle schede 2 e 3 dei messaggi: non esposte;
- le prove di selezione del messaggio a mano con la trasmissione accesa le fa il proprietario.


## Aggiornamento del 2026-10-25: reverse di JTDX 2.2.159 (sorgenti pubblici e REA)

Analisi completa in `docs/analisi-jtdx-reverse-rea.md` (indirizzi, tabelle, confronto con il sorgente). Il sorgente pubblico (ramo `master` di `jtdx-project/jtdx`, 2.2.159, ultimo commit 2022-03-01)
e' stato confrontato con `jtdx.exe` e `jtdxjt9.exe` (REA + Ghidra, che hanno i simboli) nei punti che contano: **coincidono**. Le correzioni qui sotto **hanno la precedenza** sul testo precedente di questo file,
che resta com'e' per la storia. Tutto cio' che segue vale per JTDX 2.2.159; la sigla «sez.» rimanda al documento di analisi.

### Cose sbagliate nel testo precedente

| Dove (sopra) | Cosa diceva | Com'e' |
|---|---|---|
| «AutoSeq: il numero» | livello da 0 a 7; «AutoSeq 4 e oltre» sceglie con le priorita'; i livelli 4 e 5 non descritti | i livelli sono **0, 1, 2, 3, 6, 7**; **4 e 5 non esistono**. Le liste dei desiderati valgono da 1 a 7. Cio' che cambia fra i livelli e' **quando** gira `process_Auto` (0: subito, nessuna scelta di chi chiamare; 1: subito, prende il primo che chiama; 2/6: all'inizio del periodo di trasmissione; 3/7: a fine decodifica, con la trasmissione che riparte se il messaggio cambia) e la **ricerca nei CQ** altrui (solo 6/7) (sez. 2.6) |
| «Sequenza osservata» | il QSO si chiude e si registra quando arriva RR73/RRR/73 | **si registra quando JTDX comincia a trasmettere RR73 o 73** (RRR non conta: con la casella RRR si registra al Tx5); il FIN richiede anche la chiusura del DX. Prova: `ALL.TXT`, «Transmitting ... RR73» e «QSO logged» a 6 ms (sez. 2.8) |
| `AutoQSOLogging` | «registra da solo a QSO chiuso» | registra all'**invio** del primo 73/RR73; con Autolog acceso JTDX **ferma Tx** a S73/FIN (senza Autolog, con «1 QSO» spento e se il QSO era sulla frequenza del proprio CQ, passa da solo al prossimo chiamante o al CQ) (sez. 2.4) |
| «Il periodo»: «una trasmissione parte all'inizio del periodo scelto, o subito se conviene» e «Da verificare: il limite non risulta» | limite non documentato | si puo' partire in qualunque istante dei primi **13,64 s (FT8) / 6,04 s (FT4)** del periodo (`fTR < 99` e' sempre vero: «Allow late starts»); l'avvio tardivo trasmette la **coda** del messaggio dall'istante giusto; se AutoSeq cambia messaggio dopo l'avvio c'e' una **ritrasmissione** (`Retransmitting` in `ALL.TXT`, fino a 13,3 s in FT8, 4,1 s in FT4) (sez. 2.9) |
| `TxMinuteButton` | FT4: «TX 00» | acceso «TX 00», **spento «TX 7.5»** |
| `NDepth` | profondita' di decodifica | **non conta in FT8 ne' in FT4**: in FT8 contano `NFT8Cycles` (3, 6, 9 passate), `FT8Sensitivity` (0, 1 soglie basse, 2 sottopassate) e `NFT8QSORXfreqSensitivity`; in FT4 `NFT4Depth` (sez. 5.1) |
| `AutoTxButton` «AutoTX» | trasmissione automatica | e' l'impostazione `QuickCall`: il doppio clic (o il `Reply`) **accende Enable Tx** |
| Contatori «massimo 5» | valore massimo 5 | il valore e' il **numero di trasmissioni** dello stesso messaggio (la successiva non parte); di fabbrica 2 / 3 / 5 / 4 e tutti spenti (sez. 2.5) |

### Cose nuove

- **Il cervello della sequenza e' `QsoHistory`** (17 stati, tabella per stazione), alimentato da `displayDecodedText` (ricevuto) e `displayTransmittedText` (inviato) e letto da `process_Auto`: stati, classificazione dei messaggi e transizioni
  completi in sez. 2.1-2.4. FIN solo se le due chiusure si sono incrociate; RR73/73 senza rapporto ricevuto non fanno niente.
- **Contatori** (sez. 2.5): AnswerCQ **non vale per le stazioni volute (priorita' 2-4 e 17-19) ne' per i nuovi DXCC**; `reply_other` libera il DX solo se il contatore e' acceso, ma `readFromStdout` **ferma Tx** quando il DX parla con un altro entro la
  **banda di guardia (FT8 51 Hz, FT4 84)** o sempre con `SeqHaltTxReplyOther`; cooldown dopo una rinuncia; CQ diretto a un'altra zona = `count 1`; «1 QSO» ferma anche quando un contatore libera il DX.
- **Watchdog** (sez. 2.7): conta **minuti con trasmissione recente** (32 s FT8, 16 s FT4), si azzera con un messaggio **diverso dal precedente** e con RCALL/RREPORT.
- **Scelta di chi chiamare** (sez. 3): priorita' 0-31 (nuovo CQZ 30-31, ITUZ 26-27, DXCC 22-23 e su banda 20-21, locatore 15-16 e 13-14, prefisso 11-12 e 9-10, nominativo 7-8 e 5-6, desiderati 2-4 e 17-19), tre blocchi (risposte al mio CQ
  nell'ultimo periodo; nei 5 minuti; CQ degli altri con AutoSeq 6/7) e le regole di sorpasso; senza evidenziazioni di «nuovo» JTDX risponde a chiunque, con evidenziazioni e «Risp B4» spento ignora chi ha priorita' 0. **I filtri di visualizzazione
  nascondono le righe a `QsoHistory`** (e a UDP con `filterUDP`).
- **Periodo**: opposto a quello della riga cliccata; **AutoSeq non lo cambia mai**; con AutoSeq 6/7 si cercano CQ solo nel periodo che segue quello decodificato (sez. 3.1).
- **Il doppio clic sceglie il messaggio dal testo della riga** (Tx2, Tx3, Tx4 o Tx5 se contiene il mio nominativo; Tx1/Tx2 se e' un CQ) (sez. 2.10). Con AutoSeq acceso i sei pallini `txrb` sono **disattivati** (`enableTab1TXRB(false)`).
- **Decoder** (sez. 4 e 5): i filtri sui falsi sono una catena (sincronismo duro `nsync >= 7`, punteggi di sincronismo, BP 30 iterazioni e OSD con `nharderrors + dmin < 60`, plausibilita' del testo con `chkfalse8` + `chkflscall` +
  `chkgrid`, duplicati a 45 Hz) basata anche su `ALLCALL7.TXT` (100 515 nominativi) e su una tabella geografica locatore-prefisso; in FT8 si decodifica con cicli, sensibilita', thread a fette di banda, **cronologia dei periodi** (somma
  dei simboli dell'ultimo periodo per CQ e chiamate a noi), analisi a priori guidate dallo stato del QSO e il decoder dedicato `ft8s` sul canale del QSO.

### Cosa e' confermato

Tx1-Tx6 e i testi; pallino «Next» contro pulsante «Now» (quest'ultimo riavvia la trasmissione in corso); ripetizione del messaggio senza risposta e avanzamento con la risposta; regola del periodo opposto; casella RRR (cambia solo il Tx4); «Salta Tx1» (partenza
da Tx2, con il rapporto che si aggiorna a ogni periodo); `Reply` UDP = stesso percorso del doppio clic (`replyToUDP` -> `processMessage`); i quattro contatori e le loro chiavi; lista nera e «Canc DX».

### Aggiornamento serale del 2026-10-25: evidenziazioni, «gia' lavorato» e schede Sequenza/Generali/Notifiche

Letto in `jtdx.exe` 2.2.159 (simboli presenti) e nei sorgenti; il dettaglio voce per voce, con indirizzi, chiavi e valori di fabbrica, sta in `docs/impostazioni-motore-proprio.md` («Le schede Sequenza, Generali e Notifiche come in JTDX»).

- **«Gia' lavorato» = cio' che resta quando niente e' nuovo**: `DisplayText::displayDecodedText` (0x4eb170) calcola il presupposto «gia' lavorato» solo se almeno una delle sei evidenziazioni di «nuovo» e' accesa, poi controlla zona CQ, ITU, paese, locatore, prefisso, nominativo (in assoluto e poi per banda/modo) e il primo «nuovo» trovato cancella barra e sottolineatura. Il barrato (`workedStriked`, di fabbrica acceso) o la sottolineatura riguardano **solo il testo del messaggio** (da colonna 21/23: `DisplayText::appendText`), non ora e rapporto; il colore «Gia' lavorato» c'e' solo con `workedColor` (di fabbrica spento).
- **Banda e modo** (`LogBook::match*`): casella «per banda» da sola = stessa banda in qualunque modo; «per modo» da sola = stesso modo su qualunque banda; tutte e due = stessa banda e stesso modo. FT8 e FT4 sono modi diversi. Il log e' `wsjtx_log.adi`, filtrabile (nominativo, griglia, data) dalla scheda Generali (`ADIF::load`).
- **Sequenza**: i quattro contatori, `SeqHaltTxReplyOther` (di fabbrica acceso) e `StrictDirectionalCQ` sono in `Configuration::read_settings`; `CallPriorityCQ` porta ai livelli 6/7 di AutoSeq (ricerca fra i CQ) e `MaxDistance` fa preferire il piu' lontano a parita' (sez. 2.6 e 3.4 sopra). In Aurora il pannello QSO e la scheda Impostazioni mostrano e cambiano lo **stesso** stato (i controlli `autoseq`, `single_qso`, `skip_tx1`, `answer_b4`), le due priorita' e i contatori stanno in `settings-motore.json` e arrivano alla sequenza a caldo.
- **Generali**: `TxQSYAllowed` disabilita il campo della frequenza Tx e blocca `setRig` durante la trasmissione; `MonitorOFF` spegne Monitor all'avvio; `MonitorLastUsed` riporta la radio sull'ultima frequenza solo riaccendendo Monitor; `Decode52`, `After73`, `IDint` valgono solo fuori da FT8 e FT4 (letti da `setStopHSym`, dalle azioni JT65/JT9/T10 e da `guiUpdate` dentro `!m_mode.startsWith("FT")`).
- **Notifiche**: `differentBackground` + `cyanBackground`/`yellowBackground` danno ai messaggi che contengono il mio nominativo lo sfondo ciano o giallo; `highlightDXCall`/`highlightDXGrid` evidenziano il DX; `AlertOn*` (con `OB` = su banda) e `BeepOn*` sono i due canali dei suoni.

### Da verificare ancora

Esecuzione reale di FIN/S73 con Autolog (letta nel sorgente, non provata in aria); la logica riga per riga di `ft8s`, `ft8sd`, `ft8mf1`, `ft8mfcq`; il modo Hound; il motivo di `count = 1` per i CQ diretti.


## Verifica del 2026-10-25 (seconda parte): sequenza, contatori, avvio tardivo e banda satura

Il sequenziatore di Aurora (`core/crates/qso-native`) e' stato riscritto come copia fedele di JTDX e verificato punto per punto sul binario 2.2.159 (indirizzi tra parentesi) oltre che sul sorgente pubblico.
Qui sotto: per ogni regola se **coincide**, e' stata **corretta** o **non e' verificabile**.

### Come e' fatto ora

- `history.rs` e' `QsoHistory` (`qsohistory.cpp`): 17 stati, tabella per nominativo base, `message()` per i messaggi ricevuti (R) e inviati (S), `autoseq_call`/`autoseq_pick` con i tre blocchi A (risposte al mio CQ nell'ultimo
  periodo), B (entro 300 s dal mio ultimo CQ), C (CQ degli altri, solo AutoSeq 6/7) e i flag 1, 16, 32, 64, 128, 256. Gli orari sono secondi del giorno (u32, `wrapping_sub`).
- `classify.rs` e' `displayDecodedText` / `displayTransmittedText`; `sequencer.rs` e' `readFromStdout`, `process_Auto` e il pezzo di `guiUpdate` che sceglie e avvia il messaggio.
- Differenze volute rispetto al sorgente: il continente sconosciuto (`??`) lascia passare i filtri di direzione del CQ; gli orari sono secondi del giorno; la prova della parita' del periodo usa il periodo di decodifica
  voluto, non l'orario di avvio del decodificatore (nel binario quel confronto si comporta in modo strano); l'ordine di scansione e' quello di una `BTreeMap`.

### Regola per regola

| Regola | Esito | Prova nel binario | Nel codice |
|---|---|---|---|
| Registro del QSO al primo 73/RR73 trasmesso; RRR non conta; guardia anti-doppione di 2 periodi | **coincide** | `guiUpdate` `0x4c4350` (`m_sentFirst73`) | `log_qso`, test `scenari.rs` |
| Autolog: Tx fermo a FIN/S73; senza Autolog il DX e' liberato dopo l'RR73 se `tx == 1` o il DX e' lontano dalla nostra frequenza | **coincide** | `process_Auto` `0x47d3e0` | `auto_stop_tx`, `with_autolog_the_transmission_stops_at_the_end_of_the_qso` |
| Contatori (AnswerCQ, AnswerInCall, SentRReport, SentRR7373): contano le trasmissioni, la (N+1)-esima non parte | **coincide** | `process_Auto` | `process_auto`; di fabbrica Aurora usa la configurazione del proprietario (spenti, spenti, 5, 5), i valori di fabbrica di JTDX (2/3/5/4) sono `RepeatLimits::jtdx_factory_values()` |
| AnswerCQ non vale per i desiderati (priorita' 2-4 e da 17) se non c'e' `StrictDirectionalCQ` | **coincide** | `process_Auto` | `regole_jtdx.rs` |
| Pausa per stazione (`m_counter`) e `calllist` (5 minuti salvo SNR migliore) | **coincide** | `process_Auto` | `history.rs`, `regole_jtdx.rs` |
| Passaggio automatico a Tx6/CQ quando un contatore libera il DX; `m_reply_other`, `m_reply_me`, `m_reply_CQ73` | **coincide** | `process_Auto` | test dedicati |
| Livelli AutoSeq 0, 1, 2, 3, 6, 7 (CallMode e CallPriorityCQ) | **coincide** | `process_Auto`, `guiUpdate` | test per i livelli 0/1/3/7 |
| Banda di guardia (FT8 51 Hz, FT4 84) e Tx fermo se il DX parla con altri entro la banda o sempre con `SeqHaltTxReplyOther` | **coincide** | `readFromStdout` `0x63c0a0` | `halt_tx_reason`, `Options.guard_hz` |
| Watchdog: conta i minuti UTC solo se l'ultimo Tx e' finito da non oltre 32 s (FT8) / 16 s (FT4); azzerato da un messaggio nuovo, da RCALL/RREPORT e dall'attivita' dell'utente | **coincide** | `txwatchdog`, `eventFilter` | `advance_clock`, test `user_activity` |
| Avvio tardivo: finestra 13,64 s (FT8) / 6,04 s (FT4); si trasmette la **coda** (`m_ic = (mstr - 500) * 48`); `m_restart` -> «Retransmitting» | **coincide** | `guiUpdate` `0x4c4350`, `Modulator` `0x6401f0`/`0x640050` | `tx-native` (`max_late`, `cut_head`, `Transmitter::replace`), `Tx.retransmit`, riga `Retransmitting` in `ALL.TXT` |
| Hound (modo DXpedition): oltre 3 ripetizioni di R+rapporto, RRR73 = log e Tx fermo, mai CQ | **coincide** per le parti lette; **non verificabile** il resto | `process_Auto`, `readFromStdout` | `Options.hound`, test dedicato; resta minimo |
| Scelta di chi chiamare con le priorita' 0-31 e le regole di sorpasso | **coincide** | `displayDecodedText` `0x4eb170`, `autoseq` `0x494220` | `history.rs`; le priorita' arrivano dal registro (`caller_info_provider` in `engine-native/src/lib.rs`, senza LoTW) |
| Doppio clic con QuickCall (AutoTX) accende Abilita Tx | **coincide** | sez. 11.8 | `TxDriver::handle` |
| Esc, Invia, AutoSeq con testo libero, `split`/`pbTxLock` | **corretto** | sez. 11.8 di `analisi-jtdx-reverse-rea.md` | vedi la tabella li' |
| «Best caller first» (Decodium 1.0.711): SNR limitato a [-24, 10] -> (snr+24)*30/34; attesa fino a 90 s -> 0..20; DXCC nuovo +40 o nuovo su banda +25; gia' lavorato su banda -35, altrove -12; fissato = 1000; ordinamento stabile, tolleranza 1e-9 | **coincide** con le costanti del binario 1.0.711 | `analisi-decodium-banda-satura.md` | `score.rs`; scelta per punteggio con `Options.caller_selection = BestCallerFirst` (blocchi A e B; il blocco C resta di JTDX) |
| Tx in zona libera (regola A) | **non esiste ne' in JTDX ne' in Decodium** (nessuna stringa, nessun confronto con le frequenze sentite) | ricerca di stringhe e riferimenti su entrambi i binari | **funzione nuova** (`freefreq.rs`, `TxDriver::set_free_tx`): spenta di base (variabile `AURORA_FREE_TX=1`), solo con Split acceso, solo all'inizio di un QSO nostro e dopo due trasmissioni senza risposta, mai a QSO avviato |

### Cosa resta

- Regole G (falsi: conferma per ripetizione e per distanza), H (Auto Call su CQ comuni), K (cadenza del CQ), L (Multi Answer Mode e CQ paralleli) di `docs/analisi-decodium-banda-satura.md`: **non realizzate**; non sono comportamenti di JTDX.
- LoTW nelle priorita' (31, 29, ...): Aurora non lo conosce.
- La logica riga per riga di `ft8s`, `ft8sd`, `ft8mf1`, `ft8mfcq` e il motivo di `count = 1` per i CQ diretti restano **non verificabili** dall'esterno.
- La prova in aria (avvio tardivo, ritrasmissione, FIN con Autolog) la fa il proprietario: nei test c'e' un orologio finto, non una radio.

## Scelta del proprietario: «nuovo su banda» per paese e zone

Per paese (DXCC) e zone CQ/ITU, se la casella «Banda» e' accesa vale la sola banda in qualunque modo anche quando e' accesa «Banda+Modo» (JTDX in quel caso guarda banda e modo insieme). Un paese gia' collegato su quella banda, con qualunque nominativo e in qualunque modo, non da' «DXCC BANDA». Locatore, prefisso e nominativo restano come JTDX (`criterion_news` in `engine-jtdx/src/worked.rs`).
