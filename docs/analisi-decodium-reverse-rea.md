# Decodium 1.0.711 letto con REA e Ghidra

Analisi del 2026-10-25, in sola lettura. Il programma installato non e' mai stato avviato e non e' stato modificato (REA ne analizza una copia nella
cartella temporanea). Scopo: riempire i vuoti che il sorgente pubblico (1.0.705) lascia sulla copia installata (1.0.711): la ricerca dei candidati FT4
(`SYNC2D`), i pesi di «Best caller first», le differenze del decoder FT8, e annotare altre novita'. Questo documento aggiorna i punti che
`analisi-decodium-decoder.md`, `analisi-decodium-banda-satura.md` e `analisi-decodium-cat.md` davano per non recuperabili.

## In breve

1. **REA e Ghidra funzionano su `decodium.exe`** (47,9 MB, 28 578 procedure, nessun simbolo): l'analisi finisce in **5-5,5 minuti** (3,7 senza
   `Scalar Operand References`) con **8,2 GB di memoria**. I due fallimenti precedenti avevano una causa sola e banale: REA lancia Ghidra con **2 GB di heap**
   (`GHIDRA_HEADLESS_MAXMEM` vale 2G se non impostata) e con tutti gli analizzatori predefiniti, tre dei quali usano il decompilatore per ogni funzione.
   Ricetta nella sezione 1. Una volta aperta la sessione, ogni decompilazione costa **da 1,5 a 9 secondi**, anche per funzioni da 10 000 righe.
2. **«Best caller first» (punto b): pesi recuperati per intero.** Punteggio = SNR (0-30 punti tra -24 e +10 dB, SNR ignoto = -10 dB) + attesa in coda (0-20
   punti in 90 s) + DXCC nuovo (+40 su qualsiasi banda, +25 se nuovo solo su questa banda) - gia' lavorato (-35 su questa banda, oggi o sempre; -12 se
   lavorato su un'altra banda o in altri giorni); un ordine messo a mano vale 1000 e non si tocca piu'. Ordinamento stabile per inserimento, a
   parita' vince chi e' arrivato prima. Spento di base (verificato nel codice di caricamento e nel testo dell'interfaccia).
3. **FT4 (punto a): la novita' e' un secondo cercatore di candidati, `sync2d`** (e identico per FT2). Invece di cercare picchi nello spettro mediato
   del periodo, correla lo spettrogramma (finestra di 1 simbolo, passo 12 ms) con le **quattro matrici di Costas di FT4 su tutti i ritardi
   possibili** (93 ritardi, 1,1 s), tiene per ogni frequenza il ritardo migliore, e prende i **primi 6 picchi** (`DECODIUM_FT4_SYNC2D_N`). Modo 1, **che
   e' quello di base** (lo imposta `applyLowCpuRuntimeProfile` all'avvio): i 6 picchi del 2D **piu' al massimo 5 candidati del vecchio
   metodo** (i piu' forti, a oltre 10 Hz dai nuovi); modo 2: solo i 6 del 2D; modo 0: come la 1.0.705. La griglia cieca ogni 25 Hz del profilo 4 viene
   aggiunta dopo, come prima. Sembra pensata per **fare meno tentativi per passata** (11 invece di decine) senza perdere i segnali veri.
4. **FT8 (punto c): nei punti controllati l'eseguibile coincide con il sorgente 1.0.705.** Tetto dei recuperi col BP esatto (12, contatore per thread
   mai azzerato), `neuralSync` e `turboFeedback` (restano interruttori con valori «proxy»: OSD d'ordine 3 e 50 iterazioni di BP, segnali con
   `min(1, righe/5)` e 50), scadenze, grappolo a 45 Hz, compagni di CQ a +-3,125 Hz, tetti sui candidati, soglie di validazione: tutto uguale. Non
   c'e' nulla di nuovo da portare in Aurora per FT8 da questa lettura.
5. **Altre novita' della 1.0.711** (punto d): all'avvio di una connessione a `rigctld` di rete Decodium **adotta lo stato della radio in sola lettura e
   rimanda per 750 ms le scritture di modo/VFO/split**, poi salta la riscrittura della frequenza Tx/split a riposo; un'euristica di «PC debole»
   (`looksLikeWeakComputer`); `sync2d` anche per FT2; uscita di sicurezza della FFT su GPU della cascata dopo uno stallo dell'interfaccia.
6. **Il repository pubblico e' fermo alla 1.0.705** (commit `3d4b0b0`, 2026-10-20, controllato il 2026-10-25): le modifiche 1.0.706-711 non sono in
   nessun sorgente, quindi il binario resta l'unica fonte.

## Fonti e come leggere i riferimenti

| Fonte | Cosa e' | Come e' citata |
|---|---|---|
| `C:\Users\<utente>\AppData\Local\Programs\Decodium\decodium.exe` | versione **1.0.711**, 47 932 416 byte, SHA-256 `1f3e400add4d471e1a9e1dce90e135d81bc689f9951d501486d4c212d019fdce` (lo stesso dei documenti precedenti); PE x86-64, base `0x140000000`, 12 sezioni, compilato con MinGW/GCC e Qt 6, senza tabella dei simboli | `REA 0x...`: indirizzo virtuale di una funzione, istruzione o dato |
| Sorgente pubblico di Decodium | `iu8lmc/Decodium-4.0-Core-Shannon`, commit `3d4b0b0` (1.0.705, GPL 3), in `%TEMP%\decodium-src\repo`: letto, mai compilato ne' eseguito | `src file:riga` |
| QML installato | `qml/decodium/components/SettingsTab3.qml`, testo in chiaro | `QML` |
| REA 6.7.0, Ghidra 12.1.4, JDK 25.0.3 | gia' installati (`C:\tools\ghidra_12.1.4_PUBLIC`; il JDK e' quello di Android Studio nel `PATH`) | sezione 1 |

I nomi `FUN_14xxxxxxx` sono quelli di Ghidra: il programma non ha simboli, quindi i **nomi veri** (`run_ft4_decode`, ...) sono dedotti dalle stringhe di log,
dalle chiamate a Qt (importate, quindi con il loro nome, per esempio `QString::arg`) e dal confronto riga per riga con il sorgente 1.0.705. Dove un nome o un
significato e' un'**ipotesi** lo si dice. Decodium e' GPL 3 come Aurora: del pseudocodice si riportano le **idee e i numeri**, non blocchi di codice.

