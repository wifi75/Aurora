# Aurora en Linux

[Italiano](linux.md) · [English](linux.en.md) · [Deutsch](linux.de.md) · [Français](linux.fr.md) · **Español** · [Português](linux.pt.md)

Versión Linux **no probada por el autor y sin soporte oficial**. GitHub la compila automáticamente con el mismo código que la versión para Windows, así que cada novedad llega también a Linux; el autor, sin embargo, desarrolla y prueba Aurora solo en Windows.

## Descargarla ya lista

Cada release tiene dos archivos para Linux de 64 bits (x86_64), preparados automáticamente por GitHub:

- [AppImage](https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-x86_64.AppImage): funciona en casi todas las distribuciones. `chmod +x Aurora-Linux-x86_64.AppImage` y luego `./Aurora-Linux-x86_64.AppImage`.
- [Paquete .deb](https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-amd64.deb) para Debian, Ubuntu, Mint: `sudo apt install ./Aurora-Linux-amd64.deb`.

## Instalación paso a paso

### Ubuntu, Debian, Linux Mint (paquete .deb)

La forma más sencilla: el paquete `.deb` pone Aurora en el menú de aplicaciones y descarga solo las bibliotecas necesarias. Abre una terminal (Ubuntu: `Ctrl`+`Alt`+`T`) y copia los comandos de uno en uno.

1. Descarga el paquete
   ```bash
   wget https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-amd64.deb
   ```
2. Instálalo (pide tu contraseña)
   ```bash
   sudo apt install ./Aurora-Linux-amd64.deb
   ```
3. Instala Hamlib para el control de la radio (CAT)
   ```bash
   sudo apt install libhamlib-utils
   ```
4. Da a tu usuario permiso para usar el puerto serie y luego **cierra la sesión y vuelve a entrar**
   ```bash
   sudo usermod -aG dialout $USER
   ```
5. Inicia Aurora desde el menú de aplicaciones (busca «Aurora»), o:
   ```bash
   aurora-desktop
   ```

> Vale para Ubuntu 22.04 y posteriores, Debian 12 y posteriores, Linux Mint 21 y posteriores. En Ubuntu 20.04 y Debian 11 las bibliotecas son demasiado antiguas: usa una versión más reciente de la distribución.

### AppImage en Ubuntu y Debian

Si prefieres no instalar nada, el AppImage es un único archivo que se abre con doble clic. Ubuntu y Debian necesitan la biblioteca FUSE 2:

```bash
sudo apt install libfuse2t64 libhamlib-utils
wget https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-x86_64.AppImage
chmod +x Aurora-Linux-x86_64.AppImage
./Aurora-Linux-x86_64.AppImage
```

Ubuntu 24.04 y posteriores, Debian 13: `libfuse2t64`. Ubuntu 22.04, Debian 12: `libfuse2`. Para tenerlo en el menú de aplicaciones, mueve el archivo a `~/Applications` o usa una herramienta como Gear Lever o AppImageLauncher.

### Fedora

1. Instala las bibliotecas y Hamlib
   ```bash
   sudo dnf install fuse fuse-libs webkit2gtk4.1 libappindicator-gtk3 hamlib
   ```
2. Descarga el AppImage y hazlo ejecutable
   ```bash
   wget https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-x86_64.AppImage
   chmod +x Aurora-Linux-x86_64.AppImage
   ```
3. Da a tu usuario permiso para usar el puerto serie y luego **cierra la sesión y vuelve a entrar**
   ```bash
   sudo usermod -aG dialout $USER
   ```
4. Inicia Aurora (doble clic en el archivo, o):
   ```bash
   ./Aurora-Linux-x86_64.AppImage
   ```

### Arch Linux, Manjaro, EndeavourOS

1. Instala las bibliotecas y Hamlib
   ```bash
   sudo pacman -S --needed fuse2 webkit2gtk-4.1 libappindicator-gtk3 hamlib
   ```
2. Descarga el AppImage y hazlo ejecutable
   ```bash
   wget https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-x86_64.AppImage
   chmod +x Aurora-Linux-x86_64.AppImage
   ```
