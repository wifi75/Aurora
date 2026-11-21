# CQ WW RTTY DX Contest - spiegazione del punteggio

| Riga | Cosa fa | Regolamento |
|---|---|---|
| `bands=80,40,20,15,10` | le cinque bande | Bands |
| `points=...` e `mult=...` | ripiego se lo script non va (paesi solo DXCC, zone dal database) | QSO Points, Multipliers |
| `punti()` (script) | stesso paese DXCC/WAE 1, stesso continente 2, altrimenti 3 | QSO Points |
| `moltiplicatori()` (script) | per banda: zona (dallo scambio, altrimenti dal database), paese DXCC/WAE (non per i /MM), QTH dei K e VE fra le 63 sigle | Multipliers |

Punteggio = punti x (zone + paesi + QTH), ogni banda a parte.
