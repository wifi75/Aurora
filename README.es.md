# Aurora

[![Download](https://img.shields.io/github/downloads/wifi75/Aurora/total?label=Download&color=2ea44f&logo=github)](https://github.com/wifi75/Aurora/releases)
[![Sin virus](https://img.shields.io/badge/Sin%20virus-0%20detecciones-2ea44f?logo=virustotal&logoColor=white)](https://www.virustotal.com/gui/file/6263104fb767835928f9e6db027e146f31644c9b89c6f68cd248f30e9f7f4f49)
![Visite](https://visitor-badge.laobi.icu/badge?page_id=wifi75.Aurora&left_text=Visite&left_color=%23555&right_color=%238e7bff)
[![Piattaforma](https://img.shields.io/badge/Piattaforma-Windows%20%7C%20Linux%20%7C%20macOS-0078D4?logo=windows&logoColor=white)](https://github.com/wifi75/Aurora/blob/main/docs/linux.md)
[![Linux](https://github.com/wifi75/Aurora/actions/workflows/linux.yml/badge.svg)](https://github.com/wifi75/Aurora/actions/workflows/linux.yml)
[![macOS](https://github.com/wifi75/Aurora/actions/workflows/macos.yml/badge.svg)](https://github.com/wifi75/Aurora/actions/workflows/macos.yml)
[![Windows](https://github.com/wifi75/Aurora/actions/workflows/windows.yml/badge.svg)](https://github.com/wifi75/Aurora/actions/workflows/windows.yml)
[![Rust](https://img.shields.io/badge/Rust-1.99%20%C2%B7%20edition%202024-000000?logo=rust&logoColor=white)](https://www.rust-lang.org)
[![Axum](https://img.shields.io/badge/API-Axum%200.8-1A1A1A)](https://github.com/tokio-rs/axum)
[![Tokio](https://img.shields.io/badge/Async-Tokio%201.54-1A1A1A)](https://tokio.rs)
[![SQLite](https://img.shields.io/badge/DB-SQLite-003B57?logo=sqlite&logoColor=white)](https://www.sqlite.org)
[![Tauri](https://img.shields.io/badge/Desktop-Tauri%202-24C8DB?logo=tauri&logoColor=white)](https://tauri.app)
[![React](https://img.shields.io/badge/UI-React%2019-61DAFB?logo=react&logoColor=black)](https://react.dev)
[![TypeScript](https://img.shields.io/badge/TypeScript-7.0-3178C6?logo=typescript&logoColor=white)](https://www.typescriptlang.org)
[![Vite](https://img.shields.io/badge/Build-Vite%208-646CFF?logo=vite&logoColor=white)](https://vite.dev)
[![WebGPU](https://img.shields.io/badge/Waterfall-WebGPU%20%2B%20WebGL2-005A9C)](https://developer.mozilla.org/docs/Web/API/WebGPU_API)
[![Lingue](https://img.shields.io/badge/Lingue-6-FF5C7A)](app/src/i18n)
[![Test](https://img.shields.io/badge/Test-1721%20Rust%20%C2%B7%20839%20UI-3FD6C8)](#sviluppo)
[![SemVer](https://img.shields.io/badge/SemVer-2.0.0-3F4551)](https://semver.org/lang/it/)
[![Keep a Changelog](https://img.shields.io/badge/Changelog-Keep%20a%20Changelog-E05735)](CHANGELOG.md)
[![Manutenzione](https://img.shields.io/badge/Mantenuto-s%C3%AC%20(2026)-brightgreen)](CHANGELOG.md)
[![Licenza](https://img.shields.io/badge/Licenza-GPL--3.0-blue)](LICENSE)

*A Aurora, mi hija: la luz de la que este programa toma su nombre.* ♥

[English](README.md) · [Italiano](README.it.md) · [Deutsch](README.de.md) · [Français](README.fr.md) · **Español** · [Português](README.pt.md)

> ### Descargar Aurora
> [![Download](https://img.shields.io/badge/Download-Aurora--Setup.exe-1E90FF?logo=windows&logoColor=white)](https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Setup.exe) [![Download](https://img.shields.io/badge/Download-Aurora--Linux--x86__64.AppImage-FCC624?logo=linux&logoColor=black)](https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-x86_64.AppImage) [![Download](https://img.shields.io/badge/Download-Aurora--Linux--amd64.deb-A81D33?logo=debian&logoColor=white)](https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-amd64.deb) [![Download](https://img.shields.io/badge/Download-Aurora--macOS.dmg-000000?logo=apple&logoColor=white)](https://github.com/wifi75/Aurora/releases/latest/download/Aurora-macOS.dmg) [![latest](https://img.shields.io/github/v/release/wifi75/Aurora?label=latest)](https://github.com/wifi75/Aurora/releases/latest)
>
> La versión 0.11.188 se analizó con [VirusTotal](https://www.virustotal.com) (unos 70 antivirus) y Windows Defender: ningún antivirus la detecta ([resultado](https://www.virustotal.com/gui/file/6263104fb767835928f9e6db027e146f31644c9b89c6f68cd248f30e9f7f4f49)).
>
> **[Instalador de Windows (última versión)](https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Setup.exe)** · [Página de la última versión](https://github.com/wifi75/Aurora/releases/latest)
>
> [![Linux](https://img.shields.io/badge/Gu%C3%ADa-Linux-E95420?logo=ubuntu&logoColor=white)](https://github.com/wifi75/Aurora/blob/main/docs/linux.es.md)
>
> 🐧 **[Linux (no probada por el autor, sin soporte): AppImage, .deb o compilación con un comando](https://github.com/wifi75/Aurora/blob/main/docs/linux.es.md)**
>
> 🍎 **[macOS (no probada por el autor, sin soporte): Aurora-macOS.dmg para Apple Silicon e Intel](https://github.com/wifi75/Aurora/releases/latest/download/Aurora-macOS.dmg)** — no firmada por Apple: en el primer arranque clic derecho › Abrir (o `xattr -cr /Applications/Aurora.app`). Para el CAT instala Hamlib con `brew install hamlib`. Las actualizaciones se instalan desde la app («Instalar y reiniciar»), como en Windows: deja Aurora en Aplicaciones, no abierta desde el .dmg; en el primer arranque tras una actualización macOS puede volver a pedir confirmación, porque Aurora no está firmada por Apple.

Una estación digital FT8, FT4, JT65 y JT9 de nuevo concepto: interfaz de impacto, automatización de los QSO y control remoto desde el teléfono. Tiene un motor propio de decodificación y transmisión FT8; Aurora es cerebro, motor y pantalla.

Diseño y desarrollo: Tiziano Cassone (IU3WJQ)

## Estado

Versión 0.11.235: motor propio FT8, FT4, JT65 y JT9 con recepción y transmisión, CAT para todas las radios y pantalla de la radio, además de las páginas PSK31/PSK63, RTTY, CW, SSTV y Satélites. La transmisión con la radio real todavía debe probarla el propietario.

- **Estación**: lista de mensajes como en JTDX (país con bandera, distancia, filas coloreadas por tipo, indicativo llamado y llamadas dirigidas a ti resaltados), cascada WebGPU/WebGL2 con barras Rx/Tx arrastrables, Tx1–Tx6 con la fase del QSO, barra de bandas y modo (FT8/FT4) con cambio desde la página.
- **Cola de llamantes**: clic derecho sobre una estación que llama para ponerla en cola (etiqueta «IN CODA» con su puesto); con el QSO registrado, Aurora responde a la primera; el interruptor «Risp. a tutti» pone en cola automáticamente a quien te llama; «Canc DX» detiene al instante la transmisión. Solo con el motor propio.
- **Modos**: FT8, FT4, JT65 y JT9 desde el selector (JT65 y JT9 decodifican al final del minuto: tras cambiar de modo hacen falta uno o dos minutos); interruptor «JT65+JT9» para decodificarlos juntos; Hound con saltos de frecuencia y «Filtro»; modo SWL como JTDX. Estado y paridad con JTDX en `docs/2026-10-26-jt65-jt9.md` (documentación en italiano).
- **Controles como JTDX**: AutoSeq, 1 QSO, Risp B4, Bypassa, Salta Tx1, Periodo Tx, AutoTX, Abilita Tx, Tx/Rx Split, frecuencias Tx/Rx con copia, Canc DX, Gen.Msgs; el doble clic en un mensaje fija el DX y JTDX lo llama en el momento justo (opción en Ajustes → General).
- **Transmisión**: CQ, mensaje libre, frecuencia Tx, nivel Pwr vertical, STOP TX siempre activo (discreto en reposo, completo al transmitir); la tecla Esc hace lo mismo y apaga AutoTX y Abilita Tx. No tiene opciones: siempre está habilitada (permanecen STOP TX, los topes y las automatizaciones apagadas por defecto).
- **Automatizaciones**: respuestas a los CQ con novedades; arrancan apagadas y en prueba en vacío, con topes, dead-man y STOP.
- **Ajustes por tema**: seis grupos (Estación, Recepción, Transmisión y QSO, Avisos, Red y logs, Sistema) con páginas como Radio, Cascada, Secuencia, Servicios de log y Programas conectados; búsqueda en todos los ajustes, errores junto al campo.
- **Seis idiomas** (italiano, inglés, alemán, francés, español, portugués) y un tooltip en cada control.
- **Ayuda y estadísticas**: ayuda breve integrada en seis idiomas; página Estadísticas con diplomas, QSO por banda y países que faltan; tus mensajes transmitidos aparecen como filas `Tx`.
- **DX Cluster, reloj, registro**: ventana DX Cluster (servidores telnet públicos o propios, doble clic = la radio va a la frecuencia del spot), cliente NTP con indicador del reloj, ventana Log de Log4OM2 de solo lectura.
- **Herramientas de la estación**: nivel del audio de entrada con control deslizante y AUTO sobre el volumen de grabación de Windows, historial de mensajes recibidos (30 días, con búsqueda), spots del cluster en la cascada, diagnóstico automático (reloj, audio, indicativo), página Propagación con PSK Reporter y hamqsl.com (**encendidos de fábrica, se pueden apagar**) y mapa de los QSO y de las estaciones recibidas.
- **Mapa DX** (al estilo GridTracker): 15 fondos, mapa nocturno automático, línea gris con crepúsculos, Sol y Luna, estaciones en el aire que se desvanecen, líneas en arco animadas, cuadrículas trabajadas y confirmadas por banda y modo, clic con cuadrícula, distancia, país y zonas CQ/UIT; separable.
- **Radio**: pantalla de la radio arriba con la pantalla ampliada (segundo VFO, instrumentos, PRE/ATT, AGC, NB/NR/notch, ganancias, RIT/XIT), cada elemento se activa en Ajustes > General; la cascada sigue el ancho del filtro de la radio leído por CAT; un rigctld huérfano se cierra solo; en Windows «Restaurar Hamlib» vuelve a descargar rigctld si el antivirus lo quita.
- **DXpediciones**: calendario NG3K (Servicios en línea, apagado por defecto) junto a los CQ: en el aire, escuchadas ahora, próximas e historial.
- **Ajustes**: barra lateral con búsqueda, «?» con la explicación de cada elemento, página Información (versión, autor, licencia, enlaces).
- **Teléfono**: PWA en la misma red, con la página Estación en pestañas y botones pensados para el dedo (pantallas de hasta 640 px); vinculación con código (duración decidida por ti) o acceso sin código para las pruebas.
- **Logs e integraciones**: envío directo a Wavelog/Cloudlog, ClubLog, QRZ.com y LoTW (TQSL); integraciones UDP con multicast y opciones por programa, XML de N1MM; página de exportación ADIF con «Guardar como».
- **De WSJT-X 3**: decodificación a7/a8 y FT8 multihilo, Wait and Reply/Call, cambio de banda automático, full duplex, verificación del Fox (OTP), bloqueo de la Tx por UDP, parada con ROE alta, mensajes no codificables bloqueados, filtros de WSJT-X 3, Hamlib 4.7.2.
- **Display y clúster**: displays de radio estilo Yaesu FT-2000 e Icom IC-7610; DXpediciones NG3K resaltadas en el DX Cluster con el color que elijas; vista previa en directo de la cascada en los ajustes; diagnóstico de fallos siempre activo.
- **Contest (beta)**: página dedicada con 24 concursos FT8/FT4 y VHF (reglamentos y fechas de cada año), secuencia de concurso como WSJT-X, duplicados, puntos, multiplicadores y puntuación prevista en directo, categoría y operadores, log con Cabrillo, EDI y ADIF.
- **UltraFOX**: implementación propia en Rust con el código polar AQP-127 (implementación de Tiziano Cassone IU3WJQ), compatible con SuperFox de WSJT-X (transmisión y recepción). Experimental.

Lo que falta: `docs/mancante.md`. Resultados de las pruebas con JTDX: `docs/fase0-rapporto.md` (documentación en italiano).

## Instalación (Windows)

La forma más sencilla: el instalador `Aurora_X.Y.Z_x64-setup.exe` (programa con icono en la bandeja, inicio con Windows y actualizaciones firmadas). Cómo se instala, dónde están los datos y cómo se hace una versión: `docs/installer-windows.md` (documentación en italiano).

## Inicio rápido (Windows)

```
.\scripts\avvia-sviluppo.ps1 -Motore
```

Inicia el core (puerto 8787) y la interfaz (puerto 5173) en una sola pestaña de Windows Terminal; con `-Motore` usa el motor de Aurora en la tarjeta de sonido (`-Audio "USB Audio"` para elegirla). Abre la página **desde la dirección IP del PC** (por ejemplo `http://192.168.1.225:8787`), no desde `localhost`: es la misma dirección que usa el teléfono.

Para tener también la cascada, define `AURORA_AUDIO_DEVICE` (parte del nombre de la entrada de audio, por ejemplo `USB Audio`): sin ella, la cascada queda vacía.

Para servir la interfaz compilada desde el core (sin el servidor de desarrollo): `cd app && npm run build`, y luego inicia Aurora con `AURORA_UI_DIR=app\dist` y `AURORA_BIND=0.0.0.0:8787`.

### Conectar un teléfono

```
.\scripts\associa-telefono.ps1 -Porta 8787
```

Muestra un código de 8 cifras y un QR; el código vale durante el tiempo que decidas en Ajustes → Acceso (10 minutos por defecto), es de un solo uso y se bloquea tras cinco intentos fallidos. Sin contraseña: el acceso es por dispositivo vinculado y se revoca desde la lista.

Durante las pruebas se puede activar el **acceso sin código** (Ajustes → Acceso, o `AURORA_OPEN_ACCESS=1`): vale solo para quien está en la red doméstica, nunca para internet ni proxies, y se apaga en cualquier momento desde el PC.

### CAT: la radio para todos los modelos

En Ajustes → Radio CAT se elige cómo controlar la radio (frecuencia, modo, PTT, split): **Hamlib** mediante `rigctld` (más de 300 modelos, la opción por defecto) u OmniRig. Perfiles con nombre, detección de la radio y estado en tiempo real. Guía del usuario y pruebas con la radio real en `docs/cat-radio.md` (documentación en italiano).

## Configuración

Variables de entorno (lista completa en `.env.example`): `AURORA_BIND`, `AURORA_DATA_DIR`, `AURORA_UI_DIR`, `AURORA_ENGINE` (`native` / `sim`), `AURORA_UDP_LISTEN`, `AURORA_UDP_FORWARD`, `AURORA_AUDIO_DEVICE`, `AURORA_HISTORY_DAYS`, `AURORA_CTY_PATH`, `AURORA_WORKED_ADIF`, `AURORA_RIG` (`none`, `omnirig`, `hamlib`), `AURORA_RIGCTLD`, `AURORA_EXPORT_UDP`, `AURORA_EXPORT_FILE`, `AURORA_TX_POWER`, `AURORA_PAIR_TTL_SECS`, `AURORA_OPEN_ACCESS`, `AURORA_TRUST_LOCAL`, `AURORA_AUTO_*`.
Las preferencias de visualización (colores, filtros, idioma, bandas, macros) se guardan en el navegador; frecuencias, programación, reglas de la automatización, duración del código y radio (`radio.json`: modelo, puerto, perfiles) están en `data/`.

## Estructura

```
core/crates/   aurora-core  udp-bridge  udp-hub  engine-jtdx  engine-sim
               dxcc (cty.dat)  audio-spectrum  logbook  qso-engine  qso-export
               rig-control  auth  remote-api  aurora
app/src/       pages  components  settings  lib  i18n  render  state  api
scripts/       avvia-sviluppo.ps1  associa-telefono.ps1
docs/          especificación, planes, informes, API, mancante.md (en italiano)
```

## Documentación (en italiano)

- Especificación: `docs/2026-10-24-aurora-design.md` · API: `docs/api.md` · Flujos UDP: `docs/udp-flussi.md`
- Planes: `docs/2026-10-24-aurora-piano*.md` · Informes: `docs/fase0-rapporto.md`, `docs/piano2-rapporto.md`, `docs/piano3-rapporto.md`, `docs/piano4-rapporto.md`
- Lo que falta: `docs/mancante.md` · Memoria del proyecto: `memory.md` · Historial: `CHANGELOG.md`

<a id="sviluppo"></a>

## Desarrollo

```
cd core && cargo test --workspace      # 1686 pruebas del core
cd app && npm install && npm test      # 839 pruebas de la interfaz
cd app && npm run build                # compilación de producción (tsc + vite)
cd core && cargo clippy --workspace --all-targets
```

En el PC con Aurora encendida, las pruebas del core se lanzan con `--target-dir target/testrun` (de lo contrario Windows bloquea `aurora.exe`).


## Licencia

Aurora es software libre, publicado con la **GNU General Public License versión 3** (`GPL-3.0-only`: el motor propio contiene algoritmos de JTDX y WSJT-X, publicados con la GPL 3): texto completo en [`LICENSE`](LICENSE). Copyright (C) 2026 Tiziano Cassone (IU3WJQ). Las bibliotecas de terceros y sus avisos están en [`THIRD-PARTY-NOTICES.md`](THIRD-PARTY-NOTICES.md) y [`docs/licenze-terze-parti.md`](docs/licenze-terze-parti.md) (documentación en italiano).

Quien redistribuya Aurora, incluso modificada, debe hacerlo con la misma licencia y poner a disposición los fuentes. Las bibliotecas de terceros que incluye la interfaz (`leaflet` para el mapa, licencia BSD-2-Clause, `world-atlas` y `topojson-client`, licencia ISC; las teselas del fondo del mapa son de Esri y National Geographic, con su atribución en el mapa, y las demás dependencias enumeradas en `app/package.json` y `core/Cargo.toml`) conservan sus propias licencias.