3. Da a tu usuario permiso para usar el puerto serie y luego **cierra la sesión y vuelve a entrar**
   ```bash
   sudo usermod -aG uucp $USER
   ```
4. Inicia Aurora (doble clic en el archivo, o):
   ```bash
   ./Aurora-Linux-x86_64.AppImage
   ```

> En Arch el grupo del puerto serie se llama `uucp`, no `dialout`.

## Compilarla uno mismo (un comando)

Solo necesitas `git`: el script instala por sí solo los paquetes del sistema, Rust y Node.js si faltan o son demasiado antiguos.

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

## Primer inicio

1. Aurora arranca en la bandeja del sistema (junto al reloj) y abre su ventana.
2. En **Ajustes › Radio** elige **Hamlib**, el modelo de la radio y el puerto (normalmente `/dev/ttyUSB0` o `/dev/ttyACM0`; para verlos: `ls /dev/ttyUSB* /dev/ttyACM*`). PTT: **CAT**.
3. En los ajustes de audio elige la tarjeta de sonido de la radio para entrada y salida (normalmente «USB Audio CODEC»).
4. Pulsa **Monitor** y mira la banda.

## Qué falta respecto a Windows

- OmniRig (solo Windows): el CAT va por Hamlib.
- PTT directo en las líneas RTS/DTR del puerto serie: por ahora solo en Windows.
- Control del volumen de grabación (deslizador y AUTO del nivel de audio): usa el mezclador del sistema.
- Actualización automática desde el programa.
- Control **Pwr**: en Windows ajusta el volumen de la tarjeta de sonido; en Linux baja la señal generada (como JTDX). Ajusta una vez el volumen de salida en el mezclador del sistema.
- TQSL para LoTW: indica su ruta a mano en los ajustes.

## Dónde están los datos

Ajustes, log y registros en `~/.local/share/Aurora` (o la carpeta indicada en `AURORA_DATA_DIR`).

## Desinstalar

Paquete .deb:

```bash
sudo apt remove aurora
```

AppImage: basta con borrar el archivo. Los ajustes y el log quedan en `~/.local/share/Aurora`: borra también esa carpeta si quieres quitarlo todo (antes haz una copia del log).

## Problemas habituales

**El AppImage no arranca y habla de FUSE**  
Instala FUSE 2: `sudo apt install libfuse2t64` (o `libfuse2` en versiones más antiguas) y vuelve a probar.

**No veo el icono en la bandeja (GNOME, Ubuntu)**  
GNOME necesita la extensión AppIndicator: `sudo apt install gnome-shell-extension-appindicator`, luego cierra la sesión y vuelve a entrar.

**«Permission denied» en el puerto serie**  
Tu usuario no está en el grupo `dialout` (Arch: `uucp`): `sudo usermod -aG dialout $USER`, luego cierra la sesión y vuelve a entrar. Para comprobarlo: `groups`.

**Aurora dice que falta rigctld**  
Instala Hamlib: `sudo apt install libhamlib-utils` (Fedora y Arch: `hamlib`). Si está en otra carpeta, indica la ruta en la variable `AURORA_RIGCTLD`.

**La ventana sale blanca o vacía**  
Pasa con algunas tarjetas gráficas: inicia con `WEBKIT_DISABLE_DMABUF_RENDERER=1 aurora-desktop`.

**El teléfono no se conecta**  
Comprueba que el PC y el teléfono estén en la misma red y abre el puerto: `sudo ufw allow 8787/tcp` (Fedora: `sudo firewall-cmd --add-port=8787/tcp --permanent && sudo firewall-cmd --reload`).

**No oigo ni veo señales**  
Comprueba en `pavucontrol` (o en los ajustes de sonido del sistema) que la tarjeta de la radio no esté silenciada y que el nivel de grabación no esté a cero.

## Informar de un problema

Abre un aviso en <https://github.com/wifi75/Aurora/issues> con **[Linux]** en el título, tu distribución, la versión de Aurora y lo que pasó. Las correcciones propuestas (pull requests) son bienvenidas.
