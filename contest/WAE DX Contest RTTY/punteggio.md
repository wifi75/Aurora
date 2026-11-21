# WAE DX Contest RTTY - spiegazione del punteggio

| Pezzo | Cosa fa | Regolamento |
|---|---|---|
| `bands=80,40,20,15,10` | le cinque bande | 2 |
| `points=any=1` | 1 punto per QSO | 9 |
| `mult_band=80:4,40:3,20:2,15:2,10:2` | pesi dei moltiplicatori, anche quelli dello script | 7 |
| `ammesso()` (script) | solo le bande: tutti lavorano tutti | 11 |
| `moltiplicatori()` (script) | paesi WAE (europei) e paesi DXCC o aree (non europei) per tutti, per banda | 7, 11 |
| `qtc = 2` (contest.toml) | QTC in tutte e due le direzioni fra continenti diversi, 1 punto ciascuno | 8, 11 |

Punteggio = (QSO + QTC) x somma dei moltiplicatori pesati.

## Cosa il motore NON copre
- Regola del 000, limite delle 36 ore, regola dei 10 minuti.
