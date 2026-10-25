# ALL.TXT e file scaricati: JTDX 2.2.159 e Decodium 1.0.711

Analisi del 2026-10-25, in sola lettura, con REA/Ghidra sugli eseguibili installati (`jtdx.exe`, `jtdxjt9.exe`, `decodium.exe`; stessa ricetta di `analisi-decodium-reverse-rea.md`: 10 GB di heap, sessione persistente) piu' i sorgenti GPL di JTDX (`mainwindow.cpp`, `lib/decoder.f90`) e i file veri sul disco. Ghidra e' stato chiuso a fine lavoro. Gli indirizzi di JTDX sono quelli dei simboli veri; quelli di Decodium (`FUN_14...`) sono nomi di Ghidra, il significato e' dedotto dalle stringhe e dalle chiamate a Qt.

## 1. `ALL.TXT` di JTDX

### Dove, quando, chi scrive
- File mensile `AAAAMM_ALL.TXT` nella cartella dei dati (`%LOCALAPPDATA%\JTDX`): il nome e' costruito a ogni scrittura con `currentDateTimeUtc2().toString("yyyyMM_") + "ALL.TXT"` (`writeToALLTXT` `REA 0x4900a0`). **Nessuna soglia di dimensione e nessuna pulizia**: si cambia file solo al cambio di mese. Sul disco: 61 file, 109 MB, il piu' grande 10 MB; nessuno con righe di debug.
- Ogni evento **apre il file** con `WriteOnly|Append|Text` (0x16), scrive con `QTextStream` e **lo chiude** (niente handle tenuto aperto, niente flush a parte): si puo' leggere o cancellare con JTDX in funzione. Fine riga **CR LF** (modo testo di Windows), codifica locale a 8 bit (nei messaggi solo ASCII). Se non riesce ad aprire: finestra «File Open Error».
- Opzioni (Impostazioni > Report; `JTDX.ini`): `WriteDecodedALLTXT` (di base **true**: scrive tutte le decodifiche; se false scrive solo le righe che contengono il proprio nominativo base) e `WriteDecodedDebugALLTXT` (di base **false**, esclusiva con la prima; aggiunge righe di debug). Le righe di trasmissione e «QSO logged» **non si possono spegnere**. Verificato in `Configuration::read_settings` (`REA 0x50b240`). Voce di menu «Erase ALL.TXT» (`REA 0x4b77f0`): cancella solo il file del mese e riarma l'intestazione.

### Le righe (tutte verificate su 61 file: 1 964 987 decodifiche, 53 840 frequenza, 28 306 Tx, 721 Retransmitting, 2 066 QSO logged)

