# Aurora

[![Download](https://img.shields.io/github/downloads/wifi75/Aurora/total?label=Download&color=2ea44f&logo=github)](https://github.com/wifi75/Aurora/releases)
![Visite](https://visitor-badge.laobi.icu/badge?page_id=wifi75.Aurora&left_text=Visite&left_color=%23555&right_color=%238e7bff)
[![Piattaforma](https://img.shields.io/badge/Piattaforma-Windows%20%7C%20Linux-0078D4?logo=windows&logoColor=white)](docs/linux.md)
[![Linux](https://github.com/wifi75/Aurora/actions/workflows/linux.yml/badge.svg)](https://github.com/wifi75/Aurora/actions/workflows/linux.yml)
[![Rust](https://img.shields.io/badge/Rust-1.99%20%C2%B7%20edition%202024-000000?logo=rust&logoColor=white)](https://www.rust-lang.org)
[![Axum](https://img.shields.io/badge/API-Axum%200.8-1A1A1A)](https://github.com/tokio-rs/axum)
[![Tokio](https://img.shields.io/badge/Async-Tokio%201.54-1A1A1A)](https://tokio.rs)
[![SQLite](https://img.shields.io/badge/DB-SQLite-003B57?logo=sqlite&logoColor=white)](https://www.sqlite.org)
[![React](https://img.shields.io/badge/UI-React%2019-61DAFB?logo=react&logoColor=black)](https://react.dev)
[![TypeScript](https://img.shields.io/badge/TypeScript-7.0-3178C6?logo=typescript&logoColor=white)](https://www.typescriptlang.org)
[![Vite](https://img.shields.io/badge/Build-Vite%208-646CFF?logo=vite&logoColor=white)](https://vite.dev)
[![WebGPU](https://img.shields.io/badge/Waterfall-WebGPU%20%2B%20WebGL2-005A9C)](https://developer.mozilla.org/docs/Web/API/WebGPU_API)
[![Lingue](https://img.shields.io/badge/Lingue-6-FF5C7A)](app/src/i18n)
[![Test](https://img.shields.io/badge/Test-1414%20Rust%20%C2%B7%20765%20UI-3FD6C8)](#sviluppo)
[![SemVer](https://img.shields.io/badge/SemVer-2.0.0-3F4551)](https://semver.org/lang/it/)
[![Keep a Changelog](https://img.shields.io/badge/Changelog-Keep%20a%20Changelog-E05735)](CHANGELOG.md)
[![Manutenzione](https://img.shields.io/badge/Mantenuto-s%C3%AC%20(2026)-brightgreen)](CHANGELOG.md)
[![Licenza](https://img.shields.io/badge/Licenza-GPL--3.0-blue)](LICENSE)

*Für Aura, meine Tochter: das Licht, nach dem Aurora benannt ist.* ♥

[English](README.md) · [Italiano](README.it.md) · **Deutsch** · [Français](README.fr.md) · [Español](README.es.md) · [Português](README.pt.md)

> ### Aurora herunterladen
> [![Download](https://img.shields.io/badge/Download-Aurora--Setup.exe-1E90FF?logo=windows&logoColor=white)](https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Setup.exe) [![latest](https://img.shields.io/github/v/release/wifi75/Aurora?label=latest)](https://github.com/wifi75/Aurora/releases/latest)
>
> **[Windows-Installer (neueste Version)](https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Setup.exe)** · [Seite der neuesten Version](https://github.com/wifi75/Aurora/releases/latest)
>
> [![Linux AppImage](https://img.shields.io/badge/Linux-AppImage-FCC624?logo=linux&logoColor=black)](https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-x86_64.AppImage) [![Linux .deb](https://img.shields.io/badge/Debian%20%C2%B7%20Ubuntu-.deb-A81D33?logo=debian&logoColor=white)](https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-amd64.deb) [![Linux](https://img.shields.io/badge/Anleitung-Linux-E95420?logo=ubuntu&logoColor=white)](docs/linux.de.md)
>
> 🐧 **[Linux (vom Autor nicht getestet, ohne Support): AppImage, .deb oder Bauen mit einem Befehl](docs/linux.de.md)**

Eine neuartige digitale Station für FT8, FT4, JT65 und JT9: eindrucksvolle Oberfläche, QSO-Automatisierung und Fernsteuerung per Smartphone. Sie hat eine eigene Engine zum Dekodieren und Senden von FT8 (JTDX bleibt ein optionaler Vergleich); Aurora ist Gehirn, Motor und Bildschirm.

Konzept und Entwicklung: Tiziano Cassone (IU3WJQ)

## Stand

Version 0.11.101: eigene Engine für FT8, FT4, JT65 und JT9 mit Empfang und Senden, CAT für alle Funkgeräte und Anzeige des Funkgeräts. Das Senden mit einem echten Funkgerät muss der Eigentümer noch testen.

- **Station**: Nachrichtenliste wie in JTDX (Land mit Flagge, Entfernung, nach Typ gefärbte Zeilen, gerufenes Rufzeichen und Anrufe an dich hervorgehoben), WebGPU/WebGL2-Wasserfall mit verschiebbaren Rx/Tx-Balken, Tx1–Tx6 mit der QSO-Phase, Band- und Modusleiste (FT8/FT4) mit Umschaltung direkt auf der Seite.
- **Anruferwarteschlange**: Rechtsklick auf eine rufende Station reiht sie ein (Kennzeichnung «IN CODA» mit Platz); nach dem geloggten QSO antwortet Aurora dem Ersten in der Schlange; der Schalter «Risp. a tutti» reiht automatisch jeden ein, der dich ruft; «Canc DX» stoppt das Senden sofort. Nur mit der eigenen Engine.
- **Modi**: FT8, FT4, JT65 und JT9 über die Auswahl (JT65 und JT9 dekodieren am Ende der Minute: nach einem Moduswechsel ein bis zwei Minuten einplanen); Schalter «JT65+JT9» zum gemeinsamen Dekodieren; Hound mit Frequenzsprüngen und «Filtro»; SWL-Modus wie in JTDX. Stand und Gleichstand mit JTDX in `docs/2026-10-26-jt65-jt9.md` (Dokumentation auf Italienisch).
- **Bedienelemente wie in JTDX**: AutoSeq, 1 QSO, Risp B4, Bypassa, Salta Tx1, Periodo Tx, AutoTX, Abilita Tx, Tx/Rx Split, Tx/Rx-Frequenzen mit Kopieren, Canc DX, Gen.Msgs; ein Doppelklick auf eine Nachricht setzt das DX, und JTDX ruft es im richtigen Moment (Option unter Einstellungen → Allgemein).
- **Senden**: CQ, freie Nachricht, Tx-Frequenz, vertikaler Pwr-Pegel, STOP TX immer aktiv (dezent in Ruhe, voll beim Senden); die Esc-Taste macht dasselbe und schaltet AutoTX und Abilita Tx aus. Es gibt keine Option: Senden ist immer freigegeben (es bleiben STOP TX, die Obergrenzen und die standardmäßig ausgeschalteten Automatisierungen).
- **Automatisierungen**: Antworten auf CQ-Rufe mit Neuigkeiten; sie starten ausgeschaltet und im Probelauf ohne Senden, mit Obergrenzen, Dead-Man und STOP.
- **Einstellungen nach Thema**: sechs Gruppen (Station, Empfang, Senden und QSO, Hinweise, Netz und Logs, System) mit Seiten wie Funkgerät, Wasserfall, Sequenz, Log-Dienste und Verbundene Programme; Suche über alle Einträge, Fehler direkt am Feld.
- **Sechs Sprachen** (Italienisch, Englisch, Deutsch, Französisch, Spanisch, Portugiesisch) und ein Tooltip an jedem Bedienelement.
- **Hilfe und Statistik**: kurze In-App-Hilfe in sechs Sprachen; Statistikseite mit Diplomen, QSOs pro Band und fehlenden Ländern; deine gesendeten Nachrichten erscheinen als `Tx`-Zeilen.
- **DX-Cluster, Uhr, Log**: DX-Cluster-Fenster (öffentliche oder eigene Telnet-Server, Doppelklick = das Funkgerät geht auf die Frequenz des Spots), NTP-Client mit Uhrenanzeige, echte JTDX-Einstellungen aus seiner Datei gelesen und schreibgeschütztes Log-Fenster von Log4OM2.
- **Stationswerkzeuge**: Eingangspegel des Audiosignals mit Regler und AUTO auf der Windows-Aufnahmelautstärke, Verlauf der empfangenen Nachrichten (30 Tage, durchsuchbar), Cluster-Spots im Wasserfall, automatische Diagnose (Uhr, Audio, Rufzeichen), Ausbreitungsseite mit PSK Reporter und hamqsl.com (**aus, bis du sie einschaltest**) und Karte der QSOs und empfangenen Stationen.
- **Funkgerät**: Anzeige des Funkgeräts oben mit erweiterter Anzeige (zweites VFO, Instrumente, PRE/ATT, AGC, NB/NR/Notch, Verstärkungen, RIT/XIT), jeder Eintrag unter Einstellungen > Allgemein einschaltbar; der Wasserfall folgt der per CAT gelesenen Filterbreite des Funkgeräts; ein verwaister rigctld beendet sich selbst.
- **DXpeditionen**: NG3K-Kalender (Online-Dienste, standardmäßig aus) neben den CQs: in der Luft, jetzt gehört, kommende und Verlauf.
- **Einstellungen**: Seitenleiste mit Suche, «?» mit Erklärung zu jedem Eintrag, Infoseite (Version, Autor, Lizenz, Links).
- **Smartphone**: PWA im selben Netzwerk, mit der Stationsseite in Reitern und fingergroßen Schaltflächen (Bildschirme bis 640 px); Kopplung mit Code (Gültigkeitsdauer legst du fest) oder Zugang ohne Code zum Testen.
- **Logs und Integrationen**: direkter Upload zu Wavelog/Cloudlog, ClubLog, QRZ.com und LoTW (TQSL); UDP-Integrationen mit Multicast und Optionen je Programm, N1MM-XML; ADIF-Exportseite mit „Speichern unter“.
- **Aus WSJT-X 3**: a7/a8-Dekodierung und Multithread-FT8, Wait and Reply/Call, automatischer Bandwechsel, Vollduplex, Fox-Prüfung (OTP), Sendesperre per UDP, Stopp bei hohem SWR, nicht kodierbare Nachrichten gesperrt, Filter wie WSJT-X 3, Hamlib 4.7.2.
- **Anzeige und Cluster**: Funkanzeige im Stil Yaesu FT-2000 und Icom IC-7610; NG3K-DXpeditionen im DX-Cluster in einer Farbe deiner Wahl hervorgehoben; Live-Vorschau des Wasserfalls in den Einstellungen; Absturzdiagnose immer aktiv.

Was fehlt: `docs/mancante.md`. Ergebnisse der Tests mit JTDX: `docs/fase0-rapporto.md` (Dokumentation auf Italienisch).

## Installation (Windows)

Der einfachste Weg: der Installer `Aurora_X.Y.Z_x64-setup.exe` (Programm mit Tray-Symbol, Start mit Windows und signierte Updates). Wie man installiert, wo die Daten liegen und wie man ein Release erstellt: `docs/installer-windows.md` (Dokumentation auf Italienisch).

## Schnellstart (Windows)

```
.\scripts\avvia-sviluppo.ps1 -Jtdx
```

Startet den Core (Port 8787) und die Oberfläche (Port 5173) in einem einzigen Tab von Windows Terminal; mit `-Jtdx` verbindet es sich mit dem echten JTDX und findet `cty.dat` und das QSO-Log selbst. Öffne die Seite **über die IP-Adresse des PCs** (zum Beispiel `http://192.168.1.225:8787`), nicht über `localhost`: es ist dieselbe Adresse, die das Smartphone verwendet.

Für den Wasserfall setze `AURORA_AUDIO_DEVICE` (Teil des Namens des Audioeingangs, zum Beispiel `USB Audio`): ohne bleibt der Wasserfall leer.

Um die gebaute Oberfläche vom Core aus auszuliefern (ohne Entwicklungsserver): `cd app && npm run build`, dann Aurora mit `AURORA_UI_DIR=app\dist` und `AURORA_BIND=0.0.0.0:8787` starten.

### Ein Smartphone verbinden

```
.\scripts\associa-telefono.ps1 -Porta 8787
```

Zeigt einen 8-stelligen Code und einen QR-Code; der Code gilt so lange, wie du es unter Einstellungen → Zugang festlegst (Standard 10 Minuten), ist einmalig verwendbar und sperrt sich nach fünf falschen Versuchen. Kein Passwort: Der Zugang gilt pro gekoppeltem Gerät und wird in der Liste widerrufen.

Während der Tests kann der **Zugang ohne Code** eingeschaltet werden (Einstellungen → Zugang oder `AURORA_OPEN_ACCESS=1`): Er gilt nur für Geräte im Heimnetz, nie für das Internet oder Proxys, und lässt sich am PC jederzeit ausschalten.

### Mit echtem JTDX

Siehe `docs/guida-collegamento-jtdx.md` (Dokumentation auf Italienisch): Aurora sitzt zwischen JTDX und GridTracker2 (UDP-Hub), ohne einem der beiden etwas wegzunehmen, und das geloggte QSO erreicht Log4OM2 wie zuvor. Der Band- und Moduswechsel nutzt dasselbe OmniRig wie JTDX (`AURORA_RIG=omnirig`).

### CAT: das Funkgerät für alle Modelle

Unter Einstellungen → Radio CAT wählst du, wie das Funkgerät gesteuert wird (Frequenz, Modus, PTT, Split): **Hamlib** über `rigctld` (über 300 Modelle, die Standardwahl) oder OmniRig. Benannte Profile, Erkennung des Funkgeräts und Echtzeitstatus. Anleitung für Anwender und Tests mit echtem Funkgerät in `docs/cat-radio.md` (Dokumentation auf Italienisch).

## Konfiguration

Umgebungsvariablen (vollständige Liste in `.env.example`): `AURORA_BIND`, `AURORA_DATA_DIR`, `AURORA_UI_DIR`, `AURORA_ENGINE` (`sim` / `jtdx`), `AURORA_UDP_LISTEN`, `AURORA_UDP_FORWARD`, `AURORA_AUDIO_DEVICE`, `AURORA_HISTORY_DAYS`, `AURORA_CTY_PATH`, `AURORA_WORKED_ADIF`, `AURORA_RIG` (`none`, `omnirig`, `hamlib`), `AURORA_RIGCTLD`, `AURORA_EXPORT_UDP`, `AURORA_EXPORT_FILE`, `AURORA_TX_POWER`, `AURORA_PAIR_TTL_SECS`, `AURORA_OPEN_ACCESS`, `AURORA_TRUST_LOCAL`, `AURORA_AUTO_*`.
Die Anzeigeeinstellungen (Farben, Filter, Sprache, Bänder, Makros) liegen im Browser; Frequenzen, Zeitplan, Automatisierungsregeln, Codedauer und Funkgerät (`radio.json`: Modell, Port, Profile) liegen in `data/`.

## Struktur

```
core/crates/   aurora-core  udp-bridge  udp-hub  engine-jtdx  engine-sim
               dxcc (cty.dat)  audio-spectrum  logbook  qso-engine  qso-export
               rig-control  auth  remote-api  aurora
app/src/       pages  components  settings  lib  i18n  render  state  api
scripts/       avvia-sviluppo.ps1  associa-telefono.ps1
docs/          Spezifikation, Pläne, Berichte, API, JTDX-Anleitung, mancante.md (auf Italienisch)
```

## Dokumentation (auf Italienisch)

- Spezifikation: `docs/2026-10-24-aurora-design.md` · API: `docs/api.md` · UDP-Datenströme: `docs/udp-flussi.md`
- Pläne: `docs/2026-10-24-aurora-piano*.md` · Berichte: `docs/fase0-rapporto.md`, `docs/piano2-rapporto.md`, `docs/piano3-rapporto.md`, `docs/piano4-rapporto.md`
- Was fehlt: `docs/mancante.md` · Projektgedächtnis: `memory.md` · Verlauf: `CHANGELOG.md`

<a id="sviluppo"></a>

## Entwicklung

```
cd core && cargo test --workspace      # 343 Tests des Core
cd app && npm install && npm test      # 323 Tests der Oberfläche
cd app && npm run build                # Produktions-Build (tsc + vite)
cd core && cargo clippy --workspace --all-targets
```

Auf dem PC, auf dem Aurora läuft, werden die Core-Tests mit `--target-dir target/testrun` gestartet (sonst sperrt Windows `aurora.exe`).

## Lizenz

Aurora ist freie Software und steht unter der **GNU General Public License Version 3** (`GPL-3.0-only`: die eigene Engine enthält Algorithmen aus JTDX und WSJT-X, die unter der GPL 3 stehen): vollständiger Text in [`LICENSE`](LICENSE). Copyright (C) 2026 Tiziano Cassone (IU3WJQ). Die Bibliotheken Dritter und ihre Hinweise stehen in [`THIRD-PARTY-NOTICES.md`](THIRD-PARTY-NOTICES.md) und [`docs/licenze-terze-parti.md`](docs/licenze-terze-parti.md) (Dokumentation auf Italienisch).

Wer Aurora weitergibt, auch verändert, muss dies unter derselben Lizenz tun und den Quellcode zur Verfügung stellen. Die in der Oberfläche enthaltenen Bibliotheken Dritter (`leaflet` für die Karte, Lizenz BSD-2-Clause, `world-atlas` und `topojson-client`, Lizenz ISC; die Hintergrundkacheln der Karte stammen von Esri und National Geographic, mit ihrer Quellenangabe auf der Karte, und die übrigen in `app/package.json` und `core/Cargo.toml` aufgeführten Abhängigkeiten) behalten ihre eigenen Lizenzen.