## 1. Come si fa girare REA su `decodium.exe` (ricetta ripetibile)

### 1.1 Perche' falliva

Letto in `rea-agents` (`dist/ghidra/GhidraJavaLaunch.js`, `dist/config/environment.js`, `dist/config/providerStartupTimeout.js`, `bridge/ghidra/*.java`):

- REA importa il programma con `analyzeHeadless` in un progetto temporaneo (`-readOnly -deleteProject`, sulla **copia** `%TEMP%\rea-ghidra-<hash>\target-1f3e400add4d.exe`), poi
  esegue uno script ponte (`ReaGhidraBridge.java`) che apre una porta locale e resta in ascolto **finche' vive il processo `rea`**.
- Memoria Java: `GHIDRA_HEADLESS_MAXMEM`, poi `GHIDRA_MAXMEM`, poi **`2G`**. E' questo il limite che dava `OutOfMemoryError` dopo 9 minuti.
- Scadenza: `REA_GHIDRA_STARTUP_TIMEOUT_MS`, predefinita **330 000 ms** (5,5 minuti, il «`provider_timeout`» del primo tentativo); massimo 2 147 483 647 ms.
- Analizzatori: `REA_GHIDRA_DISABLED_ANALYZERS` (fino a 32 nomi separati da virgola, nomi esatti del programma). Uno **sconosciuto** per quel programma
  fa abortire tutto dopo l'importazione (con «Unknown Ghidra analyzer for this program»), cioe' si perdono circa 45 secondi.
- **Un comando `rea` alla volta riapre Ghidra e rifa' l'analisi**: `rea decompile`, `rea search`, `rea xrefs` da riga di comando costano ciascuno 5 minuti.
  Per lavorare serve **una sola sessione che resta aperta**: il server MCP di REA (`rea --mcp`), che mantiene il ponte vivo tra una chiamata e l'altra.

### 1.2 Impostazioni che hanno funzionato

PowerShell (stesse variabili per `rea` da riga di comando e per il server MCP):

```powershell
$env:GHIDRA_INSTALL_DIR           = 'C:\tools\ghidra_12.1.4_PUBLIC'
$env:GHIDRA_HEADLESS_MAXMEM       = '10G'        # il processo ha usato 8,2 GB; la macchina ne ha 31,8, 8-15 liberi
$env:REA_GHIDRA_STARTUP_TIMEOUT_MS = '10800000'  # 3 ore; l'analisi ne e' durata ~5 minuti
$env:REA_GHIDRA_DISABLED_ANALYZERS = 'Call Convention ID,Decompiler Switch Analysis,Stack,Create Address Tables,Non-Returning Functions - Discovered,Demangler GNU,Demangler Microsoft,PDB Universal,Embedded Media,x86 Constant Reference Analyzer,Function ID'
```

Perche' questi analizzatori spenti: `Call Convention ID` e `Decompiler Switch Analysis` chiamano il decompilatore su ogni funzione (60 s di tolleranza ciascuna);
`Stack`, `x86 Constant Reference Analyzer` e `Create Address Tables` fanno analisi per funzione che qui non serve; i demangler non servono (programma senza simboli) e `PDB Universal`,
`Embedded Media`, `Function ID` non hanno nulla da fare. **`Decompiler Parameter ID` e `Aggressive Instruction Finder` sono gia' spenti di base.**
**Lasciare acceso `Scalar Operand References`**: senza, `xrefs` risponde sempre vuoto (le stringhe e i dati non hanno riferimenti e non si risale da una stringa di log alla funzione);
con, costa circa 100 secondi in piu' e gli `xrefs` ci sono.

I nomi validi per questo programma (`analyzeHeadless ... -noanalysis` con un breve script che stampa `getCurrentAnalysisOptionsAndValues`) sono:
`ASCII Strings`, `Aggressive Instruction Finder`, `Apply Data Archives`, `Call Convention ID`, `Call-Fixup Installer`, `Condense Filler Bytes`, `Create Address Tables`,
`Data Reference`, `Decompiler Parameter ID`, `Decompiler Switch Analysis`, `Demangler GNU`, `Demangler Microsoft`, `Disassemble Entry Points`, `Embedded Media`,
`External Entry References`, `Function ID`, `Function Start Search`, `MinGW Relocations`, `Non-Returning Functions - Discovered`, `Non-Returning Functions - Known`,
`PDB MSDIA`, `PDB Universal`, `Reference`, `Scalar Operand References`, `Shared Return Calls`, `Stack`, `Subroutine References`,
`Variadic Function Signature Override`, `WindowsPE x86 Propagate External Parameters`, `WindowsResourceReference`, `x86 Constant Reference Analyzer`.
(`GCC Exception Handlers`, `DWARF` e `Windows x86 PE RTTI Analyzer` **non** esistono qui: dare errore.)

### 1.3 Sessione persistente (script usato)

Un piccolo programma Node avvia `rea --mcp`, apre il binario una volta e serve le richieste lette da una cartella (`q/NNN.req.json` -> `out/NNN.res.json`),
cosi' si lanciano chiamate a mano o da script senza riaprire Ghidra. Il protocollo MCP su stdio e' JSON per riga (`initialize`, `notifications/initialized`, `tools/call`).
`open_binary` risponde subito: l'analisi parte alla **prima** richiesta vera (qui `binary_overview`).

