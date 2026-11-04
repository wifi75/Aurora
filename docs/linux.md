# Aurora su Linux

**Italiano** · [English](linux.en.md) · [Deutsch](linux.de.md) · [Français](linux.fr.md) · [Español](linux.es.md) · [Português](linux.pt.md)

Versione **della comunità**: è compilata dallo stesso codice della versione per Windows, quindi ogni novità arriva anche su Linux. Il programma è sviluppato e provato su Windows; la versione Linux la provano i suoi utenti, **senza assistenza ufficiale**.

## Scaricarla già pronta

Ogni release ha due file per Linux a 64 bit (x86_64), preparati in automatico da GitHub:

- [AppImage](https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-x86_64.AppImage): va su quasi tutte le distribuzioni. `chmod +x Aurora-Linux-x86_64.AppImage` e poi `./Aurora-Linux-x86_64.AppImage`.
- [Pacchetto .deb](https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-amd64.deb) per Debian, Ubuntu, Mint: `sudo apt install ./Aurora-Linux-amd64.deb`.

## Compilarla da sé (un comando)

Servono solo `git` e **Node.js 20** o più recente. Rust, se manca, lo installa lo script.

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

## Cosa manca rispetto a Windows

- OmniRig (esiste solo su Windows): per il CAT si usa Hamlib.
- PTT diretto sulle linee RTS/DTR della seriale: per ora solo su Windows.
- Regolazione del volume di registrazione (cursore e AUTO del livello audio): si usa il mixer del sistema.
- Aggiornamento automatico dal programma.
- TQSL per LoTW: va indicato il percorso a mano nelle impostazioni.

## Dove sono i dati

Impostazioni, log e registri in `~/.local/share/Aurora` (o la cartella indicata da `AURORA_DATA_DIR`).

## Problemi

Apri una segnalazione su <https://github.com/wifi75/Aurora/issues> scrivendo nel titolo **[Linux]**, con distribuzione, versione di Aurora e cosa è successo. Le correzioni proposte (pull request) sono benvenute.
