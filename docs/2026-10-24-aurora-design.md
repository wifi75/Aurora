# Aurora — Specifica di design

Data: 2026-10-24 · Stato: bozza per revisione · Autore: Tiziano Cassone

## 1. Obiettivo

Aurora è una stazione digitale FT8/FT4 di nuova concezione per radioamatori:
interfaccia grafica d'impatto, automazione dei QSO e controllo remoto completo da
telefono (Android e iPhone), da casa o da fuori.

Nasce dallo studio di JTDX (`C:\JTDX64\159`, build 159, Qt5/C++/Fortran, fork GPL di WSJT-X).
**JTDX resta il motore** di decodifica, CAT e trasmissione. Aurora è il cervello e il volto
sopra di esso (opzione "A" scelta il 2026-10-24).

### Criteri di successo
1. Decode di JTDX visibili in Aurora entro 1 secondo dalla fine del ciclo.
2. Cascata fluida a 60 fps su PC e ad almeno 30 fps su un telefono di fascia media.
3. Un QSO completo (CQ → 73) condotto in automatico, con log corretto, senza toccare JTDX.
4. Telefono utilizzabile da LAN, Tailscale e Cloudflare con lo stesso dispositivo associato.
5. STOP TX efficace entro un ciclo di trasmissione da qualsiasi interfaccia.

### 1.1 Parità funzionale e funzioni nuove (requisito aggiunto il 2026-10-24)
Aurora deve offrire **tutte le funzioni che oggi ci sono in JTDX**, con grafica molto migliorata, e in più funzioni nuove.
- **Inventario obbligatorio in Fase 0:** elenco completo delle funzioni di JTDX 159 (menu, impostazioni, modi, filtri di
  decodifica, CAT, log, avvisi, tasti rapidi) in `docs/fase0-parita-funzioni.md`, ciascuna con lo stato:
  *esposta dal protocollo UDP* / *solo nella finestra di JTDX* / *da riscrivere in Aurora*.
- **Limite dell'opzione A:** le impostazioni interne del motore (profondità di decodifica, filtri, alcune opzioni
  di rete CAT/audio) non passano dal protocollo UDP. Finché il motore è JTDX restano nella sua finestra, oppure si
  riscrivono in Aurora (opzione B parziale) o si modificano i file di configurazione di JTDX. La scelta funzione per
  funzione si fa leggendo l'inventario.
- **Funzioni nuove da definire con Tiziano** (proposte iniziali): avvisi per stazioni/paesi cercati, statistiche e
  premi (DXCC, WAZ, griglie) con avanzamento, propagazione e mappa del terminatore, notifiche push sul telefono,
  riproduzione dei cicli decodificati, temi di colore multipli.

### Fuori ambito (per ora)
Decoder proprio (opzione B), altri modi oltre FT8/FT4, app nativa degli store, multi-stazione.
L'architettura non deve però impedire un decoder proprio in futuro (vedi `Engine` al §4).

## 2. Ipotesi da verificare nello spike (Fase 0)

| # | Ipotesi | Se falsa |
|---|---|---|
| H1 | JTDX 159 emette e accetta il protocollo UDP di WSJT-X (Heartbeat, Status, Decode, Reply, HaltTx, FreeText, QSOLogged, Configure) | Riesaminare con REA + Ghidra e valutare un adattatore, oppure leggere i sorgenti JTDX |
| H2 | Aurora può catturare dalla scheda audio mentre JTDX la usa (WASAPI condiviso) | Loopback del flusso, oppure un cavo/dispositivo virtuale |
| H3 | `rigctld-jtdx.exe` consente a Aurora di leggere/impostare frequenza e PTT mentre JTDX è aperto | Passare tutto il CAT dal solo protocollo UDP di JTDX |
| H4 | JTDX si può tenere minimizzato/nascosto in modo stabile come motore | Finestra minimizzata in una sessione dedicata |
| H5 | Il comando Reply dell'UDP avvia un QSO completo con gli stessi automatismi del doppio clic | Studiare quali messaggi Reply/FreeText servono a guidare la sequenza a mano |