```js
// reasrv.mjs <exe> <cartella-richieste> <cartella-risposte>
import {spawn} from 'node:child_process'; import fs from 'node:fs'; import path from 'node:path';
const [,, exe, qdir, odir] = process.argv;
const p = spawn('node', [process.env.APPDATA + '/npm/node_modules/rea-agents/scripts/rea.mjs', '--mcp'], {stdio: ['pipe','pipe','inherit']});
let buf = '', id = 0; const pend = new Map();
p.stdout.on('data', d => { buf += d; let i; while ((i = buf.indexOf('\n')) >= 0) { const l = buf.slice(0, i); buf = buf.slice(i + 1);
  if (!l.trim()) continue; const m = JSON.parse(l); if (m.id !== undefined && pend.has(m.id)) { pend.get(m.id)(m); pend.delete(m.id); } } });
const call = (method, params) => new Promise(r => { const i = ++id; pend.set(i, r); p.stdin.write(JSON.stringify({jsonrpc: '2.0', id: i, method, params}) + '\n'); });
await call('initialize', {protocolVersion: '2025-06-18', capabilities: {}, clientInfo: {name: 'aurora', version: '1'}});
p.stdin.write(JSON.stringify({jsonrpc: '2.0', method: 'notifications/initialized'}) + '\n');
await call('tools/call', {name: 'open_binary', arguments: {path: exe}});
for (;;) {                                   // una richiesta alla volta: {"tool":"...","args":{...}}; "__quit" chiude Ghidra
  for (const f of fs.readdirSync(qdir).filter(f => f.endsWith('.req.json')).sort()) {
    const base = f.replace('.req.json', ''), req = JSON.parse(fs.readFileSync(path.join(qdir, f), 'utf8'));
    fs.renameSync(path.join(qdir, f), path.join(qdir, base + '.taken'));
    if (req.tool === '__quit') { await call('tools/call', {name: 'close_binary', arguments: {}}); p.kill(); process.exit(0); }
    fs.writeFileSync(path.join(odir, base + '.res.json'), JSON.stringify(await call('tools/call', {name: req.tool, arguments: req.args || {}})));
  }
  await new Promise(r => setTimeout(r, 500));
}
```

