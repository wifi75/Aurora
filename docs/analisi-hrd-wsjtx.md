# Analisi: come WSJT-X 3.3.0-beta6 parla con Ham Radio Deluxe (HRD Rig Control)

Solo descrizione della logica, per una reimplementazione indipendente. Fonti: sorgenti `Transceiver/HRDMessage.cpp/.hpp`, `HRDTransceiver.cpp/.hpp`; binario `wsjtx.exe` (Ghidra, senza simboli).

Legenda: [S+B] = confermato da sorgenti e binario; [S] = solo sorgenti (nel binario non verificato o non verificabile).

## 1. Trasporto e framing

- TCP, porta predefinita **7809** [S] (il valore predefinito viene passato a `network_server_lookup(server, 7809)`; "server" e' una stringa host[:porta] scelta dall'utente). Una sola connessione, socket sincrono (attese bloccanti).
- Due dialetti:
  - **v5** (provato per primo; funziona anche con HRD v6) [S+B: nel binario il test `protocol == 1` seleziona il ramo "testo + CR", altrimenti il ramo binario].
  - **v4** (HRD 4, solo testo).
- **Messaggio v5** (little-endian, tutto) [S; costanti magiche confermate nel binario: i byte `CD AB 34 12` e `34 12 CD AB` compaiono una volta sola, nella routine di costruzione]:

  | offset | dim | contenuto |
  |---|---|---|
  | 0 | u32 | lunghezza totale del messaggio (intestazione 16 + payload) |
  | 4 | u32 | magic 1 = 0x1234ABCD |
  | 8 | u32 | magic 2 = 0xABCD1234 |
  | 12 | u32 | 0 (riservato) |
  | 16 | UTF-16LE | testo del comando + terminatore U+0000 (2 byte) |

  Lunghezza = 16 + 2*(numero di caratteri + 1). Nessun CR/LF. Stesso formato per le risposte.
- **Parsing risposta v5** [S]: servono almeno 16 byte; magic errati = errore ("invalid magic values"); dimensione < 16 o > 1 MiB = errore; se i byte ricevuti sono meno della lunghezza dichiarata si continua a leggere dal socket e si concatena; il payload deve avere lunghezza pari (>= 2) e finire con U+0000. Il testo restituito all'applicazione e' quello **fino al primo U+0000** (HRD usa NUL come separatore di valori e terminatore; dopo il primo NUL il resto va scartato).
- **Messaggio v4** [S+B]: testo locale (toLocal8Bit) `comando` + `\r` (0x0D), senza intestazione. Risposta: tutti i byte letti in una sola lettura, `trimmed()` (spazi e CR/LF tolti ai lati). In v4 non c'e' framing: si assume che la risposta arrivi in un solo segmento.
- **Contesto radio** [S+B: stringa "get radio" e anteposizione `[n] ` visibili in FUN_140412800]: ogni comando operativo e' preceduto da `[N] ` dove N e' l'indice numerico della radio corrente (es. `[1] get frequency`). I comandi di handshake (`get context`, `get id`, `get version`, `get radios`, `get radio`) sono inviati **senza** contesto (`prepend_context=false`); `get radio` dentro la verifica di contesto invece va con il contesto `[N] get radio`.
- Parsing di `get radios`: elenco separato da virgole `1:IC-7300,2:FT-991` (indice:nome). Si prende `entries[0]` = indice unsigned, `entries[1]` = nome. Nota: nomi con `:` o `,` romperebbero il parsing (WSJT-X non li gestisce).

## 2. Connessione e handshake

Sequenza di `do_start` [S; stringhe dei comandi tutte confermate nel binario ai riferimenti 0x140419678...0x14041a668]:

