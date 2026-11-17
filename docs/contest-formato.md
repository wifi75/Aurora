# Il formato a cartella dei contest

Proposta: ogni contest ha una **cartella** in `contest/` (radice del repository, accanto a `core/`, `app/`, `desktop/`). Una correzione si fa in un solo punto e Aurora legge i file. Primo contest migrato: `contest/ARRL 10 Meter Contest/`. Il lettore nel core non c'e' ancora (vedi «Da fare nel motore»).

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
| `cabrillo_contest` | valore di `CONTEST:` nell'intestazione Cabrillo (`ARRL-10`) |
| `[edizione]` `mese`, `fine_settimana_pieno`, `giorno_inizio`, `ora_inizio`, `durata_ore` | la regola delle date come dati: ennesimo fine settimana pieno del mese, giorno e ora UTC d'inizio, durata |
| `ore_massime`, `pausa_minima_min`, `invio_log_giorni` | ore operabili, pausa minima, giorni per inviare il log |
| `[[edizione.date]]` `anno`, `inizio`, `fine`, `fonte`, `confermata`, `nota` | date calcolate o lette dal calendario; `confermata = false` = da confermare |
| `[bande] ammesse` | bande ammesse (`28 MHz`); il motore le legge da `bands=` |
| `[modi] ammessi`, `[[modi.limite]]` | modi e limiti di frequenza per modo (`frequenza_massima_khz`) |
| `[[scambio.ricevuto]]`, `[[scambio.inviato]]` | campi dello scambio: `nome`, `etichetta_it`, `etichetta_en`, `tipo` (`rst`, `numero`, `testo`), `quando` (condizione come in `punteggio.regole`, sul paese del nominativo; `any` = sempre; `my_dxcc:` / `not_my_dxcc:` per la stazione propria), `predefinito` (per modo), `elenco` (file `.dom`), `valori`, `progressivo` (numero che avanza), `motore = false` (campo non ancora gestito) |
| `[doppi] per` | `"modo"`, `"banda"`, `"banda+modo"`, `"sempre"`: su cosa si fa il doppio |
| `[categorie]` | `operatore`, `potenza`, `modo`, `sovrapposizioni`, `altre`; sotto-tabelle per le categorie con vincoli (`[categorie.ms]`) |
| `[cabrillo]` | `contest`, `obbligatori`, `facoltativi`, valori ammessi per `category_*`, `invio` |
| `[[scheda]]` | i pannelli del contest (sezione «Schede») |

## `prove.toml`

`[prova]` (nominativo e locatore propri di fantasia, data), un `[[qso]]` per QSO con `banda`, `modo`, `freq_khz`, `ora_utc`, `nominativo`, `rst_inviato`, `scambio_inviato`, `rst_ricevuto`, `scambio_ricevuto` e `atteso = { ammesso, doppio, punti, moltiplicatori = ["valore@modo"], motivo }`; `[atteso]` con i totali (`qso_ammessi`, `doppi`, `non_ammessi`, `punti`, moltiplicatori, `punteggio`). `[[prova_futura]]` raccoglie i casi che il motore non sa ancora riconoscere: non entrano nei totali.

## Schede

Le schede sono i pannelli che si aprono con il contest e cambiano da contest a contest. Si dichiarano con `[[scheda]]`, ognuna con `tipo`, `titolo` (`{ it, en }`) e `ordine`. Se `contest.toml` non ne ha, Aurora le **deriva dalle regole**: una scheda `moltiplicatori` per ogni tipo di `mult=` (come fa oggi `SsbContestView.tsx` con la griglia `score.griglia` e `score.mancanti`), la scheda `statistiche` di base e il punteggio. Perche' le schede dichiarate non sono obbligatorie.

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

## Convivenza con i vecchi `.contest`

Finche' un contest non e' migrato vale il suo `.contest` (`desktop/src-tauri/contest`, `docs/contest-file.md`). Un contest con la cartella in `contest/` vale al posto del suo `.contest`: l'elenco li fonde e chi ha un `.contest` con lo stesso nome nei dati dell'utente lo conserva. L'importa/esporta `.zip` resta com'e'.

## Cartelle incluse e dell'utente

- Le cartelle di `contest/` vanno nell'installatore, in **sola lettura**, e si aggiornano con le versioni.
- Nella cartella dei dati dell'utente (`%APPDATA%\Aurora\contest`) una cartella **con lo stesso nome** sostituisce quella inclusa (tutta, non file per file).
- Una cartella nuova dell'utente (nome diverso) e' un contest in piu'.

## Controllo automatico previsto

Una prova automatica (in `core/crates/contest/tests`) deve scorrere `contest/*/` e controllare che:
1. ci siano i file obbligatori: `contest.toml`, `regolamento.md`, `punteggio.regole`, `prove.toml`;
2. `contest.toml` si legga, `punteggio.regole` sia accettato dal motore e i file in `dati/` e `macros/` nominati esistano;
3. `prove.toml` dia i punteggi attesi (QSO per QSO e totali) applicando `punteggio.regole` ai QSO di esempio.

## Da fare nel motore

1. Il lettore della cartella (TOML, `punteggio.regole`, `dati/`, `macros/`) e l'elenco che la fonde coi vecchi `.contest`; il TOML richiede un crate (`toml`).
2. Il controllo automatico sopra e un comando che lo lancia.
3. **Limite di frequenza per modo** (`[[modi.limite]]`, QSO CW sotto 28300 kHz): oggi `bands=` guarda solo la banda; serve una condizione (`freq:`) o la lettura del limite in `ammesso`.
4. **Regioni ITU dei mobili marittimi** (moltiplicatore con valori 1, 2, 3 per i nominativi `/MM`): serve un tipo di moltiplicatore o una condizione `maritime`.
5. **Scambio per condizione** (`quando`): oggi il campo dello scambio non cambia secondo il paese del nominativo; la riga di inserimento lo dovrebbe mostrare.
6. **Doppi per modo** letti da `[doppi]` (oggi `m_bDupeNoMode` e le chiavi del `.contest`).
7. Penalita' NIL e nominativo errato: solo con i log degli altri, fuori portata.
8. **Schede dichiarate**: l'interfaccia di `SsbContestView.tsx` oggi disegna solo i moltiplicatori (`Mults`: chip per valore o tabella per banda, in base a `score.griglia` e `score.mancanti`), il punteggio dalle voci di `score.tipi` e le statistiche fisse (`ssbc.statsTitle`). Non sa ancora: scegliere o ordinare le schede, mostrare moltiplicatori per modo come colonne, colonne di statistiche diverse da contest a contest (scambio, moltiplicatori per banda), le schede `ritmo`, `controllo_mult`, `mappa` come pannelli separati, i titoli delle schede dichiarate.
9. Macro e dominio: `m_sStateListFile` punta a `dom/`; nel formato nuovo gli elenchi stanno in `dati/` e la riga `elenco` dello scambio li nomina.
