# REF Contest CW - spiegazione del punteggio

| Pezzo | Cosa fa | Regolamento |
|---|---|---|
| `bands=80,40,20,15,10` | le cinque bande HF (il 160 m e' a parte) | 3c |
| `ammesso()` (script) | bande; gli stranieri solo con i francesi (e i /MM) | 4 |
| `punti()` (script) | francesi 6/15/1/2, stranieri 1/3, /MM 3 | 6 |
| `moltiplicatori()` (script) | dipartimento dallo scambio, prefisso d'oltremare dal database, paesi DXCC per i francesi, per banda | 7 |
| `points`, `mult` | ripiego se lo script non va (solo i punti degli stranieri) | - |

Punteggio = punti x moltiplicatori, ogni banda a parte.

## Cosa il motore NON copre
- Sotto-parte 160 m, regola del riposo (28 ore) e dei 10 minuti, penalita'.
