# Aurora on Linux

[Italiano](linux.md) · **English** · [Deutsch](linux.de.md) · [Français](linux.fr.md) · [Español](linux.es.md) · [Português](linux.pt.md)

Linux version **not tested by the author, without official support**. GitHub builds it automatically from the same code as the Windows version, so every new feature reaches Linux too; the author, however, develops and tests Aurora only on Windows.

## Download it ready-made

Every release has two files for 64-bit Linux (x86_64), built automatically by GitHub:

- [AppImage](https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-x86_64.AppImage): runs on almost every distribution. `chmod +x Aurora-Linux-x86_64.AppImage`, then `./Aurora-Linux-x86_64.AppImage`.
- [.deb package](https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-amd64.deb) for Debian, Ubuntu, Mint: `sudo apt install ./Aurora-Linux-amd64.deb`.

## Step-by-step installation

### Ubuntu, Debian, Linux Mint (.deb package)

The easiest way: the `.deb` package puts Aurora in the applications menu and fetches the libraries it needs. Open a terminal (Ubuntu: `Ctrl`+`Alt`+`T`) and copy the commands one at a time.

1. Download the package
   ```bash
   wget https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-amd64.deb
   ```
2. Install it (asks for your password)
   ```bash
   sudo apt install ./Aurora-Linux-amd64.deb
   ```
3. Install Hamlib for radio control (CAT)
   ```bash
   sudo apt install libhamlib-utils
   ```
4. Give your user permission to use the serial port, then **log out and back in**
   ```bash
   sudo usermod -aG dialout $USER
   ```
5. Start Aurora from the applications menu (search for «Aurora»), or:
   ```bash
   aurora-desktop
   ```

> Works on Ubuntu 22.04 and later, Debian 12 and later, Linux Mint 21 and later. On Ubuntu 20.04 and Debian 11 the libraries are too old: use a newer release of the distribution.

### AppImage on Ubuntu and Debian

If you prefer not to install anything, the AppImage is a single file you start with a double click. Ubuntu and Debian need the FUSE 2 library:

```bash
sudo apt install libfuse2t64 libhamlib-utils
wget https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-x86_64.AppImage
chmod +x Aurora-Linux-x86_64.AppImage
./Aurora-Linux-x86_64.AppImage
```

Ubuntu 24.04 and later, Debian 13: `libfuse2t64`. Ubuntu 22.04, Debian 12: `libfuse2`. To have it in the applications menu, move the file to `~/Applications` or use a tool such as Gear Lever or AppImageLauncher.

### Fedora

1. Install the libraries and Hamlib
   ```bash
   sudo dnf install fuse fuse-libs webkit2gtk4.1 libappindicator-gtk3 hamlib
   ```
2. Download the AppImage and make it executable
   ```bash
   wget https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-x86_64.AppImage
   chmod +x Aurora-Linux-x86_64.AppImage
   ```
3. Give your user permission to use the serial port, then **log out and back in**
   ```bash
   sudo usermod -aG dialout $USER
   ```
4. Start Aurora (double-click the file, or):
   ```bash
   ./Aurora-Linux-x86_64.AppImage
   ```

### Arch Linux, Manjaro, EndeavourOS

1. Install the libraries and Hamlib
   ```bash
   sudo pacman -S --needed fuse2 webkit2gtk-4.1 libappindicator-gtk3 hamlib
   ```
2. Download the AppImage and make it executable
   ```bash
   wget https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-x86_64.AppImage
   chmod +x Aurora-Linux-x86_64.AppImage
   ```
3. Give your user permission to use the serial port, then **log out and back in**
   ```bash
   sudo usermod -aG uucp $USER
   ```
4. Start Aurora (double-click the file, or):
   ```bash
   ./Aurora-Linux-x86_64.AppImage
   ```

> On Arch the serial port group is called `uucp`, not `dialout`.

## Build it yourself (one command)

You only need `git`: the script installs the system packages, Rust and Node.js by itself if they are missing or too old.

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

## First start

1. Aurora starts in the system tray (near the clock) and opens its window.
2. In **Settings › Radio** pick **Hamlib**, your radio model and the port (usually `/dev/ttyUSB0` or `/dev/ttyACM0`; to list them: `ls /dev/ttyUSB* /dev/ttyACM*`). PTT: **CAT**.
3. In the audio settings pick the radio's sound card for input and output (usually «USB Audio CODEC»).
4. Press **Monitor** and watch the band.

## What is missing compared with Windows

- OmniRig (Windows only): CAT goes through Hamlib.
- Direct PTT on the serial RTS/DTR lines: Windows only for now.
- Recording volume control (audio level slider and AUTO): use the system mixer.
- Automatic update from the program.
- **Pwr** slider: on Windows it sets the sound card volume; on Linux it lowers the generated signal (like JTDX). Set the output volume in the system mixer once.
- TQSL for LoTW: set its path by hand in the settings.

## Where the data lives

Settings, log and records in `~/.local/share/Aurora` (or the folder set in `AURORA_DATA_DIR`).

## Uninstalling

.deb package:

```bash
sudo apt remove aurora
```

AppImage: just delete the file. Settings and log stay in `~/.local/share/Aurora`: delete that folder too if you want to remove everything (back up your log first).

## Common problems

**The AppImage does not start and mentions FUSE**  
Install FUSE 2: `sudo apt install libfuse2t64` (or `libfuse2` on older releases), then try again.

**I can't see the tray icon (GNOME, Ubuntu)**  
GNOME needs the AppIndicator extension: `sudo apt install gnome-shell-extension-appindicator`, then log out and back in.

**«Permission denied» on the serial port**  
Your user is not in the `dialout` group (Arch: `uucp`): `sudo usermod -aG dialout $USER`, then log out and back in. To check: `groups`.

**Aurora says rigctld is missing**  
Install Hamlib: `sudo apt install libhamlib-utils` (Fedora and Arch: `hamlib`). If it lives elsewhere, set its path in the `AURORA_RIGCTLD` variable.

**The window is white or empty**  
It happens with some graphics cards: start with `WEBKIT_DISABLE_DMABUF_RENDERER=1 aurora-desktop`.

**The phone does not connect**  
Check that PC and phone are on the same network and open the port: `sudo ufw allow 8787/tcp` (Fedora: `sudo firewall-cmd --add-port=8787/tcp --permanent && sudo firewall-cmd --reload`).

**I hear or see no signals**  
Check in `pavucontrol` (or the system sound settings) that the radio's card is not muted and the recording level is not at zero.

## Reporting a problem

Open an issue at <https://github.com/wifi75/Aurora/issues> with **[Linux]** in the title, your distribution, the Aurora version and what happened. Proposed fixes (pull requests) are welcome.
