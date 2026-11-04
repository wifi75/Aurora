# Aurora unter Linux

[Italiano](linux.md) · [English](linux.en.md) · **Deutsch** · [Français](linux.fr.md) · [Español](linux.es.md) · [Português](linux.pt.md)

Linux-Version, **vom Autor nicht getestet und ohne offiziellen Support**. GitHub baut sie automatisch aus demselben Code wie die Windows-Version, daher kommt jede Neuerung auch unter Linux an; der Autor entwickelt und testet Aurora jedoch nur unter Windows.

## Fertig herunterladen

Jede Release enthält zwei Dateien für 64-Bit-Linux (x86_64), automatisch von GitHub gebaut:

- [AppImage](https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-x86_64.AppImage): läuft auf fast jeder Distribution. `chmod +x Aurora-Linux-x86_64.AppImage`, dann `./Aurora-Linux-x86_64.AppImage`.
- [.deb-Paket](https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-amd64.deb) für Debian, Ubuntu, Mint: `sudo apt install ./Aurora-Linux-amd64.deb`.

## Installation Schritt für Schritt

### Ubuntu, Debian, Linux Mint (.deb-Paket)

Der einfachste Weg: Das `.deb`-Paket legt Aurora ins Anwendungsmenü und holt die nötigen Bibliotheken selbst. Öffne ein Terminal (Ubuntu: `Strg`+`Alt`+`T`) und kopiere die Befehle einzeln.

1. Paket herunterladen
   ```bash
   wget https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-amd64.deb
   ```
2. Installieren (fragt nach deinem Passwort)
   ```bash
   sudo apt install ./Aurora-Linux-amd64.deb
   ```
3. Hamlib für die Funkgerätesteuerung (CAT) installieren
   ```bash
   sudo apt install libhamlib-utils
   ```
4. Deinem Benutzer die serielle Schnittstelle erlauben, dann **ab- und wieder anmelden**
   ```bash
   sudo usermod -aG dialout $USER
   ```
5. Aurora aus dem Anwendungsmenü starten (nach «Aurora» suchen) oder:
   ```bash
   aurora-desktop
   ```

> Gilt für Ubuntu 22.04 und neuer, Debian 12 und neuer, Linux Mint 21 und neuer. Unter Ubuntu 20.04 und Debian 11 sind die Bibliotheken zu alt: eine neuere Version der Distribution verwenden.

### AppImage unter Ubuntu und Debian

Wenn du nichts installieren willst: Das AppImage ist eine einzige Datei, die per Doppelklick startet. Ubuntu und Debian brauchen dafür die Bibliothek FUSE 2:

```bash
sudo apt install libfuse2t64 libhamlib-utils
wget https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-x86_64.AppImage
chmod +x Aurora-Linux-x86_64.AppImage
./Aurora-Linux-x86_64.AppImage
```

Ubuntu 24.04 und neuer, Debian 13: `libfuse2t64`. Ubuntu 22.04, Debian 12: `libfuse2`. Damit es im Anwendungsmenü erscheint, die Datei nach `~/Applications` verschieben oder ein Werkzeug wie Gear Lever oder AppImageLauncher verwenden.

### Fedora

1. Bibliotheken und Hamlib installieren
   ```bash
   sudo dnf install fuse fuse-libs webkit2gtk4.1 libappindicator-gtk3 hamlib
   ```
2. AppImage herunterladen und ausführbar machen
   ```bash
   wget https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-x86_64.AppImage
   chmod +x Aurora-Linux-x86_64.AppImage
   ```
3. Deinem Benutzer die serielle Schnittstelle erlauben, dann **ab- und wieder anmelden**
   ```bash
   sudo usermod -aG dialout $USER
   ```
4. Aurora starten (Doppelklick auf die Datei oder):
   ```bash
   ./Aurora-Linux-x86_64.AppImage
   ```

### Arch Linux, Manjaro, EndeavourOS

1. Bibliotheken und Hamlib installieren
   ```bash
   sudo pacman -S --needed fuse2 webkit2gtk-4.1 libappindicator-gtk3 hamlib
   ```
2. AppImage herunterladen und ausführbar machen
   ```bash
   wget https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-x86_64.AppImage
   chmod +x Aurora-Linux-x86_64.AppImage
   ```
