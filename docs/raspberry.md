# Aurora su Raspberry Pi (ARM a 64 bit)

**Italiano** · [English](raspberry.en.md) · [Deutsch](raspberry.de.md) · [Français](raspberry.fr.md) · [Español](raspberry.es.md) · [Português](raspberry.pt.md)

> **Attenzione: la versione per ARM è nuova (0.11.349).** È compilata in automatico da GitHub, **non è ancora stata provata dall'autore su una Raspberry vera** e non ha assistenza ufficiale. Se la provi, scrivi com'è andata su <https://github.com/wifi75/Aurora/issues> e manda il pacchetto di assistenza (**Impostazioni › Accesso e assistenza › Assistenza**).

## Cosa serve

Aurora è un programma impegnativo: decodifica in tempo reale più segnali insieme, quindi non gira su ogni Raspberry.

- **Raspberry Pi 5 con 4 GB di RAM o più**: consigliata.
- **Raspberry Pi 4 con 4 GB**: può bastare per FT8 e FT4, ma con poco margine; i modi pesanti (come Q65) sono incerti.
- **Pi 3, Zero e modelli più vecchi**: non adatti.
- **Sistema a 64 bit**: Raspberry Pi OS 64 bit basato su Debian 12 «Bookworm» o successivo (con ambiente grafico), oppure Ubuntu 22.04 o successivo a 64 bit. Il sistema a 32 bit **non funziona**.
- Almeno 2 GB liberi sulla scheda o sull'SSD (meglio un SSD USB o una scheda veloce).
- Orologio del sistema sincronizzato (FT8 richiede l'ora esatta).
- Scheda audio USB o interfaccia della radio, e il collegamento alla radio (USB/CAT).

Per controllare, in un terminale:

```bash
uname -m
free -h
```

`uname -m` deve rispondere `aarch64`; `free -h` mostra la RAM.

## Installazione con il pacchetto .deb

Apri un terminale e copia i comandi uno alla volta.

1. Scarica il pacchetto
   ```bash
   wget https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-arm64.deb
   ```
2. Installalo (chiede la tua password)
   ```bash
   sudo apt install ./Aurora-Linux-arm64.deb
   ```
3. Installa Hamlib per il controllo della radio (CAT)
   ```bash
   sudo apt install libhamlib-utils
   ```
4. Dai al tuo utente il permesso di usare la porta seriale, poi **esci dalla sessione e rientra**
   ```bash
   sudo usermod -aG dialout $USER
   ```
5. Avvia Aurora dal menu delle applicazioni (cerca «Aurora»), oppure:
   ```bash
   aurora-desktop
   ```

## In alternativa: AppImage

Un unico file, senza installare nulla. Serve la libreria FUSE 2: `libfuse2t64` su Debian 13 e Ubuntu 24.04; su Debian 12 «Bookworm» e Ubuntu 22.04 il pacchetto si chiama `libfuse2`.

```bash
sudo apt install libfuse2t64 libhamlib-utils
wget https://github.com/wifi75/Aurora/releases/latest/download/Aurora-Linux-aarch64.AppImage
chmod +x Aurora-Linux-aarch64.AppImage
./Aurora-Linux-aarch64.AppImage
```

## Primi passi

1. In **Impostazioni** scegli l'ingresso e l'uscita audio (la scheda USB della radio).
2. Inserisci il tuo nominativo e il locatore.
3. Collega la radio in **Impostazioni › Radio**.
4. Controlla che l'orologio del sistema sia a posto.

## Consigli per i modelli deboli (Pi 4)

- Chiudi gli altri programmi.
- Non tenere aperte finestre pesanti (mappa, più cascate insieme).
- Usa un modo alla volta.
- Usa l'alimentatore ufficiale e raffredda la scheda (ventola o dissipatore).
- Se la decodifica ritarda (nel pacchetto di assistenza compare l'avviso «decodifica lenta»), segnalalo nelle issue.

## Disinstallare

```bash
sudo apt remove aurora-multimode
```

Le impostazioni e il log restano in `~/.local/share/Aurora`.

Per le altre distribuzioni e per i problemi comuni vedi la [guida Linux generale](linux.md).
