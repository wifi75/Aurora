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

## Seconda lettura (sera del 2026-11-16, prima della 0.11.277)

40 righe di errori (quasi tutte 0.11.276), 0 pacchetti, 1 riga antivirus. Righe lette e poi tolte dal servizio (`errori`, `antivirus_errori`); le tabelle storiche (`attivi`, `installazioni`, `storico`) restano.

- **Antivirus: primo dato vero.** 3 rapporti di **Windows Defender** (0.11.276, Windows 10 Pro 22H2): all'avvio mancano `dist\index.html`, `hamlib\rigctld.exe`, `hamlib\libhamlib-4.dll` e `hamlib\rotc…` (file messi in quarantena). Da guardare dopo ogni versione: controllo su VirusTotal e segnalazione a Microsoft se serve (TASK.md, passo 10).
- **Installatore:** 4 installazioni «riuscite con avvisi» perche' `aurora-desktop.exe` era tenuto da altri programmi (un servizio di assistenza remota in spagnolo e in russo, e un antivirus «Total Security»).
- **Audio (Windows 10):** 18 righe «audio verso il motore: il motore resta indietro, buttati blocchi vecchi» su un PC Windows 10 Pro (da guardare: decodifica lenta insieme?), piu' Xrun e «decodifica lenta» su altri Windows 10.
- **Blocchi:** 2 «programma bloccato (finestra)» (0.11.276), da confrontare con l'avvio lento; 1 «avvio fallito: la porta e' gia' usata dal programma aurora-desktop» (una seconda copia aperta).
- **CAT/PTT:** rumore solito: «rigctld ha chiuso il collegamento: la radio non risponde a Hamlib» (radio spenta o porta/velocita' sbagliate), `rigctld non trovato` (1), OmniRig che non risponde, `PTT non confermato spento alla chiusura` (2, Windows 11 Pro 26H2: «T: parametro non valido (-1)» e un `rigctld` chiuso dall'host).
- **Nessun crash vero, nessuna riga dell'area «avvio»** (il rapporto di avvio lento e' nuovo e parte dalla 0.11.277).

## Terza lettura (mattina del 2026-11-17, dopo la 0.11.281)

60 righe di errori (0.11.279: 20, 0.11.278: 16, 0.11.276: 9, 0.11.281: 7, 0.11.280: 3), 2 pacchetti (un file di blocco del 9 novembre, versione 0.11.219, rimasto sul PC di un utente; una richiesta di assistenza della 0.11.278), 1 riga antivirus. Installazioni attive negli ultimi due giorni: 0.11.273 56, 0.11.276 54, 0.11.278 18, 0.11.281 17, 0.11.277 11. Nessun crash nuovo, nessuna riga dell'area «avvio».

- **Antivirus (Windows Defender, Windows 10 Pro 22H2, 0.11.278, 2 rapporti):** mancano `dist\index.html`, `hamlib\rigctld.exe`, `hamlib\libhamlib-4.dll`, `hamlib\rotctld.exe` e `cty\cty.dat`. Stesso caso di ieri: Defender mette in quarantena i file di Hamlib. Da fare: VirusTotal, segnalazione a Microsoft, sezione della Guida (esclusione della cartella) e messaggio chiaro all'avvio.
- **PTT non spento (Windows 11 Home 25H2, 0.11.279, circa 18 righe):** «PTT non spento: errore sulla porta seriale: la connessione con la radio si e' interrotta», 14 tentativi di spegnimento, e «modo PKTUSB non scritto nel VFO di trasmissione» con lo stesso errore su un altro PC. Radio USB-seriale che si scollega; rischio di radio rimasta in trasmissione.
- **Split acceso da fuori (0.11.276 e 0.11.279, 2 rapporti):** il messaggio «split acceso fuori da Aurora (radio o programma collegato alla condivisione): nessun comando di split di Aurora…». Conferma che lo split non lo manda Aurora.
- **CAT:** 36 righe, quasi tutte «la radio non risponde a Hamlib»; su macOS «errore di Hamlib», su Linux «rigctld si e' fermato».
- **Audio e decodifica:** circa 13 rapporti «il motore resta indietro, buttati blocchi vecchi» (Windows 10 e 11, da 0.11.273 a 0.11.278) e «decodifica lenta» su un Mac (0.11.278) e due Windows.
