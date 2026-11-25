# Aurora no Raspberry Pi (ARM de 64 bits)

[Italiano](raspberry.md) · [English](raspberry.en.md) · [Deutsch](raspberry.de.md) · [Français](raspberry.fr.md) · [Español](raspberry.es.md) · **Português**

> **Atenção: a versão para ARM é nova (0.11.349).** O GitHub compila-a automaticamente, **o autor ainda não a testou num Raspberry real** e não há assistência oficial. Se a experimentar, conte-nos como correu em <https://github.com/wifi75/Aurora/issues> e envie o pacote de assistência (**Definições › Acesso e assistência › Assistência**).

## O que é preciso

O Aurora é um programa exigente: descodifica vários sinais em tempo real, por isso não funciona em todos os Raspberry.

- **Raspberry Pi 5 com 4 GB de RAM ou mais**: recomendado.
- **Raspberry Pi 4 com 4 GB**: pode chegar para FT8 e FT4, mas com pouca margem; os modos pesados (como Q65) são incertos.
- **Pi 3, Zero e modelos mais antigos**: não são adequados.
- **Sistema de 64 bits**: Raspberry Pi OS de 64 bits baseado no Debian 12 «Bookworm» ou posterior (com ambiente gráfico), ou Ubuntu 22.04 ou posterior de 64 bits. O sistema de 32 bits **não funciona**.
- Pelo menos 2 GB livres no cartão ou no SSD (melhor um SSD USB ou um cartão rápido).
- Relógio do sistema sincronizado (o FT8 exige a hora exata).
- Uma placa de som USB ou a interface do rádio, e a ligação ao rádio (USB/CAT).

Para verificar, num terminal:

```bash
uname -m
free -h
```

`uname -m` tem de responder `aarch64`; `free -h` mostra a RAM.

## Instalação com o pacote .deb

Abra um terminal e copie os comandos um de cada vez.

1. Descarregue o pacote
   ```bash
   wget https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-arm64.deb
   ```
2. Instale-o (pede a sua palavra-passe)
   ```bash
   sudo apt install ./Aurora-Linux-arm64.deb
   ```
3. Instale o Hamlib para o controlo do rádio (CAT)
   ```bash
   sudo apt install libhamlib-utils
   ```
4. Dê ao seu utilizador permissão para usar a porta série e depois **termine a sessão e volte a entrar**
   ```bash
   sudo usermod -aG dialout $USER
   ```
5. Inicie o Aurora a partir do menu de aplicações (procure «Aurora»), ou:
   ```bash
   aurora-desktop
   ```

## Alternativa: AppImage

Um único ficheiro, sem instalar nada. É preciso a biblioteca FUSE 2: `libfuse2t64` no Debian 13 e Ubuntu 24.04; no Debian 12 «Bookworm» e Ubuntu 22.04 o pacote chama-se `libfuse2`.

```bash
sudo apt install libfuse2t64 libhamlib-utils
wget https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-aarch64.AppImage
chmod +x Aurora-Linux-aarch64.AppImage
./Aurora-Linux-aarch64.AppImage
```

## Primeiros passos

1. Em **Definições** escolha a entrada e a saída de áudio (a placa de som USB do rádio).
2. Introduza o seu indicativo e o locator.
3. Ligue o rádio em **Definições › Rádio**.
4. Verifique se o relógio do sistema está certo.

## Conselhos para os modelos mais fracos (Pi 4)

- Feche os outros programas.
- Não deixe janelas pesadas abertas (mapa, várias cascatas ao mesmo tempo).
- Use um modo de cada vez.
- Use a fonte de alimentação oficial e arrefeça a placa (ventoinha ou dissipador).
- Se a descodificação atrasar (o pacote de assistência mostra o aviso «descodificação lenta»), comunique-o nas issues.

## Desinstalar

```bash
sudo apt remove aurora-multimode
```

As definições e o registo ficam em `~/.local/share/Aurora`.

Para outras distribuições e problemas comuns, veja o [guia geral de Linux](linux.pt.md).