REA (https://github.com/morluto/rea, `npx rea-agents setup`, Node 22/24/26) con Ghidra
(non ancora installato, Java 25 presente, supporto Windows sperimentale) si usa **solo** se H1 o H5
risultano falsi o poco documentati. I sorgenti pubblici di JTDX/WSJT-X vengono letti prima del binario.

## 3. Architettura

```
 PC radioamatore (Windows)
┌─────────────────────────────────────────────────────┐
│ JTDX (motore: decode, CAT, TX)                      │
│    ▲ UDP: Reply/HaltTx/FreeText     │ decode, stato │
│    │                                ▼               │
│ Aurora Core (Rust, servizio)                        │
│  udp-bridge · audio-spectrum · rig-control          │
│  qso-engine · logbook · remote-api · auth           │
└───────────────┬─────────────────────────────────────┘
                │ HTTPS + WebSocket (binario)
     ┌──────────┼──────────────────┐
 App desktop    PWA Android        PWA iPhone
 (Tauri 2)      (installabile)     (installata)
```

Principi:
- **Il core decide, le interfacce mostrano.** Le automazioni girano sul PC anche con il
  telefono scollegato.
- **Una sola interfaccia web** per desktop e telefono (layout adattivo). Il guscio Tauri
  serve solo a dare all'app desktop icona, avvio automatico e finestra nativa.
- **JTDX è sostituibile:** il core parla con un'astrazione `Engine` (decode in ingresso,
  comandi in uscita). Oggi l'unica implementazione è `JtdxUdpEngine`.

## 4. Componenti del core

| Componente | Compito | Note |
|---|---|---|
| `engine` | Trait `Engine` + `JtdxUdpEngine` | Isola il resto da JTDX |
| `udp-hub` | Riceve il flusso UDP di JTDX, lo **inoltra identico** ai programmi gia' presenti (GridTracker2) e rimanda a JTDX i loro comandi | Vedi `docs/udp-flussi.md`: nella stazione di Tiziano JTDX parla con GridTracker2 (porta 2333) e con Log4OM2 (2237); Aurora non deve rompere questo giro |
| `udp-bridge` | Codifica/decodifica il protocollo UDP (Qt `QDataStream`) | Test con registrazioni reali di pacchetti |
| `audio-spectrum` | Cattura audio, FFT, quantizzazione a 8 bit, emissione righe | Funziona anche con JTDX fermo |
| `rig-control` | Frequenza, banda, modo, PTT via `rigctld` | Dopo H3 |
| `qso-engine` | Macchina a stati per QSO + valutazione regole | §6 |
| `logbook` | QSO, DXCC/CQ/ITU/zone lavorate, duplicati per banda/modo, ADIF | SQLite (vedi §8) |
| `remote-api` | HTTPS, WebSocket, OpenAPI | Axum |
| `auth` | Associazione dispositivi, token, TOTP, registro accessi | §7 |
| `config` | File TOML, validazione, ricarica a caldo | Mai segreti nel repo |

## 5. Interfaccia e grafica

### 5.1 Tecnologia grafica (massima resa, solo tecnologie stabili)
- **WebGPU** per la cascata e gli effetti, con **fallback automatico a WebGL2**.
  WebGPU è disponibile in WebView2 (Tauri su Windows), Chrome/Android e Safari recente su iPhone;
  dove manca si usa WebGL2 con le stesse shader portate in GLSL.
- **Cascata a 60 fps:** le righe arrivano come byte, vanno in una texture ad anello sulla GPU;
  la scorrimento è un offset nella shader, **nessuna ricopiatura**. La colormap è una
  texture 1D intercambiabile. Effetti opzionali: bloom sui picchi, persistenza (scia),
  vista prospettica 3D inclinata.
