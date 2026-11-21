# UBA DX SSB Contest - spiegazione del punteggio

| Riga | Cosa fa | Regolamento |
|---|---|---|
| `bands=80,40,20,15,10` | le cinque bande | Bands |
| `valid=not_dxcc:UA,UA9,UA2,EU` | Russia e Bielorussia (EU nel database dei paesi) fuori | Restrictions |
| `points=...` | belgi: ON 1, UE 2, altri 3; gli altri: ON 10, UE 3, altri 1 | QSO points |
| `mult=exch/...skip:XXX` | sezioni UBA dallo scambio dei belgi, XXX esclusa | Multipliers |
| `mult=wpx/...` | prefissi belgi per i non belgi | Multipliers |
| `mult=dxcc/...` | paesi UE per i non belgi, ogni paese per i belgi | Multipliers |
| `punteggio()` (script) | bonus dei QSO belgi per i non belgi | Bonus |

Punteggio = (punti + bonus) x moltiplicatori, ogni banda a parte.

## Cosa il motore NON copre
- Controllo incrociato dei log (QSO non trovati, scambi sbagliati).
- Elenco chiuso delle sezioni UBA: conta la parola dello scambio.
