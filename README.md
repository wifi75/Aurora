# Aurora

[![Download](https://img.shields.io/github/downloads/wifi75/Aurora/total?label=Download&color=2ea44f&logo=github)](https://github.com/wifi75/Aurora/releases)
[![Virus free](https://img.shields.io/badge/Virus%20free-0%20detections-2ea44f?logo=virustotal&logoColor=white)](https://www.virustotal.com/gui/file/6263104fb767835928f9e6db027e146f31644c9b89c6f68cd248f30e9f7f4f49)
![Visite](https://visitor-badge.laobi.icu/badge?page_id=wifi75.Aurora&left_text=Visite&left_color=%23555&right_color=%238e7bff)
[![Piattaforma](https://img.shields.io/badge/Piattaforma-Windows%20%7C%20Linux%20%7C%20macOS-0078D4?logo=windows&logoColor=white)](https://github.com/wifi75/Aurora/blob/main/docs/linux.md)
[![Linux](https://github.com/wifi75/Aurora/actions/workflows/linux.yml/badge.svg)](https://github.com/wifi75/Aurora/actions/workflows/linux.yml)
[![macOS](https://github.com/wifi75/Aurora/actions/workflows/macos.yml/badge.svg)](https://github.com/wifi75/Aurora/actions/workflows/macos.yml)
[![Windows](https://github.com/wifi75/Aurora/actions/workflows/windows.yml/badge.svg)](https://github.com/wifi75/Aurora/actions/workflows/windows.yml)
[![Rust](https://img.shields.io/badge/Rust-1.99%20%C2%B7%20edition%202024-000000?logo=rust&logoColor=white)](https://www.rust-lang.org)
[![Axum](https://img.shields.io/badge/API-Axum%200.8-1A1A1A)](https://github.com/tokio-rs/axum)
[![Tokio](https://img.shields.io/badge/Async-Tokio%201.54-1A1A1A)](https://tokio.rs)
[![SQLite](https://img.shields.io/badge/DB-SQLite-003B57?logo=sqlite&logoColor=white)](https://www.sqlite.org)
[![Tauri](https://img.shields.io/badge/Desktop-Tauri%202-24C8DB?logo=tauri&logoColor=white)](https://tauri.app)
[![React](https://img.shields.io/badge/UI-React%2019-61DAFB?logo=react&logoColor=black)](https://react.dev)
[![TypeScript](https://img.shields.io/badge/TypeScript-7.0-3178C6?logo=typescript&logoColor=white)](https://www.typescriptlang.org)
[![Vite](https://img.shields.io/badge/Build-Vite%208-646CFF?logo=vite&logoColor=white)](https://vite.dev)
[![WebGPU](https://img.shields.io/badge/Waterfall-WebGPU%20%2B%20WebGL2-005A9C)](https://developer.mozilla.org/docs/Web/API/WebGPU_API)
[![Lingue](https://img.shields.io/badge/Lingue-6-FF5C7A)](app/src/i18n)
[![Test](https://img.shields.io/badge/Test-1414%20Rust%20%C2%B7%20831%20UI-3FD6C8)](#sviluppo)
[![SemVer](https://img.shields.io/badge/SemVer-2.0.0-3F4551)](https://semver.org/lang/it/)
[![Keep a Changelog](https://img.shields.io/badge/Changelog-Keep%20a%20Changelog-E05735)](CHANGELOG.md)
[![Manutenzione](https://img.shields.io/badge/Mantenuto-s%C3%AC%20(2026)-brightgreen)](CHANGELOG.md)
[![Licenza](https://img.shields.io/badge/Licenza-GPL--3.0-blue)](LICENSE)

*Dedicated to Aurora, my daughter: the light this program is named after.* ♥

**English** · [Italiano](README.it.md) · [Deutsch](README.de.md) · [Français](README.fr.md) · [Español](README.es.md) · [Português](README.pt.md)

> ### Download Aurora
> [![Download](https://img.shields.io/badge/Download-Aurora--Setup.exe-1E90FF?logo=windows&logoColor=white)](https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Setup.exe) [![Download](https://img.shields.io/badge/Download-Aurora--Linux--x86__64.AppImage-FCC624?logo=linux&logoColor=black)](https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-x86_64.AppImage) [![Download](https://img.shields.io/badge/Download-Aurora--Linux--amd64.deb-A81D33?logo=debian&logoColor=white)](https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-amd64.deb) [![Download](https://img.shields.io/badge/Download-Aurora--macOS.dmg-000000?logo=apple&logoColor=white)](https://github.com/wifi75/Aurora/releases/latest/download/Aurora-macOS.dmg) [![latest](https://img.shields.io/github/v/release/wifi75/Aurora?label=latest)](https://github.com/wifi75/Aurora/releases/latest)
>
> Version 0.11.188 was checked with [VirusTotal](https://www.virustotal.com) (about 70 antivirus engines) and Windows Defender: no antivirus flags it ([report](https://www.virustotal.com/gui/file/6263104fb767835928f9e6db027e146f31644c9b89c6f68cd248f30e9f7f4f49)).
>
> **[Windows installer (latest version)](https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Setup.exe)** · [Latest release page](https://github.com/wifi75/Aurora/releases/latest)
>
> [![Linux](https://img.shields.io/badge/Guide-Linux-E95420?logo=ubuntu&logoColor=white)](https://github.com/wifi75/Aurora/blob/main/docs/linux.en.md)
>
> 🐧 **[Linux (not tested by the author, no support): AppImage, .deb or one-command build](https://github.com/wifi75/Aurora/blob/main/docs/linux.en.md)**
>
> 🍎 **[macOS (not tested by the author, no support): Aurora-macOS.dmg for Apple Silicon and Intel](https://github.com/wifi75/Aurora/releases/latest/download/Aurora-macOS.dmg)** — not signed by Apple: on first launch right-click › Open (or `xattr -cr /Applications/Aurora.app`). For CAT install Hamlib with `brew install hamlib`. Updates install from inside the app (“Install and restart”), as on Windows: keep Aurora in Applications, not opened from the .dmg; on the first start after an update macOS may ask for confirmation again, because Aurora is not signed by Apple.

A new kind of FT8, FT4, JT65 and JT9 digital station: a striking interface, QSO automation and remote control from your phone. It has its own FT8 decode and transmit engine; Aurora is brain, engine and screen.

Design and development: Tiziano Cassone (IU3WJQ)

## Status

Version 0.11.210: own FT8, FT4, JT65 and JT9 engine with receive and transmit, CAT for all radios and a radio display. Transmitting with a real radio still has to be tested by the owner.

- **Station**: message list as in JTDX (country with flag, distance, rows coloured by type, called callsign and calls directed at you highlighted), WebGPU/WebGL2 waterfall with draggable Rx/Tx bars, Tx1–Tx6 with the QSO phase, band and mode bar (FT8/FT4) with switching from the page.
- **Caller queue**: right-click on a calling station to queue it (an «IN CODA» tag with its position); once the QSO is logged Aurora answers the first in line; the «Risp. a tutti» switch queues whoever calls you automatically; «Canc DX» stops transmission immediately. Own engine only.
- **Modes**: FT8, FT4, JT65 and JT9 from the selector (JT65 and JT9 decode at the end of the minute: after changing mode allow one to two minutes); «JT65+JT9» switch to decode them together; Hound with frequency hops and «Filtro»; SWL mode as in JTDX. Status and parity with JTDX in `docs/2026-10-26-jt65-jt9.md` (documentation in Italian).
- **JTDX-style controls**: AutoSeq, 1 QSO, Risp B4, Bypassa, Salta Tx1, Periodo Tx, AutoTX, Abilita Tx, Tx/Rx Split, Tx/Rx frequencies with copy, Canc DX, Gen.Msgs; double-clicking a message sets the DX and JTDX calls it at the right moment (option in Settings → General).
- **Transmission**: CQ, free message, Tx frequency, vertical Pwr level, STOP TX always active (discreet at rest, full while transmitting); the Esc key does the same and turns off AutoTX and Abilita Tx. It has no option: it is always enabled (STOP TX, the caps and the automations switched off by default remain).
- **Automations**: replies to CQs with something new; they start switched off and in dry-run mode, with caps, dead-man and STOP.
- **Settings by topic**: six groups (Station, Receive, Transmit and QSO, Alerts, Network and logs, System) with pages such as Radio, Cascade, Sequence, Log services and Connected programs; search across every item, errors shown next to the field.
- **Six languages** (Italian, English, German, French, Spanish, Portuguese) and a tooltip on every control.
- **Help and statistics**: short in-app help in six languages; Statistics page with awards, QSOs per band and missing countries; your own transmitted messages appear as `Tx` rows.
- **DX Cluster, clock, log**: DX Cluster window (public or your own telnet servers, double-click = the radio goes to the spot frequency), NTP client with clock indicator, read-only Log4OM2 Log window.
- **Station tools**: input audio level with slider and AUTO on the Windows recording volume, history of received messages (30 days, searchable), cluster spots on the waterfall, automatic diagnostics (clock, audio, callsign), Propagation page with PSK Reporter and hamqsl.com (**on from the factory, can be switched off**) and the DX Map (below).
- **DX Map** (GridTracker-style): 15 backgrounds, automatic night map, greyline with twilight, Sun and Moon, fading on-air stations colored by status, animated arc lines, worked and confirmed squares by band and mode, click for square, distance, country and CQ/ITU zones, propagation heat map from PSK Reporter with lines to who receives you, customizable «Who calls whom» lines, on-air squares QSX/CQ/CQDX/QRZ with legend; detachable to another screen.
- **Radio**: radio display at the top with the extended display (second VFO, meters, PRE/ATT, AGC, NB/NR/notch, gains, RIT/XIT), every item can be switched on in Settings > General; the waterfall follows the radio's filter width read over CAT; an orphaned rigctld shuts itself down.
- **DXpeditions**: NG3K calendar (Online services, off by default) next to the CQs: on the air, heard now, upcoming and history.
- **Settings**: sidebar with search, «?» with an explanation on every item, About page (version, author, licence, links).
- **Phone**: PWA on the same network, with the Station page in tabs and finger-sized buttons (screens up to 640 px); pairing with a code (duration decided by you) or code-free access for testing.
- **Logs and integrations**: direct upload to Wavelog/Cloudlog, ClubLog, QRZ.com and LoTW (TQSL); UDP integrations with multicast and per-program options, N1MM XML; ADIF export page with «Save as».
- **From WSJT-X 3**: a7/a8 decoding and multithread FT8, Wait and Reply/Call, band hopping, full duplex, Fox verification (OTP), Tx block over UDP, stop on high SWR, unencodable messages blocked, WSJT-X 3 filters, Hamlib 4.7.2.
- **Display and cluster**: Yaesu FT-2000 and Icom IC-7610 style radio displays; NG3K DXpeditions highlighted in the DX Cluster in a colour of your choice; live waterfall preview in the settings; always-on crash diagnostics.
- **Contest (beta)**: dedicated page with 24 FT8/FT4 and VHF contests (rules and yearly dates), contest sequencing as in WSJT-X, live dupes, points, multipliers and projected score, category and operators, log with Cabrillo, EDI and ADIF.
- **UltraFOX**: Aurora's own Rust implementation with the AQP-127 polar code (implementation by Tiziano Cassone IU3WJQ), compatible with WSJT-X SuperFox (transmit and receive). Experimental.

What is missing: `docs/mancante.md`. Results of the tests with JTDX: `docs/fase0-rapporto.md` (documentation in Italian).

## Installation (Windows)

The easiest way: the installer `Aurora_X.Y.Z_x64-setup.exe` (tray application, start with Windows and signed updates). How to install, where the data lives and how to make a release: `docs/installer-windows.md` (documentation in Italian).

## Quick start (Windows)

```
.\scripts\avvia-sviluppo.ps1 -Motore
```

This starts the core (port 8787) and the interface (port 5173) in a single Windows Terminal tab; with `-Motore` it uses Aurora's own engine on the sound card (`-Audio "USB Audio"` to pick it). Open the page **from the PC's IP address** (for example `http://192.168.1.225:8787`), not from `localhost`: it is the same address the phone uses.

To get the waterfall too, set `AURORA_AUDIO_DEVICE` (part of the audio input name, for example `USB Audio`): without it the waterfall stays empty.

To serve the built interface from the core (without the development server): `cd app && npm run build`, then start Aurora with `AURORA_UI_DIR=app\dist` and `AURORA_BIND=0.0.0.0:8787`.

### Connecting a phone

```
.\scripts\associa-telefono.ps1 -Porta 8787
```

It shows an 8-digit code and a QR; the code is valid for the time you decide in Settings → Access (default 10 minutes), is single-use and locks after five wrong attempts. No password: access is per paired device and is revoked from the list.

During tests you can switch on **code-free access** (Settings → Access, or `AURORA_OPEN_ACCESS=1`): it only applies to those on your home network, never to the internet or proxies, and can be switched off from the PC at any time.

### CAT: the radio for every model

In Settings → Radio CAT you choose how to control the radio (frequency, mode, PTT, split): **Hamlib** through `rigctld` (over 300 models, the default) or OmniRig. Named profiles, radio detection and real-time status. User guide and tests with a real radio in `docs/cat-radio.md` (documentation in Italian).

## Configuration

Environment variables (full list in `.env.example`): `AURORA_BIND`, `AURORA_DATA_DIR`, `AURORA_UI_DIR`, `AURORA_ENGINE` (`native` / `sim`), `AURORA_UDP_LISTEN`, `AURORA_UDP_FORWARD`, `AURORA_AUDIO_DEVICE`, `AURORA_HISTORY_DAYS`, `AURORA_CTY_PATH`, `AURORA_WORKED_ADIF`, `AURORA_RIG` (`none`, `omnirig`, `hamlib`), `AURORA_RIGCTLD`, `AURORA_EXPORT_UDP`, `AURORA_EXPORT_FILE`, `AURORA_TX_POWER`, `AURORA_PAIR_TTL_SECS`, `AURORA_OPEN_ACCESS`, `AURORA_TRUST_LOCAL`, `AURORA_AUTO_*`.
Display preferences (colours, filters, language, bands, macros) are kept in the browser; frequencies, schedule, automation rules, code duration and radio (`radio.json`: model, port, profiles) are kept in `data/`.

## Structure

```
core/crates/   aurora-core  udp-bridge  udp-hub  engine-jtdx  engine-sim
               dxcc (cty.dat)  audio-spectrum  logbook  qso-engine  qso-export
               rig-control  auth  remote-api  aurora
app/src/       pages  components  settings  lib  i18n  render  state  api
scripts/       avvia-sviluppo.ps1  associa-telefono.ps1
docs/          specification, plans, reports, API, mancante.md (in Italian)
```

## Documentation (in Italian)

- Specification: `docs/2026-10-24-aurora-design.md` · API: `docs/api.md` · UDP streams: `docs/udp-flussi.md`
- Plans: `docs/2026-10-24-aurora-piano*.md` · Reports: `docs/fase0-rapporto.md`, `docs/piano2-rapporto.md`, `docs/piano3-rapporto.md`, `docs/piano4-rapporto.md`
- What is missing: `docs/mancante.md` · Project memory: `memory.md` · History: `CHANGELOG.md`

<a id="sviluppo"></a>

## Development

```
cd core && cargo test --workspace      # 343 core tests
cd app && npm install && npm test      # 323 interface tests
cd app && npm run build                # production build (tsc + vite)
cd core && cargo clippy --workspace --all-targets
```

On the PC where Aurora is running, run the core tests with `--target-dir target/testrun` (otherwise Windows locks `aurora.exe`).


## License

Aurora is free software, released under the **GNU General Public License version 3** (`GPL-3.0-only`: the own engine contains algorithms from JTDX and WSJT-X, released under GPL 3): full text in [`LICENSE`](LICENSE). Copyright (C) 2026 Tiziano Cassone (IU3WJQ). Third-party libraries and their notices are in [`THIRD-PARTY-NOTICES.md`](THIRD-PARTY-NOTICES.md) and [`docs/licenze-terze-parti.md`](docs/licenze-terze-parti.md) (documentation in Italian).

Anyone who redistributes Aurora, even modified, must do so under the same licence and make the source available. The third-party libraries included in the interface (`leaflet` for the map, BSD-2-Clause licence, `world-atlas` and `topojson-client`, ISC licence; the map background tiles are from Esri and National Geographic, with their attribution on the map, and the other dependencies listed in `app/package.json` and `core/Cargo.toml`) keep their own licences.