1. Connessione TCP (timeout predefinito di Qt, 30 s). Errore = "Failed to connect to Ham Radio Deluxe".
2. Rilevamento protocollo: impostato `v5`, si invia `get context` (senza contesto). Qualsiasi eccezione (nessuna risposta, risposta non valida) = protocollo `none`. Se `none`: si **chiude e riapre** il socket, si imposta `v4` e si rimanda `get context`. (La risposta non viene letta nel contenuto: basta che arrivi.) Se anche questo fallisce l'errore sale.
3. Il file "HRD Interface Information.txt" nella cartella dati dell'app viene scritto con tutti i risultati (diagnostica; non necessario).
4. `get id` -> stringa (solo diagnostica).
5. `get version` -> stringa (solo diagnostica; WSJT-X non ne deriva comportamenti, vedi sez. 8).
6. `get radios` -> `idx:nome,idx:nome...`. Vuoto = errore "no rig found".
7. `get radio` -> nome radio corrente. Vuoto = "no rig found". (Questo e' l'unico punto in cui `current_radio_` e' ancora 0, quindi le richieste dopo il primo `send_command` con contesto lo aggiornano: vedi sez. 7.)
8. Con contesto:
   - `get vfo-count` -> intero (1 o 2).
   - `get buttons` -> elenco con virgole; **gli spazi nei nomi sostituiti con `~`** (forma che serve nei comandi `set/get button-select`).
   - `get dropdowns` -> elenco nomi con virgole (spazi **non** sostituiti qui).
   - per ogni dropdown `dd`: `get dropdown-list {dd}` -> voci separate da virgola (split senza SkipEmpty: le voci vuote contano come indice; HRD per IC-7300 restituisce una voce vuota nella lista Data, difetto noto di HRD).
   - `get sliders` -> elenco nomi (spazi -> `~`); per ognuno `get slider-range <nomeRadio> <slider>` -> intervallo (solo diagnostica).
9. Individuazione dei controlli per **espressione regolare sul nome** (cfr. sez. 9).
10. Se `vfo_count == 1` e c'e' una coppia VFO A/B (o un toggle): stato noto per radio tipo Icom: legge `get frequency` e il modo, commuta su B (`set button-select <VFO B o toggle> 1`), rilegge frequenza/modo (= "altra frequenza"), torna su A, e deduce se l'avvio era su A o B confrontando.
11. Prova di risoluzione in frequenza: se f e' multiplo di 10 Hz, imposta `f - f%100 + 55`, rilegge, dal delta stabilisce troncamento/arrotondamento a 10/20/100 Hz (delta -5, 5, -15, -55, 45; riprova con +51 e delta 9 per 20 Hz arrotondato), poi **ripristina** `set frequency-hz f`. Il valore di risoluzione viene restituito al livello superiore (serve per compensare gli arrotondamenti della radio).

## 3. Comandi: testo esatto e risposte

Tutti con prefisso `[N] ` salvo dove indicato. Una risposta `OK` e' l'unica accettata per i comandi `set` (altrimenti errore "didn't respond ... as expected").

| Comando | Risposta attesa | Note |
|---|---|---|
| `get context` (senza `[N]`) | qualsiasi risposta valida | solo rilevamento v5/v4 [S+B] |
| `get id` / `get version` (senza `[N]`) | testo | diagnostica [S+B] |
| `get radios` (senza `[N]`) | `1:Nome1,2:Nome2` | [S+B] |
| `get radio` (senza `[N]` all'avvio; con `[N]` nel controllo) | nome radio | [S+B] |
| `get vfo-count` | intero | [S+B] |
| `get buttons` | `nome,nome,...` | [S+B] |
| `get dropdowns` | `nome,nome,...` | [S+B] |
| `get dropdown-list {Nome}` | `voce,voce,...` | graffe attorno al nome [S+B] |
| `get sliders` | `nome,...` | [S+B] |
| `get slider-range <Radio> <slider>` | `min,max,...` | [S+B] |
| `get frequency` | intero in Hz | [S+B] |
| `get frequencies` | `RX-TX` in Hz separati da `-` (es. `14074000-14075000`) | radio a 2 VFO; split su `-`; [0]=VFO principale, [1]=secondo [S+B] |
| `set frequency-hz <Hz>` | `OK` | [S+B] |
| `set frequencies-hz <Hz1> <Hz2>` | `OK` | due VFO; ordine come in `get frequencies` [S+B] |
| `get dropdown-text {Nome}` | `Nome: Valore` (parte dopo il primo `:` trimmata) | l'indice nella lista = posizione del valore; senza `:` = -1 [S+B] |
| `set dropdown <Nome~con~tilde> <Valore~con~tilde> <indice>` | `OK` | spazi -> `~` sia nel nome che nel valore; l'indice e' la posizione 0-based nella lista di `get dropdown-list` [S+B] |
| `get button-select <Nome~con~tilde>` | `1` o `0` (altro = errore) | [S+B] |
| `set button-select <Nome~con~tilde> 1` oppure `... 0` | `OK` | `1` = premuto/attivo; `0` = rilascio (usato per PTT off e per toggle) [S+B] |

Nota: i nomi dei pulsanti mantengono `~` al posto degli spazi (sostituzione fatta una volta all'elenco); i nomi dei dropdown sono invece conservati con spazi e sostituiti solo nel comando `set dropdown`; in `get dropdown-text` e `get dropdown-list` il nome va **con spazi** dentro `{ }`.

## 4. Frequenza

- Lettura radio a un VFO: `get frequency` (unsigned). Radio a due VFO: `get frequencies` -> `A-B`.
- Impostazione:
  - Prima, se il modo e' noto, imposta il modo; poi frequenza; poi **ripete il modo** (un cambio banda puo' richiamare un modo memorizzato che annulla il primo).
  - A un VFO oppure due VFO non invertiti: `set frequency-hz f`.
  - Due VFO con VFO invertiti (`reversed_`): legge `get frequencies`, poi `set frequencies-hz <freqA_attuale> f` (cioe' mantiene il VFO A e scrive f nel secondo).
- Polling: l'esito della frequenza di lettura e' attendibile solo se `vfo_count>1` o PTT non attivo (radio a singolo VFO in TX leggono male); in TX con singolo VFO la lettura di frequenza e di modo viene **saltata**.

## 5. Modo

- Il modo e' una voce di un dropdown (non esiste un comando "set mode"). Dropdown del modo VFO A: nome che corrisponde a `^(Main Mode|Mode|Mode A)$`; VFO B: `^(Sub Mode|Mode B)$` (opzionale).
- Mappa modo interno -> voci (in ordine di priorita'; le voci piu' specializzate trovate piu' avanti nella lista sono provate per prime, cercando all'indietro):
  - CW: `CW|CW(N)|CW-LSB|CWL`; CW-R: `CW-R|CW-R(N)|CW|CW-USB|CWU`
  - LSB: `LSB(N)|LSB`; USB: `USB(N)|USB`
  - DIG_U: `DIG|DIGU|DATA-U|PKT-U|DATA|AFSK|USER-U|USB`; DIG_L: `DIG|DIGL|DATA-L|PKT-L|DATA-R|USER-L|LSB`
  - FSK: `DIG|FSK|RTTY|RTTY-LSB`; FSK-R: `DIG|FSK-R|RTTY-R|RTTY|RTTY-USB`
  - AM: `AM|DSB|SAM|DRM`; FM: `FM|FM(N)|FM-N|WFM`; DIG_FM: `PKT-FM|PKT|DATA(FM)|FM`
- Lettura: `get dropdown-text {Modo}` -> voce -> modo interno tramite la mappa inversa; modo non riconosciuto = errore.
- Impostazione: `set dropdown <Modo> <voce> <indice>`.
- **Modo dati** (DIG_U/DIG_L/DIG_FM = "data on"; altri = off), tre meccanismi in ordine:
  1. Pulsante toggle `^(Data)$` ma **solo se il nome radio comincia con "IC"** (sui Kenwood "Data" e' il DSP): `set button-select Data 1` per dati, `... 0` per non dati.
  2. Coppia `^(DATA-ON\(mid\))$` / `^(DATA-OFF)$`: `set button-select <ON|OFF> 1`.
  3. Dropdown `Data` con voci on `^(On|Data1|D1|D1-FIL1)$` e off `^(Off)$`: `set dropdown Data <voce> <indice>`.
- In lettura, se il dropdown Data e' diverso da off, USB->DIG_U, LSB->DIG_L, FM->DIG_FM.
- Se si e' in split, il modo viene "razionalizzato" anche nell'altro VFO (impostando B tramite dropdown Sub/Mode B oppure commutando temporaneamente VFO/RX e impostando il dropdown A, poi tornando indietro, poi riselezionando `TX A`/`TX B` se esistono).

## 6. VFO A/B e split

Controlli (regex su nomi pulsanti; negli elenchi gli spazi sono gia' `~`):
- RX A/B: `^(RX~A)$`, `^(RX~B)$` (Yaesu FTdx3000: **attendere 350 ms** dopo averli premuti) [S+B: `msleep(0x15e)` nel binario].
- VFO A/B: `^(VFO~A|Main)$`, `^(VFO~B|Sub)$`; toggle: `^(A~/~B|VFO~A/B)$`.
- Split on: `^(Spl~On|Spl_On|Split|Split~On)$`; split off: `^(Spl~Off|Spl_Off|Split~Off)$`.
- Dropdown `Split` con voci `On`/`Off`; altrimenti dropdown `Receiver` con voci `RX / Off` (=RX A) e `Mute / RX` (=RX B).
- TX A: `^(TX~main|TX~-~A|TX~A)$`; TX B: `^(TX~sub|TX~-~B|TX~B)$`.

Impostazione dello split (`do_tx_frequency(tx, mode)`; tx=0 = split off):
1. Rilegge se i VFO sono invertiti: via dropdown `Receiver` (`get dropdown-text {Receiver}` == "RX / Off" => A attivo; "Mute / RX" => B) oppure via `get button-select` su RX A/VFO A (e se non A, su RX B/VFO B). `reversed_ = rx_B`.
2. Se split on: imposta modo del VFO TX (dropdown B se esiste, altrimenti commuta temporaneamente VFO e imposta dropdown A + modo dati, poi torna). Poi la frequenza TX:
   - invertito: `get frequencies`, `set frequencies-hz <tx> <freq[1]>`;
   - non invertito, due VFO: `get frequencies`, `set frequencies-hz <freq[0]> <tx>`;
   - non invertito, un VFO con A/B o toggle: premi VFO B (o toggle), `set frequency-hz <tx>`, eventualmente modo, premi VFO A (o toggle).
3. Poi lo stato split, il primo disponibile:
   - pulsanti: se esiste "split off" e split spento: `set button-select <SplOff> 1`; altrimenti `set button-select <SplOn> 1|0`;
   - dropdown `Split`: `set dropdown Split On|Off <indice>`;
   - radio senza controllo split ma con VFO A/B e TX A/B: si simula lo split scegliendo il pulsante TX (`TX A` / `TX B`) dopo aver selezionato il ricevitore giusto.
- Lettura split nel polling: (a) se esiste "split off" (tipico Icom) lo split **non viene letto** (si assume l'ultimo impostato); (b) pulsante Split senza coppia TX A/B: `get button-select`; (c) dropdown Split: se la voce non e' interpretabile si marca "sola scrittura" e non si rilegge piu'; (d) coppia VFO A/B + TX A/B: `split = (rx_B == tx_A)` e `reversed = rx_B`.

## 7. PTT

[S] `do_ptt(on)`:
- Se si usa HRD per il PTT: se esiste `^(TX~Alt|TX~Data)$` **e** la sorgente audio e' "rear", `set button-select TX~Alt 1|0` (selezione ingresso audio posteriore); altrimenti `set button-select TX 1|0` (pulsante `^(TX)$`). Se nessun pulsante esiste, nessuna azione (non errore).
- Se non si usa HRD per il PTT, il PTT passa a un'altra interfaccia (qui "Hamlib dummy" avvolto).
- Dopo il comando lo stato PTT interno e' aggiornato senza lettura di conferma. Nel polling c'e' comunque la lettura `get button-select TX` (e TX~Alt) ma serve solo a registrare nel log.
- Nota di robustezza: per le radio a singolo VFO, `send_command` con contesto introduce **50 ms di pausa** prima di ogni comando [S+B: `msleep(0x32)`] perche' alcune radio (Icom) altrimenti non eseguono i comandi.

## 8. Contesto radio e gestione errori

- `send_command(cmd, prepend_context=true)` fa prima (se non ricorsivo) `get radio` con contesto corrente per verificare che la radio non sia cambiata: il nome ottenuto deve essere nell'elenco `radios_` (acquisito all'avvio), altrimenti errore "rig has disappeared or changed". `current_radio_` viene aggiornato all'indice trovato. Cioe' **prima di ogni comando se ne spedisce un altro (`get radio`)**: costoso (raddoppia il traffico) ma e' il comportamento reale.
- Socket non connesso = errore "send command failed". Scrittura incompleta = errore. Lettura: `waitForReadyRead` (timeout Qt 30 s) con **30 tentativi** [S] sui soli timeout; altro errore socket = errore immediato. Dopo 30 timeout = "retries exhausted".
- In v5, risposta con intestazione valida ma incompleta: si legge ancora e si accoda.
- I comandi `set` richiedono `OK` esatto. `get button-select` richiede `1`/`0` altrimenti "didn't respond as expected".
- Ogni errore si propaga come eccezione al livello superiore, che normalmente fa fermare il polling/ritenta la connessione. Nessuna logica di riconnessione automatica nel modulo.
- Un errore di modo non supportato (`lookup_mode` senza voce) e' un errore a se': "rig doesn't support mode".

## 9. Polling (`do_poll`)

Ogni ciclo (intervallo scelto dall'utente) [S]:
1. Legge tutti i pulsanti rilevanti (solo per tracciamento): VFO A/B/toggle, split on/off, RX A/B, TX A/B, PTT, PTT alt, modo dati on/off/toggle, e dropdown Receiver, Mode A/B, Data, Split.
2. Calcola split/reversed come in sez. 6.
3. Frequenza: se `vfo_count > 1` -> `get frequencies`, RX = `freq[reversed?1:0]`, altra = `freq[reversed?0:1]`; altrimenti (un VFO) `get frequency` solo se non si trasmette.
4. Modo: `get dropdown-text {Modo A}` (+ correzione dati) se `vfo_count>1` o non in TX.

Il numero di richieste per ciclo e' elevato (decine con radio ricche di controlli): per una nuova implementazione basta restringersi ai soli controlli effettivamente utili (frequenza, modo, split, PTT).

## 10. Differenze v4 / v5 / v6

- v6 risponde come v5 (WSJT-X prova v5 per primo "funziona anche per v6"). Nel codice **non** si distingue v5 da v6 e il valore di `get version` non viene usato.
- v4: nessuna intestazione, comandi terminati da CR, risposta come testo semplice; stesso insieme di comandi (nessuna differenza di sintassi nel codice).
- Fallback v4: chiusura e riapertura TCP, perche' una v4 non risponde a un messaggio binario e lascia il socket in uno stato non utilizzabile.
- I nomi dei comandi nel binario sono identici (stringhe `get frequency`, `get frequencies`, `set frequency-hz`, `set frequencies-hz`, `get dropdown-text {`, `set dropdown `, `get button-select `, `set button-select `, `get radio`, `get radios`, `get id`, `get version`, `get context`, `get vfo-count`, `get buttons`, `get dropdowns`, `get dropdown-list {`, `get sliders`, `get slider-range `).

## 11. Casi particolari per radio

- **Icom**: nome radio che comincia con `IC` abilita il pulsante `Data`; "Split off" presente => lo split non e' leggibile e viene dedotto; in pratica singolo VFO => 50 ms di pausa prima di ogni comando, avvio con commutazione A/B per stato noto; IC-7300 ha il dropdown Data con una voce vuota (indice sfalsato, difetto di HRD, nessun rimedio).
- **Yaesu FTdx3000**: 350 ms dopo `RX A`/`RX B`.
- **Kenwood**: il pulsante `Data` e' DSP, non va usato per il modo dati (ecco perche' il filtro sul prefisso `IC`).
- **Radio a due ricevitori**: dropdown `Receiver` (`RX / Off`, `Mute / RX`): "invertito" solo quando A e' muto e B attivo.
- Radio che arrotondano la frequenza a 10/20/100 Hz: risoluzione misurata all'avvio (sez. 2.11).
- Radio senza pulsante PTT in HRD: il PTT viene semplicemente ignorato (si presume gestito altrove).

## 12. Cosa e' confermato dal binario

- Confermate (stringhe e sito di chiamata in Ghidra): tutti i comandi della tabella sez. 3, il file "HRD Interface Information.txt", la spaziatura `[N] ` e la struttura di `send_command` (due ramificazioni con test su protocollo, `get radio` ricorsivo, attese `msleep(0x32)` = 50 ms e `msleep(0x15e)` = 350 ms, messaggio "failed to write command"), le due costanti magiche 0x1234ABCD / 0xABCD1234 (un'occorrenza ciascuna), il testo di errore "invalid magic values", il flusso `get radios` -> `get radio` -> `get vfo-count` -> `get buttons` -> `get dropdowns` -> `get dropdown-list` -> `get sliders` -> `get slider-range`.
- Solo dai sorgenti: valori numerici di porta (7809, nel binario e' un immediato in una funzione senza simboli, non cercata a fondo), numero di tentativi (30), regex dei pulsanti/dropdown (le stringhe regex sono incorporate in oggetti Qt e non sono facilmente enumerabili dal decompilato), formato esatto del payload (letto dai sorgenti, il binario conferma solo le costanti magiche), differenza di comportamento PTT "rear audio", logica di mappatura dei modi.
- Non esiste alcuna differenza dimostrabile fra v5 e v6 nel client.
