# UK-EI DX CW Contest - regolamento riassunto

Contest del UK and Ireland Contest Club (UKEICC, con l'RSGB) per promuovere i contatti fra stazioni UK/EI e DX. CW e SSB hanno lo stesso regolamento ma sono due gare in due fine settimana diversi (cartelle «UK-EI DX CW Contest» e «UK-EI DX SSB Contest»).

- **Fonte:** «UK/EI DX Contests: Rules», versione 19 del 2 maggio 2026, <https://www.ukeicc.com/dx-contest-rules.php> (UKEICC); le date vengono dalla tabella 2026-2030 della sezione 2. Notizie RSGB: <https://rsgb.org/main/?p=47445>.
- **Data di lettura:** 2026-11-24. Le regole cambiano: ricontrollare a ogni edizione.
- Cabrillo: il regolamento non scrive la riga `CONTEST:`; i caricamenti si chiamano «DX CW Contest». Il nome non e' in nessun elenco Cabrillo letto: **da verificare**. Con `cabrillo_contest` vuoto Aurora scrive il nome della cartella.

## Date
- CW: sabato 12:00 UTC - domenica 12:00 UTC (24 ore). **2026: 25-26 aprile.** 2027: 24-25 aprile. Tabella fino al 2030 nel regolamento (CW: ultimo fine settimana di aprile circa; SSB: fine settimana che arretra da fine ottobre a fine agosto), tutta in `contest.toml`.
- Log solo online, in Cabrillo, **entro 24 ore** dalla fine; risultati provvisori dopo la scadenza, ricorsi 24 ore, risultati finali entro 48 ore; file UBN per e-mail entro 7 giorni.
- Le stazioni di Russia e Bielorussia non possono mandare il log (dal luglio 2024), ma i loro QSO danno punti e moltiplicatori agli altri.

## Bande e modo
- 3,5, 7, 14, 21 e 28 MHz, solo CW. Segmenti preferiti (80 e 20 m): 3510-3560 kHz e 14000-14060 kHz. Fuori dai segmenti pubblicati tutti i QSO della banda sono annullati. La domenica mattina evitare le frequenze dei bollettini su 80 e 40 m.
- Tutti lavorano tutti. UK/EI = stazioni situate e operanti in EI, G, GD, GI, GJ, GM, GU, GW; DX = tutte le altre (anche l'Europa continentale).

## Scambio
- RS(T) (facoltativo) + numero progressivo da 1, una sola sequenza per tutte le bande. Le stazioni UK/EI mandano anche il **codice di distretto** di due lettere (155 codici, elenco nel regolamento; l'Irlanda del Nord usa AN, AR, DR, DW, FE, TY, non BT). Se l'altro non manda il numero si scrive 0 e il QSO vale lo stesso.

## Punti per QSO
| Chi chiama | con | 80 e 40 m | 20, 15 e 10 m |
|---|---|---|---|
| UK/EI | UK/EI o Europa | 4 | 2 |
| UK/EI | DX (fuori Europa) | 8 | 4 |
| Europa | UK/EI | 4 | 2 |
| Europa | Europa | 2 | 1 |
| Europa | DX | 4 | 2 |
| DX | UK/EI | 8 | 4 |
| DX | Europa | 4 | 2 |
| DX | DX | 2 | 1 |
- **Doppio notturno:** per le sole stazioni UK/EI tutti i QSO dalle 01:00 alle 04:59 UTC valgono il doppio, in aggiunta a ogni altro punteggio.

## Moltiplicatori (per banda)
- I paesi DXCC lavorati su ogni banda, esclusi EI, G, GD, GI, GJ, GM, GU, GW; i codici di distretto UK/EI lavorati su ogni banda.
- I QSO con stazioni UK/EI valgono solo per i distretti, non per il paese DXCC.
- Punteggio = punti di tutti i QSO x moltiplicatori di tutte le bande.

## Doppi
- Il regolamento non li definisce (si parla solo di errori nelle penalita').

## Categorie
- Single-op Unassisted (nessun aiuto di segnalazione) e Assisted (aiuto ammesso, no autospot); Multi-op (sempre 24 ore, High, Assisted). Overlay: Single-Element Antenna (una sola antenna per tutte le bande) e Rookie (licenza da tre anni o meno). Un solo segnale alla volta per i singoli operatori.
- Tempo: 24 ore o 12 ore (pause di almeno 60 minuti). Potenza: High (massimo di licenza o 1500 W), Low (100 W), QRP (5 W); senza indicazione High.
- Squadre di 2 o 3 membri (ognuno manda il suo log, il nome della squadra va nel campo del caricamento); per i club con piu' di 3 iscritti i migliori tre sono la squadra 1, i tre successivi la 2 e cosi' via.
- Tutta la stazione entro 500 m; remoto da una sola stazione. Solo il proprio nominativo (o quello del club per un operatore singolo).

## Penalita'
- Nominativo o numero sbagliato: si perdono i punti del QSO, piu' una penalita' di due QSO equivalenti. NIL: si perdono i punti del QSO, senza penalita'. Fuori segmento: tutti i QSO della banda annullati.

## Cosa fa Aurora e cosa no
- Punti per chi calcola (UK/EI, europeo o DX) con i punti doppi su 80 e 40 m, doppio notturno 01:00-04:59 per UK/EI, paesi DXCC e distretti UK/EI per banda, doppi per banda.
- Non fa: penalita' (nominativo, numero, NIL), controllo dei segmenti, categorie e squadre, `CATEGORY-*` e il nome Cabrillo (da verificare), controllo incrociato dei log.

## Differenze rispetto al vecchio `.contest` («UK/EI DX Contest», solo CW)
1. Stesse regole di punteggio; ora con le date 2026-2030, l'elenco chiuso dei 155 distretti, la parte SSB in una cartella a parte e lo scambio dichiarato.
2. Le macro mandano `<NRS> <MYSECT>`: il numero e' progressivo e il proprio distretto si scrive nella scheda del contest («La mia sezione / stato / club»), non nelle macro.

## Punti da verificare
- Il regolamento non scrive la regola dei doppi: si tiene una stazione per banda.
- Il nome `CONTEST:` del Cabrillo.
- Europa = paesi del continente europeo di cty.dat (la Russia asiatica conta come DX); il regolamento non elenca i paesi.
