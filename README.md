# Aurora

[![Rust](https://img.shields.io/badge/Rust-1.98%20%C2%B7%20edition%202024-000000?logo=rust&logoColor=white)](https://www.rust-lang.org)
[![Axum](https://img.shields.io/badge/API-Axum%200.8-1A1A1A)](https://github.com/tokio-rs/axum)
[![Tokio](https://img.shields.io/badge/Async-Tokio%201.54-1A1A1A)](https://tokio.rs)
[![SQLite](https://img.shields.io/badge/DB-SQLite-003B57?logo=sqlite&logoColor=white)](https://www.sqlite.org)
[![React](https://img.shields.io/badge/UI-React%2019-61DAFB?logo=react&logoColor=black)](https://react.dev)
[![TypeScript](https://img.shields.io/badge/TypeScript-7.0-3178C6?logo=typescript&logoColor=white)](https://www.typescriptlang.org)
[![Vite](https://img.shields.io/badge/Build-Vite%208-646CFF?logo=vite&logoColor=white)](https://vite.dev)
[![WebGPU](https://img.shields.io/badge/Waterfall-WebGPU%20%2B%20WebGL2-005A9C)](https://developer.mozilla.org/docs/Web/API/WebGPU_API)
[![Lingue](https://img.shields.io/badge/Lingue-6-FF5C7A)](app/src/i18n)
[![Test](https://img.shields.io/badge/Test-580%20Rust%20%C2%B7%20512%20UI-3FD6C8)](#sviluppo)
[![SemVer](https://img.shields.io/badge/SemVer-2.0.0-3F4551)](https://semver.org/lang/it/)
[![Keep a Changelog](https://img.shields.io/badge/Changelog-Keep%20a%20Changelog-E05735)](CHANGELOG.md)
[![Manutenzione](https://img.shields.io/badge/Mantenuto-s%C3%AC%20(2026)-brightgreen)](CHANGELOG.md)
[![Licenza](https://img.shields.io/badge/Licenza-GPL--3.0-blue)](LICENSE)

Stazione digitale FT8/FT4 di nuova concezione: interfaccia d'impatto, automazione dei QSO e controllo remoto da telefono. Ha un motore proprio di decodifica e trasmissione FT8 (JTDX resta un confronto facoltativo); Aurora è cervello, motore e schermo.

Progettazione e sviluppo: Tiziano Cassone (IU3WJQ)

## Stato

Versione 0.11.5: motore FT8 proprio con ricezione e trasmissione, CAT per tutte le radio e display della radio. La trasmissione con la radio vera va ancora provata dal proprietario.

- **Stazione**: elenco dei messaggi come in JTDX (paese con bandiera, distanza, righe colorate per tipo, nominativo chiamato e chiamate verso di te in evidenza), cascata WebGPU/WebGL2 con le barre Rx/Tx trascinabili, Tx1–Tx6 con la fase del QSO, barra di bande e modo (FT8/FT4) con cambio dalla pagina.
- **Comandi come JTDX**: AutoSeq, 1 QSO, Risp B4, Bypassa, Salta Tx1, Periodo Tx, AutoTX, Abilita Tx, Tx/Rx Split, frequenze Tx/Rx con copia, Canc DX, Gen.Msgs; il doppio clic su un messaggio imposta il DX e JTDX lo chiama al momento giusto (opzione in Impostazioni → Generali).
- **Trasmissione**: CQ, messaggio libero, frequenza Tx, livello Pwr verticale, STOP TX sempre attivo (discreto a riposo, pieno in trasmissione); il tasto Esc fa lo stesso e spegne AutoTX e Abilita Tx. Non ha opzioni: è sempre abilitata (restano STOP TX, i tetti e le automazioni spente di base).
- **Automazioni**: risposte ai CQ con novità, partono spente e in prova a vuoto, con tetti, dead-man e STOP.
- **Impostazioni a schede** come JTDX (generali, radio, audio, sequenza, macro TX, segnalazioni, frequenze, notifiche, filtri, programmazione, avanzate) più accesso, finestre e bande.
- **Sei lingue** (italiano, inglese, tedesco, francese, spagnolo, portoghese) e un tooltip su ogni controllo.
- **Guida e statistiche**: guida in-app breve nelle sei lingue; pagina Statistiche con premi, QSO per banda e paesi mancanti; i tuoi messaggi trasmessi compaiono come righe `Tx`.
- **DX Cluster, orologio, log**: finestra DX Cluster (server telnet pubblici o propri, doppio clic = la radio va sulla frequenza dello spot), client NTP con indicatore dell'orologio, impostazioni vere di JTDX lette dal suo file e finestra Log di Log4OM2 in sola lettura.
- **Strumenti della stazione**: livello dell'audio in ingresso con cursore e AUTO sul volume di registrazione di Windows, storico dei messaggi ricevuti (30 giorni, ricercabile), spot del cluster sulla cascata, diagnosi automatica (orologio, audio, nominativo), pagina Propagazione con PSK Reporter e hamqsl.com (**spenti finche' non li accendi**) e mappa dei QSO e delle stazioni ricevute.
- **Telefono**: PWA sulla stessa rete, con la pagina Stazione a schede e pulsanti da dito (schermi fino a 640 px); associazione con codice (durata decisa da te) oppure accesso senza codice per le prove.

Cosa manca: `docs/mancante.md`. Esiti delle prove con JTDX: `docs/fase0-rapporto.md`.

## Installazione (Windows)

Il modo piu' semplice: l'installatore `Aurora_X.Y.Z_x64-setup.exe` (programma con vassoio, avvio con Windows e aggiornamenti firmati). Come si installa, dove stanno i dati e come si fa un rilascio: `docs/installer-windows.md`.

## Avvio rapido (Windows)

```
.\scripts\avvia-sviluppo.ps1 -Jtdx
```

Avvia il core (porta 8787) e l'interfaccia (porta 5173) in una sola tab di Windows Terminal; con `-Jtdx` si collega al JTDX reale e trova da solo `cty.dat` e il log dei QSO. Apri la pagina **dall'indirizzo IP del PC** (per esempio `http://192.168.1.225:8787`), non da `localhost`: è lo stesso indirizzo che usa il telefono.

Per avere anche la cascata imposta `AURORA_AUDIO_DEVICE` (parte del nome dell'ingresso audio, per esempio `USB Audio`): senza, la cascata resta vuota.

Per servire l'interfaccia compilata dal core (senza il server di sviluppo): `cd app && npm run build`, poi avvia Aurora con `AURORA_UI_DIR=app\dist` e `AURORA_BIND=0.0.0.0:8787`.

### Collegare un telefono

```
.\scripts\associa-telefono.ps1 -Porta 8787
```

Mostra un codice di 8 cifre e un QR; il codice vale per il tempo che decidi in Impostazioni → Accesso (predefinito 10 minuti), è monouso e si blocca dopo cinque tentativi sbagliati. Nessuna password: l'accesso è per dispositivo associato e si revoca dall'elenco.

Durante le prove si può accendere l'**accesso senza codice** (Impostazioni → Accesso, oppure `AURORA_OPEN_ACCESS=1`): vale solo per chi è sulla rete di casa, mai per internet o proxy, e dal PC si spegne in qualunque momento.

### Con JTDX reale

Vedi `docs/guida-collegamento-jtdx.md`: Aurora si mette tra JTDX e GridTracker2 (hub UDP) senza togliere nulla a nessuno dei due, e il QSO registrato arriva a Log4OM2 come prima. Il cambio banda e modo usa lo stesso OmniRig di JTDX (`AURORA_RIG=omnirig`).

### CAT: la radio per tutti i modelli

In Impostazioni → Radio CAT si sceglie come comandare la radio (frequenza, modo, PTT, split): **Hamlib** tramite `rigctld` (oltre 300 modelli, la scelta di base) oppure OmniRig. Profili con nome, rilevamento della radio e stato in tempo reale. Guida per l'utente e prove con la radio vera in `docs/cat-radio.md`.

## Configurazione

Variabili d'ambiente (elenco completo in `.env.example`): `AURORA_BIND`, `AURORA_DATA_DIR`, `AURORA_UI_DIR`, `AURORA_ENGINE` (`sim` o `jtdx`), `AURORA_UDP_LISTEN`, `AURORA_UDP_FORWARD`, `AURORA_AUDIO_DEVICE`, `AURORA_HISTORY_DAYS`, `AURORA_CTY_PATH`, `AURORA_WORKED_ADIF`, `AURORA_RIG` (`none`, `omnirig`, `hamlib`), `AURORA_RIGCTLD`, `AURORA_EXPORT_UDP`, `AURORA_EXPORT_FILE`, `AURORA_TX_POWER`, `AURORA_PAIR_TTL_SECS`, `AURORA_OPEN_ACCESS`, `AURORA_TRUST_LOCAL`, `AURORA_AUTO_*`.
Le preferenze di visualizzazione (colori, filtri, lingua, bande, macro) stanno nel browser; frequenze, programmazione, regole dell'automazione, durata del codice e radio (`radio.json`: modello, porta, profili) stanno in `data/`.

## Struttura

```
core/crates/   aurora-core (tipi, bus)  udp-bridge  udp-hub  engine-jtdx  engine-sim
               dxcc (cty.dat)  audio-spectrum  logbook  qso-engine (automazioni)  qso-export
               rig-control (CAT: Hamlib e OmniRig, frequenze, programmazione)  auth  remote-api (HTTP/WebSocket)  aurora (binario)
app/src/       pages  components  settings (una scheda per funzione)  lib  i18n (6 lingue)  render (cascata)  state  api
scripts/       avvia-sviluppo.ps1  associa-telefono.ps1
docs/          specifica, piani, rapporti, API, guida JTDX, mancante.md
```

## Documentazione

- Specifica: `docs/2026-10-24-aurora-design.md` · API: `docs/api.md` · Flussi UDP: `docs/udp-flussi.md`
- Piani: `docs/2026-10-24-aurora-piano*.md` · Rapporti: `docs/fase0-rapporto.md`, `docs/piano2-rapporto.md`, `docs/piano3-rapporto.md`, `docs/piano4-rapporto.md`
- Cosa manca: `docs/mancante.md` · Memoria di progetto: `memory.md` · Cronologia: `CHANGELOG.md`

## Sviluppo

```
cd core && cargo test --workspace      # 343 test del core
cd app && npm install && npm test      # 323 test dell'interfaccia
cd app && npm run build                # build di produzione (tsc + vite)
cd core && cargo clippy --workspace --all-targets
```

Sul PC con Aurora accesa, i test del core si lanciano con `--target-dir target/testrun` (altrimenti Windows blocca `aurora.exe`).

## Licenza

Aurora e' software libero, rilasciato con la **GNU General Public License versione 3** (`GPL-3.0-only`: il motore proprio contiene algoritmi di JTDX e WSJT-X, rilasciati con la GPL 3): testo completo in [`LICENSE`](LICENSE). Copyright (C) 2026 Tiziano Cassone (IU3WJQ). Le librerie di terzi e i loro avvisi sono in [`THIRD-PARTY-NOTICES.md`](THIRD-PARTY-NOTICES.md) e [`docs/licenze-terze-parti.md`](docs/licenze-terze-parti.md).

Chi ridistribuisce Aurora, anche modificata, deve farlo con la stessa licenza e rendere disponibili i sorgenti. Le librerie di terzi che l'interfaccia include (`leaflet` per la mappa, licenza BSD-2-Clause, `world-atlas` e `topojson-client`, licenza ISC;  le tessere dello sfondo della mappa sono di Esri e National Geographic, con la loro attribuzione sulla mappa, e le altre dipendenze elencate in `app/package.json` e `core/Cargo.toml`) mantengono le proprie licenze.
