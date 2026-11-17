# Le schede dei contest

Catalogo delle schede (pannelli) che un contest puo' dichiarare in `contest/<nome>/contest.toml` con `[[scheda]]` (formato in `docs/contest-formato.md`), confronto con quanto Aurora ha oggi e lavoro che manca.

Fonte: tre schermate di un programma da gara per CQ WW DX (CW e SSB), viste dal proprietario. Qui si descrivono solo le **funzioni** delle schede. Cio' che e' solo intuito dalla schermata (non verificato nell'uso) e' segnato **[intuito]**.

Stima del lavoro: S = meno di una giornata, M = alcuni giorni, L = piu' di una settimana (compresi core, API, interfaccia, sei lingue piu' olandese e prove).

## 1. Stato attuale di Aurora

La pagina dei modi testuali in modalita' contest (`SsbContestView.tsx`) ha tre colonne: Log (`DigiLog.tsx`), centro (inserimento del QSO, controllo del nominativo, punteggio, moltiplicatori, statistiche) e DX Cluster (`ClusterPanel.tsx`), piu' la scheda mobile del rotore (`components/rotor/`). Le schede sono **fisse**: i `[[scheda]]` di `contest.toml` oggi non sono letti da nessuno (il lettore della cartella non c'e' ancora). Esiste anche la pagina `ContestPage.tsx` per i contest digitali/VHF (altro motore, `/api/v1/contest/*`), con ritmo, mappa dei moltiplicatori, avvisi e consiglio di banda: non e' toccata da questo documento.

Dati gia' forniti dal core per il contest dei modi testuali (`/api/v1/contest-testo/punteggio`, `Punteggio` in `core/crates/contest/src/punti_testo.rs`): QSO, doppi, punti, moltiplicatori, punteggio, righe per banda (`RigaBanda`: qso, doppi, punti, molt, cw, phone, digital), `griglia` (moltiplicatori lavorati con le bande), `mancanti` (valori che mancano dei tipi a elenco chiuso), `rate10`, `rate60`, `media_ora`, `recenti` (ultimi QSO con moltiplicatori nuovi). Altri endpoint: `ammesso` (controllo di un nominativo: doppio, punti, moltiplicatori nuovi, scambio atteso), `cabrillo`, `qtc`. Per il resto: `/api/v1/dxcc-matrix` (matrice dei paesi lavorati), `/api/v1/log/qsos`, `/api/v1/frequencies`, cluster (`cluster_*`), rotore (`rotator_*`), tastiera WinKeyer.

## 2. Catalogo dei tipi di scheda

Nomi in snake_case. Dove un tipo e' gia' in `docs/contest-formato.md` e' indicato; i tipi nuovi sono proposti.

Parametri comuni a tutte le schede: `tipo`, `titolo = { it, en }`, `ordine`; proposti anche `visibile` (aperta all'avvio del contest o solo dal menu) e `posizione` (colonna o finestra mobile).

### 2.1 `check_call`, controllo del nominativo (scheda «Check Call»)

- **Mostra**: per il nominativo digitato, una riga per banda (160..10) con numero del QSO, ora, nominativo e scambio ricevuto dell'ultimo QSO con lui; evidenzia la banda attuale; in basso «Call: XXX».
- **Parametri**: `bande` (elenco, di base le sei HF dei contest), `colonne` (`numero`, `ora`, `nominativo`, `scambio`), `per_modo` (una riga per banda e modo, per i contest con doppi per banda+modo).
- **Dati**: il log del contest (QSO per nominativo, banda, ora, commento con lo scambio), la banda attuale. Tutto gia' nel browser.
- **In Aurora**: si, con altro nome. Tabella «QSO precedenti» (`ssbc-prev`) in `SsbContestView.tsx`, calcolata da `previousQsos()` in `app/src/lib/contestEntry.ts`; mostra banda, modo, ora, numero dello scambio, ma non evidenzia la banda attuale e non ha il nominativo della riga. Il doppio e' segnalato a parte da `ssbc-alert`.
- **Manca**: evidenziare la banda attuale, la colonna nominativo, scelta delle bande dal contest (oggi sempre le sei HF), versione per modo. **S**.

### 2.2 `check_mult`, controllo del moltiplicatore (scheda «Check Mult»)

Nel catalogo di `contest-formato.md` e' `controllo_mult` (un valore dello scambio e se e' nuovo). Qui e' piu' ricca.

