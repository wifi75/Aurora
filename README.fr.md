# Aurora

[![Download](https://img.shields.io/github/downloads/wifi75/Aurora/total?label=Download&color=2ea44f&logo=github)](https://github.com/wifi75/Aurora/releases)
![Visite](https://visitor-badge.laobi.icu/badge?page_id=wifi75.Aurora&left_text=Visite&left_color=%23555&right_color=%238e7bff)
[![Piattaforma](https://img.shields.io/badge/Piattaforma-Windows%20%7C%20Linux-0078D4?logo=windows&logoColor=white)](https://github.com/wifi75/Aurora/blob/main/docs/linux.md)
[![Linux](https://github.com/wifi75/Aurora/actions/workflows/linux.yml/badge.svg)](https://github.com/wifi75/Aurora/actions/workflows/linux.yml)
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

*À Aurora, ma fille : la lumière qui a donné son nom à ce programme.* ♥

[English](README.md) · [Italiano](README.it.md) · [Deutsch](README.de.md) · **Français** · [Español](README.es.md) · [Português](README.pt.md)

> ### Télécharger Aurora
> [![Download](https://img.shields.io/badge/Download-Aurora--Setup.exe-1E90FF?logo=windows&logoColor=white)](https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Setup.exe) [![latest](https://img.shields.io/github/v/release/wifi75/Aurora?label=latest)](https://github.com/wifi75/Aurora/releases/latest)
>
> **[Installateur Windows (dernière version)](https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Setup.exe)** · [Page de la dernière version](https://github.com/wifi75/Aurora/releases/latest)
>
> [![Linux AppImage](https://img.shields.io/badge/Linux-AppImage-FCC624?logo=linux&logoColor=black)](https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-x86_64.AppImage) [![Linux .deb](https://img.shields.io/badge/Debian%20%C2%B7%20Ubuntu-.deb-A81D33?logo=debian&logoColor=white)](https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-amd64.deb) [![Linux](https://img.shields.io/badge/Guide-Linux-E95420?logo=ubuntu&logoColor=white)](https://github.com/wifi75/Aurora/blob/main/docs/linux.fr.md)
>
> 🐧 **[Linux (non testée par l'auteur, sans assistance) : AppImage, .deb ou compilation en une commande](https://github.com/wifi75/Aurora/blob/main/docs/linux.fr.md)**

Une station numérique FT8, FT4, JT65 et JT9 d'un genre nouveau : interface percutante, automatisation des QSO et commande à distance depuis un téléphone. Elle possède son propre moteur de décodage et d'émission FT8 ; Aurora est cerveau, moteur et écran.

Conception et développement : Tiziano Cassone (IU3WJQ)

## État

Version 0.11.186 : moteur propre FT8, FT4, JT65 et JT9 avec réception et émission, CAT pour toutes les radios et affichage de la radio. L'émission avec une vraie radio reste à tester par le propriétaire.

- **Station** : liste des messages comme dans JTDX (pays avec drapeau, distance, lignes colorées par type, indicatif appelé et appels qui te sont adressés mis en évidence), cascade WebGPU/WebGL2 avec barres Rx/Tx déplaçables, Tx1–Tx6 avec la phase du QSO, barre de bandes et de mode (FT8/FT4) avec changement depuis la page.
- **File des appelants** : clic droit sur une station qui appelle pour la mettre en file (étiquette «IN CODA» avec sa place) ; une fois le QSO enregistré, Aurora répond au premier ; l'interrupteur «Risp. a tutti» met automatiquement en file qui t'appelle ; «Canc DX» arrête aussitôt l'émission. Moteur propre uniquement.
- **Modes** : FT8, FT4, JT65 et JT9 depuis le sélecteur (JT65 et JT9 décodent en fin de minute : après un changement de mode, compter une à deux minutes) ; interrupteur «JT65+JT9» pour les décoder ensemble ; Hound avec sauts de fréquence et «Filtro» ; mode SWL comme JTDX. État et parité avec JTDX dans `docs/2026-10-26-jt65-jt9.md` (documentation en italien).
- **Commandes comme JTDX** : AutoSeq, 1 QSO, Risp B4, Bypassa, Salta Tx1, Periodo Tx, AutoTX, Abilita Tx, Tx/Rx Split, fréquences Tx/Rx avec copie, Canc DX, Gen.Msgs ; le double-clic sur un message définit le DX et JTDX l'appelle au bon moment (option dans Paramètres → Général).
- **Émission** : CQ, message libre, fréquence Tx, niveau Pwr vertical, STOP TX toujours actif (discret au repos, plein en émission) ; la touche Échap fait de même et éteint AutoTX et Abilita Tx. Il n'a pas d'option : il est toujours activé (restent STOP TX, les plafonds et les automatismes éteints par défaut).
- **Automatismes** : réponses aux CQ avec du nouveau ; ils démarrent éteints et en essai à vide, avec plafonds, dead-man et STOP.
- **Réglages par thème** : six groupes (Station, Réception, Émission et QSO, Alertes, Réseau et logs, Système) avec des pages comme Radio, Cascade, Séquence, Services de log et Programmes reliés ; recherche dans tous les réglages, erreurs affichées près du champ.
- **Six langues** (italien, anglais, allemand, français, espagnol, portugais) et une infobulle sur chaque commande.
- **Aide et statistiques** : courte aide intégrée en six langues ; page Statistiques avec diplômes, QSO par bande et pays manquants ; tes propres messages émis apparaissent comme lignes `Tx`.
- **DX Cluster, horloge, journal** : fenêtre DX Cluster (serveurs telnet publics ou personnels, double-clic = la radio se place sur la fréquence du spot), client NTP avec indicateur d'horloge, fenêtre Log de Log4OM2 en lecture seule.
- **Outils de la station** : niveau de l'audio en entrée avec curseur et AUTO sur le volume d'enregistrement de Windows, historique des messages reçus (30 jours, avec recherche), spots du cluster sur la cascade, diagnostic automatique (horloge, audio, indicatif), page Propagation avec PSK Reporter et hamqsl.com (**activés d’usine, désactivables**) et carte des QSO et des stations reçues.
- **Carte DX** (façon GridTracker) : 15 fonds, carte de nuit automatique, ligne grise avec crépuscules, Soleil et Lune, stations en l’air qui s’effacent, lignes en arc animées, carrés travaillés et confirmés par bande et mode, clic avec carré, distance, pays et zones CQ/UIT ; détachable.
- **Radio** : affichage de la radio en haut avec l'affichage étendu (second VFO, instruments, PRE/ATT, AGC, NB/NR/notch, gains, RIT/XIT), chaque élément activable dans Paramètres > Général ; la cascade suit la largeur du filtre de la radio lue par CAT ; un rigctld resté orphelin se ferme tout seul.
- **DXpéditions** : calendrier NG3K (Services en ligne, éteint par défaut) à côté des CQ : en ondes, entendues maintenant, à venir et historique.
- **Paramètres** : barre latérale avec recherche, «?» avec l'explication de chaque élément, page À propos (version, auteur, licence, liens).
- **Téléphone** : PWA sur le même réseau, avec la page Station en onglets et des boutons adaptés au doigt (écrans jusqu'à 640 px) ; association par code (durée choisie par toi) ou accès sans code pour les essais.
- **Logs et intégrations** : envoi direct vers Wavelog/Cloudlog, ClubLog, QRZ.com et LoTW (TQSL) ; intégrations UDP avec multicast et options par programme, XML N1MM ; page d’export ADIF avec « Enregistrer sous ».
- **Depuis WSJT-X 3** : décodage a7/a8 et FT8 multithread, Wait and Reply/Call, changement de bande automatique, full duplex, vérification du Fox (OTP), blocage de l’émission par UDP, arrêt si ROS élevé, messages non codables bloqués, filtres de WSJT-X 3, Hamlib 4.7.2.
- **Afficheur et cluster** : afficheurs radio style Yaesu FT-2000 et Icom IC-7610 ; DXpéditions NG3K mises en évidence dans le DX Cluster dans la couleur choisie ; aperçu en direct de la cascade dans les réglages ; diagnostic des plantages toujours actif.

Ce qui manque : `docs/mancante.md`. Résultats des essais avec JTDX : `docs/fase0-rapporto.md` (documentation en italien).

## Installation (Windows)

La façon la plus simple : l'installateur `Aurora_X.Y.Z_x64-setup.exe` (programme avec icône dans la zone de notification, démarrage avec Windows et mises à jour signées). Comment installer, où se trouvent les données et comment faire une version : `docs/installer-windows.md` (documentation en italien).

## Démarrage rapide (Windows)

```
.\scripts\avvia-sviluppo.ps1 -Motore
```

Lance le core (port 8787) et l'interface (port 5173) dans un seul onglet de Windows Terminal ; avec `-Motore`, il utilise le moteur d'Aurora sur la carte son (`-Audio "USB Audio"` pour la choisir). Ouvre la page **depuis l'adresse IP du PC** (par exemple `http://192.168.1.225:8787`), pas depuis `localhost` : c'est la même adresse que celle utilisée par le téléphone.

Pour avoir aussi la cascade, définis `AURORA_AUDIO_DEVICE` (une partie du nom de l'entrée audio, par exemple `USB Audio`) : sans cela, la cascade reste vide.

Pour servir l'interface compilée depuis le core (sans le serveur de développement) : `cd app && npm run build`, puis lance Aurora avec `AURORA_UI_DIR=app\dist` et `AURORA_BIND=0.0.0.0:8787`.

### Connecter un téléphone

```
.\scripts\associa-telefono.ps1 -Porta 8787
```

Affiche un code à 8 chiffres et un QR ; le code est valable pendant la durée que tu fixes dans Paramètres → Accès (10 minutes par défaut), est à usage unique et se bloque après cinq tentatives erronées. Pas de mot de passe : l'accès se fait par appareil associé et se révoque depuis la liste.

Pendant les essais, on peut activer l'**accès sans code** (Paramètres → Accès, ou `AURORA_OPEN_ACCESS=1`) : il ne vaut que pour ceux qui sont sur le réseau domestique, jamais pour internet ni les proxys, et se désactive à tout moment depuis le PC.

### CAT : la radio pour tous les modèles

Dans Paramètres → Radio CAT, on choisit comment commander la radio (fréquence, mode, PTT, split) : **Hamlib** via `rigctld` (plus de 300 modèles, le choix par défaut) ou OmniRig. Profils nommés, détection de la radio et état en temps réel. Guide de l'utilisateur et essais avec une vraie radio dans `docs/cat-radio.md` (documentation en italien).

## Configuration

Variables d'environnement (liste complète dans `.env.example`) : `AURORA_BIND`, `AURORA_DATA_DIR`, `AURORA_UI_DIR`, `AURORA_ENGINE` (`native` / `sim`), `AURORA_UDP_LISTEN`, `AURORA_UDP_FORWARD`, `AURORA_AUDIO_DEVICE`, `AURORA_HISTORY_DAYS`, `AURORA_CTY_PATH`, `AURORA_WORKED_ADIF`, `AURORA_RIG` (`none`, `omnirig`, `hamlib`), `AURORA_RIGCTLD`, `AURORA_EXPORT_UDP`, `AURORA_EXPORT_FILE`, `AURORA_TX_POWER`, `AURORA_PAIR_TTL_SECS`, `AURORA_OPEN_ACCESS`, `AURORA_TRUST_LOCAL`, `AURORA_AUTO_*`.
Les préférences d'affichage (couleurs, filtres, langue, bandes, macros) sont conservées dans le navigateur ; fréquences, programmation, règles d'automatisation, durée du code et radio (`radio.json` : modèle, port, profils) sont dans `data/`.

## Structure

```
core/crates/   aurora-core  udp-bridge  udp-hub  engine-jtdx  engine-sim
               dxcc (cty.dat)  audio-spectrum  logbook  qso-engine  qso-export
               rig-control  auth  remote-api  aurora
app/src/       pages  components  settings  lib  i18n  render  state  api
scripts/       avvia-sviluppo.ps1  associa-telefono.ps1
docs/          spécification, plans, rapports, API, mancante.md (en italien)
```

## Documentation (en italien)

- Spécification : `docs/2026-10-24-aurora-design.md` · API : `docs/api.md` · Flux UDP : `docs/udp-flussi.md`
- Plans : `docs/2026-10-24-aurora-piano*.md` · Rapports : `docs/fase0-rapporto.md`, `docs/piano2-rapporto.md`, `docs/piano3-rapporto.md`, `docs/piano4-rapporto.md`
- Ce qui manque : `docs/mancante.md` · Mémoire du projet : `memory.md` · Historique : `CHANGELOG.md`

<a id="sviluppo"></a>

## Développement

```
cd core && cargo test --workspace      # 343 tests du core
cd app && npm install && npm test      # 323 tests de l'interface
cd app && npm run build                # build de production (tsc + vite)
cd core && cargo clippy --workspace --all-targets
```

Sur le PC où Aurora est lancée, les tests du core se lancent avec `--target-dir target/testrun` (sinon Windows verrouille `aurora.exe`).

## Licence

Aurora est un logiciel libre, publié sous la **GNU General Public License version 3** (`GPL-3.0-only` : le moteur propre contient des algorithmes de JTDX et WSJT-X, publiés sous GPL 3) : texte complet dans [`LICENSE`](LICENSE). Copyright (C) 2026 Tiziano Cassone (IU3WJQ). Les bibliothèques tierces et leurs avis sont dans [`THIRD-PARTY-NOTICES.md`](THIRD-PARTY-NOTICES.md) et [`docs/licenze-terze-parti.md`](docs/licenze-terze-parti.md) (documentation en italien).

Quiconque redistribue Aurora, même modifiée, doit le faire sous la même licence et mettre les sources à disposition. Les bibliothèques tierces incluses dans l'interface (`leaflet` pour la carte, licence BSD-2-Clause, `world-atlas` et `topojson-client`, licence ISC ; les tuiles de fond de la carte sont d'Esri et de National Geographic, avec leur attribution sur la carte, et les autres dépendances listées dans `app/package.json` et `core/Cargo.toml`) conservent leurs propres licences.