- **Mappa globale:** MapLibre GL JS con proiezione a globo, strato personalizzato per archi
  luminosi animati tra le stazioni e per il terminatore giorno/notte.
- **Grafici di serie temporali** (SNR, conteggi per banda): uPlot (leggero).
- **Sul telefono:** stesso codice, con livelli di qualità (frame rate, risoluzione, bloom)
  scelti da una misura dei frame all'avvio e modificabili a mano.
- Nessuna libreria 3D generale (Three.js) nel percorso critico: le shader sono scritte a mano
  per tenere basso peso e latenza.

### 5.2 Schermate
- **Live:** cascata a tutto schermo, decode sovrapposti alle frequenze, tocco su una stazione per rispondere.
- **Stazione:** banda, frequenza, modo, stato TX/RX, potenza; **STOP TX sempre visibile**.
- **Mappa:** stazioni sentite e QSO in tempo reale, filtri per banda e DXCC.
- **Automazioni:** regole, interruttori, cronologia con motivo di ogni decisione.
- **Log:** ricerca, duplicati, esportazione ADIF.
- **Dispositivi e accessi:** telefoni associati, revoca, registro accessi.

### 5.3 Identità visiva (cancello decisionale da chiudere con Tiziano prima del codice UI)
Modalità: **Operare** (usata a lungo, spesso di notte, anche su telefono con una mano).
Quadranti proposti: varianza 4, movimento 6 (alto sulla cascata, nullo sui controlli), densità 7.
Le altre sette scelte (palette, tipografia, geometria, densità, profondità, pulsanti, movimento)
verranno dichiarate in un file di **design token** committato prima dei componenti. Direzioni proposte:

**Direzione scelta: "Notte radio" (2026-10-24, confermata dopo aver visto cinque proposte).** Console industriale: fondo blu notte tinto (`#0b0e14`), pannelli `#111722`, bordi `#243046`, testo `#cdd7e6`, testo secondario `#6f7f96`; accento ciano `#3fd6c8` per stato RX e regole AUTO, ambra `#f2a93b` per gli eventi rari (nuovo DXCC/zona), rosso `#ff5d5d` solo per STOP TX. La colormap della cascata va da blu notte a verde acqua, ciano, giallo, bianco. Angoli di raggio 4, dati in monospazio, una sola famiglia di icone. Le altre direzioni valutate (Fosforo, Aurora boreale, Carta e inchiostro, Cabina ambra) restano come varianti di tema generate dai token.

Regole già fissate: entrambe le modalità chiara e scura; tutti i valori estetici dai token;
bersagli ≥48 px su touch, testo dei campi ≥16 px; niente animazioni su azioni ripetute;
solo `transform`/`opacity`; `prefers-reduced-motion` rispettato; una sola famiglia di icone;
font self-hosted. Piè di pagina in una riga: "Aurora vX.Y.Z — Progettazione e sviluppo: Tiziano Cassone",
letto dall'endpoint di stato del core.

## 6. Automazione dei QSO

**Modello:** una regola = *condizione → azione → limiti*.

- Condizioni: DXCC/zona/griglia non lavorati, banda/modo, SNR minimo, stazione già in log (per banda/modo),
  orario, numero di cicli senza risposta, lista di chiamate escluse.
- Azioni: Reply a una stazione, CQ (anche diretto), cambio banda, attesa.
- **Macchina a stati per QSO:** IDLE → CALLING → REPORT_SENT → REPORT_RECEIVED → RR73/73 → LOGGED, con
  timeout e ripetizioni; ogni transizione è registrata con il motivo.
- Concorrenza: un solo QSO attivo per volta (FT8/FT4 ha un solo canale TX).

### Salvaguardie (non disattivabili dal telefono, impostate sul PC)
1. Tetto di cicli TX consecutivi e di ore di TX automatico al giorno.
2. **Dead-man switch:** una trasmissione *avviata dal telefono* si ferma se la connessione cade oltre N secondi;
   le regole automatiche proseguono solo se abilitate esplicitamente.
