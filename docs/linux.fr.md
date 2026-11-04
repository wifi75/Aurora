# Aurora sous Linux

[Italiano](linux.md) · [English](linux.en.md) · [Deutsch](linux.de.md) · **Français** · [Español](linux.es.md) · [Português](linux.pt.md)

Version **communautaire** : elle est compilée à partir du même code que la version Windows, donc chaque nouveauté arrive aussi sous Linux. Le programme est développé et testé sous Windows ; la version Linux est testée par ses utilisateurs, **sans assistance officielle**.

## La télécharger toute prête

Chaque release contient deux fichiers pour Linux 64 bits (x86_64), préparés automatiquement par GitHub :

- [AppImage](https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-x86_64.AppImage) : fonctionne sur presque toutes les distributions. `chmod +x Aurora-Linux-x86_64.AppImage`, puis `./Aurora-Linux-x86_64.AppImage`.
- [Paquet .deb](https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-amd64.deb) pour Debian, Ubuntu, Mint : `sudo apt install ./Aurora-Linux-amd64.deb`.

## Installation pas à pas

### Ubuntu, Debian, Linux Mint (paquet .deb)

Le plus simple : le paquet `.deb` met Aurora dans le menu des applications et récupère tout seul les bibliothèques nécessaires. Ouvre un terminal (Ubuntu : `Ctrl`+`Alt`+`T`) et copie les commandes une par une.

1. Télécharge le paquet
   ```bash
   wget https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-amd64.deb
   ```
2. Installe-le (il demande ton mot de passe)
   ```bash
   sudo apt install ./Aurora-Linux-amd64.deb
   ```
3. Installe Hamlib pour le contrôle de la radio (CAT)
   ```bash
   sudo apt install libhamlib-utils
   ```
4. Donne à ton utilisateur la permission d'utiliser le port série, puis **déconnecte-toi et reconnecte-toi**
   ```bash
   sudo usermod -aG dialout $USER
   ```
5. Lance Aurora depuis le menu des applications (cherche « Aurora »), ou :
   ```bash
   aurora-desktop
   ```

> Valable pour Ubuntu 22.04 et suivantes, Debian 12 et suivantes, Linux Mint 21 et suivantes. Sur Ubuntu 20.04 et Debian 11 les bibliothèques sont trop anciennes : utilise une version plus récente de la distribution.

### AppImage sous Ubuntu et Debian

Si tu préfères ne rien installer, l'AppImage est un seul fichier qui se lance d'un double-clic. Ubuntu et Debian ont besoin de la bibliothèque FUSE 2 :

```bash
sudo apt install libfuse2t64 libhamlib-utils
wget https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-x86_64.AppImage
chmod +x Aurora-Linux-x86_64.AppImage
./Aurora-Linux-x86_64.AppImage
```

Ubuntu 24.04 et suivantes, Debian 13 : `libfuse2t64`. Ubuntu 22.04, Debian 12 : `libfuse2`. Pour l'avoir dans le menu des applications, déplace le fichier dans `~/Applications` ou utilise un outil comme Gear Lever ou AppImageLauncher.

### Fedora

1. Installe les bibliothèques et Hamlib
   ```bash
   sudo dnf install fuse fuse-libs webkit2gtk4.1 libappindicator-gtk3 hamlib
   ```
2. Télécharge l'AppImage et rends-la exécutable
   ```bash
   wget https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-x86_64.AppImage
   chmod +x Aurora-Linux-x86_64.AppImage
   ```
3. Donne à ton utilisateur la permission d'utiliser le port série, puis **déconnecte-toi et reconnecte-toi**
   ```bash
   sudo usermod -aG dialout $USER
   ```
4. Lance Aurora (double-clic sur le fichier, ou) :
   ```bash
   ./Aurora-Linux-x86_64.AppImage
   ```

### Arch Linux, Manjaro, EndeavourOS

1. Installe les bibliothèques et Hamlib
   ```bash
   sudo pacman -S --needed fuse2 webkit2gtk-4.1 libappindicator-gtk3 hamlib
   ```
