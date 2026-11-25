# Aurora sur Raspberry Pi (ARM 64 bits)

[Italiano](raspberry.md) · [English](raspberry.en.md) · [Deutsch](raspberry.de.md) · **Français** · [Español](raspberry.es.md) · [Português](raspberry.pt.md)

> **Attention : la version ARM est nouvelle (0.11.349).** Elle est compilée automatiquement par GitHub, **l'auteur ne l'a pas encore testée sur une vraie Raspberry** et il n'y a pas d'assistance officielle. Si vous l'essayez, dites-nous comment cela s'est passé sur <https://github.com/wifi75/Aurora/issues> et envoyez le paquet d'assistance (**Paramètres › Accès et assistance › Assistance**).

## Ce qu'il faut

Aurora est un programme exigeant : il décode plusieurs signaux en temps réel, donc il ne tourne pas sur toutes les Raspberry.

- **Raspberry Pi 5 avec 4 Go de RAM ou plus** : recommandée.
- **Raspberry Pi 4 avec 4 Go** : peut suffire pour FT8 et FT4, mais avec peu de marge ; les modes lourds (comme Q65) sont incertains.
- **Pi 3, Zero et modèles plus anciens** : non adaptés.
- **Système 64 bits** : Raspberry Pi OS 64 bits basé sur Debian 12 « Bookworm » ou plus récent (avec environnement graphique), ou Ubuntu 22.04 ou plus récent, en 64 bits. Le système 32 bits **ne fonctionne pas**.
- Au moins 2 Go libres sur la carte ou le SSD (mieux : un SSD USB ou une carte rapide).
- Horloge du système synchronisée (FT8 exige l'heure exacte).
- Une carte son USB ou l'interface de la radio, et la liaison avec la radio (USB/CAT).

Pour vérifier, dans un terminal :

```bash
uname -m
free -h
```

`uname -m` doit répondre `aarch64` ; `free -h` affiche la RAM.

## Installation avec le paquet .deb

Ouvrez un terminal et copiez les commandes une par une.

1. Téléchargez le paquet
   ```bash
   wget https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-arm64.deb
   ```
2. Installez-le (demande votre mot de passe)
   ```bash
   sudo apt install ./Aurora-Linux-arm64.deb
   ```
3. Installez Hamlib pour le contrôle de la radio (CAT)
   ```bash
   sudo apt install libhamlib-utils
   ```
4. Donnez à votre utilisateur le droit d'utiliser le port série, puis **fermez la session et rouvrez-la**
   ```bash
   sudo usermod -aG dialout $USER
   ```
5. Lancez Aurora depuis le menu des applications (cherchez « Aurora »), ou :
   ```bash
   aurora-desktop
   ```

## Autre solution : AppImage

Un seul fichier, rien à installer. Il faut la bibliothèque FUSE 2 : `libfuse2t64` sur Debian 13 et Ubuntu 24.04 ; sur Debian 12 « Bookworm » et Ubuntu 22.04 le paquet s'appelle `libfuse2`.

```bash
sudo apt install libfuse2t64 libhamlib-utils
wget https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-aarch64.AppImage
chmod +x Aurora-Linux-aarch64.AppImage
./Aurora-Linux-aarch64.AppImage
```

## Premiers pas

1. Dans **Paramètres**, choisissez l'entrée et la sortie audio (la carte son USB de la radio).
2. Saisissez votre indicatif et votre locator.
3. Connectez la radio dans **Paramètres › Radio**.
4. Vérifiez que l'horloge du système est correcte.

## Conseils pour les modèles plus faibles (Pi 4)

- Fermez les autres programmes.
- Ne gardez pas de fenêtres lourdes ouvertes (carte, plusieurs cascades à la fois).
- Utilisez un seul mode à la fois.
- Utilisez l'alimentation officielle et refroidissez la carte (ventilateur ou dissipateur).
- Si le décodage prend du retard (le paquet d'assistance affiche l'avertissement « décodage lent »), signalez-le dans les issues.

## Désinstaller

```bash
sudo apt remove aurora-multimode
```

Les paramètres et le journal restent dans `~/.local/share/Aurora`.

Pour les autres distributions et les problèmes courants, voir le [guide Linux général](linux.fr.md).