3. **Prova a vuoto:** simula le decisioni senza trasmettere. È la modalità predefinita alla prima attivazione.
4. STOP TX globale (HaltTx a JTDX + blocco del `qso-engine`) da ogni interfaccia e da scorciatoia sul PC.
5. Registro di ogni decisione con motivo leggibile.
6. Responsabilità: la trasmissione automatica va usata entro le regole della licenza dell'operatore
   (controllo remoto, identificazione, ecc.). Aurora lo ricorda alla prima attivazione.

## 7. Connessioni e sicurezza

Un solo server, **trasporti intercambiabili**, stessa autenticazione.

| Modalità | Funzionamento | Esposizione |
|---|---|---|
| **A · LAN** | `aurora.local` (mDNS), HTTPS con CA locale fissata nell'associazione via QR | Solo rete domestica |
| **B · Tailscale** | Pubblicazione sulla tailnet con HTTPS e MagicDNS | Nessuna porta aperta |
| **C · Cloudflare** | `cloudflared` (tunnel in uscita) su dominio dell'utente, con Cloudflare Access davanti | Pubblica, difesa doppia |

- Il telefono prova A, poi B, poi C, con lo stesso token: passa da casa a fuori senza riassociarsi.
- **Policy per trasporto:** su C i comandi di trasmissione sono **disattivati per impostazione predefinita** e,
  se attivati, richiedono TOTP. Impostazione modificabile solo dal PC.
- **Associazione:** codice QR monouso mostrato sul PC → token per dispositivo (revocabile, scadenza configurabile),
  salvato nel dispositivo; sul core solo l'hash. Registro accessi con trasporto, dispositivo, IP.
- Nessun account con password: l'accesso è per dispositivo associato. (Scostamento consapevole dallo standard
  "admin di default": non esistono credenziali da pubblicare, perché non c'è un login a password.)
- HTTPS ovunque; cookie/token mai in localStorage dove evitabile; validazione di ogni input al confine dell'API;
  limiti di frequenza sui comandi; CORS ristretto; log senza dati sensibili; niente segreti nel repo (`.env.example`).
- Banda adattiva: spettro compresso e frame rate ridotto su connessioni lente (circa 4–15 KB/s).

## 8. Dati

- **SQLite embedded (rusqlite)** per log e configurazione: scelta motivata, non di comodo — è un'app locale
  monoutente con un solo processo scrittore, senza server da gestire. Scostamento dallo standard
  (PostgreSQL) da confermare; il livello `logbook` è dietro un'interfaccia, quindi sostituibile.
- Migrazioni versionate; backup automatico del file; esportazione/importazione ADIF.
- Dati personali: l'unico dato personale sono chiamate e griglie dei corrispondenti, già pubbliche in radio;
  nessuna telemetria.

## 9. Stack (versioni verificate sui registri il 2026-10-24)

**Core (Rust edition 2024, rustc 1.98.0 installato):**
`tokio 1.54.1` · `axum 0.8.9` · `tower-http 0.7.1` · `tokio-tungstenite 0.30.0` · `rustls 0.23.46` ·
`rcgen 0.14.10` · `cpal 0.18.2` · `rustfft 6.4.1` · `rusqlite 0.40.2` · `serde 1.0.229` · `tracing 0.1.44` ·
`utoipa 6.0.0` · `argon2 0.6.0` · `totp-rs 6.0.0` · `mdns-sd 0.21.6` · `qrcode 0.14.1` · `tauri 2.12.5`

**Interfaccia:**
`react 19.3.1` · `typescript 7.0.2` · `vite 8.3.5` · `@vitejs/plugin-react 6.1.2` · `@tanstack/react-query 5.104.5` ·
`zustand 5.0.16` · `zod 4.6.7` · `react-router 8.5.0` · `maplibre-gl 6.15.0` · `uplot 1.6.32` · `tailwindcss 4.3.3` ·
`vitest 5.0.5` · `@tauri-apps/api 2.12.4`

