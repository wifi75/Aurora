# Aurora su Linux

**Italiano** · [English](linux.en.md) · [Deutsch](linux.de.md) · [Français](linux.fr.md) · [Español](linux.es.md) · [Português](linux.pt.md)

Versione Linux **non provata dall'autore e senza assistenza ufficiale**. È compilata in automatico da GitHub dallo stesso codice della versione per Windows, quindi ogni novità arriva anche su Linux; l'autore però sviluppa e prova Aurora solo su Windows.

## Scaricarla già pronta

Ogni release ha due file per Linux a 64 bit (x86_64), preparati in automatico da GitHub:

- [AppImage](https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-x86_64.AppImage): va su quasi tutte le distribuzioni. `chmod +x Aurora-Linux-x86_64.AppImage` e poi `./Aurora-Linux-x86_64.AppImage`.
- [Pacchetto .deb](https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-amd64.deb) per Debian, Ubuntu, Mint: `sudo apt install ./Aurora-Linux-amd64.deb`.

## Installazione passo passo

### Ubuntu, Debian, Linux Mint (pacchetto .deb)

Il modo più semplice: il pacchetto `.deb` installa Aurora nel menu delle applicazioni e scarica da solo le librerie che servono. Apri un terminale (Ubuntu: `Ctrl`+`Alt`+`T`) e copia i comandi uno alla volta.

1. Scarica il pacchetto
   ```bash
   wget https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-amd64.deb
   ```
2. Installalo (chiede la tua password)
   ```bash
   sudo apt install ./Aurora-Linux-amd64.deb
   ```
3. Installa Hamlib per il controllo della radio (CAT)
   ```bash
   sudo apt install libhamlib-utils
   ```
4. Dai al tuo utente il permesso di usare la porta seriale, poi **esci dalla sessione e rientra**
   ```bash
   sudo usermod -aG dialout $USER
   ```
5. Avvia Aurora dal menu delle applicazioni (cerca «Aurora»), oppure:
   ```bash
   aurora-desktop
   ```

> Vale per Ubuntu 22.04 e successive, Debian 12 e successive, Linux Mint 21 e successive. Per Ubuntu 20.04 e Debian 11 le librerie sono troppo vecchie: usa una versione più recente della distribuzione.

### AppImage su Ubuntu e Debian

Se preferisci non installare niente, l'AppImage è un unico file che si avvia con un doppio clic. Su Ubuntu e Debian serve la libreria FUSE 2:

```bash
sudo apt install libfuse2t64 libhamlib-utils
wget https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-x86_64.AppImage
chmod +x Aurora-Linux-x86_64.AppImage
./Aurora-Linux-x86_64.AppImage
```

Ubuntu 24.04 e successive, Debian 13: `libfuse2t64`. Ubuntu 22.04, Debian 12: `libfuse2`. Per averla nel menu delle applicazioni, sposta il file in `~/Applicazioni` o usa uno strumento come Gear Lever o AppImageLauncher.

### Fedora

1. Installa le librerie e Hamlib
   ```bash
   sudo dnf install fuse fuse-libs webkit2gtk4.1 libappindicator-gtk3 hamlib
   ```
2. Scarica l'AppImage e rendila eseguibile
   ```bash
   wget https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-x86_64.AppImage
   chmod +x Aurora-Linux-x86_64.AppImage
   ```
3. Dai al tuo utente il permesso di usare la porta seriale, poi **esci dalla sessione e rientra**
   ```bash
   sudo usermod -aG dialout $USER
   ```
4. Avvia Aurora (doppio clic sul file, oppure):
   ```bash
   ./Aurora-Linux-x86_64.AppImage
   ```

### Arch Linux, Manjaro, EndeavourOS

1. Installa le librerie e Hamlib
   ```bash
   sudo pacman -S --needed fuse2 webkit2gtk-4.1 libappindicator-gtk3 hamlib
   ```
2. Scarica l'AppImage e rendila eseguibile
   ```bash
   wget https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-x86_64.AppImage
   chmod +x Aurora-Linux-x86_64.AppImage
   ```
3. Dai al tuo utente il permesso di usare la porta seriale, poi **esci dalla sessione e rientra**
   ```bash
   sudo usermod -aG uucp $USER
   ```
4. Avvia Aurora (doppio clic sul file, oppure):
   ```bash
   ./Aurora-Linux-x86_64.AppImage
   ```

> Su Arch il gruppo della porta seriale si chiama `uucp`, non `dialout`.

## Compilarla da sé (un comando)

Serve solo `git`: lo script installa da solo i pacchetti di sistema, Rust e Node.js se mancano o sono troppo vecchi.

```bash
git clone https://github.com/wifi75/Aurora.git
cd Aurora
./scripts/compila-linux.sh
```

Lo script installa i pacchetti di sistema (chiede la password di `sudo`), compila l'interfaccia e il programma e alla fine scrive dove sono il `.deb` e l'`.AppImage` (in `desktop/src-tauri/target/release/bundle/`). La prima volta ci vogliono 10-20 minuti; le volte dopo molto meno.

Opzioni:

| | |
|---|---|
| `--solo-deb` | prepara solo il `.deb` (più veloce) |
| `--solo-appimage` | prepara solo l'`.AppImage` |
| `--senza-pacchetti` | non installa pacchetti (li hai già, o la tua distribuzione non è tra quelle riconosciute) |