2. Télécharge l'AppImage et rends-la exécutable
   ```bash
   wget https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-x86_64.AppImage
   chmod +x Aurora-Linux-x86_64.AppImage
   ```
3. Donne à ton utilisateur la permission d'utiliser le port série, puis **déconnecte-toi et reconnecte-toi**
   ```bash
   sudo usermod -aG uucp $USER
   ```
4. Lance Aurora (double-clic sur le fichier, ou) :
   ```bash
   ./Aurora-Linux-x86_64.AppImage
   ```

> Sous Arch le groupe du port série s'appelle `uucp`, pas `dialout`.

## La compiler soi-même (une commande)

Il faut seulement `git` : le script installe tout seul les paquets système, Rust et Node.js s'ils manquent ou sont trop anciens.

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

## Premier lancement

1. Aurora démarre dans la zone de notification (près de l'horloge) et ouvre sa fenêtre.
2. Dans **Réglages › Radio** choisis **Hamlib**, le modèle de la radio et le port (souvent `/dev/ttyUSB0` ou `/dev/ttyACM0` ; pour les voir : `ls /dev/ttyUSB* /dev/ttyACM*`). PTT : **CAT**.
3. Dans les réglages audio choisis la carte son de la radio en entrée et en sortie (souvent « USB Audio CODEC »).
4. Appuie sur **Monitor** et regarde la bande.

## Ce qui manque par rapport à Windows

- OmniRig (Windows seulement) : le CAT passe par Hamlib.
- PTT direct sur les lignes RTS/DTR du port série : Windows seulement pour l'instant.
- Réglage du volume d'enregistrement (curseur et AUTO du niveau audio) : utilise le mixeur du système.
- Mise à jour automatique depuis le programme.
- TQSL pour LoTW : indique son chemin à la main dans les réglages.

## Où sont les données

Réglages, log et enregistrements dans `~/.local/share/Aurora` (ou le dossier indiqué par `AURORA_DATA_DIR`).

## Désinstaller

Paquet .deb :

```bash
sudo apt remove aurora
```

AppImage : il suffit de supprimer le fichier. Les réglages et le log restent dans `~/.local/share/Aurora` : supprime aussi ce dossier si tu veux tout enlever (sauvegarde d'abord ton log).

## Problèmes courants

**L'AppImage ne démarre pas et parle de FUSE**  
Installe FUSE 2 : `sudo apt install libfuse2t64` (ou `libfuse2` sur les versions plus anciennes), puis réessaie.

**Je ne vois pas l'icône dans la barre (GNOME, Ubuntu)**  
GNOME a besoin de l'extension AppIndicator : `sudo apt install gnome-shell-extension-appindicator`, puis déconnecte-toi et reconnecte-toi.

**« Permission denied » sur le port série**  
Ton utilisateur n'est pas dans le groupe `dialout` (Arch : `uucp`) : `sudo usermod -aG dialout $USER`, puis déconnecte-toi et reconnecte-toi. Pour vérifier : `groups`.

**Aurora dit que rigctld manque**  
Installe Hamlib : `sudo apt install libhamlib-utils` (Fedora et Arch : `hamlib`). S'il est ailleurs, indique son chemin dans la variable `AURORA_RIGCTLD`.

**La fenêtre est blanche ou vide**  
Cela arrive avec certaines cartes graphiques : lance avec `WEBKIT_DISABLE_DMABUF_RENDERER=1 aurora-desktop`.

**Le téléphone ne se connecte pas**  
Vérifie que le PC et le téléphone sont sur le même réseau et ouvre le port : `sudo ufw allow 8787/tcp` (Fedora : `sudo firewall-cmd --add-port=8787/tcp --permanent && sudo firewall-cmd --reload`).

**Je n'entends ou ne vois aucun signal**  
Vérifie dans `pavucontrol` (ou les réglages son du système) que la carte de la radio n'est pas muette et que le niveau d'enregistrement n'est pas à zéro.

## Signaler un problème

Ouvre un signalement sur <https://github.com/wifi75/Aurora/issues> avec **[Linux]** dans le titre, ta distribution, la version d'Aurora et ce qui s'est passé. Les corrections proposées (pull requests) sont les bienvenues.
