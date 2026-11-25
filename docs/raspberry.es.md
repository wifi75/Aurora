# Aurora en Raspberry Pi (ARM de 64 bits)

[Italiano](raspberry.md) · [English](raspberry.en.md) · [Deutsch](raspberry.de.md) · [Français](raspberry.fr.md) · **Español** · [Português](raspberry.pt.md)

> **Atención: la versión para ARM es nueva (0.11.349).** GitHub la compila automáticamente, **el autor todavía no la ha probado en una Raspberry real** y no tiene asistencia oficial. Si la pruebas, cuéntanos cómo te fue en <https://github.com/wifi75/Aurora/issues> y envía el paquete de asistencia (**Ajustes › Acceso y asistencia › Asistencia**).

## Qué hace falta

Aurora es un programa exigente: decodifica varias señales en tiempo real, por eso no funciona en todas las Raspberry.

- **Raspberry Pi 5 con 4 GB de RAM o más**: recomendada.
- **Raspberry Pi 4 con 4 GB**: puede bastar para FT8 y FT4, pero con poco margen; los modos pesados (como Q65) son inciertos.
- **Pi 3, Zero y modelos más antiguos**: no son adecuados.
- **Sistema de 64 bits**: Raspberry Pi OS de 64 bits basado en Debian 12 «Bookworm» o posterior (con entorno gráfico), o Ubuntu 22.04 o posterior de 64 bits. El sistema de 32 bits **no funciona**.
- Al menos 2 GB libres en la tarjeta o el SSD (mejor un SSD USB o una tarjeta rápida).
- Reloj del sistema sincronizado (FT8 exige la hora exacta).
- Una tarjeta de sonido USB o la interfaz de la radio, y la conexión con la radio (USB/CAT).

Para comprobarlo, en una terminal:

```bash
uname -m
free -h
```

`uname -m` debe responder `aarch64`; `free -h` muestra la RAM.

## Instalación con el paquete .deb

Abre una terminal y copia los comandos de uno en uno.

1. Descarga el paquete
   ```bash
   wget https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-arm64.deb
   ```
2. Instálalo (pide tu contraseña)
   ```bash
   sudo apt install ./Aurora-Linux-arm64.deb
   ```
3. Instala Hamlib para el control de la radio (CAT)
   ```bash
   sudo apt install libhamlib-utils
   ```
4. Da a tu usuario permiso para usar el puerto serie, y luego **cierra la sesión y vuelve a entrar**
   ```bash
   sudo usermod -aG dialout $USER
   ```
5. Inicia Aurora desde el menú de aplicaciones (busca «Aurora»), o:
   ```bash
   aurora-desktop
   ```

## Alternativa: AppImage

Un único archivo, sin instalar nada. Hace falta la biblioteca FUSE 2: `libfuse2t64` en Debian 13 y Ubuntu 24.04; en Debian 12 «Bookworm» y Ubuntu 22.04 el paquete se llama `libfuse2`.

```bash
sudo apt install libfuse2t64 libhamlib-utils
wget https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-aarch64.AppImage
chmod +x Aurora-Linux-aarch64.AppImage
./Aurora-Linux-aarch64.AppImage
```

## Primeros pasos

1. En **Ajustes** elige la entrada y la salida de audio (la tarjeta de sonido USB de la radio).
2. Escribe tu indicativo y tu locator.
3. Conecta la radio en **Ajustes › Radio**.
4. Comprueba que el reloj del sistema esté en hora.

## Consejos para los modelos más débiles (Pi 4)

- Cierra los demás programas.
- No dejes abiertas ventanas pesadas (mapa, varias cascadas a la vez).
- Usa un modo cada vez.
- Usa la fuente de alimentación oficial y refrigera la placa (ventilador o disipador).
- Si la decodificación se retrasa (el paquete de asistencia muestra el aviso «decodificación lenta»), avísalo en las issues.

## Desinstalar

```bash
sudo apt remove aurora-multimode
```

Los ajustes y el registro quedan en `~/.local/share/Aurora`.

Para otras distribuciones y problemas comunes, consulta la [guía general de Linux](linux.es.md).
