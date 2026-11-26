# Il formato a cartella dei contest

Proposta: ogni contest ha una **cartella** in `contest/` (radice del repository, accanto a `core/`, `app/`, `desktop/`). Una correzione si fa in un solo punto e Aurora legge i file. Contest gia' migrati (elenco): `contest/ARRL 10 Meter Contest/`, `contest/ARRL International DX-CW Contest/`, `contest/ARRL International DX-SSB Contest/`, `contest/CQ WW DX CW Contest/`, `contest/CQ WW DX SSB Contest/`, `contest/CQ WW RTTY DX Contest/`, `contest/ARRL RTTY Roundup/`, `contest/UBA DX CW Contest/`, `contest/UBA DX SSB Contest/`, `contest/REF Contest CW/`, `contest/EUDX Contest/`, `contest/WAE DX Contest CW/`, `contest/WAE DX Contest RTTY/`. **Stato:** il lettore c'e' (`core/crates/contest/src/cartelle.rs`): Aurora legge le cartelle e le usa come i `.contest`; le schede dichiarate sono lette e tenute ma l'interfaccia non le usa ancora (vedi «Da fare nel motore»).

```
contest/<Nome del contest>/
  contest.toml     metadati e dichiarazioni (TOML)
  regolamento.md   regolamento riassunto in italiano, con fonte (URL), edizione, data di lettura e differenze trovate
  punteggio.regole le regole del punteggio nella sintassi della sezione [Score] di un .contest, senza l'intestazione
  punteggio.md     (facoltativo) spiegazione riga per riga e cio' che il motore non copre
  punteggio.rhai   (solo se servono casi speciali: la sezione [Script] di un .contest)
  dati/            elenchi usati dalle regole e dalla riga di inserimento (.dom)
  macros/          le macro di partenza del contest (.mc)
  prove.toml       QSO di esempio con il punteggio atteso
```

## I file

