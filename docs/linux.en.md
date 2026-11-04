# Aurora on Linux

[Italiano](linux.md) · **English** · [Deutsch](linux.de.md) · [Français](linux.fr.md) · [Español](linux.es.md) · [Português](linux.pt.md)

**Community** version: it is built from the same code as the Windows version, so every new feature reaches Linux too. The program is developed and tested on Windows; the Linux version is tested by its users, **without official support**.

## Download it ready-made

Every release has two files for 64-bit Linux (x86_64), built automatically by GitHub:

- [AppImage](https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-x86_64.AppImage): runs on almost every distribution. `chmod +x Aurora-Linux-x86_64.AppImage`, then `./Aurora-Linux-x86_64.AppImage`.
- [.deb package](https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-amd64.deb) for Debian, Ubuntu, Mint: `sudo apt install ./Aurora-Linux-amd64.deb`.

## Build it yourself (one command)

You only need `git` and **Node.js 20** or newer. If Rust is missing, the script installs it.

```bash
git clone https://github.com/wifi75/Aurora.git
cd Aurora
./scripts/compila-linux.sh
```

The script installs the system packages (it asks for your `sudo` password), builds the interface and the program, and at the end shows where the `.deb` and `.AppImage` are (in `desktop/src-tauri/target/release/bundle/`). The first time takes 10-20 minutes; later builds are much faster.

Options:

| | |
|---|---|
| `--solo-deb` | builds only the `.deb` (faster) |
| `--solo-appimage` | builds only the `.AppImage` |
| `--senza-pacchetti` | installs no packages (you already have them, or your distribution isn't one of those recognised) |

## Updating

There is no automatic update on Linux. Download the new AppImage or `.deb`, or rebuild:

```bash
cd Aurora
git pull
./scripts/compila-linux.sh --senza-pacchetti
```

## System packages

The script recognises apt (Debian, Ubuntu, Mint), dnf (Fedora), pacman (Arch, Manjaro) and zypper (openSUSE). On other distributions install the equivalents of these Debian packages, then run the script with `--senza-pacchetti`:

```
build-essential curl wget file pkg-config libssl-dev libwebkit2gtk-4.1-dev libgtk-3-dev
libayatana-appindicator3-dev librsvg2-dev libxdo-dev libasound2-dev libudev-dev libhamlib-utils
```

## Radio, audio and permissions

- **CAT**: Hamlib is used. Install it from your distribution (`libhamlib-utils` on Debian/Ubuntu, `hamlib` elsewhere): Aurora finds `rigctld` by itself in `/usr/bin` or `/usr/local/bin`. For another path: the `AURORA_RIGCTLD` variable.
- **Serial port**: your user must be in the `dialout` group (`uucp` on Arch): `sudo usermod -aG dialout $USER`, then log out and back in.
- **PTT**: use PTT via CAT (Hamlib).
- **Audio**: pick the radio's sound card in Aurora's settings (ALSA, also through PulseAudio or PipeWire).
- **Phone**: as on Windows, open `http://<PC-IP>:8787` on the phone; if a firewall is active, open port 8787.

## What is missing compared with Windows

- OmniRig (Windows only): CAT goes through Hamlib.
- Direct PTT on the serial RTS/DTR lines: Windows only for now.
- Recording volume control (audio level slider and AUTO): use the system mixer.
- Automatic update from the program.
- TQSL for LoTW: set its path by hand in the settings.

## Where the data lives

Settings, log and records in `~/.local/share/Aurora` (or the folder set in `AURORA_DATA_DIR`).

## Problems

Open an issue at <https://github.com/wifi75/Aurora/issues> with **[Linux]** in the title, your distribution, the Aurora version and what happened. Proposed fixes (pull requests) are welcome.