- **Mostra**: per il moltiplicatore che porta il nominativo digitato (paese e zona CQ nel contest visto), una riga per banda con il QSO che l'ha fatto (numero, ora, nominativo, zona); banda corrente evidenziata; sotto il nome del paese con prefisso e zona («United States: K  Zona 4») e una riga **«Ant: 307/127 Sun: 1301/2249»** che sembra la direzione dell'antenna (percorso corto/lungo) e le ore UTC di alba e tramonto sul percorso **[intuito]**.
- **Parametri**: `moltiplicatore` (`dxcc`, `zona_cq`, `zona_itu`, `stato`, `prefisso`...; piu' d'uno), `bande`, `mostra_direzione` (bool), `mostra_alba_tramonto` (bool).
- **Dati**: per ogni tipo di moltiplicatore, chi lo ha fatto su ogni banda (oggi `griglia` ha solo valore e bande, **senza il QSO che l'ha fatto**), paese e zone del nominativo (`getDxcc`), direzione (`bearingDeg` in `app/src/lib/geo.ts`, lat/lon del paese), alba e tramonto (da calcolare: non trovato un calcolo nell'interfaccia).
- **In Aurora**: parziale. `callMultipliers()` (`contestEntry.ts`) e il riquadro `ssbc-callmult` dicono per tipo se il moltiplicatore del nominativo e' nuovo o lavorato; non c'e' la riga per banda con il QSO.
- **Manca**: nel core, la griglia con il primo QSO (nominativo, ora, numero) per valore e banda: nuovo campo `da: { call, time_ms, nr }` per cella di `griglia`, o endpoint `GET /api/v1/contest-testo/mult?tipo=&valore=`. Nell'interfaccia: la tabella per banda, la riga dei percorsi, il calcolo di alba e tramonto. **M**.

### 2.3 `qsy_rapidi`, salti rapidi di frequenza (scheda «Quick QSY»)

