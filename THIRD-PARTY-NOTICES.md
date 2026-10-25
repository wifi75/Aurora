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

## Hamlib e DLL incluse nell'installatore di Windows

Nella cartella `hamlib` accanto al programma l'installatore porta, **non modificati**, i file di Hamlib copiati dall'installazione di JTDX o WSJT-X (nessun download): `rigctld.exe`, `rigctl.exe`, `libhamlib-4.dll`, `libusb-1.0.dll`, `libwinpthread-1.dll`. Versione e SHA-256 sono in `hamlib\VERSIONE.txt`.

| Componente | Licenza | Sorgenti |
|---|---|---|
| [Hamlib](https://hamlib.github.io), libreria (`libhamlib-4.dll`, driver delle radio) | GNU LGPL 2.1 o successiva | https://github.com/Hamlib/Hamlib (stessa versione/commit di `VERSIONE.txt`) |
| Hamlib, programmi (`rigctld`, `rigctl`) | GNU GPL 2.0 o successiva (ridistribuiti secondo la GPL-3.0 di Aurora) | idem |
| [libusb](https://libusb.info) (`libusb-1.0.dll`) | GNU LGPL 2.1 o successiva | https://github.com/libusb/libusb |
| libwinpthread (MinGW-w64) | licenza permissiva in stile MIT, © 2011 mingw-w64 project | https://www.mingw-w64.org |

Aurora lancia `rigctld` come programma a parte e gli parla su TCP. Si possono sostituire i file con altre versioni compatibili (LGPL). I testi delle licenze (`LGPL-2.1.txt`, `GPL-3.0.txt`, `libwinpthread-licenza.txt`) e `LEGGIMI-HAMLIB.txt` sono nella stessa cartella; dettagli in `docs/cat-radio.md`.
**VSPE** (Eterlogic) e i suoi driver sono software di terzi con licenza propria: Aurora non li include ne' li modifica.

## Dati e servizi esterni

- **Sfondo della mappa** (facoltativo, si spegne con «Sfondo reale»): tessere *National Geographic World Map* di **Esri** (`server.arcgisonline.com`), con l'attribuzione «© Esri — National Geographic, Esri, DeLorme, NAVTEQ, UNEP-WCMC, USGS, NASA, ESA, METI, NRCAN, GEBCO, NOAA, iPC» mostrata sulla mappa. Le tessere restano soggette ai termini di Esri; non sono incluse nel programma.
- **PSK Reporter** (`pskreporter.info`) e **hamqsl.com** (dati di N0NBH): facoltativi, spenti di default, solo lettura; i dati restano dei loro autori.
- **cty.dat** (elenco dei prefissi): non incluso; lo legge dall'installazione di JTDX dell'utente.
- **JTDX** e **WSJT-X**: Aurora non ne contiene il codice; li comanda dall'esterno (UDP e accessibilita' di Windows) e ne cita il nome solo per dire con cosa lavora. Appartengono ai rispettivi autori.
- **Decodium** (GPL-3.0): studiato come ispirazione (`docs/confronto-decodium.md`), nessun suo codice e' stato copiato.
- **Microsoft Edge WebView2**: usato dal programma Windows per mostrare la finestra; non incluso (e' un componente di Windows).

I testi completi delle licenze MIT, Apache-2.0, BSD, ISC e OFL sono nei sorgenti di ciascun pacchetto (cartelle `node_modules/<pacchetto>` e `~/.cargo/registry`).
