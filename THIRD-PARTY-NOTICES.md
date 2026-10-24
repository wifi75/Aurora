# Avvisi di terze parti

Aurora (© 2026 Tiziano Cassone, IU3WJQ) e' software libero: si puo' ridistribuire e modificare secondo i termini della **GNU General Public License versione 3 o, a scelta, successiva** (vedi `LICENSE`). Usa le componenti qui sotto, che mantengono le loro licenze. L'elenco completo e aggiornato delle dipendenze con la licenza di ciascuna e' in `docs/licenze-terze-parti.md`.

## Componenti principali

| Componente | Licenza | Uso |
|---|---|---|
| [Tauri](https://tauri.app) 2 e i suoi plugin | MIT o Apache-2.0 | il programma Windows (finestra, vassoio, avvio automatico, notifiche) |
| [Tokio](https://tokio.rs), [Axum](https://github.com/tokio-rs/axum), Hyper, Serde e le altre librerie Rust | MIT o Apache-2.0 (alcune BSD, ISC, Zlib, Unicode, MPL-2.0) | il core |
| [SQLite](https://www.sqlite.org) tramite `rusqlite` | dominio pubblico (SQLite), MIT (rusqlite) | gli archivi |
| [cpal](https://github.com/RustAudio/cpal) | Apache-2.0 | l'ingresso audio per la cascata |
| [minisign-verify](https://github.com/jedisct1/rust-minisign-verify) | MIT | la verifica della firma degli aggiornamenti |
| [React](https://react.dev), [TanStack Query](https://tanstack.com/query), [Zustand](https://github.com/pmndrs/zustand), [Zod](https://zod.dev) | MIT | l'interfaccia |
| [Leaflet](https://leafletjs.com) 1.9 | BSD-2-Clause, © Vladimir Agafonkin, © Leaflet contributors | la mappa |
| [topojson-client](https://github.com/topojson/topojson-client) e [world-atlas](https://github.com/topojson/world-atlas) | ISC (i dati di world-atlas derivano da [Natural Earth](https://www.naturalearthdata.com), dominio pubblico) | i confini della mappa disegnata dal programma |
| [flag-icons](https://github.com/lipis/flag-icons) | MIT | le bandiere dei paesi |
| [IBM Plex Sans](https://github.com/IBM/plex) e [JetBrains Mono](https://github.com/JetBrains/JetBrainsMono) (pacchetti `@fontsource`) | SIL Open Font License 1.1 | i caratteri dell'interfaccia (© IBM Corp.; © The JetBrains Mono Project Authors) |

## Dati e servizi esterni

- **Sfondo della mappa** (facoltativo, si spegne con «Sfondo reale»): tessere *National Geographic World Map* di **Esri** (`server.arcgisonline.com`), con l'attribuzione «© Esri — National Geographic, Esri, DeLorme, NAVTEQ, UNEP-WCMC, USGS, NASA, ESA, METI, NRCAN, GEBCO, NOAA, iPC» mostrata sulla mappa. Le tessere restano soggette ai termini di Esri; non sono incluse nel programma.
- **PSK Reporter** (`pskreporter.info`) e **hamqsl.com** (dati di N0NBH): facoltativi, spenti di default, solo lettura; i dati restano dei loro autori.
- **cty.dat** (elenco dei prefissi): non incluso; lo legge dall'installazione di JTDX dell'utente.
- **JTDX** e **WSJT-X**: Aurora non ne contiene il codice; li comanda dall'esterno (UDP e accessibilita' di Windows) e ne cita il nome solo per dire con cosa lavora. Appartengono ai rispettivi autori.
- **Decodium** (GPL-3.0): studiato come ispirazione (`docs/confronto-decodium.md`), nessun suo codice e' stato copiato.
- **Microsoft Edge WebView2**: usato dal programma Windows per mostrare la finestra; non incluso (e' un componente di Windows).

I testi completi delle licenze MIT, Apache-2.0, BSD, ISC e OFL sono nei sorgenti di ciascun pacchetto (cartelle `node_modules/<pacchetto>` e `~/.cargo/registry`).
