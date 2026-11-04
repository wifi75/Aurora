# Aurora no Linux

[Italiano](linux.md) · [English](linux.en.md) · [Deutsch](linux.de.md) · [Français](linux.fr.md) · [Español](linux.es.md) · **Português**

Versão **da comunidade**: é compilada a partir do mesmo código da versão para Windows, por isso cada novidade chega também ao Linux. O programa é desenvolvido e testado no Windows; a versão Linux é testada pelos seus utilizadores, **sem suporte oficial**.

## Transferi-la já pronta

Cada release tem dois ficheiros para Linux de 64 bits (x86_64), preparados automaticamente pelo GitHub:

- [AppImage](https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-x86_64.AppImage): funciona em quase todas as distribuições. `chmod +x Aurora-Linux-x86_64.AppImage` e depois `./Aurora-Linux-x86_64.AppImage`.
- [Pacote .deb](https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-amd64.deb) para Debian, Ubuntu, Mint: `sudo apt install ./Aurora-Linux-amd64.deb`.

## Instalação passo a passo

### Ubuntu, Debian, Linux Mint (pacote .deb)

A forma mais simples: o pacote `.deb` põe o Aurora no menu de aplicações e transfere sozinho as bibliotecas necessárias. Abre um terminal (Ubuntu: `Ctrl`+`Alt`+`T`) e copia os comandos um de cada vez.

1. Transfere o pacote
   ```bash
   wget https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-amd64.deb
   ```
2. Instala-o (pede a tua palavra-passe)
   ```bash
   sudo apt install ./Aurora-Linux-amd64.deb
   ```
3. Instala o Hamlib para o controlo do rádio (CAT)
   ```bash
   sudo apt install libhamlib-utils
   ```
4. Dá ao teu utilizador permissão para usar a porta série e depois **termina a sessão e volta a entrar**
   ```bash
   sudo usermod -aG dialout $USER
   ```
5. Inicia o Aurora a partir do menu de aplicações (procura «Aurora»), ou:
   ```bash
   aurora-desktop
   ```

> Vale para Ubuntu 22.04 e posteriores, Debian 12 e posteriores, Linux Mint 21 e posteriores. No Ubuntu 20.04 e no Debian 11 as bibliotecas são demasiado antigas: usa uma versão mais recente da distribuição.

### AppImage no Ubuntu e no Debian

Se preferires não instalar nada, o AppImage é um único ficheiro que se abre com duplo clique. O Ubuntu e o Debian precisam da biblioteca FUSE 2:

```bash
sudo apt install libfuse2t64 libhamlib-utils
wget https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-x86_64.AppImage
chmod +x Aurora-Linux-x86_64.AppImage
./Aurora-Linux-x86_64.AppImage
```

Ubuntu 24.04 e posteriores, Debian 13: `libfuse2t64`. Ubuntu 22.04, Debian 12: `libfuse2`. Para o teres no menu de aplicações, move o ficheiro para `~/Applications` ou usa uma ferramenta como Gear Lever ou AppImageLauncher.

### Fedora

1. Instala as bibliotecas e o Hamlib
   ```bash
   sudo dnf install fuse fuse-libs webkit2gtk4.1 libappindicator-gtk3 hamlib
   ```
2. Transfere o AppImage e torna-o executável
   ```bash
   wget https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-x86_64.AppImage
   chmod +x Aurora-Linux-x86_64.AppImage
   ```
3. Dá ao teu utilizador permissão para usar a porta série e depois **termina a sessão e volta a entrar**
   ```bash
   sudo usermod -aG dialout $USER
   ```
4. Inicia o Aurora (duplo clique no ficheiro, ou):
   ```bash
   ./Aurora-Linux-x86_64.AppImage
   ```

### Arch Linux, Manjaro, EndeavourOS

1. Instala as bibliotecas e o Hamlib
   ```bash
   sudo pacman -S --needed fuse2 webkit2gtk-4.1 libappindicator-gtk3 hamlib
   ```
