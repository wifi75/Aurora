# ARRL Field Day - spiegazione del punteggio

| Riga | Cosa fa | Regolamento |
|---|---|---|
| `bands=160,80,40,20,15,10,6,2,70cm,23cm` | HF senza WARC e 60 m, piu' le bande sopra i 50 MHz che Aurora conosce | 2 |
| `points=mode:SSB,USB,LSB,AM,FM,PHONE=1` | fonia: 1 punto per QSO | 7.1.1 |
| `any=2` | CW e digitale (non CW): 2 punti per QSO | 7.1.2, 7.1.3 |

Il punteggio mostrato da Aurora e' la somma dei punti dei QSO.

## Cosa il motore NON copre
- **Moltiplicatore di potenza** (5, 2 o 1, dalla potenza massima usata): non c'e' nello scambio ne' nel log; si applica sul riepilogo ARRL.
- **Bonus** (alimentazione d'emergenza, stampa, messaggi, satellite, GOTA, giovani, sicurezza...): sono dichiarazioni con prove, non QSO; il motore ha solo bonus per QSO (`bonus=condizione=punti`) e il bonus GOTA da 5 punti per contatto non si riconosce dal log.
- Doppi: tutta la fonia e' un modo solo e tutto il digitale un modo solo, ma il motore confronta il modo scritto (SSB e USB, FT8 e PSK31 sono diversi).
- Classi A-F, GOTA, stazione VHF libera, satellite.
