# CQ WW DX SSB Contest - regolamento riassunto

- **Fonte:** «The 2026 CQ World-Wide DX Contest», <https://www.cqww.com/rules/> (nessuna data di revisione sulla pagina). Cabrillo: <https://www.cqww.com/cabrillo.htm>.
- **Data di lettura:** 2026-11-21. Edizione: 2026 (SSB 24-25 ottobre, gia' passata) e 2027.
- Le regole cambiano ogni anno: ricontrollare a ogni edizione.

## Date
- SSB: ultimo fine settimana pieno di ottobre, da sabato 00:00:00 a domenica 23:59:59 UTC (48 ore). **2026: 24-25 ottobre** (regolamento). 2027: 30-31 ottobre (calcolata, da confermare).
- Log entro 48 ore dalla fine (2026: 27 ottobre 23:59 UTC), solo caricamento web, formato Cabrillo.

## Bande e modo
- 160, 80, 40, 20, 15, 10 m. Solo fonia (il CW e' una gara a parte, a novembre).

## Scambio
- RS + zona CQ (esempio 59 05). Tutti lavorano tutti.

## Punti per QSO
- Altro continente: 3. Stesso continente, paese diverso: 1. Eccezione: paesi diversi entro i confini del Nord America: 2.
- Stesso paese: 0 punti, ma il QSO vale per i moltiplicatori di zona e di paese.
- **Nessun raddoppio sui 160 m** nel regolamento 2026.

## Moltiplicatori (per banda)
- Zone CQ (standard del diploma WAZ), una per banda.
- Paesi: lista DXCC, lista dei moltiplicatori WAE (IT9, GM/s, 4U1V, JW/b, TA1...) piu' IG9/IH9, e confini continentali. Una volta per banda.
- I mobili marittimi contano solo per la zona.

## Formula
- Punti dei QSO x (zone + paesi). Esempio del regolamento: 1000 x (30 + 70) = 100.000.

## Doppi
- Ogni stazione una volta per banda; i doppi si tolgono senza penalita'.

## Categorie
- Single Operator High (1500 W), Low (100 W), QRP (5 W), all band o single band; Single Operator Assisted con le stesse potenze.
- Overlay (riga `CATEGORY-OVERLAY`, conteggio all band, raggruppati High o Low con il QRP nel Low): Classic (una radio, senza assistenza, 24 ore su 48, pause di almeno 60 minuti), Rookie (licenza da meno di tre anni, data nel SOAPBOX), Youth (25 anni o meno, anno di nascita nel SOAPBOX).
- Multi-Single (High/Low; un segnale di run per periodo di 10 minuti, un secondo segnale su un'altra banda solo per moltiplicatori nuovi e senza CQ; il log dice quale segnale ha fatto il QSO), Multi-Two (due segnali su bande diverse, 8 cambi di banda per ora, 1500 W), Multi-Multi (un segnale per banda, 1500 W), Multi-Distributed (fino a sei segnali, uno per banda, stessa entita' DXCC e zona; `CATEGORY-STATION: DISTRIBUTED`). Checklog.

## Cabrillo
- `CONTEST: CQ-WW-SSB` (con i trattini, senza l'anno). `CATEGORY-OPERATOR` SINGLE-OP / MULTI-OP / CHECKLOG, `CATEGORY-ASSISTED`, `CATEGORY-BAND` ALL o 160M...10M, `CATEGORY-POWER` HIGH/LOW/QRP, `CATEGORY-MODE` SSB, `CATEGORY-TRANSMITTER` ONE/TWO/UNLIMITED (multi-op), `CATEGORY-OVERLAY` CLASSIC/ROOKIE/YOUTH. Le stazioni USA mettono lo stato in `LOCATION`, gli altri `DX`.
- Riga: `QSO: freq mo data ora call rst zona call rst zona t` (t = trasmettitore 0/1 solo per Multi-Single e Multi-Two).

## Penalita' del controllo dei log
- Scambio ricevuto sbagliato: QSO tolto senza penalita'. Nominativo sbagliato o NIL: QSO tolto e penalita' di due volte i punti del QSO.

## Cosa fa Aurora e cosa no
- Punti, doppi per banda, zone (dallo scambio, altrimenti dal database dei paesi), paesi DXCC/WAE per banda, /MM solo zona, punteggio, Cabrillo con `CONTEST: CQ-WW-SSB`.
- Non fa: penalita' NIL e nominativi sbagliati (servono i log degli altri), limiti delle categorie multi-operatore, riga del trasmettitore, overlay e ore del Classic (si scrivono a mano nell'intestazione).

## Differenze rispetto al vecchio `.contest` («CQ WW DX Contest», CW e fonia insieme)
1. Il vecchio file (che resta nell'elenco col suo nome se l'utente lo ha cambiato; quello intatto lo sostituisce la cartella CW) valeva per CW e fonia con le date del 2016 e tutti i modi: ora due cartelle, CW e SSB, con il modo e le date giuste.
2. Le zone venivano dal database dei paesi: un W6 in zona 3 o un UA9 contavano la zona del prefisso. Ora vale la zona ricevuta.
3. I /MM contavano anche come paese: ora solo zona.

## Punti da verificare
- Che il «paese» di un W/VE lavorato dalla propria nazione conti (si', il regolamento lo dice per zona e paese con 0 punti).
- Date 2027 calcolate, non ancora sul sito.
- `LOCATION` obbligatorio per gli USA: Aurora non lo scrive, si aggiunge a mano.