2. Transfere o AppImage e torna-o executável
   ```bash
   wget https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-x86_64.AppImage
   chmod +x Aurora-Linux-x86_64.AppImage
   ```
3. Dá ao teu utilizador permissão para usar a porta série e depois **termina a sessão e volta a entrar**
   ```bash
   sudo usermod -aG uucp $USER
   ```
4. Inicia o Aurora (duplo clique no ficheiro, ou):
   ```bash
   ./Aurora-Linux-x86_64.AppImage
   ```

> No Arch o grupo da porta série chama-se `uucp`, não `dialout`.

## Compilá-la tu mesmo (um comando)

Só precisas de `git`: o script instala sozinho os pacotes do sistema, o Rust e o Node.js se faltarem ou forem demasiado antigos.

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

## Primeiro arranque

1. O Aurora arranca na área de notificação (junto ao relógio) e abre a sua janela.
2. Em **Definições › Rádio** escolhe **Hamlib**, o modelo do rádio e a porta (normalmente `/dev/ttyUSB0` ou `/dev/ttyACM0`; para as ver: `ls /dev/ttyUSB* /dev/ttyACM*`). PTT: **CAT**.
3. Nas definições de áudio escolhe a placa de som do rádio na entrada e na saída (normalmente «USB Audio CODEC»).
4. Carrega em **Monitor** e observa a banda.

## O que falta em relação ao Windows

- OmniRig (só Windows): o CAT passa pelo Hamlib.
- PTT direto nas linhas RTS/DTR da porta série: por agora só no Windows.
- Controlo do volume de gravação (cursor e AUTO do nível de áudio): usa o misturador do sistema.
- Atualização automática a partir do programa.
- TQSL para o LoTW: indica o caminho à mão nas definições.

## Onde ficam os dados

Definições, log e registos em `~/.local/share/Aurora` (ou a pasta indicada em `AURORA_DATA_DIR`).

## Desinstalar

Pacote .deb:

```bash
sudo apt remove aurora
```

AppImage: basta apagar o ficheiro. As definições e o log ficam em `~/.local/share/Aurora`: apaga também essa pasta se quiseres remover tudo (antes faz uma cópia do log).

## Problemas comuns

**O AppImage não arranca e fala de FUSE**  
Instala o FUSE 2: `sudo apt install libfuse2t64` (ou `libfuse2` em versões mais antigas) e tenta de novo.

**Não vejo o ícone na área de notificação (GNOME, Ubuntu)**  
O GNOME precisa da extensão AppIndicator: `sudo apt install gnome-shell-extension-appindicator`, depois termina a sessão e volta a entrar.

**«Permission denied» na porta série**  
O teu utilizador não está no grupo `dialout` (Arch: `uucp`): `sudo usermod -aG dialout $USER`, depois termina a sessão e volta a entrar. Para verificar: `groups`.

**O Aurora diz que falta o rigctld**  
Instala o Hamlib: `sudo apt install libhamlib-utils` (Fedora e Arch: `hamlib`). Se estiver noutra pasta, indica o caminho na variável `AURORA_RIGCTLD`.

**A janela fica branca ou vazia**  
Acontece com algumas placas gráficas: arranca com `WEBKIT_DISABLE_DMABUF_RENDERER=1 aurora-desktop`.

**O telemóvel não se liga**  
Confirma que o PC e o telemóvel estão na mesma rede e abre a porta: `sudo ufw allow 8787/tcp` (Fedora: `sudo firewall-cmd --add-port=8787/tcp --permanent && sudo firewall-cmd --reload`).

**Não ouço nem vejo sinais**  
Confirma no `pavucontrol` (ou nas definições de som do sistema) que a placa do rádio não está sem som e que o nível de gravação não está a zero.

## Comunicar um problema

Abre um pedido em <https://github.com/wifi75/Aurora/issues> com **[Linux]** no título, a tua distribuição, a versão do Aurora e o que aconteceu. Correções propostas (pull requests) são bem-vindas.
