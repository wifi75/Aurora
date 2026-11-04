#!/usr/bin/env bash
# Compila Aurora per Linux con un solo comando (non provata dall'autore, senza assistenza).
#
#   ./scripts/compila-linux.sh                 -> installa i pacchetti di sistema, compila e prepara .deb e .AppImage
#   ./scripts/compila-linux.sh --solo-deb      -> prepara solo il .deb (piu' veloce)
#   ./scripts/compila-linux.sh --solo-appimage -> prepara solo l'.AppImage (va su quasi tutte le distribuzioni)
#   ./scripts/compila-linux.sh --senza-pacchetti -> non installa nulla con il gestore dei pacchetti (li hai gia')
#
# Distribuzioni riconosciute: Debian/Ubuntu/Mint (apt), Fedora (dnf), Arch/Manjaro (pacman), openSUSE (zypper).
# Guida completa: docs/linux.md
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
installa_pacchetti=1
bundles="deb,appimage"
for arg in "$@"; do
    case "$arg" in
        --senza-pacchetti) installa_pacchetti=0 ;;
        --solo-deb) bundles="deb" ;;
        --solo-appimage) bundles="appimage" ;;
        -h|--help) sed -n '2,10p' "$0"; exit 0 ;;
        *) echo "Opzione sconosciuta: $arg (vedi --help)"; exit 1 ;;
    esac
done

passo() { printf '\n\033[1;36m== %s\033[0m\n' "$1"; }
errore() { printf '\n\033[1;31m%s\033[0m\n' "$1"; exit 1; }

sudo_cmd=""
if [ "$(id -u)" -ne 0 ]; then sudo_cmd="sudo"; fi

if [ "$installa_pacchetti" -eq 1 ]; then
    passo "1/4 Pacchetti di sistema"
    if command -v apt-get >/dev/null; then
        $sudo_cmd apt-get update
        $sudo_cmd apt-get install -y build-essential curl wget file pkg-config libssl-dev \
            libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev libxdo-dev \
            libasound2-dev libudev-dev libhamlib-utils
    elif command -v dnf >/dev/null; then
        $sudo_cmd dnf install -y gcc gcc-c++ make curl wget file pkgconf-pkg-config openssl-devel \
            webkit2gtk4.1-devel gtk3-devel libappindicator-gtk3-devel librsvg2-devel libxdo-devel \
            alsa-lib-devel systemd-devel hamlib
    elif command -v pacman >/dev/null; then
        $sudo_cmd pacman -S --needed --noconfirm base-devel curl wget file openssl \
            webkit2gtk-4.1 gtk3 libappindicator-gtk3 librsvg xdotool alsa-lib systemd hamlib
    elif command -v zypper >/dev/null; then
        $sudo_cmd zypper install -y -t pattern devel_basis
        $sudo_cmd zypper install -y curl wget file openssl-devel webkit2gtk3-soup2-devel gtk3-devel \
            libappindicator3-1 librsvg-devel xdotool alsa-devel systemd-devel hamlib
    else
        errore "Distribuzione non riconosciuta: installa a mano i pacchetti elencati in docs/linux.md, poi rilancia con --senza-pacchetti"
    fi
else
    passo "1/4 Pacchetti di sistema: saltati (--senza-pacchetti)"
fi

passo "2/4 Rust e Node.js"
if ! command -v cargo >/dev/null; then
    if [ -x "$HOME/.cargo/bin/cargo" ]; then
        export PATH="$HOME/.cargo/bin:$PATH"
    else
        echo "Rust non c'e': lo installo con rustup (solo per il tuo utente, in ~/.cargo)."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
        export PATH="$HOME/.cargo/bin:$PATH"
    fi
fi
node_major=0
if command -v node >/dev/null; then node_major="$(node -p 'process.versions.node.split(".")[0]')"; fi
if [ "$node_major" -lt 20 ]; then
    # le distribuzioni hanno spesso un Node.js troppo vecchio: si installa il 22 solo per il tuo utente, con nvm (in ~/.nvm)
    echo "Node.js 20 o piu' recente non c'e': installo Node.js 22 con nvm (solo per il tuo utente, in ~/.nvm)."
    export NVM_DIR="$HOME/.nvm"
    if [ ! -s "$NVM_DIR/nvm.sh" ]; then
        curl -fsSL https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.3/install.sh | bash
    fi
    set +u
    # shellcheck disable=SC1091
    . "$NVM_DIR/nvm.sh"
    nvm install 22
    nvm use 22
    set -u
fi
echo "Rust: $(cargo --version)"
echo "Node: $(node -v)"

passo "3/4 Interfaccia"
cd "$root/app"
npm ci
npm run build

passo "4/4 Programma"
cd "$root/desktop"
npm ci
npx tauri build --bundles "$bundles"
out="$root/desktop/src-tauri/target/release/bundle"
passo "Fatto"
find "$out/deb" "$out/appimage" -maxdepth 1 \( -name '*.deb' -o -name '*.AppImage' \) 2>/dev/null | sed 's/^/  /' || true
echo
echo "Installa il .deb con:    sudo apt install ./<file>.deb"
echo "Oppure avvia l'AppImage: chmod +x <file>.AppImage && ./<file>.AppImage"
