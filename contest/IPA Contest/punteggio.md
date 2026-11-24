# IPA Contest - spiegazione del punteggio

| Riga | Cosa fa | Regolamento |
|---|---|---|
| `bands=80,40,20,15,10` | le cinque bande senza WARC | 2 |
| `points=any=1` | ripiego di 1 punto per QSO (lo script lo sostituisce) | 6 |
| `punti(qso)` (script) | 5 punti se lo scambio ricevuto contiene la parola IPA (stazione IPARC), altrimenti 1 | 6 |
| `moltiplicatori(qso)` (script) | solo dai QSO con una stazione IPARC: paese DXCC per banda, per gli USA lo stato (la parola dello scambio dopo IPA che non e' un numero) per banda | 6 |
| `band_score=1` | punteggio = somma, banda per banda, dei punti x moltiplicatori della banda | 6 |

Doppi: stessa stazione, stessa banda e stesso modo.

## Cosa il motore NON copre
- Segmenti di frequenza (fuori nessun QSO vale), numero progressivo che riparte da 001 a ogni modo, classi A/B/C.
- CW e SSB sono classifiche separate: nello stesso log i punti e i moltiplicatori di una banda si sommano fra i due modi.
- Una stazione IPARC USA senza stato nello scambio non da' moltiplicatore.
