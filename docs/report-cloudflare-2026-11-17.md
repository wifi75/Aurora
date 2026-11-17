# Rapporti Cloudflare del 2026-11-17

Dalle 10:57Z del 2026-11-16 alle 10:27Z del 2026-11-17: 53 righe di errori, 5 pacchetti. Installazioni attive oggi: 93 (0.11.281: 40, 0.11.282: 17,
0.11.276: 8, 0.11.278: 6, 0.11.283: 2, le altre una o due).

## Pacchetti (5)
- Tre `installazione` della 0.11.281 dello stesso PC (Celeron N3060, 2 core, Windows 10 Pro/Enterprise 18363): «installazione fermata durante la copia dei
  file», cartella `hamlib` vecchia non tolta dopo 4 tentativi (file bloccati da un programma o dall'antivirus), prima installazione («versione
  installata prima: nessuna»). Poi un aggiornamento silenzioso 0.11.281 → 0.11.282 riuscito, ma con gli stessi avvisi.
- Un `crash` 0.11.101 e un blocco 0.11.219 (vecchi, dello stesso giorno 2026-11-09).

## Errori
- **Blocchi della finestra** (0.11.282, lo stesso Celeron, 2 core): 20-31 secondi fermi durante «ridimensionamento della finestra» e «apertura di una finestra
  a parte»; due blocchi in piu' su un PC con 4 GB (0.11.281 e 0.11.282, «nessuna attivita' della finestra da 120 s»). Hardware molto debole.
- **Audio**: «il motore resta indietro, buttati N blocchi vecchi» (10 righe, Windows 10 Pro 1909, lo stesso Celeron) e Xrun su altri due PC;
  decodifica lenta su 3 PC. Stesso quadro: PC lenti.
- **PTT (0.11.278, Windows 11 Home 26H2)**: «PTT non spento: errore sulla porta seriale: la connessione con la radio si e' interrotta» e «PTT non
  confermato spento alla chiusura: T: errore di Hamlib (-20)», piu' «FT-#(A) Link: collegamento perso» e timeout CAT: la radio perde il collegamento
  USB/seriale. Le correzioni della 0.11.282 (tentativi, avviso, protezione) sono per questo caso; quell'utente e' ancora sulla 0.11.278.
- **CAT**: «rigctld ha chiuso il collegamento: la radio non risponde a Hamlib» (la maggior parte), «rigctld non risponde su <ip>: connection timed out» e «Impossibile
  stabilire la connessione» (4 PC con Windows 11 Pro 26H2/25H2: da guardare, il collegamento locale con rigctld potrebbe essere bloccato da un antivirus o
  dal firewall), «porta COM# non aperta: file non trovato» (la COM non c'e' piu').
- **Hamlib «parametro non valido (-1)»**: «copia A=B non riuscita: G» e «modo USB non scritto nel VFO di trasmissione» (due PC, 0.11.282): la radio non
  accetta l'operazione; probabilmente rumore, senza effetto.
- **Antivirus**: 1 rapporto «file di Aurora mancanti all'avvio» (0.11.281, Windows 10 Pro 22H2, Windows Defender): dist\index.html, rigctld, libhamlib,
  rotctld, cty.dat. Altri due avvisi di installazione: `aurora-desktop.exe` tenuto da AVG Antivirus e dal servizio Compatibilita' programmi di Windows.

## Da guardare
- Installazione che si ferma su PC con antivirus o file bloccati: la cartella `hamlib` vecchia non si toglie in 4 tentativi.
- Connessione locale a rigctld che scade su Windows 11 recente (da capire se e' firewall o antivirus).
- PC lenti: blocchi della finestra fino a 31 s e blocchi di audio; valutare priorita' piu' bassa del motore o un modo «PC lento» piu' deciso.
