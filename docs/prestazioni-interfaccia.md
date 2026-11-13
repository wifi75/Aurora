# Prestazioni e stabilita' dell'interfaccia

Verifica di CPU, memoria, render e stabilita' dell'interfaccia (`app/`), fatta sulla 0.11.243. Il motore non e' stato avviato: le misure
vengono da prove `vitest` mirate (jsdom, React in modalita' sviluppo, Profiler di React) e dalla compilazione `vite build`. I tempi assoluti
in jsdom sono piu' alti di quelli di WebView2 con il codice di produzione; contano i rapporti fra prima e dopo e soprattutto il numero di render.

Regola seguita: nessuna modifica senza misura e senza cambiare cio' che si vede.

## Come si e' misurato

- **Negozio**: 20 000 eventi `status` e `decode` dati a `useStation.apply`, tempo medio per evento.
- **Render per evento**: 40 eventi `status` (dieci secondi a quattro al secondo), ognuno nel suo `act()` come arrivano davvero dal WebSocket,
  contando i render con il Profiler.
- **Attivita' di banda**: `DecodeList` con 200 righe (il massimo del negozio).
- **Finestra RX dei modi testuali**: 40 000 caratteri (il massimo di `digiBus`, circa 1200 righe) e 350 caratteri nuovi che fanno entrare
  righe in fondo e uscire righe dall'inizio, come in un QSO RTTY lungo.
- **Pagina del telefono**: `PhoneLive` aperta, due secondi senza eventi.
- **Memoria**: 5000 nominativi diversi chiesti a `lookupCallsign` (il precaricamento dei nominativi in vista).
- **Pacchetto**: `npx vite build`, peso dei file `.js` e `.css`.

## Prima e dopo

| Misura | Prima | Dopo |
| --- | --- | --- |
| Costo di un evento `status` nel negozio | 8,4-8,8 µs | 9,1-9,6 µs (confronto dello stato con quello di prima) |
| Costo di un evento `decode` nel negozio | 1,7 µs | 1,7 µs |
| Componente che legge lo stato intero, 40 stati identici | 40 render | 1 render |
| Componente che legge solo alcuni campi, 40 stati con l'S-meter che cambia | 40 render (stato intero) | 1 render (`useStatusPick`) |
| Attivita' di banda (200 righe), 40 stati con stazioni in coda | 40 render, 350-410 ms | 0 render |
| Attivita' di banda, 40 stati senza coda | 1 render | 1 render |
| Attivita' di banda, primo disegno | 146-158 ms | 144-146 ms |
| Finestra RX, un carattere nuovo con righe che entrano ed escono | 10,5-11,6 ms (piu' il ridisegno di tutta la pagina PSK · RTTY · CW) | 5,6-6,4 ms (solo la finestra) |
| Finestra RX, primo disegno di 40 000 caratteri | 32-55 ms | 81-92 ms (una volta all'apertura: ogni riga e' un componente a se') |
| Pagina PSK · RTTY · CW, render per carattere ricevuto e per stato del motore | tutta la pagina | nessuno (si ridisegna solo la finestra RX) |
| Pagina del telefono, due secondi fermi | 10 render, 383 ms | 8-9 render minuscoli, 2 ms |
| Cache dei nominativi dopo 5000 nominativi | 5000 voci (senza limite) | 3000 voci |
| JavaScript del pacchetto | 9 001 277 byte | 9 002 767 byte (+1,5 kB) |
| CSS del pacchetto | 1 018 308 byte | 1 018 308 byte |

Il pacchetto: i file piu' grandi sono `countries-10m` (3,7 MB, carta del mondo dettagliata, caricata solo dalle mappe), `App` (2,1 MB),
il blocco condiviso chiamato `Logo` (1,9 MB: react-query, zod e i testi in sette lingue condivisi fra la pagina e le Impostazioni) e
`countries-50m` (0,76 MB). La pagina si carica dal disco (Tauri) o dalla rete di casa (telefono): il peso non e' un problema di CPU.

## Problemi trovati e corretti

1. **Lo stato del motore era sempre un oggetto nuovo** (`app/src/state/station.ts`, caso `status` di `apply`, ora riga 256). Il motore
   manda lo stato quattro volte al secondo, quasi sempre uguale; zod crea un oggetto nuovo a ogni arrivo e chi leggeva `s.status` intero o un
   suo elenco si ridisegnava sempre. Ora `shareStatus` (riga 196) tiene gli stessi oggetti per le parti uguali e lo stato di prima se non
   cambia niente.
2. **Attivita' di banda ridisegnata quattro volte al secondo con stazioni in coda** (`app/src/components/DecodeList.tsx:380`,
   `useStation((st) => st.status?.queue)`): l'elenco della coda era un array nuovo a ogni stato e la lista di 200 righe si ridisegnava di
   continuo (circa 9 ms per volta in jsdom). Corretto dal punto 1, senza toccare `DecodeList`.
3. **Componenti che leggevano lo stato intero per uno o due campi**: `WaterfallStack.tsx` (la cascata di Fox e Contest), `CheckAll.tsx`,
   `settings/JtdxTabs.tsx`, `settings/WaterfallPreview.tsx`, `pages/FoxPage.tsx` (pagina e spettro), `pages/DigiPage.tsx`. Con la radio che
   da' l'S-meter lo stato cambia davvero a ogni arrivo; ora leggono solo i campi che usano con `useStatusPick`.
4. **La pagina PSK · RTTY · CW si ridisegnava a ogni carattere ricevuto** (`pages/DigiPage.tsx`, prima righe 142-143 e 229-232): il testo
   della finestra RX era letto dalla pagina. Ora lo legge solo `DigiRxLive` (`components/DigiRxText.tsx`), che fa anche scorrere la finestra
   in fondo come prima.
5. **Finestra RX: chiavi per indice** (`components/DigiRxText.tsx`). Quando una riga usciva dall'inizio del testo (oltre 40 000 caratteri)
   tutte le chiavi scalavano e React riscriveva tutte le righe. Ora `digiBus` conta i caratteri tagliati (`getDigiBase`), ogni riga ha per
   chiave la sua posizione assoluta ed e' un componente memorizzato: a ogni carattere si ridisegna solo l'ultima riga.
6. **Pagina del telefono ridisegnata intera quattro volte al secondo** (`pages/PhoneLive.tsx`, `usePeriod`): l'orologio del periodo stava
   nella pagina (cluster, log e messaggi Tx compresi). Ora i secondi e la barretta sono due piccoli componenti (`PeriodSecs`, `PeriodFill`)
   con lo stesso HTML di prima.
7. **Cache dei nominativi senza limite** (`components/CallsignTip.tsx`, mappe `cache` e `ready`): con la pagina aperta per giorni il
   precaricamento cerca migliaia di nominativi e la memoria cresceva senza fine. Ora al massimo 3000 (`MAX_CACHED`), si dimenticano i cercati
   per primi; una risposta arrivata dopo che il nominativo e' stato dimenticato serve a chi l'aspettava ma non si conserva (la prova l'ha
   scoperto: senza questo controllo `ready` superava il limite).
8. **Indirizzo `blob:` non liberato** (`lib/voice.ts`, `addSoundFile`): caricando un suono con lo stesso nome di uno gia' usato, l'indirizzo
   del vecchio restava in memoria fino alla chiusura. Ora si libera.
9. **Errori fuori dalla pagina** (`App.tsx`): solo il contenuto della pagina era dentro `PageGuard`; un errore nella barra in fondo
   (`Footer`, ridisegnata due volte al secondo con i dati del motore), nella barra dei desiderati o nel rotore smontava tutta l'interfaccia
   (React 19 svuota la radice). Ora anche queste tre parti stanno dentro `PageGuard`: in caso di errore spariscono solo loro e tornano al
   cambio di pagina. `PageGuard` non aggiunge elementi: l'aspetto non cambia.

Prove nuove: `state/station.test.ts` (stato condiviso), `components/DigiRxText.test.tsx` (aspetto della finestra e righe di mezzo che
restano gli stessi elementi), `components/CallsignTip.test.tsx` (limite della cache).

## Controllato e in ordine

- `api/stream.ts`: un solo WebSocket, riconnessione con attesa crescente, guardiano del silenzio, `JSON.parse` dentro `try`, tutto chiuso da
  `close()`.
- Elenchi del negozio tutti limitati: decodifiche 200 (raccolte a gruppi ogni 60 ms), messaggi Tx 50, desiderati 20, evidenziazioni 500,
  QSO appena registrati 10, avvisi per identificativo. Testo RX 40 000 caratteri, notifiche 4, segni dell'ora della cascata 40, righe
  dell'allenamento contest due periodi, segnali dei modi testuali sostituiti a ogni evento, storico SSTV dal core.
- Tutti i `setInterval` dei componenti si fermano allo smontaggio; i piu' frequenti (orologio, anello del periodo, allineamento della
  pagina) saltano il lavoro a finestra nascosta. Gli ascoltatori di eventi senza rimozione sono solo quelli globali messi una volta
  all'avvio (`main.tsx`, `lib/prefs.ts`, `lib/uiScale.ts`, `lib/appearance.ts`, `lib/freshUi.ts`, `lib/soundBank.ts`).
- Tutti i 43 `JSON.parse` sono dentro un `try`.
- `URL.createObjectURL`: `sstv/History.tsx` e `lib/foxLog.ts` liberano l'indirizzo dopo lo scaricamento; `lib/voice.ts` lo tiene in cache
  apposta (corretto il caso del punto 8).
- Le animazioni infinite del CSS (36) sono quasi tutte legate a uno stato (in trasmissione, in onda, lampeggio di un avviso), non sempre
  accese; 36 regole `prefers-reduced-motion`.

## Non corretti (da misurare in WebView2 prima di toccarli)

- **Cascata, letture della misura a ogni riga** (`components/Waterfall.tsx:62` e `:102`): `drawStamps` scrive `style.top`/`height` e legge
  `offsetTop`/`clientHeight`, `drawPlot` legge `getBoundingClientRect` per ogni riga (circa 3,5 al secondo): possibili ricalcoli forzati
  dell'impaginazione. In jsdom non si misura; serve il profiler di WebView2 (`Performance`, «Forced reflow»). Le misure si potrebbero tenere
  dal `ResizeObserver` che c'e' gia'.
- **Vetro sfocato sui pannelli** (`styles/glass.css:134`, `.panel` con `backdrop-filter: blur(14px)`; `styles/components.css:771` sulle
  notifiche): i pannelli contengono liste che cambiano spesso. Toglierlo cambia l'aspetto, quindi va prima misurato (WebView2,
  `Rendering > Paint flashing` e tempo di composizione).
