# WAE DX Contest CW - spiegazione del punteggio

| Pezzo | Cosa fa | Regolamento |
|---|---|---|
| `bands=80,40,20,15,10` | le cinque bande | 2 |
| `points=any=1` | 1 punto per QSO | 9 |
| `mult_band=80:4,40:3,20:2,15:2,10:2` | pesi dei moltiplicatori, anche quelli dello script | 7 |
| `ammesso()` (script) | bande; solo QSO fra Europa e resto del mondo | 5 |
| `moltiplicatori()` (script) | paesi WAE per i non europei, paesi DXCC e aree per gli europei, per banda | 7 |
| `qtc = 1` (contest.toml) | QTC dalla finestra QTC, 1 punto ciascuno | 8 |

Punteggio = (QSO + QTC) x somma dei moltiplicatori pesati.

## Cosa il motore NON copre
- Regola del 000, limite delle 36 ore, regola dei 10 minuti.
