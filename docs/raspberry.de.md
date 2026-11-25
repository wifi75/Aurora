# Aurora auf dem Raspberry Pi (64-Bit-ARM)

[Italiano](raspberry.md) · [English](raspberry.en.md) · **Deutsch** · [Français](raspberry.fr.md) · [Español](raspberry.es.md) · [Português](raspberry.pt.md)

> **Achtung: Die ARM-Version ist neu (0.11.349).** Sie wird automatisch von GitHub gebaut, **der Autor hat sie noch nicht auf einem echten Raspberry getestet**, und es gibt keinen offiziellen Support. Wenn du sie ausprobierst, schreib uns unter <https://github.com/wifi75/Aurora/issues>, wie es lief, und schick das Support-Paket (**Einstellungen › Zugang und Support › Support**).

## Was du brauchst

Aurora ist ein anspruchsvolles Programm: Es dekodiert mehrere Signale in Echtzeit und läuft deshalb nicht auf jedem Raspberry.

- **Raspberry Pi 5 mit 4 GB RAM oder mehr**: empfohlen.
- **Raspberry Pi 4 mit 4 GB**: kann für FT8 und FT4 reichen, aber mit wenig Reserve; schwere Modi (etwa Q65) sind unsicher.
- **Pi 3, Zero und ältere Modelle**: nicht geeignet.
- **64-Bit-System**: Raspberry Pi OS 64-Bit auf Basis von Debian 12 «Bookworm» oder neuer (mit Desktop) oder Ubuntu 22.04 oder neuer, 64-Bit. Das 32-Bit-System **funktioniert nicht**.
- Mindestens 2 GB frei auf der Karte oder SSD (besser eine USB-SSD oder eine schnelle Karte).
- Synchronisierte Systemuhr (FT8 braucht die genaue Uhrzeit).
- Eine USB-Soundkarte oder die Schnittstelle des Funkgeräts und eine Verbindung zum Funkgerät (USB/CAT).

Zum Prüfen im Terminal:

```bash
uname -m
free -h
```

`uname -m` muss `aarch64` antworten; `free -h` zeigt den RAM.

## Installation mit dem .deb-Paket

Öffne ein Terminal und kopiere die Befehle einzeln.

1. Paket herunterladen
   ```bash
   wget https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-arm64.deb
   ```
2. Installieren (fragt nach deinem Passwort)
   ```bash
   sudo apt install ./Aurora-Linux-arm64.deb
   ```
3. Hamlib für die Funkgerätesteuerung (CAT) installieren
   ```bash
   sudo apt install libhamlib-utils
   ```
4. Gib deinem Benutzer das Recht, die serielle Schnittstelle zu nutzen, und **melde dich ab und wieder an**
   ```bash
   sudo usermod -aG dialout $USER
   ```
5. Starte Aurora über das Anwendungsmenü (nach «Aurora» suchen) oder:
   ```bash
   aurora-desktop
   ```

## Alternative: AppImage

Eine einzelne Datei, nichts zu installieren. Sie braucht die Bibliothek FUSE 2: `libfuse2t64` unter Debian 13 und Ubuntu 24.04; unter Debian 12 «Bookworm» und Ubuntu 22.04 heißt das Paket `libfuse2`.

```bash
sudo apt install libfuse2t64 libhamlib-utils
wget https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-aarch64.AppImage
chmod +x Aurora-Linux-aarch64.AppImage
./Aurora-Linux-aarch64.AppImage
```

## Erste Schritte

1. Wähle in den **Einstellungen** Audio-Eingang und -Ausgang (die USB-Soundkarte des Funkgeräts).
2. Trage dein Rufzeichen und deinen Locator ein.
3. Verbinde das Funkgerät unter **Einstellungen › Funkgerät**.
4. Prüfe, ob die Systemuhr stimmt.

## Tipps für schwächere Modelle (Pi 4)

- Schließe andere Programme.
- Lass keine schweren Fenster offen (Karte, mehrere Wasserfälle gleichzeitig).
- Nutze jeweils nur einen Modus.
- Nutze das offizielle Netzteil und kühle die Platine (Lüfter oder Kühlkörper).
- Wenn die Dekodierung verspätet ist (im Support-Paket erscheint die Warnung «langsame Dekodierung»), melde es in den Issues.

## Deinstallieren

```bash
sudo apt remove aurora-multimode
```

Einstellungen und Log bleiben in `~/.local/share/Aurora`.

Für andere Distributionen und häufige Probleme siehe die [allgemeine Linux-Anleitung](linux.de.md).
