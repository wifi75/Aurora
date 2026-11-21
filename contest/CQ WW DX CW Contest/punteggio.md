# CQ WW DX CW Contest - spiegazione del punteggio

| Riga | Cosa fa | Regolamento |
|---|---|---|
| `bands=160,80,40,20,15,10` | le sei bande senza WARC | Bands |
| `points=same_dxcc=0; my_cont:NA&cont:NA=2; same_cont=1; any=3` | stesso paese 0, Nord America fra paesi diversi 2, stesso continente 1, altri 3 (vince la prima vera) | QSO Points |
| `mult=cq/band; dxcc/band` | ripiego se lo script non va: zone del database e paesi DXCC per banda | Multipliers |
| `punti()` (script) | come la riga `points`, ma con il paese DXCC+WAE (I e IT9 sono paesi diversi: 1 punto) | QSO Points |
| `moltiplicatori()` (script) | zona = primo numero 1-40 dello scambio ricevuto (altrimenti il database), paese dal database (le voci WAE come IT9 e GM/s sono paesi a parte), i /MM solo zona | Multipliers |

Punteggio = punti x (zone + paesi), contando ogni banda a parte.

## Cosa il motore NON copre
- Penalita' NIL e nominativi sbagliati.
- Paesi del Nord America decisi dal continente del database dei paesi (`NA`): coincide con i confini continentali della lista DXCC.
