# Aurora sous Linux

[Italiano](linux.md) · [English](linux.en.md) · [Deutsch](linux.de.md) · **Français** · [Español](linux.es.md) · [Português](linux.pt.md)

Version **communautaire** : elle est compilée à partir du même code que la version Windows, donc chaque nouveauté arrive aussi sous Linux. Le programme est développé et testé sous Windows ; la version Linux est testée par ses utilisateurs, **sans assistance officielle**.

## La télécharger toute prête

Chaque release contient deux fichiers pour Linux 64 bits (x86_64), préparés automatiquement par GitHub :

- [AppImage](https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-x86_64.AppImage) : fonctionne sur presque toutes les distributions. `chmod +x Aurora-Linux-x86_64.AppImage`, puis `./Aurora-Linux-x86_64.AppImage`.
- [Paquet .deb](https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-amd64.deb) pour Debian, Ubuntu, Mint : `sudo apt install ./Aurora-Linux-amd64.deb`.

## La compiler soi-même (une commande)

Il faut seulement `git` et **Node.js 20** ou plus récent. Si Rust manque, le script l'installe.

```bash
git clone https://github.com/wifi75/Aurora.git
cd Aurora
./scripts/compila-linux.sh
```

Le script installe les paquets système (il demande le mot de passe `sudo`), compile l'interface et le programme, puis indique où se trouvent le `.deb` et l'`.AppImage` (dans `desktop/src-tauri/target/release/bundle/`). La première fois, il faut 10 à 20 minutes ; ensuite beaucoup moins.

Options:

| | |
|---|---|
| `--solo-deb` | prépare seulement le `.deb` (plus rapide) |
| `--solo-appimage` | prépare seulement l'`.AppImage` |
| `--senza-pacchetti` | n'installe aucun paquet (tu les as déjà, ou ta distribution n'est pas reconnue) |

## Mettre à jour

Sous Linux, il n'y a pas de mise à jour automatique. Télécharge la nouvelle AppImage ou le nouveau `.deb`, ou recompile :

```bash
cd Aurora
git pull
./scripts/compila-linux.sh --senza-pacchetti
```

## Paquets système

Le script reconnaît apt (Debian, Ubuntu, Mint), dnf (Fedora), pacman (Arch, Manjaro) et zypper (openSUSE). Sur d'autres distributions, installe les équivalents de ces paquets Debian, puis lance le script avec `--senza-pacchetti` :

```
build-essential curl wget file pkg-config libssl-dev libwebkit2gtk-4.1-dev libgtk-3-dev
libayatana-appindicator3-dev librsvg2-dev libxdo-dev libasound2-dev libudev-dev libhamlib-utils
```

## Radio, audio et permissions

- **CAT** : Aurora utilise Hamlib. Installe-le depuis ta distribution (`libhamlib-utils` sur Debian/Ubuntu, `hamlib` ailleurs) : Aurora trouve `rigctld` toute seule dans `/usr/bin` ou `/usr/local/bin`. Pour un autre chemin : variable `AURORA_RIGCTLD`.
- **Port série** : ton utilisateur doit faire partie du groupe `dialout` (`uucp` sur Arch) : `sudo usermod -aG dialout $USER`, puis déconnecte-toi et reconnecte-toi.
- **PTT** : utilise le PTT via CAT (Hamlib).
- **Audio** : choisis la carte son de la radio dans les réglages d'Aurora (ALSA, aussi via PulseAudio ou PipeWire).
- **Téléphone** : comme sous Windows, ouvre `http://<IP-du-PC>:8787` sur le téléphone ; si un pare-feu est actif, ouvre le port 8787.

## Ce qui manque par rapport à Windows

- OmniRig (Windows seulement) : le CAT passe par Hamlib.
- PTT direct sur les lignes RTS/DTR du port série : Windows seulement pour l'instant.
- Réglage du volume d'enregistrement (curseur et AUTO du niveau audio) : utilise le mixeur du système.
- Mise à jour automatique depuis le programme.
- TQSL pour LoTW : indique son chemin à la main dans les réglages.

## Où sont les données

Réglages, log et enregistrements dans `~/.local/share/Aurora` (ou le dossier indiqué par `AURORA_DATA_DIR`).

## Problèmes

Ouvre un signalement sur <https://github.com/wifi75/Aurora/issues> avec **[Linux]** dans le titre, ta distribution, la version d'Aurora et ce qui s'est passé. Les corrections proposées (pull requests) sont les bienvenues.
