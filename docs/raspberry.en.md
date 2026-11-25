# Aurora on Raspberry Pi (64-bit ARM)

[Italiano](raspberry.md) · **English** · [Deutsch](raspberry.de.md) · [Français](raspberry.fr.md) · [Español](raspberry.es.md) · [Português](raspberry.pt.md)

> **Warning: the ARM version is new (0.11.349).** GitHub builds it automatically, **the author has not yet tested it on a real Raspberry** and there is no official support. If you try it, tell us how it went at <https://github.com/wifi75/Aurora/issues> and send the support package (**Settings › Access and support › Support**).

## What you need

Aurora is a demanding program: it decodes several signals in real time, so it does not run on every Raspberry.

- **Raspberry Pi 5 with 4 GB of RAM or more**: recommended.
- **Raspberry Pi 4 with 4 GB**: may be enough for FT8 and FT4, but with little margin; heavy modes (such as Q65) are uncertain.
- **Pi 3, Zero and older models**: not suitable.
- **64-bit system**: Raspberry Pi OS 64-bit based on Debian 12 «Bookworm» or later (with a desktop), or Ubuntu 22.04 or later, 64-bit. The 32-bit system **does not work**.
- At least 2 GB free on the card or SSD (a USB SSD or a fast card is better).
- System clock in sync (FT8 needs the exact time).
- A USB sound card or the radio's interface, and a connection to the radio (USB/CAT).

To check, in a terminal:

```bash
uname -m
free -h
```

`uname -m` must answer `aarch64`; `free -h` shows the RAM.

## Installing with the .deb package

Open a terminal and copy the commands one at a time.

1. Download the package
   ```bash
   wget https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-arm64.deb
   ```
2. Install it (asks for your password)
   ```bash
   sudo apt install ./Aurora-Linux-arm64.deb
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

## Alternative: AppImage

A single file, nothing to install. It needs the FUSE 2 library: `libfuse2t64` on Debian 13 and Ubuntu 24.04; on Debian 12 «Bookworm» and Ubuntu 22.04 the package is called `libfuse2`.

```bash
sudo apt install libfuse2t64 libhamlib-utils
wget https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-aarch64.AppImage
chmod +x Aurora-Linux-aarch64.AppImage
./Aurora-Linux-aarch64.AppImage
```

## First steps

1. In **Settings** pick the audio input and output (the radio's USB sound card).
2. Enter your callsign and locator.
3. Connect the radio in **Settings › Radio**.
4. Check that the system clock is correct.

## Tips for weaker models (Pi 4)

- Close other programs.
- Do not keep heavy windows open (map, several waterfalls at once).
- Use one mode at a time.
- Use the official power supply and cool the board (fan or heatsink).
- If decoding is late (the support package shows a «slow decoding» warning), report it in the issues.

## Uninstalling

```bash
sudo apt remove aurora-multimode
```

Settings and log stay in `~/.local/share/Aurora`.

For other distributions and common problems see the [general Linux guide](linux.en.md).
