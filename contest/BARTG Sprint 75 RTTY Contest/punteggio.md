# BARTG Sprint75 Contest - spiegazione del punteggio

| Riga | Cosa fa | Regolamento |
|---|---|---|
| `bands=80,40,20,15,10` | le cinque bande | «Bands» |
| `points=any=1` | 1 punto per QSO, una volta per banda | «Exchange» |
| `mult=dxcc` | ogni paese DXCC (anche JA, W, VE, VK), una volta sola | «Multipliers» |
| `area:JA,K,VE,VK` | ogni area di chiamata JA, W, VE, VK, una volta sola | «Multipliers» |
| `cont/product` | continenti una volta sola (al massimo 6), moltiplicano a parte | «Multipliers» |

Punteggio = punti x moltiplicatori x continenti. Come lo Sprint di gennaio e a differenza dell'HF RTTY: qui i moltiplicatori contano una volta sola, non per banda.

## Cosa il motore non copre
- **Segmenti di frequenza** per banda (80 m 3580-3615, 40 m 7040-7125, 20 m 14070-14125 con 14100 +/- 0,5 kHz a zero, 15 m 21070-21148, 10 m 28070-28189 kHz): il motore controlla solo la banda.
- Suffisso `/<area>` dei W fuori dalla propria area; cinque minuti fra un cambio di banda e l'altro nelle classi a radio singola: promemoria.
