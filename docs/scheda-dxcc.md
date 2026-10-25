# Scheda DXCC per banda e modo

La scheda dice, dai QSO gia' fatti, **quali paesi DXCC hai lavorato su quale banda e in quale modo**, e per una stazione (di solito il DX che stai chiamando)
accende sei etichette: NEW ONE, NEW BAND, NEW MODE (per il paese) e NEW GRID, GRID BAND, GRID MODE (per il locatore). Questo documento spiega la logica con
precisione, da dove viene e come si calcola. Il codice e' in `core/crates/engine-jtdx/src/matrix.rs` (la logica) e `worked.rs` (gli insiemi dei lavorati,
gli stessi che colorano le righe delle decodifiche).

## Dove si vede

- **Pagina Statistiche**: sezione «Scheda DXCC per banda e modo» in alto. Sei etichette, la stazione DX attuale (o un nominativo che scrivi per provare), la
  matrice (righe FT8, FT4, DIG; colonne 160, 80, 60, 40, 30, 20, 17, 15, 12, 10, 6, 4 e U; in alto a sinistra il totale dei paesi lavorati), i pulsanti
  *Paese* / *Totali* e, in un riquadro che si apre, la tabella di tutti i paesi lavorati con una casella per banda (filtrabile per modo e per nome).
- **Pagina Stazione, blocco Stazione DX** (pannello Trasmissione): le stesse sei etichette in piccolo per il DX attuale, sulla banda e nel modo della radio.
- Si aggiorna da sola: a ogni QSO registrato (evento `qso_logged` del flusso) la pagina rilegge la scheda.
- Colori: verde = lavorato, arancio = lavorato ma non confermato (o, nei totali, confermato solo in parte), grigio = mai. Etichette accese: ambra per il
  paese, ciano per il locatore. Tutti i colori vengono dai token del tema (chiaro e scuro).

## Da dove viene la logica

| Fonte | Cosa si e' letto | Evidenza |
|---|---|---|
| Sorgenti GPL di JTDX (`logbook/logbook.cpp`, `logbook/adif.cpp`, `displaytext.cpp`) | Le funzioni `LogBook::matchDXCC`, `matchGrid`, `ADIF::matchCountry`, `matchGrid`, `ADIF::load`, `ADIF::add` e il blocco di `DisplayText::displayDecodedText` che le chiama | Letti riga per riga |
| `jtdx.exe` 2.2.159 installato (analisi del codice macchina, simboli presenti) | `LogBook::matchDXCC` (0x61e230), `ADIF::matchCountry` (0x5ecf80), `ADIF::matchGrid` (0x5f4860), `DisplayText::displayDecodedText` (0x4eb170), `ADIF::load` (0x5f01c0) | Coincidono con i sorgenti: stessa struttura a quattro rami (vedi sotto); `matchDXCC` e' chiamata solo da `displayDecodedText`, in quattro punti (frequenza e modo, solo frequenza, solo modo, nessuno dei due) |
| `decodium.exe` 1.0.711 installato e sorgente GPL 1.0.705 (`DecodiumBridge.cpp`) | Insiemi `WorkedSets` e riempimento in `appendWorkedQsoBand`; calcolo dei flag `dxIsNew*` | Nell'eseguibile i flag sono solo `dxIsNewDxcc`, `dxIsNewDxccBand`, `dxIsNewContinent(Band)`, `dxIsNewCqZone(Band)`, `dxIsNewItuZone(Band)`, `dxIsNewGrid`, `dxIsNewGridBand`, `dxIsNewCall`, `dxIsNewCallBand`: **non esiste nessun flag «nuovo modo»** (nessuna stringa `dxIsNewMode` o simile) |

### JTDX

Per ogni decodifica JTDX chiede al log quattro cose diverse, tutte con lo stesso schema (`matchDXCC`, identica per CQ, ITU, locatore, prefisso e nominativo):

