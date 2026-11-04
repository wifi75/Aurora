# Aurora en Linux

[Italiano](linux.md) · [English](linux.en.md) · [Deutsch](linux.de.md) · [Français](linux.fr.md) · **Español** · [Português](linux.pt.md)

Versión **de la comunidad**: se compila con el mismo código que la versión para Windows, así que cada novedad llega también a Linux. El programa se desarrolla y prueba en Windows; la versión para Linux la prueban sus usuarios, **sin soporte oficial**.

## Descargarla ya lista

Cada release tiene dos archivos para Linux de 64 bits (x86_64), preparados automáticamente por GitHub:

- [AppImage](https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-x86_64.AppImage): funciona en casi todas las distribuciones. `chmod +x Aurora-Linux-x86_64.AppImage` y luego `./Aurora-Linux-x86_64.AppImage`.
- [Paquete .deb](https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-amd64.deb) para Debian, Ubuntu, Mint: `sudo apt install ./Aurora-Linux-amd64.deb`.

## Compilarla uno mismo (un comando)

Solo necesitas `git` y **Node.js 20** o más reciente. Si falta Rust, el script lo instala.

```bash
git clone https://github.com/wifi75/Aurora.git
cd Aurora
./scripts/compila-linux.sh
```

El script instala los paquetes del sistema (pide la contraseña de `sudo`), compila la interfaz y el programa y al final indica dónde están el `.deb` y el `.AppImage` (en `desktop/src-tauri/target/release/bundle/`). La primera vez tarda 10-20 minutos; las siguientes, mucho menos.

Opciones:

| | |
|---|---|
| `--solo-deb` | prepara solo el `.deb` (más rápido) |
| `--solo-appimage` | prepara solo el `.AppImage` |
| `--senza-pacchetti` | no instala paquetes (ya los tienes, o tu distribución no está entre las reconocidas) |

## Actualizar

En Linux no hay actualización automática. Descarga el nuevo AppImage o el nuevo `.deb`, o vuelve a compilar:

```bash
cd Aurora
git pull
./scripts/compila-linux.sh --senza-pacchetti
```

## Paquetes del sistema

El script reconoce apt (Debian, Ubuntu, Mint), dnf (Fedora), pacman (Arch, Manjaro) y zypper (openSUSE). En otras distribuciones instala los equivalentes de estos paquetes de Debian y lanza el script con `--senza-pacchetti`:

```
build-essential curl wget file pkg-config libssl-dev libwebkit2gtk-4.1-dev libgtk-3-dev
libayatana-appindicator3-dev librsvg2-dev libxdo-dev libasound2-dev libudev-dev libhamlib-utils
```

## Radio, audio y permisos

- **CAT**: se usa Hamlib. Instálalo desde tu distribución (`libhamlib-utils` en Debian/Ubuntu, `hamlib` en las demás): Aurora encuentra `rigctld` por sí sola en `/usr/bin` o `/usr/local/bin`. Para otra ruta: variable `AURORA_RIGCTLD`.
- **Puerto serie**: tu usuario debe estar en el grupo `dialout` (`uucp` en Arch): `sudo usermod -aG dialout $USER`, luego cierra la sesión y vuelve a entrar.
- **PTT**: usa el PTT por CAT (Hamlib).
- **Audio**: elige la tarjeta de sonido de la radio en los ajustes de Aurora (ALSA, también a través de PulseAudio o PipeWire).
- **Teléfono**: como en Windows, abre `http://<IP-del-PC>:8787` en el teléfono; si tienes un cortafuegos activo, abre el puerto 8787.

## Qué falta respecto a Windows

- OmniRig (solo Windows): el CAT va por Hamlib.
- PTT directo en las líneas RTS/DTR del puerto serie: por ahora solo en Windows.
- Control del volumen de grabación (deslizador y AUTO del nivel de audio): usa el mezclador del sistema.
- Actualización automática desde el programa.
- TQSL para LoTW: indica su ruta a mano en los ajustes.

## Dónde están los datos

Ajustes, log y registros en `~/.local/share/Aurora` (o la carpeta indicada en `AURORA_DATA_DIR`).

## Problemas

Abre un aviso en <https://github.com/wifi75/Aurora/issues> con **[Linux]** en el título, tu distribución, la versión de Aurora y lo que pasó. Las correcciones propuestas (pull requests) son bienvenidas.
