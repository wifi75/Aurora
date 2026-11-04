# Aurora unter Linux

[Italiano](linux.md) · [English](linux.en.md) · **Deutsch** · [Français](linux.fr.md) · [Español](linux.es.md) · [Português](linux.pt.md)

**Community**-Version: Sie wird aus demselben Code gebaut wie die Windows-Version, daher kommt jede Neuerung auch unter Linux an. Das Programm wird unter Windows entwickelt und getestet; die Linux-Version testen ihre Nutzer, **ohne offiziellen Support**.

## Fertig herunterladen

Jede Release enthält zwei Dateien für 64-Bit-Linux (x86_64), automatisch von GitHub gebaut:

- [AppImage](https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-x86_64.AppImage): läuft auf fast jeder Distribution. `chmod +x Aurora-Linux-x86_64.AppImage`, dann `./Aurora-Linux-x86_64.AppImage`.
- [.deb-Paket](https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-amd64.deb) für Debian, Ubuntu, Mint: `sudo apt install ./Aurora-Linux-amd64.deb`.

## Selbst bauen (ein Befehl)

Du brauchst nur `git` und **Node.js 20** oder neuer. Fehlt Rust, installiert es das Skript.

```bash
git clone https://github.com/wifi75/Aurora.git
cd Aurora
./scripts/compila-linux.sh
```

Das Skript installiert die Systempakete (es fragt nach dem `sudo`-Passwort), baut Oberfläche und Programm und zeigt am Ende, wo `.deb` und `.AppImage` liegen (in `desktop/src-tauri/target/release/bundle/`). Beim ersten Mal dauert es 10-20 Minuten, danach deutlich weniger.

Optionen:

| | |
|---|---|
| `--solo-deb` | baut nur das `.deb` (schneller) |
| `--solo-appimage` | baut nur das `.AppImage` |
| `--senza-pacchetti` | installiert keine Pakete (du hast sie schon, oder deine Distribution wird nicht erkannt) |

## Aktualisieren

Unter Linux gibt es keine automatische Aktualisierung. Lade das neue AppImage oder `.deb` herunter oder baue neu:

```bash
cd Aurora
git pull
./scripts/compila-linux.sh --senza-pacchetti
```

## Systempakete

Das Skript erkennt apt (Debian, Ubuntu, Mint), dnf (Fedora), pacman (Arch, Manjaro) und zypper (openSUSE). Auf anderen Distributionen installierst du die Entsprechungen dieser Debian-Pakete und startest das Skript mit `--senza-pacchetti`:

```
build-essential curl wget file pkg-config libssl-dev libwebkit2gtk-4.1-dev libgtk-3-dev
libayatana-appindicator3-dev librsvg2-dev libxdo-dev libasound2-dev libudev-dev libhamlib-utils
```

## Funkgerät, Audio und Rechte

- **CAT**: Es wird Hamlib verwendet. Installiere es aus deiner Distribution (`libhamlib-utils` unter Debian/Ubuntu, sonst `hamlib`): Aurora findet `rigctld` selbst in `/usr/bin` oder `/usr/local/bin`. Für einen anderen Pfad: Variable `AURORA_RIGCTLD`.
- **Serielle Schnittstelle**: Dein Benutzer muss in der Gruppe `dialout` sein (unter Arch `uucp`): `sudo usermod -aG dialout $USER`, dann ab- und wieder anmelden.
- **PTT**: PTT über CAT (Hamlib) verwenden.
- **Audio**: Wähle die Soundkarte des Funkgeräts in den Einstellungen von Aurora (ALSA, auch über PulseAudio oder PipeWire).
- **Telefon**: Wie unter Windows `http://<IP-des-PCs>:8787` im Telefon öffnen; bei aktiver Firewall Port 8787 freigeben.

## Was im Vergleich zu Windows fehlt

- OmniRig (nur Windows): CAT läuft über Hamlib.
- Direktes PTT über die RTS/DTR-Leitungen der seriellen Schnittstelle: vorerst nur Windows.
- Regelung der Aufnahmelautstärke (Pegelregler und AUTO): den Mixer des Systems verwenden.
- Automatische Aktualisierung aus dem Programm.
- TQSL für LoTW: Pfad in den Einstellungen von Hand angeben.

## Wo die Daten liegen

Einstellungen, Log und Aufzeichnungen in `~/.local/share/Aurora` (oder im Ordner aus `AURORA_DATA_DIR`).

## Probleme

Eröffne ein Issue unter <https://github.com/wifi75/Aurora/issues> mit **[Linux]** im Titel, Distribution, Aurora-Version und was passiert ist. Korrekturvorschläge (Pull Requests) sind willkommen.
