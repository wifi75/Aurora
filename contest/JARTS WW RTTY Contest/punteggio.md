# JARTS WW RTTY Contest - spiegazione del punteggio

| Riga | Cosa fa | Regolamento |
|---|---|---|
| `bands=80,40,20,15,10` | le cinque bande (3,5-28 MHz) | Bands |
| `points=same_cont=2; other_cont=3` | 2 punti nello stesso continente (stesso paese compreso), 3 fra continenti diversi | QSO points |
| `dxcc/band/except:JA,K,VE,VK` | ogni entita' DXCC una volta per banda, senza JA, W, VE, VK (in cty.dat gli Stati Uniti sono `K`); JD1 e' un'entita' a se' | Multipliers |
| `area:JA,K,VE,VK/band` | ogni area numerica di JA, W, VE e VK una volta per banda (`JA1`, `K1`, `VE3`, `VK2`; `K5ABC/1` e' area 1) | Multipliers |
| `band_score=1` | punteggio = somma, banda per banda, dei punti della banda x moltiplicatori della banda | Scoring |

Cosa il motore non copre: le stazioni marittime mobili in alto mare (2 punti e non moltiplicatori); il caso `VK/JA1YRL` (area VK0 come sola area di chiamata); la differenza fra JD1 di Ogasawara (Asia) e di Minami-tori (Oceania) e' gia' nei dati dei paesi. Il regolamento scrive la formula per ogni banda e non dice espressamente come si sommano le bande: Aurora le somma (da verificare).
