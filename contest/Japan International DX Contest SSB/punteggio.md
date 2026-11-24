# Japan International DX Contest SSB - spiegazione del punteggio

| Pezzo | Cosa fa | Regolamento |
|---|---|---|
| `ammesso()` | solo bande 160-10 m; un JA lavora i DX e i /MM, un DX lavora i JA (JD1 compreso); DX-DX, DX-/MM e JA-JA valgono zero | 2, 5 |
| `punti()` | 160 m 4 punti; 80 e 10 m 2; 40, 20 e 15 m 1 | 5 |
| `moltiplicatori()` per un DX | una prefettura (01-50) per banda, dal numero dello scambio ricevuto | 6 |
| `moltiplicatori()` per un JA | un paese DXCC per banda e una zona CQ per banda (la zona e' quella mandata dal DX, altrimenti quella del database); un /MM vale solo per la zona | 6 |

Punteggio = punti dei QSO x somma dei moltiplicatori (tutte le bande insieme).

## Cosa il motore non copre
- Il regolamento non dice se un QSO JA-JD1 valga punti: qui JD1 sta dalla parte del Giappone (prefetture 48-50 dei DX), quindi JA-JD1 e JD1-JD1 non valgono. «JAs: paesi DXCC (esclusa JD1)» conferma almeno che JD1 non e' un paese moltiplicatore per i JA.
- Il controllo dei log degli altri (open log, QSO sbagliati).
- Le categorie M1 e M2 (regola dei 10 minuti, stazione che gira e dei moltiplicatori) e la colonna della stazione nel log: promemoria.
- Le frequenze consigliate e le porzioni riservate ai JA su 1,8 e 3,5 MHz.
- Okino-Torishima (prefettura 50): per il database dei paesi e' Giappone (JA); un nominativo JD1 si legge come Ogasawara o Minami-Torishima secondo il prefisso.
