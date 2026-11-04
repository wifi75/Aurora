# Aurora no Linux

[Italiano](linux.md) · [English](linux.en.md) · [Deutsch](linux.de.md) · [Français](linux.fr.md) · [Español](linux.es.md) · **Português**

Versão **da comunidade**: é compilada a partir do mesmo código da versão para Windows, por isso cada novidade chega também ao Linux. O programa é desenvolvido e testado no Windows; a versão Linux é testada pelos seus utilizadores, **sem suporte oficial**.

## Transferi-la já pronta

Cada release tem dois ficheiros para Linux de 64 bits (x86_64), preparados automaticamente pelo GitHub:

- [AppImage](https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-x86_64.AppImage): funciona em quase todas as distribuições. `chmod +x Aurora-Linux-x86_64.AppImage` e depois `./Aurora-Linux-x86_64.AppImage`.
- [Pacote .deb](https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-amd64.deb) para Debian, Ubuntu, Mint: `sudo apt install ./Aurora-Linux-amd64.deb`.

## Compilá-la tu mesmo (um comando)

Só precisas de `git` e do **Node.js 20** ou mais recente. Se faltar o Rust, o script instala-o.

```bash
git clone https://github.com/wifi75/Aurora.git
cd Aurora
./scripts/compila-linux.sh
```

O script instala os pacotes do sistema (pede a palavra-passe do `sudo`), compila a interface e o programa e no fim indica onde estão o `.deb` e o `.AppImage` (em `desktop/src-tauri/target/release/bundle/`). Da primeira vez demora 10-20 minutos; das seguintes, muito menos.

Opções:

| | |
|---|---|
| `--solo-deb` | prepara só o `.deb` (mais rápido) |
| `--solo-appimage` | prepara só o `.AppImage` |
| `--senza-pacchetti` | não instala pacotes (já os tens, ou a tua distribuição não está entre as reconhecidas) |

## Atualizar

No Linux não há atualização automática. Transfere o novo AppImage ou o novo `.deb`, ou volta a compilar:

```bash
cd Aurora
git pull
./scripts/compila-linux.sh --senza-pacchetti
```

## Pacotes do sistema

O script reconhece apt (Debian, Ubuntu, Mint), dnf (Fedora), pacman (Arch, Manjaro) e zypper (openSUSE). Noutras distribuições instala os equivalentes destes pacotes Debian e corre o script com `--senza-pacchetti`:

```
build-essential curl wget file pkg-config libssl-dev libwebkit2gtk-4.1-dev libgtk-3-dev
libayatana-appindicator3-dev librsvg2-dev libxdo-dev libasound2-dev libudev-dev libhamlib-utils
```

## Rádio, áudio e permissões

- **CAT**: usa-se o Hamlib. Instala-o a partir da tua distribuição (`libhamlib-utils` no Debian/Ubuntu, `hamlib` nas outras): o Aurora encontra o `rigctld` sozinho em `/usr/bin` ou `/usr/local/bin`. Para outro caminho: variável `AURORA_RIGCTLD`.
- **Porta série**: o teu utilizador tem de estar no grupo `dialout` (`uucp` no Arch): `sudo usermod -aG dialout $USER`, depois termina a sessão e volta a entrar.
- **PTT**: usa o PTT por CAT (Hamlib).
- **Áudio**: escolhe a placa de som do rádio nas definições do Aurora (ALSA, também através do PulseAudio ou PipeWire).
- **Telemóvel**: como no Windows, abre `http://<IP-do-PC>:8787` no telemóvel; se tiveres uma firewall ativa, abre a porta 8787.

## O que falta em relação ao Windows

- OmniRig (só Windows): o CAT passa pelo Hamlib.
- PTT direto nas linhas RTS/DTR da porta série: por agora só no Windows.
- Controlo do volume de gravação (cursor e AUTO do nível de áudio): usa o misturador do sistema.
- Atualização automática a partir do programa.
- TQSL para o LoTW: indica o caminho à mão nas definições.

## Onde ficam os dados

Definições, log e registos em `~/.local/share/Aurora` (ou a pasta indicada em `AURORA_DATA_DIR`).

## Problemas

Abre um pedido em <https://github.com/wifi75/Aurora/issues> com **[Linux]** no título, a tua distribuição, a versão do Aurora e o que aconteceu. Correções propostas (pull requests) são bem-vindas.
