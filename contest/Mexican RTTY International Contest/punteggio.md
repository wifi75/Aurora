# Mexican RTTY International Contest - spiegazione del punteggio

| Riga | Cosa fa | Regolamento |
|---|---|---|
| `bands=80,40,20,15,10` | le cinque bande da 3,5 a 28 MHz (WARC vietate) | Bands |
| `points=dxcc:XE=4; same_dxcc=2; any=3` | 4 punti con qualunque stazione XE, 2 con lo stesso paese, 3 con un paese diverso | Scoring A |
| `mult=exch/band/if:dxcc:XE` | stato messicano (o Ciudad de Mexico) dello scambio delle stazioni XE, una volta per banda | Scoring B |
| `dxcc/band` | ogni paese DXCC una volta per banda | Scoring B |

Punteggio = somma dei punti x somma dei moltiplicatori (di tutte le bande). Nessuno script.

Dubbio: il regolamento dice «each Mexico states and Ciudad de Mexico (32 entities) and each DXCC country in each band»: letto alla lettera anche XE conta come paese DXCC (oltre allo stato). E' cosi' che il vecchio file lo contava e cosi' resta; da confermare con il manager del contest se il dubbio pesa.

Cosa il motore non copre: il controllo dei log degli altri; SO2R (ammesso, nessun effetto sul punteggio).