| Tipo | Chi la scrive | Formato esatto | Esempio |
|---|---|---|---|
| Decodifica | `MainWindow::readFromStdout` (`REA 0x494220`), per ogni riga del decoder `jtdxjt9` | `AAAAMMGG_` + riga del decoder tagliata agli spazi: `hhmmss` + SNR `%4d` + DT `%5.1f` + frequenza audio `%5d` + ` ~ ` (FT8) o ` : ` (FT4) + messaggio a 26 colonne + 1 carattere di indicatore (`jtdxjt9`: `i6.6,i4,f5.1,i5,1x,'~',1x,a26,a1`, stringa a `REA9 0x5882d0`) | `20261018_043545  -9  0.1 1908 ~ R8CEC SV2YC KN10` |
| Intestazione frequenza | `readFromStdout` quando `m_RxLog==1` (avvio, mezzanotte `nutc < nutc0`, dopo «Erase»), prima della prima decodifica | `AAAAMMGG_hhmmss  <MHz> MHz  <modo>` | `20261018_043559  7.074 MHz  FT8` |
| Cambio frequenza radio | `handle_transceiver_update` (`REA 0x4ad430`), solo con PTT spento e frequenza diversa | come sopra + ` JTDX v<versione><revisione>` | `20261018_043551  7.074055 MHz  FT8 JTDX v2.2.159c32f17-dirty` |
| Trasmissione | `guiUpdate` (`REA 0x4c4350`), quando il PTT parte (non per TUNE) | `AAAAMMGG_hhmmss.mmm(<scarto>)  Transmitting <MHz> MHz + <Hz>Hz  <modo>:  <messaggio a 37 colonne>` | `20261018_043617.739(0)  Transmitting 7.074 MHz + 1084Hz  FT8:  OA4ENG IU3ABC -14` + spazi |
| Ritrasmissione | `guiUpdate`, quando `m_restart` cambia il messaggio a trasmissione in corso | come sopra con `Retransmitting` e ` MHz +<Hz>Hz` (senza spazio dopo il +) | `20261022_162723.750(0)  Retransmitting 14.074 MHz +1084Hz  FT8:  ...` |
| QSO registrato | `acceptQSO2` (`REA 0x472db0`) via `writeToALLTXT` | `AAAAMMGG_hhmmss.mmm(<scarto>)  QSO logged: <nominativo>`, 6 ms dopo la riga Tx dell'RR73/73 | `20261018_043915.044(0)  QSO logged: OA4ENG` |
| Servizio del decoder | `jtdxjt9` (`partint*`, `decoder`), passa per `readFromStdout` | `hhmmss` + 2 spazi + testo (20 o 25 col.) + spazi + `d` (`REA9 0x59c9de`, `0x5886fc`) | `20261018_043545  partial loss of data                     d` (1 061 volte), `nfqso is out of bandwidth` (12) |
| Debug (solo con l'opzione) | `writeToALLTXT` da molti punti (elenco dei 27 chiamanti in `REA 0x4900a0`) e seconda riga di Tx con `AF TX/RX ... AutoSeq-On ...` | come «QSO logged» con testo libero; `Counters: ...` dopo i testi «... count reached» | non presenti sul disco |

Note di formato verificate:
- `<scarto>` e' lo scarto dell'orologio applicato da JTDX (`JTDXDateTime::GetOffset`, secondi, `float` in `%g`): `0` oppure `-0.22`.
- La frequenza e' `double` in MHz con **12 cifre significative, `%g`**: `7.074`, `14.08`, `7.074055`, `2e-06` (radio muta: 5 righe cosi' nei dati).
- La data delle righe di decodifica e' quella dell'**istante di scrittura**, non del periodo (dopo mezzanotte un periodo delle 23:59:45 prende il giorno nuovo); Aurora usa la data del periodo.
- Messaggio troncato a 26 caratteri nelle decodifiche, riempito a 37 in Tx.
- Indicatore (colonna 59): spazio nessuno, `^` decodifica con `ft8s/ft8sd` (66 928 righe in un mese), `*` con ipotesi a priori (3 083), `1` secondo messaggio di una coppia (2 304), `.` e `,` testo libero (vicino a +-10 Hz della frequenza Rx).
- Fonte dei dati: le decodifiche vengono dal **decoder esterno** (stdout di `jtdxjt9`), non dalla lista a video; Tx ed eventi vengono dalla finestra principale.

## 2. `ALL.TXT` di Decodium

