# BARTG HF RTTY Contest - spiegazione del punteggio

| Riga | Cosa fa | Regolamento |
|---|---|---|
| `bands=80,40,20,15,10` | le cinque bande | «Bands» |
| `points=any=1` | 1 punto per QSO, una volta per banda | «Exchange» |
| `mult=dxcc/band` | ogni paese DXCC (anche JA, W, VE, VK) una volta per banda | «Multipliers» |
| `area:JA,K,VE,VK/band` | ogni area di chiamata JA, W, VE, VK una volta per banda | «Multipliers» |
| `cont/product` | continenti una volta sola (al massimo 6), moltiplicano a parte | «Multipliers» |

Punteggio = punti x moltiplicatori x continenti.

## Cosa il motore non copre
- **Segmenti di frequenza**: 80 m 3580-3615, 40 m 7040-7125, 20 m 14070-14125 (14100 kHz +/- 500 Hz vale zero), 15 m 21070-21148, 10 m 28070-28189 kHz; fuori segmento zero punti e zero moltiplicatori. Il motore controlla solo la banda (il limite di frequenza e' per modo, non per banda): sta in `prove.toml` come prova futura.
- Suffisso `/<area>` dei W fuori dalla propria area (`W0XXX/5`): il motore usa l'area dal nominativo; chi non mette il suffisso rende il log un checklog (controllo dell'organizzatore).
- Ore (30 su 48, pause di almeno 3 ore, SOAB6 prime 6 ore), cinque minuti fra un cambio di banda e l'altro nelle classi a radio singola: promemoria.
- La frequenza nel Cabrillo deve essere quella del MARK per chi punta ai primi cinque posti.