3. Deinem Benutzer die serielle Schnittstelle erlauben, dann **ab- und wieder anmelden**
   ```bash
   sudo usermod -aG uucp $USER
   ```
4. Aurora starten (Doppelklick auf die Datei oder):
   ```bash
   ./Aurora-Linux-x86_64.AppImage
   ```

> Unter Arch heißt die Gruppe der seriellen Schnittstelle `uucp`, nicht `dialout`.

## Selbst bauen (ein Befehl)

Du brauchst nur `git`: Das Skript installiert Systempakete, Rust und Node.js selbst, wenn sie fehlen oder zu alt sind.

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

## Erster Start

1. Aurora startet im Infobereich (neben der Uhr) und öffnet sein Fenster.
2. Unter **Einstellungen › Funkgerät** **Hamlib**, das Modell und die Schnittstelle wählen (meist `/dev/ttyUSB0` oder `/dev/ttyACM0`; anzeigen mit `ls /dev/ttyUSB* /dev/ttyACM*`). PTT: **CAT**.
3. In den Audio-Einstellungen die Soundkarte des Funkgeräts für Ein- und Ausgang wählen (meist «USB Audio CODEC»).
4. **Monitor** drücken und das Band beobachten.

## Was im Vergleich zu Windows fehlt

- OmniRig (nur Windows): CAT läuft über Hamlib.
- Direktes PTT über die RTS/DTR-Leitungen der seriellen Schnittstelle: vorerst nur Windows.
- Regelung der Aufnahmelautstärke (Pegelregler und AUTO): den Mixer des Systems verwenden.
- Automatische Aktualisierung aus dem Programm.
- TQSL für LoTW: Pfad in den Einstellungen von Hand angeben.

## Wo die Daten liegen

Einstellungen, Log und Aufzeichnungen in `~/.local/share/Aurora` (oder im Ordner aus `AURORA_DATA_DIR`).

## Deinstallieren

.deb-Paket:

```bash
sudo apt remove aurora
```

AppImage: einfach die Datei löschen. Einstellungen und Log bleiben in `~/.local/share/Aurora`: auch diesen Ordner löschen, wenn alles weg soll (vorher das Log sichern).

## Häufige Probleme

**Das AppImage startet nicht und erwähnt FUSE**  
FUSE 2 installieren: `sudo apt install libfuse2t64` (oder `libfuse2` bei älteren Versionen), dann erneut versuchen.

**Ich sehe das Symbol im Infobereich nicht (GNOME, Ubuntu)**  
GNOME braucht die Erweiterung AppIndicator: `sudo apt install gnome-shell-extension-appindicator`, dann ab- und wieder anmelden.

**«Permission denied» an der seriellen Schnittstelle**  
Dein Benutzer ist nicht in der Gruppe `dialout` (Arch: `uucp`): `sudo usermod -aG dialout $USER`, dann ab- und wieder anmelden. Prüfen mit `groups`.

**Aurora meldet, dass rigctld fehlt**  
Hamlib installieren: `sudo apt install libhamlib-utils` (Fedora und Arch: `hamlib`). Liegt es woanders, den Pfad in der Variablen `AURORA_RIGCTLD` angeben.

**Das Fenster ist weiß oder leer**  
Kommt bei manchen Grafikkarten vor: mit `WEBKIT_DISABLE_DMABUF_RENDERER=1 aurora-desktop` starten.

**Das Telefon verbindet sich nicht**  
Prüfen, ob PC und Telefon im selben Netz sind, und den Port öffnen: `sudo ufw allow 8787/tcp` (Fedora: `sudo firewall-cmd --add-port=8787/tcp --permanent && sudo firewall-cmd --reload`).

**Ich höre oder sehe keine Signale**  
In `pavucontrol` (oder den Audio-Einstellungen des Systems) prüfen, dass die Karte des Funkgeräts nicht stumm ist und der Aufnahmepegel nicht auf null steht.

## Ein Problem melden

Eröffne ein Issue unter <https://github.com/wifi75/Aurora/issues> mit **[Linux]** im Titel, Distribution, Aurora-Version und was passiert ist. Korrekturvorschläge (Pull Requests) sind willkommen.