- File **unico** `%APPDATA%\Decodium\Decodium\ALL.TXT` (`AppDataLocation`), CR LF, `QFile` in aggiunta e testo (0x14). Nessuna rotazione nei due scrittori trovati: le voci di menu «Don't split / Split yearly / Split monthly / Disable writing of ALL.TXT» (chiavi `Common/splitAllTxtYearly`, `Common/splitAllTxtMonthly`, `Common/disableWritingOfAllTxt`, `Common/actionDontSplitALLTXT`) esistono come eredita' di WSJT-X ma nel file reale non agiscono (un solo `ALL.TXT`, 24 KB, 330 righe).
- Scrittori: `FUN_1406622b0` (`on_ft8_decode_ready`, `REA 0x1406622b0`) e `FUN_14131ed50` (riga per riga dal modello QML, solo se `isTx` e' falso). Le righe si accumulano per periodo in due buffer e un `QRunnable` su un `QThreadPool` a un solo thread le aggiunge al file (`FUN_141182a50`); percorso da `FUN_1412c3580`/`FUN_1412d2d60` (di base `ALL.TXT`; se ne esiste uno dei vecchi percorsi `embedded-ft2/`, `ft2/`, `IU8LMC/ft2/` usa quello).
- **Solo righe `Rx`**: nessuna riga di trasmissione, nessun «QSO logged» (Decodium tiene `qso_journal\sequencer_AAAA-MM-GG.log`, `decodium_log.adi`, `decodium_diagnostic.log`).
- Formato (stringa `%1 %2  Rx  %3%4%5%6  %7`, `REA 0x142027c6a`...): `yyMMdd_hhmmss` (UTC, inizio periodo) + ` ` + MHz `%10.3f` + `  Rx  ` + modo a 6 colonne + SNR a 4 + DT a 5 + frequenza a 5 + 2 spazi + messaggio. **Ogni decodifica compare due volte** nel file reale (165 + 165 righe, stesse decodifiche): una nel formato sopra, una con campi piu' larghi (`piece-wise`: frequenza dopo 2 spazi, SNR `  %1` a 4, DT `  %1` a 5, frequenza `  %1` a 5), perche' i due buffer vanno nello stesso file quando i due percorsi coincidono.

```
261024_215030     14.074  Rx  FT8    -12  0.1 1518  CO2NX EA8DNB -17     (formato WSJT-X)
261024_214945      14.074  Rx  FT8       -2   -0.7   2498  <...> R2BQM -12     (formato largo)
```
- Differenze con JTDX: anno a 2 cifre, nessun `~`/`:`, nessun indicatore, modo e `Rx` espliciti, file unico, nessuna riga Tx/QSO/frequenza, righe doppie.
- **Uso in lettura**: Decodium rilegge `ALL.TXT` (anche `JTDX/ALL.TXT`, `WSJT-X/ALL.TXT`, `*_ALL.TXT` degli ultimi mesi: `FUN_1400c3320`) per risolvere i nominativi hashati (`[FT8-HASH] resolved from ALL.TXT`) e per seminare `ft8A7Seed`, leggendo **gli ultimi 512 KiB** (`FUN_1401e69f0`). Il proprio `ALL.TXT` di Aurora, in formato JTDX, e' leggibile allo stesso scopo.

Verifica sui campioni: le righe reali di entrambi i programmi coincidono con le stringhe di formato trovate (test `a_decode_line_has_the_columns_of_jtdx` in `engine-native/src/alltxt.rs` con 6 righe prese dai file veri; 59 colonne con indicatore).

## 3. File scaricati

### JTDX: **non scarica nulla da solo**
Nessun controllo aggiornamenti, nessun `cty.dat`/LoTW/CALL3 da rete. Gli unici utenti di `QNetworkAccessManager` (classi con il puntatore nel costruttore) sono tutti a richiesta dell'utente:

| Cosa | URL | Funzione | Quando |
|---|---|---|---|
| `cty.dat`, `lotw-user-activity.csv` | nessuno: **dentro l'eseguibile** (risorse `:/`); la copia in `%LOCALAPPDATA%\JTDX` ha la precedenza (`LogBook::init` `REA 0x61c7a0`) | - | caricati all'avvio; non aggiornati |
| `ALLCALL7.TXT`, `CALL3.TXT` | nessuno: nell'installazione (`share\jtdx`), `CALL3.TXT` copiato nella cartella dati | `jtdxjt9` (`cwfilter`, `cqcall3`) | all'avvio |
| Campioni audio | `http://jtdx.tech/public_html/downloads/` (`SamplesURL` in `JTDX.ini`) | `SampleDownloader::impl` (`REA 0x5a75f2`) | menu «Download Samples» |
| eQSL | `http://www.eqsl.cc/qslcard/importADIF.cfm` | `EQSL::work` (`REA 0x5f7430`) | dopo ogni QSO se `EQSLSend` |
| DXSummit (spot) | `http://www.dxsummit.fi/SendSpot.aspx` (GET, attesa 10 s) | `on_pbSpotDXCall_clicked` (`REA 0x4abe..`) | pulsante, se «AllowSpotsDXSummit» |
| WSPRnet | `http://wsprnet.org/post?` | `WSPRNet::upload` | solo WSPR |
| PSK Reporter | DNS `report.pskreporter.info`, UDP/TCP 4739 | `PSK_Reporter` (`REA 0x4fe790`, timer di **300 000 ms**) | `PSKReporter=true` (nel `JTDX.ini` del proprietario lo e', e serve la radio online) |
| Pagine web | sourceforge, forum, `jtdx-improved.sourceforge.io`, `qrz.com/db/` | `on_action...triggered` | solo apertura del browser |
Traffico locale: UDP verso `127.0.0.1:2334` (Aurora). DLL (Qt5, hamlib, ...) fisse: nessun aggiornamento a parte.

### Decodium 1.0.711: scarica molto, con modalita' **Offline** che lo ferma
Stringhe di stato `Offline: ... disabled; cached data remains available`; test di rete verso `https://1.1.1.1/`, `https://www.cloudflare.com/cdn-cgi/trace`, `https://www.google.com/generate_204` (`FUN_1408c4fe0`). Verificati con REA (funzioni, soglie) e con `decodium_diagnostic.log`.

| File | URL esatti | Funzione | Frequenza | Dove salva | Se fallisce |
|---|---|---|---|---|---|
| `cty.dat` | `https://www.country-files.com/bigcty/cty.dat` poi `.../cty/cty.dat` | `FUN_140675c10` | se il file ha **31 giorni o piu'** (`daysTo < 31` = aggiornato) o forzato | cartella dati utente; l'installazione ne porta una (`cty.dat` del 23/10) | prova il secondo URL, poi «Download cty.dat fallito», resta il vecchio |
| `lotw-user-activity.csv` | `https://lotw.arrl.org/lotw-user-activity.csv` (6,3 MB, 236 766 righe `CALL,AAAA-MM-GG,hh:mm:ss`) | `FUN_140648080` -> `FUN_140740d40` | all'avvio: carica la cache; scarica se piu' vecchia di **7 giorni** (604 800 s) | `%LOCALAPPDATA%\Decodium\Decodium\lotw-user-activity.csv` | usa la cache («LotW: N users loaded from cache») |
| `grid_data.bin`, `state_data.bin` (dati di stato USA, di JTDX) | `https://downloads.sourceforge.net/project/jtdx/grid_data.bin` e `.../state_data.bin` | `FUN_140a8ff70` | **solo se mancano** (o forzato): nessun controllo di eta' | `%APPDATA%\Decodium\Decodium\jtdx-state-data\` | «download failed», rifiuta risposte non binarie, tiene la cache |
| `CALL3.TXT` | `https://sourceforge.net/projects/wsjt-x-improved/files/Additional%20Files/CALL3.TXT/download` (user-agent `Decodium/4.0 CALL3 updater`); dalla finestra anche `https://wsjt-x-improved.sourceforge.io/CALL3.TXT` e `CALL3_EME.TXT` | `FUN_14065f420` (`downloadCall3Txt`); `FUN_14057f010` | **su richiesta** (pulsante) | cartella dati; copie `CALL3.OLD`, `CALL3_backup.TXT` | rifiuta pagine HTML e formato non riconosciuto, controlla la scrittura completa |
| Aggiornamento programma | `https://api.github.com/repos/elisir80/Decodium-4.0-Core-Shannon/releases/latest` (principale), `.../iu8lmc/...` (riserva); asset `Decodium_<versione>_Setup_x64.exe` | `FUN_1408bad90`/`FUN_1408be4b0` | all'avvio (nel log 14 s dopo l'apertura) con `Update/CheckOnStartup`; `Update/LastCheckUtc`, `Update/SkippedVersion` nel `.ini` | - | «Could not check for updates» |
| `libhamlib-4.dll` | `https://n0nb.users.sourceforge.net/dll64/libhamlib-4.dll` (e `dll32`) | `FUN_140580290` | su richiesta (pulsante) | cartella del programma, backup `.backup` | messaggio d'errore |
| Satelliti (TLE) | `https://celestrak.org/NORAD/elements/gp.php?GROUP=amateur&FORMAT=tle` | `FUN_140c230d0` | cadenza non determinata | `satellites\amateur.tle` | «using cached satellite TLE data» |
| Propagazione solare | `https://www.hamqsl.com/solarxml.php`, `solar.html`, `solarsun.php` | `FUN_1417df7e0` | non determinata | cache | «using cached propagation data» |
| Esterni vari | PSK Reporter (`retrieve.pskreporter.info/query`, `report.pskreporter.info`, `mqtt.pskreporter.info:1883`), FCC `l_amat.zip`, `eqsl.cc/DownloadedFiles/eQSLMemberList.csv`, LoTW `lotwreport.adi`, ClubLog, QRZ logbook, POTA, IOTA `groups.json`, NOAA/USGS/EONET/RainViewer/mappe (OSM, ArcGIS, GEBCO, MapTiler, NASA), tagloomis, kc2g, open-meteo | funzioni `FUN_140c73ea0` (FCC/eQSL/QRZ/ClubLog/LoTW) e altre | su richiesta o per funzione attiva | cache locali | cache o funzione ferma |
| Pacchetti opzionali (voce, IA) | whisper (`github.com/iu8lmc/.../whisper-win-x64.zip`, `huggingface.co/ggerganov/whisper.cpp`), piper, llama.cpp, Qwen (`huggingface.co`) | - | su richiesta | cartella dati | - |
Non scaricato da Decodium: `ALLCALL7.TXT`, `sat.dat` (nell'installazione, aggiornati solo con il programma).

## 4. Cosa gia' fa Aurora e cosa manca
Aurora ha: `dxcc` (legge un `cty.dat`), `online-info` (PSK Reporter in lettura, `solarxml.php`), `updater` (GitHub), `dx-cluster`. Non scarica `cty.dat`, non ha elenco LoTW, non scriveva `ALL.TXT`.

### Implementato ora (`core/crates/engine-native`, solo con il motore proprio come unico motore)
- `src/alltxt.rs`: `AllTxt`, formato JTDX (file `AAAAMM_ALL.TXT` nella cartella dei dati di Aurora, mai in quella di JTDX; apre-aggiunge-chiude, CR LF; `%g` come Qt): decodifiche (FT8 `~`, FT4 `:`, messaggio a 26 colonne), intestazione frequenza prima della prima decodifica e a mezzanotte, riga di cambio frequenza con `Aurora v<versione>`, riga `Transmitting` (scritta quando il periodo di trasmissione comincia davvero, ora con i millisecondi; annullata da STOP, cambio di banda o di modo), riga `QSO logged` (all'inizio della trasmissione dell'RR73/73 che chiude il QSO). `AURORA_ALL_TXT=0` la spegne.
- `src/tx.rs`: `TxNote`, `TxDriver::due_notes`; `src/lib.rs`: aggancio nel ciclo del motore. Test: 6 in `alltxt.rs`, 1 in `tx.rs`, controllo del file nel test del motore in modo reale (79 test del crate verdi, clippy pulito).
- Differenze volute dal JTDX: data della riga di decodifica = quella del periodo; scarto orologio sempre `0`; indicatori `^ * 1 . ,` non scritti (il decoder proprio non li espone); righe di debug non scritte; `Retransmitting` non scritto (il motore non cambia messaggio a trasmissione in corso).

### Da decidere (proprietario): scaricamenti di terzi, **nulla implementato**
1. `cty.dat`: stessa fonte di Decodium (`https://www.country-files.com/bigcty/cty.dat`, riserva `.../cty/cty.dat`), controllo all'avvio, aggiornamento se il file ha **31 giorni o piu'**, salvataggio nella cartella dei dati con scrittura su file temporaneo e sostituzione solo se valido (almeno ~300 KB e prima riga `Nome: CQ: ITU: ...`); offline: resta l'ultimo valido o quello incluso. JTDX non lo scarica mai (incorporato).
2. Elenco LoTW (`https://lotw.arrl.org/lotw-user-activity.csv`, 6,3 MB): cache locale, rinnovo dopo **7 giorni**, caricamento della cache all'avvio, offline = cache; serve solo se Aurora usera' «utente LoTW» nelle priorita'.
3. `grid_data.bin`/`state_data.bin` (sourceforge JTDX): solo se Aurora vuole i dati di stato USA come Decodium; scaricare solo se mancano.
4. `CALL3.TXT`: solo su richiesta, come Decodium; JTDX lo porta nell'installazione.
5. Aggiornamento del programma: Aurora ha gia' il suo (GitHub, firmato).
6. Il resto (TLE, solare, eQSL, QRZ, ClubLog, POTA...) e' funzione di Decodium, non necessaria alla parita' con JTDX.
Regole proposte per tutti: User-Agent `Aurora/<versione>`, timeout 10-30 s, mai bloccare l'avvio (task in secondo piano), controllo di dimensione/formato prima di sostituire, messaggio «Offline: ... resta la copia locale», interruttore unico «modalita' offline».

### Altro che resta
- Indicatori di decodifica (`^ * 1`) richiedono che il decoder proprio esponga il tipo di ipotesi usata.
- Cadenza di TLE/solare/IA di Decodium non determinata (non necessaria).
- Il nome del file e il formato di Decodium (anno a 2 cifre, doppie righe) non sono replicati: Aurora scrive il formato JTDX, che Decodium sa leggere (`*_ALL.TXT`).
