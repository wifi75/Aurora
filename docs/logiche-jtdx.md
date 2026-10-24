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
| QSO | `AutoSeqButton` «AutoSeq n» | sequenza automatica dei messaggi | letto e comandato |
| QSO | `skipTx1` «Salta Tx1» | rispondendo a un CQ si parte da Tx2 invece di Tx1 | letto e comandato; la scheda barra la riga 1 |
| QSO | `singleQSOButton` «1 QSO» | dopo un QSO finito spegne Abilita Tx | letto e comandato |
| QSO | `AnsB4Button` «Risp B4» | risponde anche a chi si e' gia' lavorato | letto e comandato |
| QSO | `bypassButton` «Bypassa» | salta i filtri del testo | letto e comandato |
| QSO | `rrr1CheckBox` «RRR» | usa RRR al posto di RR73 nel Tx4 | non esposto |
| QSO | `TxMinuteButton` «TX 00/30» / «TX 15/45» | **periodo di trasmissione**: pari o dispari; il testo cambia con lo stato e col modo (in FT4 «TX 00») | letto e comandato; Aurora mostra **il testo di JTDX** |
| Trasmissione | `enableTxButton` «Abilita Tx» | arma la trasmissione automatica | letto e comandato (solo con la trasmissione aperta) |
| Trasmissione | `AutoTxButton` «AutoTX» | trasmissione automatica | letto e comandato (solo con la trasmissione aperta) |
| Trasmissione | `stopTxButton` «Ferma Tx» | ferma e disabilita | via UDP `HaltTx` (sempre permesso) |
| Trasmissione | `tuneButton` «Accordo» | portante per l'accordatore | **Mai** |
| Frequenze | `TxFreqSpinBox`, `RxFreqSpinBox` | frequenze audio Tx e Rx | letto (`tx_df`, `rx_df`), comandato (barre sulla cascata) |
| Frequenze | `pbR2T` ▲, `pbT2R` ▼, `pbTxLock` | copia Rx→Tx, Tx→Rx, Tx=Rx | comandato |
| DX | `dxCallEntry`, `dxGridEntry` | stazione in collegamento | letto; impostato col doppio clic (`Reply`) |
| DX | `ClearDxButton` «Canc DX» | libera la stazione | comandato (anche con doppio Esc) |
| DX | `wantedCall`, `wantedGrid`, `wantedCountry`, `cbShowWanted` | ricerca di una stazione | non usato |
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