Le versioni verranno ricontrollate al momento di fissarle nei `Cargo.toml`/`package.json`.
Tailwind è configurato con i token del §5.3 (mai i valori predefiniti). Se emergono incompatibilità tra
queste versioni, vince la combinazione che compila e supera i test.

## 10. Struttura del repository

```
Aurora/
  core/          # workspace Rust: engine, udp-bridge, audio-spectrum, qso-engine, logbook, remote-api, auth
  app/           # interfaccia React/TS (usata da PWA e da Tauri)
  desktop/       # guscio Tauri 2
  docs/          # specifiche, protocollo UDP osservato, guida installazione
  scripts/       # avvio sviluppo (una tab Windows Terminal, salta ciò che è già acceso)
  AGENTS.md  README.md  CHANGELOG.md  TODO.md  memory.md  .env.example
```
Nessuna cartella locale né riferimento a strumenti di assistenza nel repository.

## 11. Test

- `udp-bridge`: test unitari su pacchetti reali registrati da JTDX.
- `qso-engine`: test a tabella sulle transizioni e sulle regole; simulatore di JTDX per testare i QSO senza radio.
- `audio-spectrum`: segnali sintetici con picchi noti.
- `remote-api`/`auth`: test di integrazione su token, revoca, policy per trasporto, dead-man switch.
- Interfaccia: Vitest per la logica; verifica della cascata con frame di riferimento; prova su telefono reale in
  verticale e orizzontale, con misure degli elementi.
- Le prove manuali con la radio e il telefono le fa Tiziano; io eseguo i test automatici.

## 12. Fasi

0. **Spike**: H1–H5, registrazione dei pacchetti UDP, eventuale REA + Ghidra.
1. Core in sola lettura: `udp-bridge`, `logbook`, `remote-api`, associazione dispositivi.
2. Interfaccia Live: cascata (WebGPU + WebGL2), decode, mappa; desktop e telefono su LAN.
3. Tailscale e Cloudflare; policy per trasporto; registro accessi.
4. Comandi manuali: Reply, STOP TX, banda/frequenza.
5. Motore delle automazioni: prova a vuoto, poi reale con tutte le salvaguardie.
6. Rifinitura grafica, prestazioni su telefono, installatore e documentazione.

## 13. Rischi

| Rischio | Mitigazione |
|---|---|
| JTDX non espone tutto ciò che serve via UDP | Spike H1/H5; ripiego su Reply/FreeText o su REA |
| Conflitto audio tra JTDX e Aurora | H2; ripiego sul loopback |
| WebGPU assente su certi telefoni | Fallback WebGL2 automatico |
| Trasmissione automatica indesiderata | Prova a vuoto di default, salvaguardie del §6 |
| Esposizione su internet (C) | Cloudflare Access + token + TOTP + TX spento di default |
| Versioni molto recenti (TypeScript 7, Vite 8) | Test in CI; si fissa la combinazione che regge |

## 14. Domande ancora aperte

1. ~~Direzione grafica~~ Scelta: Notte radio (2026-10-24).
2. Licenza di Aurora: se parla con JTDX solo via UDP come processo separato non è un'opera derivata e la licenza è libera; se riusa codice di JTDX/WSJT-X deve essere GPL. Si decide dopo lo spike.
3. ~~Conferma di SQLite al posto di PostgreSQL (§8).~~ Confermato il 2026-10-24.
4. Dominio da usare per Cloudflare (modalità C): serve dalla Fase 3 (proposta: `aurora.iu3cyv.eu`).
5. ~~Assenza di login con password (§7).~~ Confermata il 2026-10-24.
6. **Come inserirsi nel flusso UDP esistente** (hub davanti a GridTracker2, multicast, o sostituzione): vedi `docs/udp-flussi.md`. Il log principale resta Log4OM2, alimentato direttamente da JTDX sulla porta 2237.