- **Mostra**: elenco di pulsanti con banda, modo e frequenza (`1835 CW`, `1845 LSB`, `1845 RTTY`, `3501 CW`, ...); un clic porta la radio li'.
- **Parametri**: `elenco` (voci `banda`, `modo`, `freq_khz`, etichetta) o riferimento a un file in `dati/`; per CQ WW sono i punti d'inizio del piano di banda; `modi` per filtrare.
- **Dati**: elenco fisso per contest, comando CAT di frequenza e modo (`change_frequency`, `change_mode` gia' esistono).
- **In Aurora**: no come scheda. Esistono le frequenze per modo e banda (`/api/v1/frequencies`, `app/src/lib/bandplan.ts`) e il comando radio.
- **Manca**: la scheda, la sezione `[[scheda.voce]]` (o file `dati/qsy.toml`) e il lettore. Nessun lavoro nel core. **S**.

### 2.4 `avvisi_spot`, avvisi dai moltiplicatori del cluster (scheda «Spot Alert»)

- **Mostra**: elenco scorrevole degli spot del cluster che sarebbero moltiplicatori **nuovi** (frequenza, nominativo, dB, ora) **[dB intuito: sembra il rapporto di uno spot da ricevitore di rete]**.
- **Parametri**: `bande`, `modi`, `scadenza_min`, `solo_nuovi_per_banda` (nuovo sulla banda dello spot o in assoluto), `suono` (avviso acustico facoltativo).
- **Dati**: spot del cluster (`ClusterSpot`: nominativo, frequenza, banda, paese), valutazione di ogni spot con le regole del contest e il log (stessa funzione di `ammesso`, ma per spot e senza essere il QSO).
- **In Aurora**: no per i contest. Il cluster ha il filtro «Nuovi» (`newOnly`) ma sulla lista dei desideri del DXCC (`wishReason`), non sui moltiplicatori del contest attivo.
- **Manca**: nel core, endpoint che valuti un elenco di spot rispetto al contest attivo (`POST /api/v1/contest-testo/spot` con `[{call, banda, modo}]` che restituisce `nuovi[]` e `doppio`), oppure calcolo nel browser dalla `griglia`+database prefissi. Interfaccia: la scheda e l'ordinamento. **M**.

### 2.5 `cluster`, DX cluster del contest (scheda «DX Cluster»)

- **Mostra**: elenco spot con ora, nominativo, frequenza; asterisco sui moltiplicatori nuovi; filtri Banda, Modo e casella «Mult» (solo moltiplicatori).
- **Parametri**: `filtri` (`banda`, `modo`, `mult`), `colonne`.
- **Dati**: gli stessi di `avvisi_spot`.
- **In Aurora**: si, come colonna di destra (`ClusterPanel.tsx`, filtri banda e modo con `MultiPick`, `lib/cluster.ts`). Manca il segno di moltiplicatore nuovo e il filtro «Mult» legato al contest attivo (oggi «Nuovi» e' sulla lista dei desideri).
- **Manca**: marca e filtro dei moltiplicatori del contest (dipende dal calcolo di 2.4). **S** dopo 2.4.

### 2.6 `qso_per_ora`, QSO per banda e ora (grafico «QSO per Banda/Ora (doppi esclusi)»)

- **Mostra**: istogramma a barre impilate, una colonna per ora sull'intera durata (48 ore), colori per banda (160 rosso, 80 verde, 40 blu, 20 giallo, 15 magenta, 10 ciano), scala a sinistra e a destra, legenda; titolo con il nominativo.
- **Parametri**: `ore` (di base la durata del contest), `bande` e `colori`, `doppi` (esclusi di base).
- **Dati**: QSO validi con banda e ora (dal log filtrato per contest o da una nuova serie nel punteggio: `GET .../punteggio` con `per_ora: [{ora_ms, banda, qso}]`).
- **In Aurora**: parziale. `ContestStatsCard` disegna le ultime 12 ore in un mini istogramma (`ctest-hours`), senza banda e senza colori.
- **Manca**: grafico completo (SVG), serie per banda, colori delle bande (probabile uso di `lib/colors.ts`), scheda dedicata. Il core puo' restare com'e' (basta il log). **M**.

### 2.7 `ritmo`, velocita' di gara (schede «Rates» / «Contest Rates»)

Nel catalogo di `contest-formato.md` e' `ritmo` (finestre in minuti). Qui ha altri elementi.

- **Mostra**: tabella delle ultime 10, 50 e 100 QSO con QSO all'ora e punti all'ora; riga **«1 Moltiplicatore Vale: 6,5 QSO o 4,7 min (3 pt)»**: quanti QSO, o quanti minuti al ritmo attuale, servono per guadagnare lo stesso punteggio di un moltiplicatore in piu'; riga con i punti per QSO e i punti per moltiplicatore («Pt/QSO: 2871 Pt/Mult: 18672»); riga «Operatore: nominativo». Versione «[Tutto il Log]» e versione del contest in corso.
- **Parametri**: `finestre_qso` (10, 50, 100), `finestre_min` (di base 10 e 60), `equivalenza_mult` (bool), `operatore`.
- **Dati**: serie dei QSO validi con ora (da qui `Last N QSO` e `Qso/h`), punti per QSO e per moltiplicatore. La formula esatta dell'equivalenza non e' chiara dalla schermata **[intuito]**: sembra il rapporto tra il guadagno di un moltiplicatore in piu' (punteggio / moltiplicatori) e i punti medi di un QSO (QSO equivalenti), diviso per il ritmo attuale (minuti equivalenti). Da verificare con piu' schermate o con un caso numerico prima di realizzarla.
- **In Aurora**: parziale. `rate10`, `rate60` e `media_ora` nel core (`ritmo()` in `punti_testo.rs`), riga «QSO/h» in `ContestStatsCard` e nel riquadro punteggio.
- **Manca**: finestre «ultimi N QSO», punti all'ora, riga dell'equivalenza, punti per QSO e per moltiplicatore, scheda dedicata; nel core il calcolo per finestre di QSO (nuovo campo `ritmi: [{n, qso_h, punti_h}]`). **M**.

### 2.8 `super_check`, nominativi simili (scheda «Super Check Partial (Log + master)», versione «N+1»)

- **Mostra**: elenco dei nominativi che somigliano a quello digitato (contengono la parte scritta), presi dal file master e dal log, colorati se gia' lavorati; la versione «N+1» elenca i nominativi che differiscono di un carattere (uno in piu', uno in meno o uno diverso) **[intuito dal titolo]**.
- **Parametri**: `sorgenti` (`log`, `master`), `modo` (`parziale`, `n_piu_1`), `minimo_caratteri`, `file_master`.
- **Dati**: elenco master di nominativi (il core gia' legge un file `MASTER.SCP` in `core/crates/aurora/src/lib.rs`, ma solo per convalidare le forme dei nominativi: non esiste un endpoint che lo esponga), log del contest.
- **In Aurora**: no come scheda.
- **Manca**: endpoint `GET /api/v1/contest-testo/simili?call=` (o ricerca nel browser con un indice del master), indice con ricerca per sottostringa e per distanza 1, scheda, colore dei gia' lavorati, scelta e aggiornamento del file master. **L**.

### 2.9 `sommario`, sommario per banda (scheda «Sommario»)

Nel catalogo di `contest-formato.md` e' `statistiche`; qui si propone di tenerlo e di aggiungere il nome `sommario` solo come caso di `statistiche` con colonne fisse, oppure come alias.

- **Mostra**: per banda QSO, CTY (paesi), ZON (zone), DUP (doppi), PUNTI, MEDIA (punti per QSO); riga TOT; «Score» sotto. Aperta con Alt-S.
- **Parametri**: `righe` (`banda`), `colonne` (`qso`, `moltiplicatore:<tipo>` per ognuno, `doppi`, `punti`, `media`), `punteggio_finale`.
- **Dati**: `bande` e `griglia` del punteggio (gia' nel core).
- **In Aurora**: si. `ContestStatsCard` in `ContestStatsDialog.tsx` (colonne QSO, doppi, punti, una per tipo di moltiplicatore con lavorati/totali, CW/fonia/digitale), con `multSummary()` in `contestEntry.ts`; riga del totale e punteggio.
- **Manca**: la colonna Media (punti per QSO) e la scelta delle colonne da `contest.toml`. **S**.

### 2.10 `punteggio`

Come in `contest-formato.md` (`mostra`). **In Aurora**: si (`ssbc-score`: QSO, punti, moltiplicatori, totale, ritmo). Manca solo `moltiplicatori_per_modo`, richiesto da ARRL 10 (vedi `contest-formato.md`, sezione «Da fare nel motore»). **S**.

### 2.11 `log_contest`, finestra del log (finestra principale)

- **Mostra**: elenco dei QSO con numero, banda, ora, nominativo, rapporto inviato e ricevuto o scambio; **«-DUPE-» in rosso** sui doppi; riga in basso in blu per il QSO in corso; barra con data e ora UTC e modo (S&P o Run).
- **Parametri**: `colonne` (numero, banda, ora, nominativo, inviato, ricevuto, scambio, punti, moltiplicatore), `evidenzia_doppi`.
- **In Aurora**: si. Colonna di sinistra `DigiLog.tsx` (colonne spostabili) e la riga di inserimento in `SsbContestView.tsx`.
- **Manca**: segno «DUPE» sui QSO doppi gia' a log (il doppio e' avvisato solo durante l'inserimento), indicatore S&P/Run e orologio UTC nella vista. **S**.

### 2.12 `matrice_paesi`, paesi di un continente per banda (scheda «Country DXCC [EU - Europe]»)

- **Mostra**: griglia dei paesi del continente (prefisso) e per ognuno una riga di caselle, una per banda (`|..****|`: simbolo se lavorata, punto se manca); serve a vedere quali bande mancano.
- **Parametri**: `continente` (`EU`, `AS`, ...; di base quello dei paesi piu' utili al contest), `bande`, `moltiplicatore = "dxcc"`, `per_modo`.
- **Dati**: paesi del continente con prefisso, griglia dei lavorati per banda.
- **In Aurora**: parziale. Il core ha `/api/v1/dxcc-matrix` (paesi e bande lavorati, `MatrixQuery`) e il pannello del DXCC; non e' legata al contest e al suo filtro di banda, modo e finestra di date. La griglia `griglia` del contest ha gli stessi dati per i contest con moltiplicatore DXCC.
- **Manca**: la scheda a caselle compatte filtrata per continente e per contest (con ARRL 10 anche per modo), da `griglia` + elenco dei paesi per continente (database prefissi). **M**.

### 2.13 `matrice_moltiplicatori`, griglie di zone, province, sezioni

Estensione della `matrice_paesi` ad altri tipi di moltiplicatore (zone CQ o ITU, province, sezioni, prefetture) con la stessa riga di caselle per banda. Per i valori a elenco chiuso (`dati/*.dom`) mostra anche quelli che mancano.

- **In Aurora**: si, ma come tabella a «✓» (`Mults` in `SsbContestView.tsx`, `MultGrid` in `ContestStatsDialog.tsx`), a catasta di chip senza banda o tabella per banda; senza il segno compatto per banda.
- **Manca**: la scheda a caselle compatte, scelta da `contest.toml`. Per questo e' un sottocaso del tipo gia' esistente `moltiplicatori` con parametro `vista = "caselle"`; non serve nuovo codice nel core. **S**.

**Nota**: le «griglie di zone CQ/ITU lavorate, province, sezioni, QTC» sono di altri contest dello stesso programma e **non sono state viste**: vedi sezione 5.

### 2.14 `rotore`, antenne (scheda «Rotori» con bussola)

- **Mostra**: elenco dei rotori con nome e direzione; bussola.
- **Parametri**: `rotori` (nomi), `bussola` (bool).
- **In Aurora**: si. `components/rotor/RotorCard.tsx` (finestra mobile aperta dal contest) e `StationRotor.tsx`, API `rotator_*` (un solo rotore). Manca: elenco di piu' rotori con nome (multiantenna), ma e' un limite dell'hardware piu' che della scheda. **S** (un rotore) / **M** (piu' rotori).

### 2.15 `velocita_cw`, velocita' del keyer (scheda «Velocita' CW»)

- **Mostra**: il valore in parole al minuto del keyer, con possibilita' di cambiarlo (aumento e riduzione).
- **In Aurora**: parziale. La velocita' WPM del CW e' nelle impostazioni (`WinKeyerFields.tsx`, `StatusBar.tsx`); non c'e' una scheda del contest con i tasti di salto (PgUp/PgDn) **[intuito: la schermata mostra solo il valore]**. **S**.

### 2.16 `cambi_banda`, due radio (scheda «Cambi Banda»: Runner e Mult)

- **Mostra**: due pannelli, per le due radio (una in CQ, l'altra a caccia di moltiplicatori), con la banda attuale e un pulsante «Qsy OK» **[funzione intuita dalla descrizione, senza prove del flusso]**.
- **In Aurora**: no. Aurora gestisce una radio sola per volta.
- **Manca**: gestione di due radio (CAT doppio, due ricevitori) e logica «una sola radio trasmette» (interblocco). Fuori dal piano attuale. **L** (come prerequisito: due CAT).

### 2.17 `stazioni_in_rete`, multi-operatore (scheda «Stazioni in Rete»)

- **Mostra**: elenco delle stazioni della squadra con banda, modo e frequenza di ognuna; serve ai contest multi-operatore per non occupare la stessa banda (regola dei moltiplicatori).
- **In Aurora**: no.
- **Manca**: rete locale fra postazioni (protocollo, unione dei log, orologio e doppi in comune). Il controllo remoto di Aurora (`remote-api`, WebSocket) e' un punto di partenza ma non e' multi-operatore. **L**.

### 2.18 `decodifica_morse` (decodificatore Morse)

- **Mostra**: la finestra del decodificatore (cascata + testo decodificato) accanto al log, con velocita' trasmessa e ricevuta.
- **In Aurora**: si, ma nella pagina dei modi testuali (decodificatore CW del motore `mode-digi`, cascata `DigiWaterfall.tsx`); non e' una scheda del contest, la vista contest a tre colonne la nasconde. **S** per riproporla.

### 2.19 `macro_tx` (finestra «TxMacro»)

- **Mostra**: campi nominativo, nome, QTH, RST, contatore per le macro, casella «OnTop». **[intuito: serve a riempire i campi delle macro `%call%`, `%name%`...]**
- **In Aurora**: si. Macro dei modi testuali (`MacroEditor.tsx`, `digiMacroEdit.ts`) e campi della riga di inserimento. Nessun lavoro. Se si volesse una finestra a se', **S**.

### 2.20 `qtc`

Come in `contest-formato.md`. **In Aurora**: si (`QtcDialog.tsx`, `qtc.rs`, `/api/v1/contest-testo/qtc`).

### 2.21 `doppi`, `mappa`

Come in `contest-formato.md`. `doppi` e' coperto dall'allarme in `ssbc-alert`; `mappa` e' nella pagina `ContestPage.tsx` (`MappaMoltiplicatori`, `/api/v1/contest/mappa`) per i contest digitali, non nella vista dei modi testuali.

## 3. Tabella di sintesi

| tipo | in Aurora | file | cosa manca | lavoro |
|---|---|---|---|---|
| `check_call` | si (come «QSO precedenti») | `SsbContestView.tsx`, `lib/contestEntry.ts` | banda attuale evidenziata, nominativo, bande per contest | S |
| `check_mult` | parziale | `SsbContestView.tsx` (`ssbc-callmult`), `lib/contestEntry.ts` | riga per banda con il QSO che ha fatto il mult (core), antenna e alba/tramonto | M |
| `qsy_rapidi` | no | `lib/bandplan.ts`, `/api/v1/frequencies` | scheda e elenco voci in `contest.toml` | S |
| `avvisi_spot` | no | `ClusterPanel.tsx` (solo lista desideri) | valutazione spot sul contest (core o browser), scheda | M |
| `cluster` | si | `ClusterPanel.tsx`, `lib/cluster.ts` | asterisco e filtro «Mult» del contest | S (dopo `avvisi_spot`) |
| `qso_per_ora` | parziale (12 ore, senza banda) | `ContestStatsDialog.tsx` (`ctest-hours`) | istogramma per banda su tutta la durata | M |
| `ritmo` | parziale | `punti_testo.rs` (`ritmo`), `ContestStatsDialog.tsx` | ultimi N QSO, punti/h, equivalenza di un mult, Pt/QSO e Pt/Mult | M |
| `super_check` | no | `core/crates/aurora/src/lib.rs` (legge MASTER.SCP solo per convalidare) | endpoint o indice, ricerca parziale e N+1, scheda | L |
| `sommario` / `statistiche` | si | `ContestStatsDialog.tsx`, `lib/contestEntry.ts` | colonna Media, colonne scelte dal contest | S |
| `punteggio` | si | `SsbContestView.tsx` | moltiplicatori per modo | S |
| `log_contest` | si | `DigiLog.tsx`, `SsbContestView.tsx` | «DUPE» sui doppi a log, S&P/Run, orologio UTC | S |
| `matrice_paesi` | parziale | `/api/v1/dxcc-matrix`, `ContestStatsDialog.tsx` (`MultGrid`) | scheda compatta per continente, legata al contest | M |
| `matrice_moltiplicatori` | si (altra forma) | `SsbContestView.tsx` (`Mults`) | vista a caselle (parametro di `moltiplicatori`) | S |
| `rotore` | si (un rotore) | `components/rotor/` | piu' rotori con nome | S / M |
| `velocita_cw` | parziale | `WinKeyerFields.tsx`, `StatusBar.tsx` | scheda con tasti di salto | S |
| `cambi_banda` | no | | due radio e interblocco | L |
| `stazioni_in_rete` | no | | rete tra postazioni, log comune | L |
| `decodifica_morse` | si (fuori dalla vista contest) | `DigiWaterfall.tsx`, `mode-digi` | riproporla nella vista | S |
| `macro_tx` | si | `MacroEditor.tsx` | nulla | - |
| `qtc` | si | `QtcDialog.tsx`, `qtc.rs` | nulla | - |
| `doppi`, `mappa` | si / pagina VHF | `ssbc-alert`, `ContestPage.tsx` | `mappa` nella vista dei modi testuali | M |

### Ordine consigliato (per valore)

1. **Lettore delle schede dichiarate** (`[[scheda]]` -> pannelli): senza questo nulla del catalogo e' scegliabile per contest. Gia' elencato in `contest-formato.md` («Da fare nel motore», punti 1 e 8). **M**.
2. `check_mult` con la riga per banda e `check_call` con la banda evidenziata (dato centrale delle gare HF, tocca il core).
3. `ritmo` completo (ultimi N QSO, equivalenza del moltiplicatore) e `qso_per_ora`: dati che gli operatori guardano di continuo.
4. `avvisi_spot` e marca «Mult» in `cluster` (valore alto nei contest, serve la valutazione degli spot).
5. `matrice_paesi` e `matrice_moltiplicatori` a caselle, `sommario` con la media.
6. `super_check` (utile ma pesante: file master, indice, ricerca).
7. `qsy_rapidi`, `velocita_cw`, «DUPE» a log (poco lavoro).
8. Due radio e multi-operatore (`cambi_banda`, `stazioni_in_rete`): richiedono architettura nuova.

## 4. Allineamento con `docs/contest-formato.md`

Non e' stato modificato. Proposte:

1. **Tipi nuovi da aggiungere al catalogo**: `check_call`, `qsy_rapidi`, `avvisi_spot`, `cluster`, `qso_per_ora`, `super_check`, `matrice_paesi`, `rotore`, `velocita_cw`, `cambi_banda`, `stazioni_in_rete`, `log_contest`, `decodifica_morse`.
2. **`controllo_mult`**: rinominare in `check_mult` (stesso nome della scheda vista, e `check_call` accanto) e aggiungere i parametri `bande`, `mostra_direzione`, `mostra_alba_tramonto`.
3. **`statistiche`**: tenere questo nome (non introdurre `sommario`); aggiungere la colonna `media` e le colonne `moltiplicatore:<tipo>`.
4. **`ritmo`**: aggiungere `finestre_qso` e `equivalenza_mult`.
5. **`mappa`**: distinguere `mappa` (geografica) da `matrice_paesi` (griglia per banda), oggi confusi nel parametro `base`.
6. **Parametri comuni**: `visibile`, `posizione`.
7. Il lettore (`contest-formato.md`, «Da fare», punto 8) elenca cio' che `SsbContestView.tsx` non sa fare; questo documento ne e' il dettaglio.

## 5. Domande aperte e schermate da chiedere

Fotografie utili (una per contest, con le schede aperte):

- **Griglie di zone CQ e ITU**: un contest a zone (per esempio il CQ WW o un contest ITU HF) con la scheda delle zone lavorate, per capire se le zone sono caselle per banda o un elenco unico.
- **Province e prefetture**: un contest nazionale a province (per esempio ARI/Italia, prefetture giapponesi) per la griglia dei moltiplicatori a elenco chiuso e la ricerca dei valori.
- **Sezioni**: un contest a sezioni (ARRL, Field Day, Sweepstakes) per la scheda delle sezioni e del terzo campo dello scambio.
- **QTC**: un contest WAE, con le finestre di invio e ricezione dei QTC.
- **Contest VHF e microonde**: una scheda con locatori e distanze (griglia dei quadrati lavorati, QRB), come ARRL VHF o IARU Regione 1.
- **Il menu «Mostra»** (nella schermata si legge la voce di menu «Mostra»): l'elenco di tutte le schede apribili, per completare il catalogo.
- **Il comportamento dei pulsanti**: «Qsy OK», «Quick QSY», Spot Alert (clic = porta la radio? colori?).
- **Alt-R, Alt-S, Alt-N**: le scorciatoie per aprire le schede.

Domande da chiarire:

1. **Equivalenza del moltiplicatore** (`ritmo`): qual e' la formula esatta di «1 Moltiplicatore Vale: 6,5 QSO o 4,7 min (3 pt)»? Serve un caso numerico completo (punti, mult, ritmo) per riprodurla.
2. **`check_mult`, riga «Ant ... Sun ...»**: confermare che sono direzione corta/lunga e ore di alba e tramonto (UTC) sul percorso; da dove vengono la posizione e il paese.
3. **Spot Alert**: il numero accanto alla frequenza e' il rapporto in dB (spot di un ricevitore di rete) o altro?
4. **Super check «N+1»**: confermare la regola di somiglianza (un carattere in piu', in meno o diverso).
5. **Cambi Banda con due radio**: cosa fa «Qsy OK» (conferma che l'operatore ha cambiato banda sulla radio dei moltiplicatori, oppure comanda la radio?).
6. Quali schede appartengono a tutti i contest HF e quali sono per contest (per decidere le schede di base che Aurora apre senza `[[scheda]]`).