Strumenti MCP di REA usati (i risultati stanno in `result.content[0].text`, un JSON; il dato utile e' in `normalized_result`):

| Strumento | Uso | Tempo |
|---|---|---|
| `binary_overview` | prima chiamata: avvia import e analisi; poi conteggi (28 578 procedure, 36 846 stringhe, 12 segmenti) | 222-328 s la prima volta |
| `search_strings` | trova una stringa (ASCII e UTF-16: le stringhe `QStringLiteral` di Qt 6 sono UTF-16 senza intestazione) | istantaneo |
| `xrefs` | chi usa quell'indirizzo (con `Scalar Operand References` acceso: restituisce l'indirizzo dell'istruzione `lea`) | istantaneo |
| `resolve_containing_procedure`, `procedure_callers`, `procedure_info` | da un indirizzo alla funzione, ai chiamanti, al corpo | istantaneo |
| `procedure_pseudo_code` | pseudocodice del decompilatore (`rea decompile <exe> <indirizzo>` da riga di comando) | 1,5 s (1 049 righe), 7,3 s (3 992), 9,2 s (10 330) |
| `read_bytes` | tabelle di dati (matrici di Costas, costanti) | istantaneo |

### 1.4 Tempi e memoria misurati (2026-10-25)

| Prova | Esito | Tempo | Memoria Java (picco) |
|---|---|---|---|
| Elenco analizzatori con `-noanalysis` (solo importazione) | ok | 3 min 12 s | non misurata |
| REA, nome di analizzatore sbagliato (`GCC Exception Handlers`) | errore dopo l'importazione | 44 s | - |
| REA, 12 analizzatori spenti (con `Scalar Operand References` spento) | ok | **222 s** | 7,98 GB (CPU 317 s) |
| REA, 11 spenti (con `Scalar Operand References` acceso: ricetta finale) | ok, due volte | **328 s** e **300 s** | 8,18-8,19 GB (CPU 386-399 s) |

Il decompilatore e' un processo a parte (`decompile.exe`, 143 MB). Con 10 GB di heap non c'e' stato nessun `OutOfMemoryError`; con 8 GB di heap probabilmente basterebbe,
con 2 GB (predefinito) no. Nella cartella temporanea restano i `rea-ghidra-*` dei tentativi falliti (150-300 MB l'uno): si possono cancellare.

Da riga di comando, con le stesse variabili d'ambiente, `rea decompile <percorso di decodium.exe> 0x140e1eb40 --format json` e' andato a buon fine in **5 min 28 s** per
una sola funzione (import + analisi + decompilazione + chiusura) e ha dato lo stesso pseudocodice della sessione persistente: va bene per una funzione, non per un'indagine. La variabile d'ambiente e' la strada verificata
per la memoria; non si e' provato a cambiare `MAXMEM` in `launch.properties`.

### 1.5 Limiti incontrati

- **Nessun simbolo**: 28 578 procedure trovate (ne servono di piu' in un programma Qt di 25 MB di codice), tutte `FUN_<indirizzo>`; le chiamate verso le DLL di Qt, libstdc++,
  OpenMP e FFTW hanno il nome vero e sono la chiave per capire il codice. Funzioni non trovate da Ghidra: non incontrate nelle parti lette.
- **Funzioni con tutto in linea**: `run_ft4_decode` e' una sola funzione da 3 992 righe di pseudocodice, perche' il compilatore ha inglobato le funzioni di appoggio. Si legge a pezzi.
- **Variabili statiche con guardia** (`__cxa_guard_acquire`): le `DECODIUM_*` si leggono una volta sola; le si riconosce dal `getenv` dentro un `if (DAT_xxx == 0)`.
- I nomi dei campi di `DecodiumBridge` (`param_1 + 0x6718`...) si ricostruiscono dall'uso e dal confronto con `DecodiumBridge.h` del sorgente (stesso ordine dei membri, passo di 8 byte per ogni `QSet`).

## 2. «Best caller first» (punto b)

### 2.1 Dove sta il codice

| Funzione (REA) | Nome dedotto | Cosa fa |
|---|---|---|
| `0x140670930` | `DecodiumBridge::setCallerScoring(bool)` | se cambia, scrive la chiave `CallerScoring` in `QSettings` (organizzazione «Decodium», applicazione «Decodium3»), registra «CallerScoring ON/OFF» e, se acceso, riordina la coda con l'etichetta `enabled` |
| `0x140615980` | caricamento impostazioni | legge `CallerScoring` con **valore predefinito falso** (e il QML dice «Default: OFF», `SettingsTab3.qml:930`) |
| `0x140645440` | `rescoreCallerQueue(tag)` | riordina la coda dei chiamanti (`m_callerQueue`, `+0x6718`) e scrive il log «CallerScoring (%1): %2» (stringa a `0x142015588`) |
| `0x140e1eb40` | `scoreAndSort(records, pesi)` | calcola il punteggio di ogni chiamante e ordina: **qui stanno i pesi** |
| `0x1412eea90` | `makeCallerRecord(...)` | per un chiamante legge dal registro (QSO gia' fatti) i quattro indicatori: DXCC nuovo, nuovo su banda, lavorato su banda, lavorato altrove |
| `0x140700e50` | variante «slot» | stesso punteggio su un elenco di decodifiche dirette a noi nello stesso periodo (log «CallerScoring (slot): %1», stringa `0x1420157d8`); eta' sempre 0 |
| `0x14070dea0`, `0x140634550`, `0x14069e5e0`, `0x1406d1f80`, `0x1407649a0` | chiamanti di `rescoreCallerQueue` | con le etichette `enqueue` (`enqueueCallerInternal`, `src DecodiumBridge.cpp:39600`), `manual-enqueue`, `process-next`, `mam-promote`, `click` |
| `0x1406d2ec0`, `0x140734060`, `0x14065b560` | spostamento manuale / pulizia | i primi due mettono a 1 il byte `+0x6738` («ordine manuale»), il terzo lo rimette a 0 |

### 2.2 Il punteggio (letto in `0x140e1eb40`, ripetuto uguale nel ramo di log di `0x140645440` e di `0x140700e50`)

Ogni voce della coda e' una stringa `"CALL FREQ SNR"` (`src DecodiumBridge.cpp:39640-39648`; senza frequenza resta il solo nominativo). L'SNR e' il **terzo campo**: se le parti sono meno di 3 non c'e' SNR; se
non e' un numero vale 0; altrimenti e' limitato a -50..+49. L'eta' e' il tempo dall'inserimento in coda (tabella `+0x6730`, millisecondi; 0 se non c'e').

```text
punteggio = 1000                                    se la voce e' bloccata (messa a mano, vedi 2.3)
altrimenti
  snr      = (SNR noto ? SNR : -10) limitato a [-24, +10]
  punteggio  = (snr + 24) * 30 / 34                 # da 0 a 30 punti; SNR ignoto = 12,35
             + min(max(eta_ms, 0), 90000) * 20 / 90000   # da 0 a 20 punti, 90 s o piu' = 20
             + (DXCC mai lavorato ? +40 : DXCC nuovo su questa banda ? +25 : 0)
             + (gia' lavorato su questa banda ? -35 : lavorato altrove ? -12 : 0)
```

I quattro indicatori (`0x1412eea90`; i nomi dei contenitori vengono dal sorgente, `DecodiumBridge.h:4172-4195`, `struct WorkedSets`, dove l'offset coincide per tutti e cinque i punti
controllati: `+0x6538`, `+0x6540`, `+0x6588`, `+0x65a0`, `+0x65b8`):

| Indicatore | Condizione | Contenitore (ipotesi dal sorgente) | Effetto |
|---|---|---|---|
| DXCC nuovo (qualsiasi banda) | l'entita' del nominativo e' riconosciuta e **non** e' in `dxccEver` (`+0x6538`) | `dxccEver` | +40 |
| DXCC nuovo su questa banda | la chiave `BANDA\|ENTITA'` **non** e' in `dxccByBand` (`+0x6540`); conta solo se il precedente non scatta | `dxccByBand` | +25 |
| Lavorato su questa banda | la chiave banda+nominativo e' in `callByBand` (`+0x6588`) **oppure** in `callTodayByBand` (`+0x65a0`) | `callByBand`, `callTodayByBand` | -35 |
| Lavorato altrove | il nominativo e' in `callEver` (`+0x65b8`); conta solo se il precedente non scatta | `callEver` | -12 |

La banda viene dalla frequenza della radio (`+0x90`); l'entita' dalla tabella dei nominativi (`+0x6600`, verosimilmente caricata da `cty.dat`, che sta nella cartella del programma). Zone, griglie e continenti **non** entrano nel
punteggio (nonostante servano a colorare le decodifiche).

**Ordinamento** (stesso `0x140e1eb40`): ordinamento per inserimento **stabile**, dal punteggio piu' alto; un elemento sorpassa il precedente solo se lo supera di piu' di `1e-9`, quindi a parita'
resta l'ordine di arrivo. Se l'ordine non cambia non si scrive niente nel log; se cambia, si sostituisce la coda con quella riordinata e si registra
`CallerScoring (<etichetta>): CALL1=63.4, CALL2=51.0, ...` (un decimale).

Esempi (calcolati con la formula): DXCC mai lavorato a -15 dB, in coda da 30 s: 7,9 + 6,7 + 40 = **54,6**; stazione gia' lavorata su questa banda a +5 dB appena arrivata:
25,6 - 35 = **-9,4**; SNR ignoto, nessuna novita', appena arrivata: **12,4**; voce messa a mano: **1000**.

### 2.3 Quando agisce e quando no

- `rescoreCallerQueue` esce subito se `callerScoring` e' spento (`+0x7c9`), se la coda ha meno di 2 voci, o se **qualcuno ha spostato a mano una voce** (`+0x6738`): da quel momento il riordino
  automatico e' sospeso finche' non si svuota la coda (e' il «un ordine impostato a mano non viene mai cambiato» del testo QML). Le voci messe con «`manual-enqueue`» finiscono anche in un
  insieme (`+0x6740`) che le tiene a **1000**: un chiamante scelto con un clic va sempre in testa.
- Il riordino scatta a ogni inserimento, a ogni promozione MAM, all'avvio del prossimo chiamante (`process-next`), al clic e quando si accende l'opzione.
- `0x140700e50` (slot) lavora solo se l'opzione e' accesa, c'e' il flag `+0x4c9c` (verosimilmente AutoCQ in corso) e **non c'e' un partner** (`+0x4c40`, verosimilmente il nominativo DX corrente, e' vuoto): tra le decodifiche
  dirette a me (dal confronto con il mio nominativo, `+0x60`) scarta quelle che finiscono con 73, RR73 o RRR e quelle gia' attive, e ordina le altre con lo stesso punteggio con eta' 0.

### 2.4 Quello che non e' certo

- I nomi dei contenitori (`callByBand`...) sono dedotti dal sorgente 1.0.705 e dai cinque offset; il significato preciso di «oggi» e «sempre» (`callTodayByBand` conta come «su questa banda») viene dal codice, il nome da quel confronto.
- Il formato della chiave `BANDA|VALORE` e' quello dichiarato nel commento del sorgente; il separatore e' `|` (0x7c) per la chiave DXCC-banda, per quella banda+nominativo non e' stato letto il carattere.
- La funzione che sposta a mano (`0x1406d2ec0`, `0x140734060`) e quella che azzera il flag (`0x14065b560`) sono state identificate dall'effetto sul byte, non dal nome.

## 3. FT4: la ricerca dei candidati `sync2d` (punto a)

### 3.1 Cosa c'e' di nuovo (e cosa no)

Confronto dei nomi `DECODIUM_*` nell'eseguibile con quelli del sorgente 1.0.705 (`grep` sul sorgente contro le stringhe del `.rdata`): **le sole variabili nuove** sono
`DECODIUM_FT4_SYNC2D`, `DECODIUM_FT4_SYNC2D_N`, `DECODIUM_FT4_SYNC2D_LEGACY` e, per FT2, `DECODIUM_FT2_SYNC2D`, `..._N`, `..._MIN`, `..._LEGACY`.
(`DECODIUM_FT4_FORCE_LIVE_DEPTH4`, citata come nuova in `analisi-decodium-decoder.md`, **e' gia' nel sorgente**: quel documento va corretto.) Tutto il resto di `run_ft4_decode`
(`REA 0x1403cfaa0`, 3 992 righe) e' sullo stesso schema del sorgente: profondita' 1/2/3/4 -> 1/3/4/4 passate, `syncmin` 1,18 (profondita' < 3) o 1,0, passate 2 e 3-4 con
`x0,88` e `x0,88*0,76`, fattore `DECODIUM_FT4_CANDIDATE_SYNCMIN_SCALE`, soglia `smax` 0,90 (profondita' < 3) o 0,75, stessa regola per la terza passata (serve che la seconda abbia trovato qualcosa, oppure profondita' 4 con
`late fine grid` attiva e almeno 8 thread, `REA 0x1404f17c0` e `0x1404720c0`), fallback LDPC d'ordine 3 e 4, griglia cieca di profondita' 4.

### 3.2 Dove si inserisce

In ogni passata (`REA 0x1403cfaa0`, righe di pseudocodice 395-720) l'ordine e' ora:

1. `getcandidates4` (vecchio metodo, derivato da quello di JTDX; `REA 0x14177f200`, `src FtxFt4Decoder.cpp:1538`) riempie l'elenco `candidato = (frequenza, rapporto)`, fino a 260 voci;
2. **nuovo**: se il modo `sync2d` e' >= 1, si cerca con `sync2d` (sotto) e si **rifa' l'elenco**;
3. la griglia cieca ogni 25 Hz da 225 a 3000 Hz di profondita' 4 si aggiunge dopo (`FUN_141625300`, con tetto 260 voci totali; poi 12,5 Hz di sfasamento all'ultima passata se `late fine grid`), come in `src FtxFt4Decoder.cpp:1541`.

**Il modo** viene da `DECODIUM_FT4_SYNC2D` (0, 1, 2; letta una volta sola) e, se vuota, da una variabile globale (`0x1417f8f60`, valore iniziale 0) che scrive `REA 0x14175c9a0` (limita a 0-2);
quella funzione e' chiamata da `REA 0x140773990`, cioe' `applyLowCpuRuntimeProfile` (log «runtime CPU profile: low=%1 ft_threads=%2 spectrum_ms=%3 reason=%4», `src DecodiumBridge.cpp:44867`) **con argomento 1**,
subito all'inizio, senza condizioni; la funzione e' chiamata all'avvio (`startup`) e a ogni cambio di profilo (6 chiamanti in tutto). Quindi **il modo di base della 1.0.711 e' 1**; per FT2 vale lo stesso (`REA 0x141759d90` imposta `_DAT_1417f8f50`, qui limitato a -1..2).

### 3.3 L'algoritmo (`REA 0x14175c9c0`, 618 righe)

Parametri (ricostruiti): segnale audio a 12 kHz (il buffer `dd` **dopo** le sottrazioni delle passate precedenti), numero di campioni (72 576 per FT4, `kFt4Nmax`), limiti di ricerca `nfa..nfb`, numero massimo di candidati
`N` (`SYNC2D_N`: da 1 a 40, 6 se assente o non valido), array di uscita (frequenza, forza). Restituisce quanti ne ha trovati.

```text
1. spettrogramma: finestra di Hann di 576 campioni (un simbolo, 48 ms), zero-padding a 2304 punti (bin di 12000/2304 = 5,2083 Hz, un tono = 4 bin),
   passo di 144 campioni (12 ms); P[riga][bin] = potenza. Righe = (campioni - 576) / 144 (500).
   Serve almeno 409 righe, altrimenti 0 candidati.
2. norm[riga][b] = 1 / (P[b] + P[b+4] + P[b+8] + P[b+12])      # potenza dei 4 toni se il tono 0 e' nel bin b; 0 se <= 1e-20
3. per ogni ritardo lag da 0 a righe-408 (93 ritardi a 72 576 campioni = 1,1 s) e ogni bin b nella banda:
     sync[lag][b] = somma su g=0..3 e k=0..3 di  P[4*(INIZIO[g]+k)+lag][b + 4*COSTAS[g][k]] * norm[4*(INIZIO[g]+k)+lag][b]
     INIZIO = {0, 33, 66, 99}                       # simboli dove cominciano le quattro matrici (tabella a REA 0x1427c7500)
     COSTAS = {{0,1,3,2},{1,0,2,3},{2,3,1,0},{3,2,0,1}}   # tabella a REA 0x1427c7520: sono le matrici di FT4
     (16 termini, ognuno e' la frazione di potenza che sta sul tono atteso: da 0 a 16, il rumore sta intorno a 4)
   best[b] = massimo su lag di sync[lag][b]            # il ritardo non si conserva: serve solo la forza per ogni frequenza
4. picchi: massimi locali di best[] (precedente <= attuale > successivo) tra max(2, nfa/5,2083) e min(nfb/5,2083, numero di bin - 14);
   ordinati per best decrescente; si prendono i primi N con distanza di almeno 4 bin (20,8 Hz) tra loro. Nessuna soglia di forza.
5. frequenza del tono 0 = (b + d) * 5,2083 Hz, con d = interpolazione parabolica su best[b-1], best[b], best[b+1], limitata a +-0,5 bin.
```

Poi, per ogni frequenza trovata (`REA 0x1403cfaa0`, righe 482-545):

- **«rapporto»** del candidato = `min(1, media di 15 valori dello spettro mediato `savg` centrati sul centro del segnale / fondo `sbase`)`, dove il centro e' a +31,25 Hz (6 bin) dal tono 0 e `savg`/`sbase` sono gli array
  del `getcandidates4` appena eseguito; poi **si prende il massimo** con il rapporto di ogni candidato vecchio entro 15 Hz (cosi' un segnale che il vecchio metodo vedeva mantiene il suo punteggio).
  Il decoder usa `snr = rapporto - 1` per i fallback LDPC (`src FtxFt4Decoder.cpp:1549`): un candidato trovato **solo** dal 2D ha rapporto <= 1, cioe' `snr` <= 0, e rientra come i candidati della griglia cieca (`snr` tra -0,10 e +0,25, `src FtxFt4Decoder.cpp:59-60`):
  e' un'**inferenza dal codice**, non l'ho verificata sul funzionamento.
- **modo 2**: l'elenco dei candidati e' sostituito dai soli candidati 2D.
- **modo 1**: l'elenco e' = candidati 2D, **seguiti da** al massimo `DECODIUM_FT4_SYNC2D_LEGACY` (5 se assente o non valida, limite 260) candidati del vecchio elenco: quelli con il rapporto piu' alto (ordinamento stabile decrescente, controllato nel confronto
  dell'ordinamento), saltando quelli a meno di 10 Hz da uno gia' presente; totale 6 + 5 = **al massimo 11 candidati per passata** prima dell'eventuale griglia.

### 3.4 Che cosa implica

- Il vecchio metodo (`getcandidates4`, derivato da quello di JTDX) cerca picchi nello **spettro medio di tutto il periodo**: un segnale debole vicino a uno forte si perde nel picco del forte, e l'ordine dei picchi dice solo quanta energia c'e', non se somiglia a un frame FT4.
  Il nuovo misura la **somiglianza ai 16 simboli di sincronismo** (4 matrici di Costas) per ogni ritardo di 12 ms e ordina per quella: e' una versione 2D (frequenza x ritardo) del `sync8` che FT8 ha gia'.
- Sono pochi candidati (11) per passata, ma le passate sono 3-4, con sottrazione dei segnali decodificati tra l'una e l'altra e rilancio del 2D sul residuo: fino a 24 candidati 2D + 20 vecchi in totale (profondita' 3-4). Nei periodi molto affollati (gara) e' un tetto basso;
  la griglia ogni 25 Hz di profondita' 4 lo compensa.
- **Non ho misure** dei guadagni (non ho eseguito Decodium ne' ho registrazioni FT4): il valore del 2D e' una conclusione dalla struttura, non da una prova.

### 3.5 Quello che non e' certo

- Il numero esatto dei bit di cui il rapporto e' limitato e i dettagli del calcolo del fondo vengono da due righe di pseudocodice con costanti a virgola mobile; l'algoritmo e' lo stesso su FT2 (`REA 0x1403fd400`, righe 712-885, con soglia `SYNC2D_MIN` (0 se assente), bin da 10,42 Hz e tetto 300 voci), la cui funzione di appoggio (`REA 0x14175a100`) **non e' stata letta**.
- Le costanti `INIZIO = {0, 33, 66, 99}` sono lette dai dati; l'ipotesi «posizione dei simboli di Costas di FT4 (0-3, 33-36, 66-69, 99-102)» coincide con il protocollo.

## 4. FT8: differenze rispetto al sorgente (punto c)

Il decoder FT8 e' in `REA 0x140e17680` (`FT8DecodeWorker::decode`, 2 358 righe), `REA 0x141608040` (passate principali e recuperi, 2 046 righe), `REA 0x1404d9b10` (`decode_main_candidate_cpp`, 10 330 righe) e `REA 0x1403e48c0` (tetti
sui candidati). Confronto con il sorgente 1.0.705 sui punti che contano:

| Punto | Sorgente 1.0.705 | Eseguibile 1.0.711 (REA) | Esito |
|---|---|---|---|
| Variabili d'ambiente FT8 | 48 `DECODIUM_FT8_*` e `DECODIUM_LDPC_*` | le stesse 48 (confronto per nome) | uguali |
| Recuperi col BP esatto (`ft8_classic_rescue`) | tetto 12 (variabile `..._CLASSIC_RESCUE`, 0-200, altrimenti 12); contatore `thread_local` mai azzerato; soglia 1 200 ms; solo con `superldpc` (`src FtxFt8Stage4.cpp:521, 585, 8740`) | tetto 12 e soglia 1 200 ms identici (`REA 0x141608040`, righe 1496-1550); il contatore e' nel blocco TLS a `+0x1710` e **gli unici due accessi diretti con quello spiazzamento sono il controllo e l'incremento** (`0x1416095ea`, `0x141609665`); altri 45 byte-pattern con indice, in codice numerico, non sono stati esaminati | uguali, salvo prova contraria. Differenza minima: un valore **negativo** nella variabile d'ambiente nel sorgente torna a 12, nell'eseguibile spegne i recuperi |
| `neuralSync` | forza l'OSD d'ordine 3 e emette `min(1, righe/5)` come «punteggio» | `REA 0x140e16ee0` emette `min(1, righe/5)`; l'OSD d'ordine 3 si imposta con `FUN_14176d0a0(3)` (`REA 0x140e17680`, righe 905-920) | uguale (resta un proxy) |
| `turboFeedback` | 50 iterazioni di BP invece di 30, emette 50 se ci sono righe | `FUN_14176d080(0x1e / 0x32)`; `REA 0x140e16f20` emette 50 se ci sono righe | uguale (resta un proxy) |
| Raggruppamento dopo una decodifica | grappolo a 45 Hz; compagno di CQ a +-3,125 Hz con sincronismo x0,92 | 45,0 e +-3,125 e 0,92 in `REA 0x141608040` (righe 1030-1118, 1646-1800) | uguali |
| Tetti sui candidati per passata (banda piena) | 1 050, 850 dalla terza, 700 dalla quarta (+200 con profilo avanzato; 1 200/950/800 supplementari) | `REA 0x1403e48c0` righe 495-503: gli stessi numeri, con soglia di 3 500 Hz | uguali |
| Validazione | al massimo 58 bit ribaltati (36 per i non standard); `ribaltati + dmin > 36,6` passa per `chkfalse8` | `REA 0x1404d9b10`: 0x3a (58), 0x24 (36) e 36,6 negli stessi confronti | uguali |
| BICM/coerente/governatore, a priori da cronologia, a7/a8, sync8 con le tre forme di spettro | descritti in `analisi-decodium-decoder.md` | **non controllati** | da verificare se servisse |

In sintesi: **per FT8 non e' emersa nessuna differenza che valga un'azione in Aurora**; i confronti coprono il bordo piu' mutevole del decoder (recuperi, contatori, scadenze, tetti), non tutto. Il tetto dei recuperi resta
«dopo i primi periodi non scatta piu'» come nella lettura del sorgente (nel sorgente e' una lettura del codice, qui e' confermato che l'eseguibile non ha un azzeramento nascosto; non e' stato provato in esecuzione).

## 5. Altre novita' della 1.0.711 (punto d)

Controllo: stringhe UTF-16 del `.rdata` dell'eseguibile (9 947) non presenti nel testo del sorgente (63 MB), tolte le sicuramente note (SQL, HTML, ecc.); le piu' interessanti:

| Novita' | Dove | Cosa fa | Per Aurora |
|---|---|---|---|
| **Avvio di CAT su `rigctld` di rete in sola lettura** | `REA 0x14170e3a0` (connessione, log «CAT NET rigctl startup: read-only state adoption; mode/VFO/split writes deferred»), `REA 0x1411f7ea0`, `REA 0x140771bc0`, `REA 0x1407bfc20` | alla connessione a una radio di rete si accende un blocco (`+0x4d88`); **dopo 750 ms** (timer, `0x2CB41780` ns) si toglie, si legge lo stato adottato («state adopted; idle split replay remains suppressed») e si fa **una sola sincronizzazione del modo** con motivo `network-rigctl-state-ready`; la riscrittura della frequenza Tx e dello split **a riposo** viene saltata («skipped idle TX-frequency/split write»); nelle stesse condizioni parte anche un timer da 300 ms (scopo non letto) | S: la regola e' quella gia' consigliata in `analisi-decodium-cat.md` (non scrivere nulla finche' non si sa lo stato) con i numeri |
| **«PC debole»** | `REA 0x141302f30` (`looksLikeWeakComputer`, esposta al QML, log «SlowPcOffer: cores=%1 ram_mb=%2 slow_start_before=%3 weak=%4») | vero se **meno di 8 192 MB di RAM** o **meno di 5 processori logici** o esiste uno dei file-segno `decodium-slow-qml-startup.flag` / `decodium-gpu-startup-pending.flag` nella cartella temporanea; serve a proporre la modalita' a basso consumo | S: come ingresso del governatore di carico di `engine-native` |
| **`sync2d` anche per FT2** | `REA 0x1403fd400` righe 712-885, impostazione di base 1 | stessa idea dell'FT4, con bin da 10,42 Hz | nessuna (Aurora non ha FT2) |
| **FFT della cascata su GPU con uscita di sicurezza** (**solo dalle stringhe**, codice non letto) | «Panadapter GPU FFT disabled for this Windows D3D11 session after a %1 ms UI stall; async CPU FFTW fallback remains active», «Windows D3D11 severe UI stall (%1 ms); disabled for this session», file `decodium-gpu-runtime-crash.flag` | dopo uno stallo dell'interfaccia la GPU si spegne per la sessione e resta la FFT su CPU; un crash del driver grafico lascia un file-segno (da come lo dice il nome) | M, solo se la cascata di Aurora usa la GPU |
| `callerScoring` | sezione 2 | | sezione 6 |
| CAT: il sorgente 1.0.705 e' per il resto confermato | stringhe `CAT profile %1: ...`, `DecoPort rig driver ...`, `[CAT-CAL]` sono nel sorgente | nessun'altra differenza trovata sul CAT con questo metodo | - |

## 6. Cosa fare in Aurora

Difficolta': **S** poche ore, **M** un giorno, **L** piu' giorni. `Prefer::Score` di Aurora oggi vale `rango*10 + (snr+24 limitato 0..36)/2` (`core/crates/qso-engine/src/brain.rs:485`): 10 punti per livello di rarita',
da 0 a 18 punti di SNR su 36 dB, nessuna attesa, nessuna penalita' per i gia' lavorati.

| # | Cosa | Dove | Diff. | Note |
|---|---|---|---|---|
| 1 | **Punteggio dei chiamanti con i pesi di Decodium** come funzione pura (`snr`, `eta`, `dxcc_nuovo`, `dxcc_nuovo_banda`, `lavorato_banda`, `lavorato_altrove`, `bloccato`) e ordinamento stabile con tolleranza `1e-9`; sostituisce o affianca `Prefer::Score` | `qso-engine/src/brain.rs` | S | prova con i quattro esempi della sezione 2.2 e con l'ordine d'arrivo a parita'; zone e griglie **non** pesano in Decodium: tenerle come passo successivo oppure a punteggio zero per confrontare |
| 2 | Eta' in coda (da quando la stazione e' stata sentita, tetto 90 s) e **blocco manuale**: un chiamante scelto a mano resta in testa, e un solo spostamento a mano sospende il riordino automatico fino a svuotare la coda | `qso-engine` (coda chiamanti, regola J del documento sulla banda satura), UI | M | |
| 3 | Indicatori dal registro: gia' lavorato su questa banda (oggi o sempre), lavorato su altra banda/giorno, DXCC mai lavorato, DXCC nuovo su banda; chiave `BANDA\|ENTITA'` | `logbook`, `dxcc` | S | in `aurora-core/src/event.rs:42` esistono gia' `NewDxcc`, `NewDxccBand` e `WorkedCall` (gia' lavorato su questa banda); «lavorato su un'altra banda o giorno» va ricavato da `NewCall`/`NewCallBand` (da verificare) |
| 4 | **FT4: `sync2d_candidates`** nel crate `ft4` come in 3.3 (spettrogramma di 576 campioni ogni 144, norma sui 4 toni, correlazione con le 4 matrici di Costas su 93 ritardi, massimo per bin, primi `N`=6 a distanza >= 4 bin, interpolazione parabolica) | `core/crates/ft4/src/sync.rs` | M | prova con segnali sintetici: due segnali a 3-6 Hz con ritardi diversi e 10-20 dB di differenza, e un solo segnale a -17,5 dB; il 2D deve ritrovare la frequenza entro 1 Hz |
| 5 | **FT4, modi 0/1/2**: elenco 2D + al massimo 5 vecchi candidati a > 10 Hz + griglia di profondita' 4; variabile/impostazione per spegnere; misurare messaggi e millisecondi contro il solo `getcandidates4` | `ft4/src/decode.rs` | S dopo il 4 | **le registrazioni FT4 non ci sono ancora** (vedi `analisi-decodium-decoder.md`, sezione «Come misurare»): senza di quelle si misura solo su sintetico |
| 6 | Avvio del CAT su `rigctld`: sola lettura per 750 ms, poi **una** sincronizzazione del modo, niente riscrittura della frequenza Tx/split a riposo | `core/crates/rig-control` | S | |
| 7 | Euristica di PC debole (RAM < 8 GB, < 5 processori, file-segno di avvio lento) come ingresso del governatore di carico | `engine-native` | S | |
| 8 | FT8: **niente da portare** da questa lettura. Quando si fara' il recupero col BP esatto, il tetto va **per periodo** (non per vita del thread, come probabilmente succede in Decodium) | `ft8/src/decode.rs` | S | |
| 9 | Tenere lo script della sezione 1.3 e la ricetta di REA in `tools/` se servira' ripetere il reverse (con la 1.0.712 e seguenti) | `tools/rea/` | S | non creato qui |

## 7. Cosa e' ancora ignoto

- Gli effetti **misurati** di `sync2d` (FT4/FT2) e di «Best caller first» in aria: non ci sono dati, ne' nelle note di versione installate ne' in quelle pubbliche.
- `FUN_14175a100` (il `sync2d` di FT2), `looksLikeWeakComputer` letto ma non il suo uso nel QML, le funzioni di appoggio dell'FT8 non toccate (`sync8` con tre forme, a priori storici, BICM, coerente).
- Il nome dei campi di `DecodiumBridge` oltre ai cinque offset del registro (`+0x6538`, `+0x6540`, `+0x6588`, `+0x65a0`, `+0x65b8`) e ai sette della coda (`+0x6718` lista, `+0x6720` e `+0x6728` dati e lunghezza, `+0x6730` orari di inserimento, `+0x6738` blocco manuale, `+0x6740` voci fissate, `+0x7c9` interruttore).
- Un confronto **completo** FT8 con il sorgente: servirebbe leggere tutto `decode_main_candidate_cpp` (10 330 righe); qui si sono confrontati i punti della tabella della sezione 4.
- Differenze sul CAT (stringhe del solo avvio di rete): non e' stata letta la logica di `0x1407bfc20` (saltare la riscrittura a riposo) oltre ai messaggi e ai timer.

## 8. Indice degli indirizzi

| Indirizzo | Nome dedotto | Sezione |
|---|---|---|
| `0x140645440` | `rescoreCallerQueue` | 2 |
| `0x140e1eb40` | punteggio e ordinamento dei chiamanti | 2 |
| `0x1412eea90` | indicatori dal registro (DXCC/banda/lavorato) | 2 |
| `0x140700e50` | punteggio «slot» | 2 |
| `0x140670930` | `setCallerScoring` | 2 |
| `0x140615980` | caricamento impostazioni (`CallerScoring`, predefinito falso) | 2 |
| `0x14070dea0`, `0x140634550`, `0x14069e5e0`, `0x1406d1f80`, `0x1407649a0` | chiamanti del riordino (`enqueue`, `manual-enqueue`, `process-next`, `mam-promote`, `click`) | 2 |
| `0x1406d2ec0`, `0x140734060`, `0x14065b560` | spostamento manuale / pulizia della coda | 2 |
| `0x1403cfaa0` | `run_ft4_decode` (tutto in linea) | 3 |
| `0x14175c9c0` | `sync2d` per FT4 | 3 |
| `0x14175c9a0`, `0x140773990` | imposta il modo `sync2d` di FT4 / `applyLowCpuRuntimeProfile` | 3 |
| `0x14177f200` | `getcandidates4` | 3 |
| `0x141625300` | aggiunge un candidato all'elenco | 3 |
| `0x1427c7500`, `0x1427c7520` | inizio e matrici di Costas di FT4 (dati) | 3 |
| `0x1403fd400`, `0x141759d90` | decoder FT2 (con `sync2d`) / imposta il modo FT2 | 5 |
| `0x140e17680` | `FT8DecodeWorker::decode` | 4 |
| `0x141608040` | passate principali e recuperi FT8 | 4 |
| `0x1404d9b10` | `decode_main_candidate_cpp` | 4 |
| `0x1403e48c0` | tetti sui candidati FT8 per passata | 4 |
| `0x14170e3a0`, `0x1411f7ea0`, `0x140771bc0`, `0x1407bfc20` | CAT di rete: avvio, fine blocco, sincronizzazione del modo, riscrittura Tx/split | 5 |
| `0x141302f30` | `looksLikeWeakComputer` | 5 |
