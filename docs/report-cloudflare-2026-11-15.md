# Rapporti Cloudflare del 2026-11-15

Installazioni attive: 163 (14/11: 123). Pacchetti scaricati in `D:\progetti\pc\aurora-rapporti\2026-11-15`.

## Installatore (4 pacchetti)
- 3 tentativi dallo stesso PC (Windows 11 Enterprise 25H2, utente senza diritti di amministratore, 22:28-22:40 UTC, 0.11.266 e
  0.11.270): cartella scelta `C:\Program Files\Aurora`; le cartelle si creano ma i file no («Impossibile scrivere» su
  `aurora-desktop.exe` e sui font). Probabile criterio aziendale o protezione delle cartelle. La cartella di base dell'installatore
  e' `%LOCALAPPDATA%\Aurora` (`installMode: currentUser`): Program Files e' stata scelta a mano.
- 1 aggiornamento 0.11.266 → 0.11.270 con Aurora aperta: taskkill e Restart Manager non la chiudono, cartella hamlib vecchia bloccata,
  copia fermata.

## CAT (la maggior parte dei messaggi)
- «rigctld ha chiuso il collegamento: la radio non risponde a Hamlib» (os error 10054): 732 volte da 2 installazioni (0.11.267),
  81 da una (0.11.268). Radio spenta, porta sbagliata o cavo: nessun difetto di Aurora visto.
- «f: errore di Hamlib (-20)» 228 volte da 2 installazioni (0.11.266).

## Altro
- Audio: «il motore resta indietro, buttati blocchi vecchi» e Xrun, pochi casi (PC lenti).
- Decodifica lenta FT8: 3 casi.
- PTT: un «T: parametro non valido (-#)» (radio senza PTT CAT, come l'IC-706 della #63) e un'uscita audio staccata.

## Controllo del 2026-11-16 (dopo la 0.11.273)

Dalle 21:53Z del 15 alle 05:26Z del 16: 8 installazioni con errori (6 gia' sulla 0.11.273), nessun errore dell'installatore ne' di Aurora.

- CAT: rigctld chiude il collegamento perche' la radio non risponde (PC in russo e polacco); una 0.11.267 l'ha ripetuto 365 volte.
- Audio: scheda USB staccata durante l'uso ("dispositivo rimosso", "non risponde piu'" in trasmissione).
- Decodifica lenta e audio scartato verso il motore su due PC poco potenti.
