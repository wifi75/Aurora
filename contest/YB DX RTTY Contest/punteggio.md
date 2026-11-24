# YB DX RTTY Contest - spiegazione del punteggio

| Riga | Cosa fa | Regolamento |
|---|---|---|
| `bands=80,40,20,15,10` | le cinque bande, senza WARC | Bands |
| `points=dxcc:YB=10; ...` | YB 10 punti; stesso paese 1, stesso continente 2, altro continente 3 | QSO points |
| `mult=wpx/band/if:dxcc:YB` | prefisso (con cifra) delle stazioni YB, per banda | Multipliers |
| `dxcc/band` | ogni paese DXCC per banda (anche l'Indonesia) | Multipliers |

Punteggio = punti x (prefissi YB + paesi), sommati su tutte le bande. Esempio del regolamento: 1000 x (50 + 70) = 120.000.

Non coperto: il regolamento letto vale per le stazioni non indonesiane; per una stazione indonesiana il motore usa comunque 10 su ogni YB.
