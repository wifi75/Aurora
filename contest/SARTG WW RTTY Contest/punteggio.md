# SARTG WW RTTY Contest - spiegazione del punteggio

| Riga | Cosa fa | Regolamento |
|---|---|---|
| `bands=80,40,20,15,10` | le cinque bande | Bands |
| `points=same_dxcc=5; same_cont=10; other_cont=15` | punti per QSO | QSO Points |
| `mult=dxcc/band` | ogni paese DXCC una volta per banda (anche K, VE, JA, VK) | Multipliers |
| `area:K,VE,JA,VK/band` | ogni area numerica di W, VE, JA e VK una volta per banda | Multipliers |
| `ammesso()` (script) | i QSO contano solo nei tre periodi: sabato 00-08 e 16-24, domenica 08-16 UTC | Dates and Times |

Punteggio = punti x moltiplicatori. Lo script controlla il giorno della settimana (il sabato e' il giorno 2 modulo 7 dall'epoca Unix) e l'ora UTC.
