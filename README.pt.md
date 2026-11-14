# Aurora — Multimode Ham Radio Station

[![Download](https://img.shields.io/github/downloads/wifi75/Aurora/total?label=Download&color=2ea44f&logo=github)](https://github.com/wifi75/Aurora/releases)
[![Sem vírus](https://img.shields.io/badge/Sem%20vírus-0%20dete%C3%A7%C3%B5es-2ea44f?logo=virustotal&logoColor=white)](https://www.virustotal.com/gui/file/6263104fb767835928f9e6db027e146f31644c9b89c6f68cd248f30e9f7f4f49)
![Visite](https://visitor-badge.laobi.icu/badge?page_id=wifi75.Aurora&left_text=Visite&left_color=%23555&right_color=%238e7bff)
[![Doar](https://img.shields.io/badge/Doar-PayPal-00457C?logo=paypal&logoColor=white)](https://www.paypal.com/donate/?hosted_button_id=MY55Q6GDQEH8L)
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
[![Lingue](https://img.shields.io/badge/Lingue-7-FF5C7A)](app/src/i18n)
[![Test](https://img.shields.io/badge/Test-2165%20Rust%20%C2%B7%201153%20UI-3FD6C8)](#sviluppo)
[![SemVer](https://img.shields.io/badge/SemVer-2.0.0-3F4551)](https://semver.org/lang/it/)
[![Keep a Changelog](https://img.shields.io/badge/Changelog-Keep%20a%20Changelog-E05735)](CHANGELOG.md)
[![Manutenzione](https://img.shields.io/badge/Mantenuto-s%C3%AC%20(2026)-brightgreen)](CHANGELOG.md)
[![Licenza](https://img.shields.io/badge/Licenza-GPL--3.0-blue)](LICENSE)

*À Aurora, a minha filha: a luz de onde este programa tira o nome.* ♥

[English](README.md) · [Italiano](README.it.md) · [Deutsch](README.de.md) · [Français](README.fr.md) · [Español](README.es.md) · **Português**

🌐 **[Site](https://aurora.iu3wjq.com/en/)**

> ### Baixar o Aurora
> [![Download](https://img.shields.io/badge/Download-Aurora--Setup.exe-1E90FF?logo=windows&logoColor=white)](https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Setup.exe) [![Download](https://img.shields.io/badge/Download-Aurora--Linux--x86__64.AppImage-FCC624?logo=linux&logoColor=black)](https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-x86_64.AppImage) [![Download](https://img.shields.io/badge/Download-Aurora--Linux--amd64.deb-A81D33?logo=debian&logoColor=white)](https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-amd64.deb) [![Download](https://img.shields.io/badge/Download-Aurora--macOS.dmg-000000?logo=apple&logoColor=white)](https://github.com/wifi75/Aurora/releases/latest/download/Aurora-macOS.dmg) [![latest](https://img.shields.io/github/v/release/wifi75/Aurora?label=latest)](https://github.com/wifi75/Aurora/releases/latest)
>
> A versão 0.11.188 foi analisada com o [VirusTotal](https://www.virustotal.com) (cerca de 70 antivírus) e o Windows Defender: nenhum antivírus a deteta ([resultado](https://www.virustotal.com/gui/file/6263104fb767835928f9e6db027e146f31644c9b89c6f68cd248f30e9f7f4f49)).
>
> **[Instalador para Windows (última versão)](https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Setup.exe)** · [Página da última versão](https://github.com/wifi75/Aurora/releases/latest)
>
> [![Linux](https://img.shields.io/badge/Guia-Linux-E95420?logo=ubuntu&logoColor=white)](https://github.com/wifi75/Aurora/blob/main/docs/linux.pt.md)
>
> 🐧 **[Linux (não testada pelo autor, sem suporte): AppImage, .deb ou compilação com um comando](https://github.com/wifi75/Aurora/blob/main/docs/linux.pt.md)**
>
> 🍎 **[macOS (não testada pelo autor, sem suporte): Aurora-macOS.dmg para Apple Silicon e Intel](https://github.com/wifi75/Aurora/releases/latest/download/Aurora-macOS.dmg)** — não assinada pela Apple: no primeiro arranque clique direito › Abrir (ou `xattr -cr /Applications/Aurora.app`). Para o CAT instala o Hamlib com `brew install hamlib`. As atualizações instalam-se a partir da app («Instalar e reiniciar»), como no Windows: mantém a Aurora em Aplicações, não aberta a partir do .dmg; no primeiro arranque depois de uma atualização o macOS pode voltar a pedir confirmação, porque a Aurora não está assinada pela Apple.

Uma estação de radioamador multimodo de nova concepção (FT8, FT4, JT65, JT9, JT4, Q65, MSK144, FST4, FST4W, WSPR, PSK31/PSK63, RTTY, CW, SSTV e satélites): interface de impacto, automação dos QSO e controle remoto pelo telefone. Tem um motor próprio de decodificação e transmissão; o Aurora é cérebro, motor e tela.

Projeto e desenvolvimento: Tiziano Cassone (IU3WJQ)

## Estado

Versão 0.11.254: motor próprio multimodo (FT8, FT4, JT65 A/B/C, JT9, JT4 A–G, Q65, MSK144, FST4, FST4W e WSPR) com recepção e transmissão, CAT para todos os rádios e display do rádio, mais as páginas PSK31/PSK63, RTTY, CW, SSTV e Satélites. A transmissão com o rádio real ainda precisa ser testada pelo proprietário.

- **Estação**: lista de mensagens como no JTDX (país com bandeira, distância, linhas coloridas por tipo, indicativo chamado e chamadas dirigidas a você em destaque), cascata WebGPU/WebGL2 com barras Rx/Tx arrastáveis, Tx1–Tx6 com a fase do QSO, barra de bandas e modo (FT8/FT4) com troca pela própria página.
- **Fila de chamadores**: clique direito em uma estação que chama para colocá-la na fila (etiqueta «IN CODA» com a posição); com o QSO registrado, o Aurora responde à primeira; o interruptor «Risp. a tutti» coloca na fila automaticamente quem chama você; «Canc DX» interrompe a transmissão na hora. Somente com o motor próprio.
- **Modos**: pelo seletor FT8, FT4, JT65 (submodos A/B/C com média dos períodos, para EME), JT9, JT4 (A–G), Q65, MSK144 (5/10/15/30 s e mensagens «Sh»), FST4 e FST4W (períodos de 15 s a 30 min), WSPR-2 (baliza com percentagem de Tx), além das páginas PSK31/PSK63, RTTY, CW, SSTV e Satélites (rastreamento, Doppler, rotor); interruptor «JT65+JT9» para decodificá-los juntos (depois de trocar de modo, esperar de um a dois minutos); Fox/Hound (também UltraFOX) com saltos de frequência e «Filtro»; modo SWL como no JTDX. Estado e paridade com o JTDX em `docs/2026-10-26-jt65-jt9.md` (documentação em italiano).
- **WSPR e FST4W**: farol com potência, «Tx Pct» e envio opcional dos spots para o WSPRnet, desligado de fábrica. Opções de decodificação do WSJT-X em Configurações › Filtros › Decodificação.
- **Comandos como no JTDX**: AutoSeq, 1 QSO, Risp B4, Bypassa, Salta Tx1, Periodo Tx, AutoTX, Abilita Tx, Tx/Rx Split, frequências Tx/Rx com cópia, Canc DX, Gen.Msgs; o clique duplo em uma mensagem define o DX e o JTDX o chama no momento certo (opção em Configurações → Geral).
- **Transmissão**: CQ, mensagem livre, frequência Tx, nível Pwr vertical, STOP TX sempre ativo (discreto em repouso, cheio durante a transmissão); a tecla Esc faz o mesmo e desliga AutoTX e Abilita Tx. Não tem opções: está sempre habilitada (permanecem STOP TX, os limites e as automações desligadas por padrão).
- **Automações**: respostas aos CQ com novidades; começam desligadas e em teste a seco, com limites, dead-man e STOP.
- **Definições por tema**: seis grupos (Estação, Receção, Transmissão e QSO, Avisos, Rede e logs, Sistema) com páginas como Rádio, Cascata, Sequência, Serviços de log e Programas ligados; pesquisa em todas as definições, erros junto ao campo.
- **Sete idiomas** (italiano, inglês, alemão, francês, espanhol, português, neerlandês) e uma dica em cada controle.
- **Ajuda e estatísticas**: ajuda breve integrada em sete idiomas; página Estatísticas com prêmios, QSO por banda e países que faltam; suas mensagens transmitidas aparecem como linhas `Tx`.
- **DX Cluster, relógio, log**: janela DX Cluster (servidores telnet públicos ou próprios, clique duplo = o rádio vai para a frequência do spot; Split DX: com «UP 5», «5UP» ou «QSX» no comentário o clique duplo coloca o rádio em split no VFO B; coluna Modo também para CW e SSB, do band plan IARU Região 1 quando o comentário não o diz), cliente NTP com indicador do relógio, janela Log do Log4OM2 somente leitura.
- **Ferramentas da estação**: nível do áudio de entrada com cursor e AUTO sobre o volume de gravação do Windows, histórico das mensagens recebidas (30 dias, com busca), spots do cluster na cascata, diagnóstico automático (relógio, áudio, indicativo), página Propagação com PSK Reporter e hamqsl.com (**ligados de fábrica, podem ser desligados**) e mapa dos QSO e das estações recebidas.
- **Mapa DX** (ao estilo GridTracker): 15 fundos, mapa noturno automático, linha cinzenta com crepúsculos, Sol e Lua, estações no ar que desvanecem, linhas em arco animadas, quadrados trabalhados e confirmados por banda e modo, clique com quadrado, distância, país e zonas CQ/UIT; destacável.
- **Rádio**: display do rádio no alto com o display estendido (segundo VFO, instrumentos, PRE/ATT, AGC, NB/NR/notch, ganhos, RIT/XIT), cada item pode ser ligado em Configurações > Geral; a cascata acompanha a largura do filtro do rádio lida via CAT; um rigctld órfão se fecha sozinho; no Windows «Restaurar o Hamlib» baixa de novo o rigctld se o antivírus o remover.
- **DXpedições**: calendário NG3K (Serviços online, desligado por padrão) ao lado dos CQ: no ar, ouvidas agora, próximas e histórico; ao lado do país a via QSL (LoTW, OQRS, manager).
- **Configurações**: barra lateral com busca, «?» com a explicação de cada item, página Sobre (versão, autor, licença, links).
- **Telefone**: PWA na mesma rede, com a página Estação em abas e botões pensados para o dedo (telas de até 640 px); associação com código (duração decidida por você) ou acesso sem código para os testes.
- **Logs e integrações**: envio direto para Wavelog/Cloudlog, ClubLog, QRZ.com e LoTW (TQSL); integrações UDP com multicast e opções por programa, XML do N1MM; página de exportação ADIF com «Guardar como».
- **Do WSJT-X 3**: descodificação a7/a8 e FT8 multithread, Wait and Reply/Call, mudança de banda automática, full duplex, verificação do Fox (OTP), bloqueio da Tx por UDP, paragem com ROE alta, mensagens não codificáveis bloqueadas, filtros do WSJT-X 3, Hamlib 4.7.2.
- **OmniRig e Cat4OM**: o OmniRig é comandado diretamente por COM como o WSJT-X (sem PowerShell, sem «acesso negado», PTT mais rápido); Cat4OM nativo, o Aurora e o Log4OM comandam o mesmo rádio ao mesmo tempo (grupo e porta encontrados sozinhos a partir da porta de gestão 17300, como o Decodium) (Configurações › CAT); split Rig/Fake It também com OmniRig e TCI; atraso Tx ajustável (0-0,5 s); se a porta COM estiver ocupada, o Aurora diz qual programa a está usando.
- **Descodificadores alinhados com o WSJT-X 3.3**: FT8 (prioridade à frequência Rx, OSD 4 para as respostas do QSO, a priori para /P e indicativos compostos), FT4 (pesquisa de candidatos do WSJT-X), Q65 (média com a lista do QSO, correção da deriva) JT65/JT9 (limiares do WSJT-X, janela Rx), MSK144, FST4/FST4W e WSPR.
- **Instalador em sete idiomas** (italiano, inglês, alemão, francês, espanhol, português, holandês); a interface também está em sete idiomas: o holandês é uma primeira versão, em revisão.
- **Visor e cluster**: visores de rádio estilo Yaesu FT-2000 e Icom IC-7610; DXpedições NG3K realçadas no DX Cluster com a cor que escolheres; pré-visualização ao vivo da cascata nas definições; diagnóstico de falhas sempre ativo.
- **Contest (beta)**: página dedicada com 24 concursos FT8/FT4 e VHF (regulamentos e datas de cada ano), sequência de concurso como o WSJT-X, duplicados, pontos, multiplicadores e pontuação prevista em direto, categoria e operadores, log com Cabrillo, EDI e ADIF.
- **UltraFOX**: implementação própria em Rust com o código polar AQP-127 (implementação de Tiziano Cassone IU3WJQ), compatível com o SuperFox do WSJT-X (transmissão e receção). Experimental.

O que falta: `docs/mancante.md`. Resultados dos testes com o JTDX: `docs/fase0-rapporto.md` (documentação em italiano).

## Instalação (Windows)

O modo mais simples: o instalador `Aurora_X.Y.Z_x64-setup.exe` (programa com ícone na bandeja, início com o Windows e atualizações assinadas). Como instalar, onde ficam os dados e como fazer uma versão: `docs/installer-windows.md` (documentação em italiano).

## Início rápido (Windows)

```
.\scripts\avvia-sviluppo.ps1 -Motore
```

Inicia o core (porta 8787) e a interface (porta 5173) em uma única aba do Windows Terminal; com `-Motore` usa o motor do Aurora na placa de som (`-Audio "USB Audio"` para a escolher). Abra a página **pelo endereço IP do PC** (por exemplo `http://192.168.1.225:8787`), não por `localhost`: é o mesmo endereço que o telefone usa.

Para ter também a cascata, defina `AURORA_AUDIO_DEVICE` (parte do nome da entrada de áudio, por exemplo `USB Audio`): sem ela, a cascata fica vazia.

Para servir a interface compilada a partir do core (sem o servidor de desenvolvimento): `cd app && npm run build`, depois inicie o Aurora com `AURORA_UI_DIR=app\dist` e `AURORA_BIND=0.0.0.0:8787`.

### Conectar um telefone

```
.\scripts\associa-telefono.ps1 -Porta 8787
```

Mostra um código de 8 dígitos e um QR; o código vale pelo tempo que você decidir em Configurações → Acesso (10 minutos por padrão), é de uso único e é bloqueado após cinco tentativas erradas. Sem senha: o acesso é por dispositivo associado e se revoga pela lista.

Durante os testes é possível ligar o **acesso sem código** (Configurações → Acesso, ou `AURORA_OPEN_ACCESS=1`): vale somente para quem está na rede doméstica, nunca para a internet ou proxies, e pode ser desligado a qualquer momento pelo PC.

### CAT: o rádio para todos os modelos

Em Configurações → Radio CAT escolhe-se como comandar o rádio (frequência, modo, PTT, split): **Hamlib** por meio do `rigctld` (mais de 300 modelos, a escolha padrão) OmniRig (diretamente por COM) ou o Cat4OM nativo; também TCI e FT-991 Link. Perfis com nome, detecção do rádio e estado em tempo real. Guia do usuário e testes com o rádio real em `docs/cat-radio.md` (documentação em italiano).

## Configuração

Variáveis de ambiente (lista completa em `.env.example`): `AURORA_BIND`, `AURORA_DATA_DIR`, `AURORA_UI_DIR`, `AURORA_ENGINE` (`native` / `sim`), `AURORA_UDP_LISTEN`, `AURORA_UDP_FORWARD`, `AURORA_AUDIO_DEVICE`, `AURORA_HISTORY_DAYS`, `AURORA_CTY_PATH`, `AURORA_WORKED_ADIF`, `AURORA_RIG` (`none`, `omnirig`, `hamlib`), `AURORA_RIGCTLD`, `AURORA_EXPORT_UDP`, `AURORA_EXPORT_FILE`, `AURORA_TX_POWER`, `AURORA_PAIR_TTL_SECS`, `AURORA_OPEN_ACCESS`, `AURORA_TRUST_LOCAL`, `AURORA_AUTO_*`.
As preferências de visualização (cores, filtros, idioma, bandas, macros) ficam no navegador; frequências, programação, regras da automação, duração do código e rádio (`radio.json`: modelo, porta, perfis) ficam em `data/`.

## Estrutura

```
core/crates/   aurora-core  udp-bridge  udp-hub  engine-jtdx  engine-sim
               dxcc (cty.dat)  audio-spectrum  logbook  qso-engine  qso-export
               rig-control  auth  remote-api  aurora
app/src/       pages  components  settings  lib  i18n  render  state  api
scripts/       avvia-sviluppo.ps1  associa-telefono.ps1
docs/          especificação, planos, relatórios, API, mancante.md (em italiano)
```

## Documentação (em italiano)

- Especificação: `docs/2026-10-24-aurora-design.md` · API: `docs/api.md` · Fluxos UDP: `docs/udp-flussi.md`
- Planos: `docs/2026-10-24-aurora-piano*.md` · Relatórios: `docs/fase0-rapporto.md`, `docs/piano2-rapporto.md`, `docs/piano3-rapporto.md`, `docs/piano4-rapporto.md`
- O que falta: `docs/mancante.md` · Memória do projeto: `memory.md` · Histórico: `CHANGELOG.md`

<a id="sviluppo"></a>

## Desenvolvimento

```
cd core && cargo test --workspace      # 1686 testes do core
cd app && npm install && npm test      # 839 testes da interface
cd app && npm run build                # build de produção (tsc + vite)
cd core && cargo clippy --workspace --all-targets
```

No PC com o Aurora ligado, os testes do core se executam com `--target-dir target/testrun` (caso contrário o Windows bloqueia o `aurora.exe`).

## Apoie o Aurora

O Aurora é gratuito e vai continuar a sê-lo. Se lhe é útil e quer ajudar o seu desenvolvimento, pode doar com o PayPal:

[![Doar com PayPal](https://img.shields.io/badge/Doar_com-PayPal-00457C?style=for-the-badge&logo=paypal&logoColor=white)](https://www.paypal.com/donate/?hosted_button_id=MY55Q6GDQEH8L)

Um grande obrigado a quem já doou: o vosso apoio conta. Os apoiantes que o desejem são agradecidos em [SOSTENITORI.md](SOSTENITORI.md).

## Licença

O Aurora é software livre, publicado com a **GNU General Public License versão 3** (`GPL-3.0-only`: o motor próprio contém algoritmos do JTDX e do WSJT-X, publicados com a GPL 3): texto completo em [`LICENSE`](LICENSE). Copyright (C) 2026 Tiziano Cassone (IU3WJQ). As bibliotecas de terceiros e seus avisos estão em [`THIRD-PARTY-NOTICES.md`](THIRD-PARTY-NOTICES.md) e [`docs/licenze-terze-parti.md`](docs/licenze-terze-parti.md) (documentação em italiano).

Se usares o código ou as tabelas do Aurora num projeto teu, peço-te por cortesia que me avises abrindo uma questão no GitHub e que cites o autor, Tiziano Cassone IU3WJQ.

Quem redistribuir o Aurora, mesmo modificado, deve fazê-lo com a mesma licença e disponibilizar os fontes. As bibliotecas de terceiros incluídas na interface (`leaflet` para o mapa, licença BSD-2-Clause, `world-atlas` e `topojson-client`, licença ISC; os blocos do fundo do mapa são da Esri e da National Geographic, com sua atribuição no mapa, e as demais dependências listadas em `app/package.json` e `core/Cargo.toml`) mantêm as próprias licenças.