- **`punteggio.regole`**: righe `chiave=valore` come la sezione `[Score]` (`bands`, `valid`, `points`, `points_band`, `mult`, `mult_band`, `bonus`, `cqN`, `special`, `times_qso`, `band_score`; vedi `docs/contest-file.md`). I commenti sono le righe che iniziano per `;` (il `#` non e' un commento per il motore attuale).
- **`punteggio.rhai`**: lo script Rhai dei casi speciali (`punti`, `moltiplicatori`, `ammesso`, `punteggio`, `turno`), solo se serve. ARRL 10 non lo ha.
- **`dati/*.dom`**: elenchi `.dom` (sezioni per prefisso `K:`, `VE:`, `XE:`, righe `ricevuto = dominio`, sinonimi compresi); righe che iniziano per `;` sono commenti.
- **`macros/*.mc`**: sezione `[Macros]` come i file attuali.
- **`prove.toml`**: vedi sotto.

## `contest.toml`

| Tabella e chiave | Significato |
|---|---|
| `[contest] id` | identificativo corto, minuscolo, con trattini |
| `nome`, `organizzatore`, `sito`, `regolamento_url`, `note` | nome, organizzatore, pagina e regolamento ufficiali, nota per l'elenco |
| `cabrillo_contest` | valore di `CONTEST:` nell'intestazione Cabrillo (`ARRL-10`); passa nel contest convertito e vale al posto del nome |
| `qtc` | QTC del WAE (`docs/contest-qtc.md`): 0 nessuno (predefinito), 1 dalle stazioni non europee alle europee (CW, SSB), 2 in entrambe le direzioni fra continenti diversi (RTTY); diventa `m_iQtc` del contest convertito. Nelle prove `qtc = N` fra i totali attesi aggiunge N QTC al punteggio |
| `[edizione]` `mese`, `fine_settimana_pieno`, `giorno_inizio`, `ora_inizio`, `durata_ore` | la regola delle date come dati: ennesimo fine settimana pieno del mese, giorno e ora UTC d'inizio, durata |
| `ore_massime`, `pausa_minima_min`, `invio_log_giorni` | ore operabili, pausa minima, giorni per inviare il log |
| `[[edizione.date]]` `anno`, `inizio`, `fine`, `fonte`, `confermata`, `nota` | date calcolate o lette dal calendario; `confermata = false` = da confermare |
| `[bande] ammesse` | bande ammesse (`28 MHz`); il motore le legge da `bands=` |
| `[modi] ammessi`, `[[modi.limite]]` | modi e limiti di frequenza per modo (`frequenza_massima_khz`) |
| `[[scambio.ricevuto]]`, `[[scambio.inviato]]` | campi dello scambio: `nome`, `etichetta_it`, `etichetta_en`, `tipo` (`rst`, `numero`, `testo`, `zona_cq`: la zona CQ, proposta dal nominativo e messa nello scambio; `zona_itu`: la zona ITU, solo suggerita sotto il nominativo), `quando` (condizione come in `punteggio.regole`, sul paese del nominativo; `any` = sempre; `my_dxcc:` / `not_my_dxcc:` per la stazione propria), `predefinito` (per modo), `elenco` (file `.dom`), `valori`, `progressivo` (numero che avanza), `motore = false` (campo non ancora gestito) |
| `[doppi] per` | `"modo"`, `"banda"`, `"banda+modo"`, `"sempre"`: su cosa si fa il doppio |
| `[categorie]` | `operatore`, `potenza`, `modo`, `sovrapposizioni`, `altre`; sotto-tabelle per le categorie con vincoli (`[categorie.ms]`) |
| `[cabrillo]` | `contest`, `obbligatori`, `facoltativi`, valori ammessi per `category_*`, `invio` |
| `[[scheda]]` | i pannelli del contest (sezione «Schede») |

## `prove.toml`

`[prova]` (nominativo e locatore propri di fantasia, data), un `[[qso]]` per QSO con `banda`, `modo`, `freq_khz`, `ora_utc`, `nominativo`, `rst_inviato`, `scambio_inviato`, `rst_ricevuto`, `scambio_ricevuto` e `atteso = { ammesso, doppio, punti, moltiplicatori = ["valore@modo"], motivo }`; `[atteso]` con i totali (`qso_ammessi`, `doppi`, `non_ammessi`, `punti`, moltiplicatori, `punteggio`). `[[prova_futura]]` raccoglie i casi che il motore non sa ancora riconoscere: non entrano nei totali.

**Piu' punti di vista** (`[[altra_stazione]]`): quando il punteggio dipende dal paese della stazione propria (ARRL DX: un W/VE conta i paesi, un DX conta gli stati) `[prova]` e i `[[qso]]` valgono per la prima stazione e ogni `[[altra_stazione]]` ne aggiunge un'altra con `nominativo_proprio`, `locatore_proprio`, i suoi `[[altra_stazione.qso]]` e i suoi totali `[altra_stazione.atteso]` (stesso formato di `[atteso]`; il giorno e' quello di `[prova]`). Il controllo automatico valuta ogni punto di vista a parte, col suo paese. Nei QSO attesi `moltiplicatori` e' `valore@modo` anche per i moltiplicatori per banda: il controllo non distingue la banda (i totali contano ogni valore nuovo di ogni banda).

## Schede

Le schede sono i pannelli che si aprono con il contest e cambiano da contest a contest. Si dichiarano con `[[scheda]]`, ognuna con `tipo`, `titolo` (`{ it, en }`) e `ordine`. Se `contest.toml` non ne ha, Aurora le **deriva dalle regole**: una scheda `moltiplicatori` per ogni tipo di `mult=` (come fa oggi `SsbContestView.tsx` con la griglia `score.griglia` e `score.mancanti`), la scheda `statistiche` di base e il punteggio. Perche' le schede dichiarate non sono obbligatorie.

Ogni scheda puo' avere `quando` (proposta, tenuto come parametro): condizione sulla stazione propria (`my_dxcc:K,VE`, `not_my_dxcc:K,VE`) che dice quando la scheda vale (ARRL DX: paesi DXCC per i W/VE, stati e province per i DX).

Catalogo dei tipi generici (proposta):

| `tipo` | Parametri | Cosa mostra |
|---|---|---|
| `moltiplicatori` | `moltiplicatore` = `stato`, `dxcc`, `zona_cq`, `zona_itu`, `sezione`, `prefisso`, `area`, `continente`, `nominativo`; `per_banda`, `per_modo` (booleani); `dominio` (file `.dom` con l'elenco chiuso, per mostrare anche i mancanti); `escludi` (paesi non contati) | griglia o elenco dei valori lavorati e di quelli che mancano, per banda e/o per modo |
| `statistiche` | `righe` (`banda`, `modo`, `ora`), `colonne` (`qso`, `doppi`, `punti`, `scambio`, `moltiplicatori`, `moltiplicatori_per_banda`, `qtc`) | tabella dei totali con le colonne scelte |
| `punteggio` | `mostra` (`punti`, `moltiplicatori`, `moltiplicatori_per_modo`, `qtc`, `punteggio_finale`) | i totali e il punteggio finale |
| `doppi` | `chiave` | controllo di un nominativo (doppio, quanti punti, moltiplicatori nuovi) |
| `ritmo` | `finestre_min` | QSO all'ora, ultima ora, QSO per ora e per banda |
| `controllo_mult` | `moltiplicatore` | «check mult»: un valore dello scambio e se e' nuovo |
| `qtc` | | gestione dei QTC (docs/contest-qtc.md) |
| `mappa` | `base` (`paesi`, `zone`) | mappa dei lavorati |

ARRL 10 dichiara: moltiplicatori degli stati USA, DC, province canadesi e stati messicani (per modo, `dati/stati.dom`), moltiplicatori dei paesi DXCC (per modo, esclusi K, VE, XE, KH6, KL7), statistiche con le colonne QSO, doppi, punti, scambio, moltiplicatori per banda e modo, punteggio finale.

## Il lettore (`cartelle.rs`)

- **Lettura e controllo** (`cartelle::carica`): `contest.toml` (con la crate `toml`; chiavi sconosciute rifiutate, cosi' un errore di battitura si vede), `punteggio.regole` (si tolgono commenti `;` e righe vuote e si controlla con `RegoleScore::da_testo`), `punteggio.rhai` (compilato), `regolamento.md` (solo testo, per mostrarlo), `dati/*.dom`, `macros/*.mc`, `prove.toml` (deve essere TOML valido; la prova di punteggio sta nel test). Obbligatori per il lettore: `contest.toml` (con `[contest] nome` e `id` corto in minuscole, cifre e trattini) e `punteggio.regole`; gli altri sono facoltativi per le cartelle dell'utente, ma il controllo automatico del repository li vuole tutti.
- **Controlli**: i rimandi a elenchi (`elenco` dello scambio, `dominio` delle schede) devono essere `dati/<nome>.dom` e il file deve esserci; nessun percorso esce dalla cartella (niente `..`, unita', radici); non si seguono i collegamenti simbolici; ogni file al massimo 2 MB e al massimo 64 file fra `dati/` e `macros/` (come i pacchetti `.zip`); `[edizione]` e `[doppi]` con valori ammessi.
- **`[[scheda]]`** (tipo, titolo per lingua, ordine e parametri propri, tenuti cosi' come sono) e **`[scambio]`** (campi ricevuti e inviati) sono letti e tenuti, anche se oggi nessuno li usa: arrivano alle API.
- **Conversione in `ContestTesto`** (`ContestCartella::in_contesto`): stesso oggetto di un `.contest`, cosi' punteggio, riga di inserimento, Cabrillo e API funzionano senza modifiche. `id` = nome della cartella; `name`, `notes` da `[contest]`; `score` e `script` dai file; `valid_cw/phone/digital` dai `modi.ammessi` (nessun modo = tutti); `dupe_no_band/mode` da `[doppi] per` (`modo` = senza banda, `banda` = senza modo, `banda+modo` o assente = tutte e due le chiavi, `sempre` = nessuna); `auto_inc` se un campo inviato ha `progressivo`; `qtc` da `[contest] qtc`; zona CQ proposta (`zona_cq`) e zona ITU suggerita (`zona_itu`) dai campi ricevuti; `work_time` da `ore_massime`; `macros` e `domain_file` indicano il file principale (quello col nome della cartella, altrimenti il primo; per i domini il primo `elenco` dello scambio, poi la prima scheda con `dominio`, poi l'unico). Macro e domini non si rileggono dal disco a ogni richiesta: il contest attivo li ha gia' in memoria.
- **Frequenze per modo** (`[[modi.limite]]`), regioni ITU dei `/MM` e scambio per condizione sono letti ma il motore non li usa ancora (vedi sotto).

## Le date: `[edizione]`

`Edizione::calcola(anno)` ricava inizio e fine dalla regola: il sabato dell'**ennesimo fine settimana pieno** del mese (sabato e domenica nello stesso mese), spostato di -1/0/+1 giorno per `venerdi`/`sabato`/`domenica`, all'ora UTC data, per `durata_ore`; la fine e' l'ultimo secondo (`23:59:59` per 48 ore da mezzanotte). Le date scritte in `[[edizione.date]]` per quell'anno valgono al posto del calcolo (`confermata` dice se vengono dal calendario).

`edizione_per(now)`: l'edizione **in corso**; altrimenti quella **appena finita** se la fine e' di meno di `invio_log_giorni` giorni fa (7 se non detto: si puo' ancora esportare il log); altrimenti la **prossima**. Sono `start` e `end` del `ContestTesto` (`aaaammgghhmmss`), quindi la «finestra del contest» non risulta piu' scaduta. ARRL 10 metri: 2026 = 12-13 dicembre, 2027 = 11-12 dicembre. Come per i `.contest`, la finestra si porta nell'anno corrente (`punti_testo::finestra`).

## Cartelle incluse e dell'utente

- Le cartelle di `contest/` vanno nell'installatore, in **sola lettura**, nella risorsa `contest-cartelle` di `tauri.conf.json` (`../../contest`) e si aggiornano con le versioni. Il programma Windows imposta `AURORA_CONTEST_FOLDERS` sulla cartella delle risorse (come `AURORA_CONTEST_DEFAULTS` per i `.contest`); in sviluppo si imposta a mano su `contest/` del repository (`aurora.env` o ambiente).
- Nella cartella dei dati dell'utente (`%APPDATA%\Aurora\contest\<nome>\`) una cartella **con lo stesso nome** sostituisce quella inclusa (tutta, non file per file). Una cartella nuova dell'utente (nome diverso) e' un contest in piu'. Le sottocartelle `macros`, `dom` e `regole` dei dati non sono contest.
- Una cartella non valida si salta con un avviso nel registro (una volta sola per errore) e non rompe le altre: se e' quella dell'utente, resta valida quella inclusa dello stesso nome.
- **Copia nella mia cartella**: `POST /api/v1/contest-testo/voce/{id}/copia` copia la cartella inclusa nei dati dell'utente (si rifiuta se esiste gia' una cartella con quel nome); da li' si modifica a mano e vale al posto di quella inclusa.
- Le cartelle si rileggono dal disco al massimo ogni 3 secondi (la riga di inserimento chiede il contest a ogni tasto); la copia le rilegge subito.

## Modifiche personali di un contest in cartella (sovrascritture)

Date, ore di lavoro e simili sono dati dell'utente e non regole: si cambiano anche in un contest incluso, senza toccare la cartella (che resta aggiornabile con le versioni). Il core le tiene in **`<dati>/contest/sovrascritture/<id>.json`** (`<id>` = nome della cartella; la sottocartella e' riservata, non e' un contest) con **solo** questi campi, tutti facoltativi (assente = valore della cartella):

| Campo | Significato |
|---|---|
| `start`, `end` | inizio e fine dell'edizione, `aaaammgghhmmss` UTC (vanno in coppia); valgono al posto delle date calcolate da `[edizione]` |
| `work_time` | ore di lavoro, 0-48 (0 = nessun limite) |
| `nrs`, `auto_inc` | numero di partenza (al massimo 64 caratteri) e incremento automatico |
| `macros` | file di macro scelto (`macros/<nome>.mc`, fra quelli della cartella o dell'utente; vuoto = nessuna macro) |

`ContestCartella::in_contesto` produce i valori della cartella; l'elenco (`voce`/`voci`, `ElencoContest::voce_di_cartella`) ci applica sopra la sovrascrittura, quindi elenco, contest attivo (`start_ms`/`end_ms`, macro scelta) e rotte vedono le date nuove. Si tengono solo i valori **diversi** da quelli della cartella: un corpo uguale ai valori della cartella non lascia nessun file. La mia sezione / stato / club resta nelle preferenze del browser (`aurora.digi.contestsect.<id>`).

- `PUT /api/v1/contest-testo/voce/{id}` per un contest in cartella ACCETTA il contest ma legge e salva solo i campi sopra: nome, regole, bande, modi... nel corpo si ignorano senza errore; risponde 200 con il contest aggiornato (come `GET`, con `sovrascritta` = i campi tenuti o `null`). Valori non validi (date, ore fuori da 0-48, macro sconosciuta, fine prima dell'inizio) danno 400 `bad_contest` e non cambiano niente.
- `DELETE /api/v1/contest-testo/voce/{id}/sovrascrittura` («Ripristina i valori del contest») toglie il file e risponde col contest come nella cartella (400 se non e' un contest a cartella, 404 se sconosciuto).
- Il core non scrive mai nella cartella inclusa. Vale anche per le cartelle dell'utente (la sovrascrittura non le modifica).
- App: nella finestra di modifica di un contest in cartella sono modificabili solo le voci personali (`VOCI_PERSONALI` in `contestVoci.ts`: Inizio, Fine, Ore di lavoro, Numero di partenza, incremento automatico, La mia sezione, Macro); il pulsante «Ripristina i valori del contest» compare se c'e' una sovrascritta.

## Convivenza con i vecchi `.contest` e regola dei nomi uguali

Il nome del contest (`[contest] nome`, senza badare alle maiuscole: `ARRL 10 meter Contest` = `ARRL 10 Meter Contest`) e' la chiave. Nell'elenco:

1. una cartella **dell'utente** vale sempre piu' di un `.contest` con lo stesso nome (l'ha messa lui);
2. una cartella **inclusa** vale al posto del `.contest` solo se questo e' la **copia intatta** di quello distribuito (stesso contenuto del file in `desktop/src-tauri/contest`, cartella dei predefiniti); se l'utente lo ha **personalizzato** (o non si puo' dire, per esempio nei test senza predefiniti) il suo `.contest` resta e la cartella non compare;
3. il `.contest` sostituito **resta nel file dell'utente** ma non si vede due volte; l'identificativo vecchio (anche con altre maiuscole) porta alla cartella, quindi un contest gia' attivo resta attivo.

Un contest a cartella non si cancella dall'elenco (`DELETE` risponde 409 `contest_readonly`: «contest incluso: copialo nella tua cartella per modificarlo») e le sue regole non si modificano; i dati personali si salvano a parte (vedi «Modifiche personali» sotto); un nuovo `.contest` con lo stesso nome di una cartella prende un identificativo diverso (`<nome> (2)`). L'importa/esporta `.zip` resta com'e': il pacchetto di un contest a cartella contiene il `.contest` convertito, le macro e l'elenco dei domini.

I `.contest` distribuiti non sono toccati (cambiarne il contenuto li farebbe sembrare personalizzati a chi ne ha la copia intatta); quello di ARRL 10 resta com'era e ha ancora `KL7` al posto di `KL` (vedi sotto).

## API

`GET /api/v1/contest-testo` (ogni voce), `/attivo` e `/voce/{id}` aggiungono: `origine` (`cartella` o `file`), `sovrascritta` (le modifiche personali di un contest in cartella o `null`), `schede` (l'elenco dichiarato, `[]` per i `.contest`), `scambio` (`{ricevuto, inviato}`, vuoto per i `.contest`); `/voce/{id}` anche `regolamento` (markdown per le cartelle, `null` per i file) e `dichiarazione` (`bande`, `modi` con `ammessi` e `limite`, `categorie`, `doppi`, `edizione` con le date dell'edizione calcolate, `cabrillo`; `null` per i file): l'app decide con questi dati e con `[Score]` quali voci delle impostazioni sono attive (`app/src/lib/contestVoci.ts`, `voceAttiva`). Nuova: `POST /api/v1/contest-testo/voce/{id}/copia` (200 con l'id; 400 se non e' una cartella, 404 se sconosciuto, 409 `contest_copy` se non si puo' copiare).

## Controllo automatico

`core/crates/contest/tests/cartelle_repository.rs` scorre `contest/*/` (il repository, `../../../contest` dal crate) e per ognuna controlla:
1. i file obbligatori: `contest.toml`, `regolamento.md`, `punteggio.regole`, `prove.toml`;
2. `contest.toml` valido, `punteggio.regole` accettato dal motore, rimandi e file di `dati/` e `macros/` leggibili, conversione in `ContestTesto` valida;
3. `prove.toml`: i QSO di esempio valutati col motore vero (`punti_testo::valuta`) e col `cty.dat` vero di Aurora (`desktop/src-tauri/cty/cty.dat`) danno, QSO per QSO, ammesso, doppio, punti e moltiplicatori nuovi attesi, e i totali (`qso_ammessi`, `doppi`, `non_ammessi`, `punti`, `moltiplicatori`, `moltiplicatori_<modo>`, `punteggio`). `[[prova_futura]]` non e' valutata.

Si lancia con `cd core && cargo test -p contest --test cartelle_repository -- --test-threads=1`. **Risultato del primo giro (ARRL 10)**: tutto torna tranne il QSO 11 (KL7ABC, CW): la prova voleva `AK@CW`, il motore dava il paese DXCC `KL@CW`. Causa: nelle regole c'era `KL7`, ma nel `cty.dat` l'Alaska ha il prefisso principale `KL`; la prova era giusta, il motore e' giusto, la lista dei paesi era sbagliata. Corretta in `punteggio.regole` e `contest.toml` (`KL`); le prove dei contest ARRL DX nei `.contest` e `tests/regole_contest_3a.rs` usano un database finto con `KL7` e non se ne accorgono.

## Da fare nel motore

1. ~~Il lettore della cartella e l'elenco che la fonde coi vecchi `.contest`~~ (fatto, vedi sopra).
2. ~~Il controllo automatico~~ (fatto: `tests/cartelle_repository.rs`). Resta un comando dedicato che lo lanci.
3. **Limite di frequenza per modo** (`[[modi.limite]]`, QSO CW sotto 28300 kHz): oggi `bands=` guarda solo la banda; serve una condizione (`freq:`) o la lettura del limite in `ammesso`.
4. **Regioni ITU dei mobili marittimi** (moltiplicatore con valori 1, 2, 3 per i nominativi `/MM`): serve un tipo di moltiplicatore o una condizione `maritime`.
5. **Scambio per condizione** (`quando`): oggi il campo dello scambio non cambia secondo il paese del nominativo; la riga di inserimento lo dovrebbe mostrare.
6. **Doppi per modo** letti da `[doppi]` (oggi `m_bDupeNoMode` e le chiavi del `.contest`).
7. Penalita' NIL e nominativo errato: solo con i log degli altri, fuori portata.
8. **Schede dichiarate**: l'interfaccia di `SsbContestView.tsx` oggi disegna solo i moltiplicatori (`Mults`: chip per valore o tabella per banda, in base a `score.griglia` e `score.mancanti`), il punteggio dalle voci di `score.tipi` e le statistiche fisse (`ssbc.statsTitle`). Moltiplicatori per modo: fatti (`molt_per_modo` e `modi` delle celle nel punteggio, riga nella scheda Statistiche, ARRL 10). Non sa ancora: scegliere o ordinare le schede, colonne di statistiche diverse da contest a contest (scambio, moltiplicatori per banda), le schede `ritmo`, `controllo_mult`, `mappa` come pannelli separati, i titoli delle schede dichiarate.
9. Macro e dominio: `m_sStateListFile` punta a `dom/`; nel formato nuovo gli elenchi stanno in `dati/` e la riga `elenco` dello scambio li nomina.