## Aggiornare

Su Linux l'aggiornamento automatico non c'è. Si scarica la nuova AppImage o il nuovo `.deb`, oppure si ricompila:

```bash
cd Aurora
git pull
./scripts/compila-linux.sh --senza-pacchetti
```

## Pacchetti di sistema

Lo script riconosce apt (Debian, Ubuntu, Mint), dnf (Fedora), pacman (Arch, Manjaro) e zypper (openSUSE). Su altre distribuzioni servono gli equivalenti di questi pacchetti Debian, poi si lancia lo script con `--senza-pacchetti`:

```
build-essential curl wget file pkg-config libssl-dev libwebkit2gtk-4.1-dev libgtk-3-dev
libayatana-appindicator3-dev librsvg2-dev libxdo-dev libasound2-dev libudev-dev libhamlib-utils
```

## Radio, audio e permessi

- **CAT**: si usa Hamlib. Installalo dalla distribuzione (`libhamlib-utils` su Debian/Ubuntu, `hamlib` altrove): Aurora trova `rigctld` da sola in `/usr/bin` o `/usr/local/bin`. Per un altro percorso: variabile `AURORA_RIGCTLD`.
- **Porta seriale**: il tuo utente deve stare nel gruppo `dialout` (su Arch `uucp`): `sudo usermod -aG dialout $USER`, poi esci e rientra.
- **PTT**: usa il PTT via CAT (Hamlib).
- **Audio**: scegli la scheda audio della radio nelle impostazioni di Aurora (ALSA, anche tramite PulseAudio o PipeWire).
- **Telefono**: come su Windows, apri `http://<IP-del-PC>:8787` dal telefono; se hai un firewall attivo, apri la porta 8787.

## Primo avvio

1. Aurora parte nella barra delle icone (vicino all'orologio) e apre la sua finestra.
2. In **Impostazioni › Radio** scegli **Hamlib**, il modello della radio e la porta (di solito `/dev/ttyUSB0` o `/dev/ttyACM0`; per vederle: `ls /dev/ttyUSB* /dev/ttyACM*`). PTT: **CAT**.
3. Nelle impostazioni dell'audio scegli la scheda audio della radio, in ingresso e in uscita (di solito «USB Audio CODEC»).
4. Premi **Monitor** e guarda la banda.

## Cosa manca rispetto a Windows

- OmniRig (esiste solo su Windows): per il CAT si usa Hamlib.
- PTT diretto sulle linee RTS/DTR della seriale: per ora solo su Windows.
- Regolazione del volume di registrazione (cursore e AUTO del livello audio): si usa il mixer del sistema.
- Aggiornamento automatico dal programma.
- Cursore **Pwr**: su Windows comanda il volume della scheda audio; su Linux abbassa il segnale generato (come JTDX). Il volume dell'uscita nel mixer del sistema si regola una volta.
- TQSL per LoTW: va indicato il percorso a mano nelle impostazioni.

## Dove sono i dati

Impostazioni, log e registri in `~/.local/share/Aurora` (o la cartella indicata da `AURORA_DATA_DIR`).

## Disinstallare

Pacchetto .deb:

```bash
sudo apt remove aurora
```

AppImage: basta cancellare il file. Le impostazioni e il log restano in `~/.local/share/Aurora`: cancella anche quella cartella se vuoi togliere tutto (prima fai una copia del log).

## Problemi comuni

**L'AppImage non parte e parla di FUSE**  
Installa FUSE 2: `sudo apt install libfuse2t64` (o `libfuse2` sulle versioni meno recenti), poi riprova.

**Non vedo l'icona nella barra (GNOME, Ubuntu)**  
Su GNOME serve l'estensione AppIndicator: `sudo apt install gnome-shell-extension-appindicator`, poi esci e rientra.

**«Permission denied» sulla porta seriale**  
Il tuo utente non è nel gruppo `dialout` (Arch: `uucp`): `sudo usermod -aG dialout $USER`, poi esci e rientra. Per controllare: `groups`.

**Aurora dice che manca rigctld**  
Installa Hamlib: `sudo apt install libhamlib-utils` (Fedora e Arch: `hamlib`). Se è in un'altra cartella, indica il percorso con la variabile `AURORA_RIGCTLD`.

**La finestra è bianca o vuota**  
Succede con alcune schede video: avvia con `WEBKIT_DISABLE_DMABUF_RENDERER=1 aurora-desktop`.

**Il telefono non si collega**  
Controlla che PC e telefono siano sulla stessa rete e apri la porta: `sudo ufw allow 8787/tcp` (Fedora: `sudo firewall-cmd --add-port=8787/tcp --permanent && sudo firewall-cmd --reload`).

**Non sento o non vedo segnali**  
Controlla in `pavucontrol` (o nelle impostazioni dell'audio del sistema) che la scheda della radio non sia muta e che il livello di registrazione non sia a zero.

## Segnalare un problema

Apri una segnalazione su <https://github.com/wifi75/Aurora/issues> scrivendo nel titolo **[Linux]**, con distribuzione, versione di Aurora e cosa è successo. Le correzioni proposte (pull request) sono benvenute.
