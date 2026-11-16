# Rapporti Cloudflare del 2026-11-16

Dalle 05:26Z alle 10:57Z: 20 righe di errori, 2 pacchetti. Installazioni attive oggi: 67. Pacchetti scaricati in
`D:\progetti\pc\aurora-rapporti\2026-11-16`.

## Pacchetti (2, nessun crash vero)
- `C4AD228F` (0.11.274, FTDX-10 con OmniRig, Windows 11 Home): aggiornamento dalla 0.11.273 riuscito alle 10:42:58, Aurora ripartita
  normalmente. Nello stderr solo «Error removing system tray icon»: la chiusura per l'aggiornamento e' stata contata come crash.
- `7B81EEBD` (0.11.273, TS-590S con Hamlib, Windows 10 Pro in tedesco): stesso caso all'aggiornamento delle 10:58, riuscito. In piu'
  `taskkill` non riesce a chiudere `aurora-desktop.exe` («Dieser Vorgang wird nicht unterstützt»): la copia lo chiude comunque.
  Stessa installazione del «programma bloccato (finestra)» delle 07:38: avvio lento, la finestra risponde dopo 42 s, con un log di
  82 798 QSO letto due volte all'avvio.

## Errori
- CAT: «rigctld ha chiuso il collegamento: la radio non risponde a Hamlib» e connessione non riuscita, 11 righe da PC in italiano, russo,
  francese e inglese (anche 0.11.240, 0.11.261, 0.11.264). Radio spenta, porta o velocita' sbagliate: nessun difetto di Aurora visto.
- CAT: «modo USB non scritto: risposta inattesa a M USB» (Hamlib) e «PKTUSB non supportato: si usa USB» (timeout della radio).
- PTT: «nessuna radio Cat4OM collegata» (Cat4OM scelto ma non collegato) e uscita audio scollegata.
- Audio: 8 Xrun su un PC con Windows 10 Home.
- Decodifica lenta: 3 PC con Windows 10.

## Da guardare
- La chiusura di Aurora per l'aggiornamento non dovrebbe produrre un pacchetto «crash»: falso allarme in tutti e due i casi.
- Avvio lento con log molto grandi (82 798 QSO, letto due volte): 42 s prima che la finestra risponda.