- **Componenti che leggono ancora lo stato intero**: `DiagnosisBar.tsx:28` (passa lo stato a `diagnose`), `radio-display/RadioDisplayHeader.tsx:42`
  e `RadioExtras.tsx:80` (usano gli strumenti della radio, che cambiano davvero), `PhoneLive.tsx:92` (passa lo stato a `useTxSequence`).
  Con il punto 1 si ridisegnano solo quando lo stato cambia davvero.
- **`FrequencyMarkers.tsx:68`**: un `setInterval` ogni 100 ms sempre acceso; non ridisegna (restituisce lo stesso valore), costo trascurabile.
- **`Footer.tsx:16`**: la barra in fondo si ridisegna due volte al secondo per l'orologio; come per il telefono si potrebbe ridisegnare solo
  l'orologio, ma la barra e' piccola.
- **Attivita' di banda**: con una decodifica nuova si ridisegnano tutte le 200 righe (3-22 ms in jsdom, una volta per periodo). Le righe
  non sono componenti memorizzati perche' dipendono da molte preferenze; non vale la complessita'.
- **Pacchetto**: il blocco `Logo` (1,9 MB) e `App` (2,1 MB) si potrebbero dividere caricando a richiesta la Guida e il dizionario olandese.

## Cosa conviene fare dopo

1. Una sessione con il profiler di WebView2 (`--auto-open-devtools-for-tabs` o il menu sviluppatore) sulla pagina Stazione con la cascata e
   il DX Cluster aperti per dieci minuti: CPU, ricalcoli forzati della cascata, tempo di composizione del vetro sfocato.
2. Lo stesso sulla pagina PSK · RTTY · CW con un QSO RTTY lungo, per confermare il guadagno della finestra RX con il codice di produzione.
3. Un'istantanea della memoria (`Memory > Heap snapshot`) dopo una notte di FT8 con il precaricamento dei nominativi acceso.
