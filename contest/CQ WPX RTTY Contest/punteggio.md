# CQ WPX RTTY Contest - spiegazione del punteggio

| Riga | Cosa fa | Regolamento |
|---|---|---|
| `bands=80,40,20,15,10` | le cinque bande (3,5, 7, 14, 21, 28 MHz) | «Bands» |
| `points=same_dxcc=1; same_cont=2; other_cont=3` | stesso paese 1, stesso continente (altro paese) 2, altro continente 3 | «QSO Points» |
| `points_band=80:2,40:2` | su 3,5 e 7 MHz i punti sono doppi (2, 4, 6) | «QSO Points» |
| `mult=wpx` | prefissi WPX, una volta sola | «Prefix Multipliers» |

Punteggio = punti x prefissi diversi. Ogni stazione si lavora una volta per banda; il prefisso conta una volta, qualunque sia la banda.

## Cosa il motore non copre
- Il prefisso dei nominativi portatili e' calcolato dal motore; la regola «designatore senza numero = zero dopo la seconda lettera» (`PA/N8BJQ` = PA0) e i `/MM`, `/M`, `/A`, `/P` che non contano come prefisso vanno controllati sul log.
- «Stesso paese» e' il paese DXCC del database di Aurora (il regolamento letto non cita l'elenco WAE).
- Ore operabili (30 su 48), pause di 60 minuti, cambi di banda dei multi-operatore, penalita' sui nominativi sbagliati: promemoria, non calcolati.