1. `WorkedBefore` = il paese e' nel log? (`matchCountry(paese, "", "")`, nessun filtro)
2. se **no**: `WorkedBeforeBandMode = false` (un paese nuovo e' nuovo anche su ogni banda e modo);
3. se **si** e c'e' almeno la banda o il modo: `WorkedBeforeBandMode = matchCountry(paese, banda, modo)`, dove **banda e modo possono essere vuoti
   indipendentemente**: banda vuota = «in qualunque banda», modo vuoto = «in qualunque modo»;
4. se il paese non e' riconosciuto (`  ,?,` nel risultato di `CountryDat::find`) o manca il nominativo: `WorkedBefore = true` e `WorkedBeforeBandMode = true`
   (cioe' **niente di nuovo**).

Le due caselle delle impostazioni (`newDXCCBand_check_box`, `newDXCCBandMode_check_box`, uguali per zona CQ, ITU, locatore, prefisso e nominativo, stringhe
«New DXCC», «New DXCC on Band/Mode») scelgono con quali argomenti si chiama: solo banda; solo modo (casella «Band/Mode» senza «Band», frequenza 0 e modo
dell'applicazione); banda e modo insieme. Esiste **un solo colore** per «New DXCC on Band/Mode»: banda, modo e banda+modo si vedono tutti con lo stesso colore.

Come JTDX conta i modi (`ADIF::load`): `MODE` in maiuscolo; se e' `MFSK` si usa `SUBMODE` (cosi' FT4 scritto da JTDX come `MFSK`/`FT4` conta come `FT4`);
`JT9*` diventa `JT9`, `JT65*` diventa `JT65`; il confronto e' fra testi: **FT8 e FT4 sono modi diversi**. Il paese e' identificato dai primi tre campi del
record di `cty.dat` (nome, continente...), non dal prefisso. La banda del decode viene dalla frequenza di dial con una tabella fissa (`ADIF::bandFromFrequency`:
160 m 1,8-2,0 MHz, 80 m 3,5-4,0, 60 m 5,1-5,45, 40 m 7,0-7,3, 30 m 10,0-10,15, 20 m 14,0-14,35, 17 m 18,068-18,168, 15 m 21,0-21,45, 12 m 24,89-24,99,
10 m 28,0-29,7, 6 m 50-54, 4 m 70-71, 2 m 144-148, 70 cm 420-450, e le microonde fino a 4 mm).

Nota: `ADIF::load` e `ADIF::add` scrivono il conteggio per «paese+banda», «paese+modo» e «paese+banda+modo» leggendo per errore il contatore del solo paese; e' un
difetto innocuo perche' conta solo se il valore e' maggiore di zero. Il filtro `STATION_CALLSIGN`/`MY_GRIDSQUARE`/data iniziale che JTDX puo' applicare al log
(`mycall`, `mygrid`, `mydate`) in Aurora non c'e': si contano tutti i QSO del log.

### Decodium

Decodium tiene insiemi di **paese e banda** (`dxccByBand`, chiave `BANDA|PAESE`), **locatore e banda** (`gridByBand`) e simili per continente, zona CQ e ITU; il
**modo conta solo per il nominativo** (`callByBandMode`, chiave banda+modo+nominativo, il «gia' lavorato» esatto) e per le opzioni di evidenziazione per modo.
Quindi in Decodium «DXCC su nuova banda» c'e', «DXCC in nuovo modo» no.

## Le sei etichette di Aurora

Ogni etichetta e' un si/no indipendente, calcolato per una stazione (nominativo, locatore se noto) **sulla banda e nel modo correnti**. Si basa sul log
unito (vedi sotto). `NEW ONE` accende anche `NEW BAND` e `NEW MODE` (un paese nuovo e' nuovo su ogni banda e in ogni modo), come in JTDX; lo stesso per
`NEW GRID` con `GRID BAND` e `GRID MODE`.

| Etichetta | Si accende se... | Equivale in JTDX a |
|---|---|---|
| **NEW ONE** | il paese DXCC della stazione non e' mai stato collegato, su nessuna banda e in nessun modo | `matchDXCC(paese, "", "")` falso |
| **NEW BAND** | il paese non e' mai stato collegato su **questa banda** (in nessun modo: anche CW e fonia dei log esterni contano) | `matchDXCC(paese, banda, "")` falso |
| **NEW MODE** | il paese non e' mai stato collegato in **questo modo** (FT8, FT4 oppure «DIG»), su nessuna banda | `matchDXCC(paese, "", modo)` falso |
| **NEW GRID** | il locatore a 4 caratteri non e' mai stato collegato | `matchGrid(g4, "", "")` falso |
| **GRID BAND** | il locatore non e' mai stato collegato su questa banda | `matchGrid(g4, banda, "")` falso |
| **GRID MODE** | il locatore non e' mai stato collegato in questo modo | `matchGrid(g4, "", modo)` falso |

La combinazione banda **e** modo insieme (la terza casella di JTDX) non e' una delle sei: e' la «casella» della matrice, quella che si colora.

### Cosa conta come «stesso modo»

- **FT8** e **FT4** sono modi distinti (due righe della scheda, due modi per «NEW MODE»).
- **DIG** raccoglie tutti gli altri modi digitali: JT65, JT9, JT4, T10, FST4, Q65, MSK144, WSPR, JS8, RTTY, PSK, MFSK (tranne FT4)... Un QSO in RTTY rende
  «DIG» gia' fatto per quel paese: un decode JT65 dello stesso paese non e' un modo nuovo.
- **CW** e **fonia** (SSB, USB, LSB, AM, FM, voce digitale) dei log esterni contano per il paese e per la banda (NEW ONE, NEW BAND), non per il modo dei decode
  e non hanno una riga nella scheda.
- `MFSK` con sottomodo `FT4` e' FT4. Un QSO senza modo nel log conta per paese e banda ma non per un modo preciso.
- Un decode senza modo (radio non letta) lascia spente NEW MODE e GRID MODE, salvo NEW ONE/NEW GRID.

### Bande

Le colonne sono 160, 80, 60, 40, 30, 20, 17, 15, 12, 10, 6 e 4 metri e **U**, che raccoglie tutto cio' che e' piu' corto di 4 metri: 2 m, 1,25 m, 70 cm,
23 cm e oltre fino alle onde millimetriche. 2200 m e 630 m non hanno colonna (contano per NEW ONE e NEW BAND, non per la matrice). La banda di un decode viene
dalla frequenza di dial con la tabella di JTDX (sopra, quindi anche 4 m, 70 cm e le altre); la banda di un QSO del log e' il campo `BAND` dell'ADIF, o, se manca,
quella della frequenza `FREQ`.

### Stazione senza paese noto

Se `cty.dat` non riconosce il nominativo (nominativi non validi, `<...>`, prefissi mancanti) **le tre etichette del paese restano spente**: non si inventa un
NEW ONE. Il locatore, se presente, funziona lo stesso (NEW GRID / GRID BAND / GRID MODE). Senza banda o senza modo le etichette «per banda» e «per modo» restano
spente, come in JTDX; fa eccezione quando il paese (o il locatore) e' nuovo del tutto.

### Esempio

Log di prova: Atlantis lavorata in FT8 su 20 m (locatore FN42) e in FT4 su 40 m (FN42); Lemuria in CW su 20 m e in FT8 su 2 m; Utopia in RTTY su 17 m.

| Stazione sulla radio | NEW ONE | NEW BAND | NEW MODE | NEW GRID | GRID BAND | GRID MODE |
|---|---|---|---|---|---|---|
| Altra Atlantis, 40 m, FT8, FN42 | no | no | no | no | no | no |
| Altra Atlantis, 15 m, FT8, FN42 | no | **si** | no | no | **si** | no |
| Lemuria, 20 m, FT4 | no | no | **si** (FT4 mai in Lemuria) | | | |
| Lemuria, 20 m, FT8 | no | no | no (FT8 c'e' su 2 m) | | | |
| Utopia, 17 m, FT8 | no | no | **si** (solo DIG) | | | |
| Utopia, 17 m, JT65 | no | no | no (RTTY e JT65 sono DIG) | | | |
| Paese mai lavorato, FN42ab, 20 m, FT8 | **si** | **si** | **si** | no | no | no |
| Atlantis, locatore JJ00 mai fatto | no | no | no | **si** | **si** | **si** |

La matrice dello stesso log: riga FT8, colonne 20 e U verdi; riga FT4, colonna 40 verde; riga DIG, colonna 17 arancio solo se il QSO e' noto come non
confermato; totale paesi lavorati 3 (Atlantis, Lemuria, Utopia). Nei totali (vista *Totali*) ogni casella mostra quanti paesi sono stati lavorati in quella
banda e modo.

## Confermato o no (verde e arancio)

Il log di Aurora e il file `wsjtx_log.adi` non dicono nulla sulle conferme: i loro QSO fanno la casella **verde**. I log esterni invece le conoscono:
nell'ADIF `QSL_RCVD`, `LOTW_QSL_RCVD`, `EQSL_QSL_RCVD`; in Log4OM2 le colonne `qslrcvd`/`qslreceived`, `lotwqslrcvd`/`lotwreceived`, `eqslqslrcvd`/`eqslreceived`
(se la tabella `Log` le ha). Regole:

- Per un **paese** (casella): **verde** se almeno un QSO e' confermato oppure senza informazione; **arancio** solo se tutti i QSO della casella sono noti come
  non confermati; **grigio** senza QSO.
- Per i **totali** (quanti paesi in una casella): verde se tutti i paesi sono confermati o senza informazione; **arancio** se almeno uno e' non confermato
  (cioe' confermato solo in parte).
- Le sei etichette non guardano le conferme: un paese lavorato e non confermato non e' un NEW ONE.

Le colonne di Log4OM2 sono state scelte per nome: se il file del proprietario le chiama in altro modo, la conferma resta sconosciuta (tutto verde) finche' non
si aggiunge il nome in `CONFIRMATION_COLUMNS` (`core/crates/logbook/src/log4om.rs`). Da verificare con un log vero.

## I dati: il log unito

La scheda e le etichette usano gli stessi QSO, letti da tutte le sorgenti e uniti **senza doppioni**:

1. i file ADIF: `aurora_log.adi` nella cartella dei dati (con il motore proprio) e il file di `AURORA_WORKED_ADIF` (di norma `wsjtx_log.adi` di JTDX);
2. il **log interno** `logbook.db` (SQLite): ogni QSO registrato da Aurora e quelli importati da un file ADIF;
3. il log di **Log4OM2**, se `AURORA_LOG4OM_DB` e' impostato, in sola lettura (`logbook::log4om::read_all`).

Lo stesso QSO presente in piu' sorgenti (per esempio JTDX lo scrive in `wsjtx_log.adi` e lo manda anche a Log4OM2) conta una volta: la chiave e'
nominativo, banda, modo, giorno e **minuto** di inizio. Chi non usa nessun log esterno ha solo il log interno e il file di Aurora: basta.

Il caricamento avviene all'avvio; la scheda e le statistiche rileggono le sorgenti a ogni richiesta (un QSO fatto in Log4OM2 compare senza riavviare) e
sostituiscono gli insiemi condivisi, quindi **anche le etichette delle righe delle decodifiche** si allineano. Quando Aurora registra un QSO, lo aggiunge
subito agli insiemi (nominativo, locatore, banda e modo).

## Le righe delle decodifiche usano la stessa logica

Le etichette colorate delle righe (`DecodeTag`: DXCC, DXCC banda, zona, griglia...) leggono gli **stessi insiemi** (`WorkedLog`): paesi lavorati, paesi per
banda, locatori e locatori per banda, ora anche paesi per modo e locatori per modo. Per non cambiare i colori esistenti la riga continua a distinguere solo
«nuovo» e «nuovo su banda» (come JTDX con la sola casella «Band»): il modo non cambia il colore della riga. NEW MODE e GRID MODE si vedono nella scheda e nel
blocco della Stazione DX. Se un giorno servira' il colore «per modo» sulla riga basta chiamare `WorkedLog::labels` per ogni decode (e' un calcolo su insiemi
in memoria, microsecondi).

## API

`GET /api/v1/dxcc-matrix` (vedi `docs/api.md`): paesi lavorati per banda e modo, contatori e, con `call` (e facoltativi `grid`, `band`, `mode`), `subject` con le
sei etichette e le caselle del suo paese; con `compact=1` senza l'elenco dei paesi.

## Cosa resta

- Un colore «per modo» sulle righe delle decodifiche (oggi solo nella scheda).
- Provare le conferme con un log vero di Log4OM2 (nomi delle colonne) e con un ADIF con `LOTW_QSL_RCVD`.
- Le righe PH e CW non ci sono per scelta (la scheda e' per i modi digitali); CW e fonia dei log esterni contano comunque per NEW ONE e NEW BAND.
