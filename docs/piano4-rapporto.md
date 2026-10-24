# Piano 4 - Dati e avvisi: rapporto

Data: 2026-10-24 · Versione: 0.4.0 · Stato: **codice completo e verificato dai test automatici; la prova con la radio la fa Tiziano**

## Cosa c'e' ora
- Crate `dxcc`: legge `cty.dat` (prefissi, eccezioni `=CALL`, zone CQ/ITU `(n)[n]`, stazioni `/P`, `/MM` senza paese). Provato anche sul file reale di JTDX.
- `engine-jtdx::worked`: QSO gia' fatti da un ADIF (lunghezze in byte, record incompleti ignorati). Provato sul log reale di 2056 QSO (circa 50 ms).
- Ogni decode porta il paese di chi trasmette e, se serve, una etichetta. Precedenza: nuovo DXCC, DXCC su nuova banda, nuova zona, nuova griglia, CQ.
- Aggiornamento immediato: un QSO chiuso (`QsoLogged`) toglie subito la novita'.
- Interfaccia: etichette, paese sotto il messaggio, pannello "CQ e novita'", avvisi sonori.

## Scelte da conoscere
- **Senza log dei QSO o senza banda nota non si etichetta nulla** (altrimenti tutto sarebbe "nuovo"); il paese si mostra comunque.
- La nuova griglia conta solo per i CQ: in un messaggio tra altre due stazioni si segnalano al piu' paese e zona.
- I file mancanti non fermano Aurora: parte senza paese/novita' e lo scrive nel registro.
- Gli avvisi sonori partono **spenti**; il browser li permette solo dopo un tocco, quindi vanno accesi (o, se erano gia' accesi, basta un tocco sulla pagina). Non suonano per i messaggi gia' presenti all'apertura.
- Il log letto e' quello all'avvio: i QSO fatti con altri programmi mentre Aurora e' accesa si vedono al riavvio.

## Da provare con la radio (Tiziano)
1. Riavviare con `.\scripts\avvia-sviluppo.ps1 -Jtdx` (trova da solo `cty.dat` e il log di JTDX).
2. Controllare che i CQ mostrino il paese e che le etichette siano plausibili con il tuo log.
3. Accendere "Avvisi sonori" e attendere un paese nuovo.

## Prossimo
Voce 6 di `docs/mancante.md`: motore delle automazioni (prova a vuoto, tetto di cicli, dead-man switch).
