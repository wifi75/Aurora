# Aurora

[![Rust](https://img.shields.io/badge/Rust-1.99%20%C2%B7%20edition%202024-000000?logo=rust&logoColor=white)](https://www.rust-lang.org)
[![Axum](https://img.shields.io/badge/API-Axum%200.8-1A1A1A)](https://github.com/tokio-rs/axum)
[![Tokio](https://img.shields.io/badge/Async-Tokio%201.54-1A1A1A)](https://tokio.rs)
[![SQLite](https://img.shields.io/badge/DB-SQLite-003B57?logo=sqlite&logoColor=white)](https://www.sqlite.org)
[![React](https://img.shields.io/badge/UI-React%2019-61DAFB?logo=react&logoColor=black)](https://react.dev)
[![TypeScript](https://img.shields.io/badge/TypeScript-7.0-3178C6?logo=typescript&logoColor=white)](https://www.typescriptlang.org)
[![Vite](https://img.shields.io/badge/Build-Vite%208-646CFF?logo=vite&logoColor=white)](https://vite.dev)
[![WebGPU](https://img.shields.io/badge/Waterfall-WebGPU%20%2B%20WebGL2-005A9C)](https://developer.mozilla.org/docs/Web/API/WebGPU_API)
[![Lingue](https://img.shields.io/badge/Lingue-6-FF5C7A)](app/src/i18n)
[![Test](https://img.shields.io/badge/Test-1414%20Rust%20%C2%B7%20765%20UI-3FD6C8)](#sviluppo)
[![SemVer](https://img.shields.io/badge/SemVer-2.0.0-3F4551)](https://semver.org/lang/it/)
[![Keep a Changelog](https://img.shields.io/badge/Changelog-Keep%20a%20Changelog-E05735)](CHANGELOG.md)
[![Manutenzione](https://img.shields.io/badge/Mantenuto-s%C3%AC%20(2026)-brightgreen)](CHANGELOG.md)
[![Licenza](https://img.shields.io/badge/Licenza-GPL--3.0-blue)](LICENSE)

*À Aura, a minha filha: a luz de onde a Aurora tira o nome.* ♥

[English](README.md) · [Italiano](README.it.md) · [Deutsch](README.de.md) · [Français](README.fr.md) · [Español](README.es.md) · **Português**

> ### Baixar o Aurora
> [![Download](https://img.shields.io/badge/Download-Aurora--Setup.exe-1E90FF?logo=windows&logoColor=white)](https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Setup.exe) [![latest](https://img.shields.io/github/v/release/wifi75/Aurora?label=latest)](https://github.com/wifi75/Aurora/releases/latest)
>
> **[Instalador para Windows (última versão)](https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Setup.exe)** · [Página da última versão](https://github.com/wifi75/Aurora/releases/latest)

Uma estação digital FT8, FT4, JT65 e JT9 de nova concepção: interface de impacto, automação dos QSO e controle remoto pelo telefone. Tem um motor próprio de decodificação e transmissão FT8 (o JTDX continua sendo uma comparação opcional); o Aurora é cérebro, motor e tela.

Projeto e desenvolvimento: Tiziano Cassone (IU3WJQ)

## Estado

Versão 0.11.71: motor próprio FT8, FT4, JT65 e JT9 com recepção e transmissão, CAT para todos os rádios e display do rádio. A transmissão com o rádio real ainda precisa ser testada pelo proprietário.

- **Estação**: lista de mensagens como no JTDX (país com bandeira, distância, linhas coloridas por tipo, indicativo chamado e chamadas dirigidas a você em destaque), cascata WebGPU/WebGL2 com barras Rx/Tx arrastáveis, Tx1–Tx6 com a fase do QSO, barra de bandas e modo (FT8/FT4) com troca pela própria página.
- **Fila de chamadores**: clique direito em uma estação que chama para colocá-la na fila (etiqueta «IN CODA» com a posição); com o QSO registrado, o Aurora responde à primeira; o interruptor «Risp. a tutti» coloca na fila automaticamente quem chama você; «Canc DX» interrompe a transmissão na hora. Somente com o motor próprio.
- **Modos**: FT8, FT4, JT65 e JT9 pelo seletor (JT65 e JT9 decodificam no fim do minuto: depois de trocar de modo, esperar de um a dois minutos); interruptor «JT65+JT9» para decodificá-los juntos; Hound com saltos de frequência e «Filtro»; modo SWL como no JTDX. Estado e paridade com o JTDX em `docs/2026-10-26-jt65-jt9.md` (documentação em italiano).
- **Comandos como no JTDX**: AutoSeq, 1 QSO, Risp B4, Bypassa, Salta Tx1, Periodo Tx, AutoTX, Abilita Tx, Tx/Rx Split, frequências Tx/Rx com cópia, Canc DX, Gen.Msgs; o clique duplo em uma mensagem define o DX e o JTDX o chama no momento certo (opção em Configurações → Geral).
- **Transmissão**: CQ, mensagem livre, frequência Tx, nível Pwr vertical, STOP TX sempre ativo (discreto em repouso, cheio durante a transmissão); a tecla Esc faz o mesmo e desliga AutoTX e Abilita Tx. Não tem opções: está sempre habilitada (permanecem STOP TX, os limites e as automações desligadas por padrão).
- **Automações**: respostas aos CQ com novidades; começam desligadas e em teste a seco, com limites, dead-man e STOP.
- **Definições por tema**: seis grupos (Estação, Receção, Transmissão e QSO, Avisos, Rede e logs, Sistema) com páginas como Rádio, Cascata, Sequência, Serviços de log e Programas ligados; pesquisa em todas as definições, erros junto ao campo.
- **Seis idiomas** (italiano, inglês, alemão, francês, espanhol, português) e uma dica em cada controle.
- **Ajuda e estatísticas**: ajuda breve integrada em seis idiomas; página Estatísticas com prêmios, QSO por banda e países que faltam; suas mensagens transmitidas aparecem como linhas `Tx`.
- **DX Cluster, relógio, log**: janela DX Cluster (servidores telnet públicos ou próprios, clique duplo = o rádio vai para a frequência do spot), cliente NTP com indicador do relógio, configurações reais do JTDX lidas do seu arquivo e janela Log do Log4OM2 somente leitura.
- **Ferramentas da estação**: nível do áudio de entrada com cursor e AUTO sobre o volume de gravação do Windows, histórico das mensagens recebidas (30 dias, com busca), spots do cluster na cascata, diagnóstico automático (relógio, áudio, indicativo), página Propagação com PSK Reporter e hamqsl.com (**desligados até você ligá-los**) e mapa dos QSO e das estações recebidas.
- **Rádio**: display do rádio no alto com o display estendido (segundo VFO, instrumentos, PRE/ATT, AGC, NB/NR/notch, ganhos, RIT/XIT), cada item pode ser ligado em Configurações > Geral; a cascata acompanha a largura do filtro do rádio lida via CAT; um rigctld órfão se fecha sozinho.
- **DXpedições**: calendário NG3K (Serviços online, desligado por padrão) ao lado dos CQ: no ar, ouvidas agora, próximas e histórico.
- **Configurações**: barra lateral com busca, «?» com a explicação de cada item, página Sobre (versão, autor, licença, links).
- **Telefone**: PWA na mesma rede, com a página Estação em abas e botões pensados para o dedo (telas de até 640 px); associação com código (duração decidida por você) ou acesso sem código para os testes.
- **Logs e integrações**: envio direto para Wavelog/Cloudlog, ClubLog, QRZ.com e LoTW (TQSL); integrações UDP com multicast e opções por programa, XML do N1MM; página de exportação ADIF com «Guardar como».
- **Do WSJT-X 3**: descodificação a7/a8 e FT8 multithread, Wait and Reply/Call, mudança de banda automática, full duplex, verificação do Fox (OTP), bloqueio da Tx por UDP, paragem com ROE alta, mensagens não codificáveis bloqueadas, filtros do WSJT-X 3, Hamlib 4.7.2.
- **Visor e cluster**: visores de rádio estilo Yaesu FT-2000 e Icom IC-7610; DXpedições NG3K realçadas no DX Cluster com a cor que escolheres; pré-visualização ao vivo da cascata nas definições; diagnóstico de falhas sempre ativo.

O que falta: `docs/mancante.md`. Resultados dos testes com o JTDX: `docs/fase0-rapporto.md` (documentação em italiano).

## Instalação (Windows)

O modo mais simples: o instalador `Aurora_X.Y.Z_x64-setup.exe` (programa com ícone na bandeja, início com o Windows e atualizações assinadas). Como instalar, onde ficam os dados e como fazer uma versão: `docs/installer-windows.md` (documentação em italiano).

## Início rápido (Windows)

```
.\scripts\avvia-sviluppo.ps1 -Jtdx
```

Inicia o core (porta 8787) e a interface (porta 5173) em uma única aba do Windows Terminal; com `-Jtdx` ele se conecta ao JTDX real e encontra sozinho o `cty.dat` e o log dos QSO. Abra a página **pelo endereço IP do PC** (por exemplo `http://192.168.1.225:8787`), não por `localhost`: é o mesmo endereço que o telefone usa.

Para ter também a cascata, defina `AURORA_AUDIO_DEVICE` (parte do nome da entrada de áudio, por exemplo `USB Audio`): sem ela, a cascata fica vazia.

Para servir a interface compilada a partir do core (sem o servidor de desenvolvimento): `cd app && npm run build`, depois inicie o Aurora com `AURORA_UI_DIR=app\dist` e `AURORA_BIND=0.0.0.0:8787`.

### Conectar um telefone

```
.\scripts\associa-telefono.ps1 -Porta 8787
```

Mostra um código de 8 dígitos e um QR; o código vale pelo tempo que você decidir em Configurações → Acesso (10 minutos por padrão), é de uso único e é bloqueado após cinco tentativas erradas. Sem senha: o acesso é por dispositivo associado e se revoga pela lista.

Durante os testes é possível ligar o **acesso sem código** (Configurações → Acesso, ou `AURORA_OPEN_ACCESS=1`): vale somente para quem está na rede doméstica, nunca para a internet ou proxies, e pode ser desligado a qualquer momento pelo PC.

### Com o JTDX real

Veja `docs/guida-collegamento-jtdx.md` (documentação em italiano): o Aurora se coloca entre o JTDX e o GridTracker2 (hub UDP) sem tirar nada de nenhum dos dois, e o QSO registrado chega ao Log4OM2 como antes. A troca de banda e modo usa o mesmo OmniRig do JTDX (`AURORA_RIG=omnirig`).

### CAT: o rádio para todos os modelos

Em Configurações → Radio CAT escolhe-se como comandar o rádio (frequência, modo, PTT, split): **Hamlib** por meio do `rigctld` (mais de 300 modelos, a escolha padrão) ou OmniRig. Perfis com nome, detecção do rádio e estado em tempo real. Guia do usuário e testes com o rádio real em `docs/cat-radio.md` (documentação em italiano).

## Configuração

Variáveis de ambiente (lista completa em `.env.example`): `AURORA_BIND`, `AURORA_DATA_DIR`, `AURORA_UI_DIR`, `AURORA_ENGINE` (`sim` / `jtdx`), `AURORA_UDP_LISTEN`, `AURORA_UDP_FORWARD`, `AURORA_AUDIO_DEVICE`, `AURORA_HISTORY_DAYS`, `AURORA_CTY_PATH`, `AURORA_WORKED_ADIF`, `AURORA_RIG` (`none`, `omnirig`, `hamlib`), `AURORA_RIGCTLD`, `AURORA_EXPORT_UDP`, `AURORA_EXPORT_FILE`, `AURORA_TX_POWER`, `AURORA_PAIR_TTL_SECS`, `AURORA_OPEN_ACCESS`, `AURORA_TRUST_LOCAL`, `AURORA_AUTO_*`.
As preferências de visualização (cores, filtros, idioma, bandas, macros) ficam no navegador; frequências, programação, regras da automação, duração do código e rádio (`radio.json`: modelo, porta, perfis) ficam em `data/`.

## Estrutura

```
core/crates/   aurora-core  udp-bridge  udp-hub  engine-jtdx  engine-sim
               dxcc (cty.dat)  audio-spectrum  logbook  qso-engine  qso-export
               rig-control  auth  remote-api  aurora
app/src/       pages  components  settings  lib  i18n  render  state  api
scripts/       avvia-sviluppo.ps1  associa-telefono.ps1
docs/          especificação, planos, relatórios, API, guia JTDX, mancante.md (em italiano)
```

## Documentação (em italiano)

- Especificação: `docs/2026-10-24-aurora-design.md` · API: `docs/api.md` · Fluxos UDP: `docs/udp-flussi.md`
- Planos: `docs/2026-10-24-aurora-piano*.md` · Relatórios: `docs/fase0-rapporto.md`, `docs/piano2-rapporto.md`, `docs/piano3-rapporto.md`, `docs/piano4-rapporto.md`
- O que falta: `docs/mancante.md` · Memória do projeto: `memory.md` · Histórico: `CHANGELOG.md`

<a id="sviluppo"></a>

## Desenvolvimento

```
cd core && cargo test --workspace      # 343 testes do core
cd app && npm install && npm test      # 323 testes da interface
cd app && npm run build                # build de produção (tsc + vite)
cd core && cargo clippy --workspace --all-targets
```

No PC com o Aurora ligado, os testes do core se executam com `--target-dir target/testrun` (caso contrário o Windows bloqueia o `aurora.exe`).

## Licença

O Aurora é software livre, publicado com a **GNU General Public License versão 3** (`GPL-3.0-only`: o motor próprio contém algoritmos do JTDX e do WSJT-X, publicados com a GPL 3): texto completo em [`LICENSE`](LICENSE). Copyright (C) 2026 Tiziano Cassone (IU3WJQ). As bibliotecas de terceiros e seus avisos estão em [`THIRD-PARTY-NOTICES.md`](THIRD-PARTY-NOTICES.md) e [`docs/licenze-terze-parti.md`](docs/licenze-terze-parti.md) (documentação em italiano).

Quem redistribuir o Aurora, mesmo modificado, deve fazê-lo com a mesma licença e disponibilizar os fontes. As bibliotecas de terceiros incluídas na interface (`leaflet` para o mapa, licença BSD-2-Clause, `world-atlas` e `topojson-client`, licença ISC; os blocos do fundo do mapa são da Esri e da National Geographic, com sua atribuição no mapa, e as demais dependências listadas em `app/package.json` e `core/Cargo.toml`) mantêm as próprias licenças.
