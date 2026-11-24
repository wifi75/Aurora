# RSGB 1.8MHz Contest - spiegazione del punteggio

| Riga | Cosa fa | Regolamento |
|---|---|---|
| `bands=160` | solo 160 m | 3 |
| `points=any=2` | 2 punti per ogni QSO | 6 |
| `exch/add:5/if:dxcc:G,GM,GW,GI,GD,GU,GJ` | +5 punti la prima volta che si lavora un distretto UK&CD (dallo scambio ricevuto, elenco `dati/distretti.dom`) | 6 |
| `dxcc/add:5/except:G,GM,GW,GI,GD,GU,GJ` | +5 punti la prima volta che si lavora un paese DXCC fuori dal UK&CD | 6 |

`add:` non moltiplica: da' 5 punti in piu' al QSO che porta il valore nuovo; punteggio = punti QSO + bonus.

## Cosa il motore non copre
- Penalita' di 2 punti per ogni errore e controllo dei log.
- I distretti e i paesi contano una volta sola nella gara, in qualunque modo; il bonus non dipende dal modo (e' cosi' nelle regole: nessun `mode`).
- UK&CD = Inghilterra (G), Scozia (GM), Galles (GW), Irlanda del Nord (GI), Isola di Man (GD), Guernsey (GU), Jersey (GJ); l'Irlanda (EI) e' un paese DXCC a parte e vale il bonus del paese.
