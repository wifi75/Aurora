# Scandinavian Activity Contest CW - spiegazione del punteggio

| Riga | Cosa fa | Regolamento |
|---|---|---|
| `bands=80,40,20,15,10` | le cinque bande senza WARC | Bands |
| `points=my_dxcc:SM,...&cont:EU=2` | stazione scandinava con un europeo: 2 punti | Scoring |
| `my_dxcc:SM,...=3` | stazione scandinava con un non europeo: 3 punti | Scoring |
| `my_cont:EU=1` | stazione non scandinava europea: 1 punto | Scoring |
| `band:80,40=3` | stazione non scandinava non europea su 80 e 40 m: 3 punti | Scoring |
| `any=1` | non europea su 20, 15 e 10 m: 1 punto | Scoring |
| `mult=dxcc/band/if:my_dxcc:...` | scandinavi: paese DXCC per banda | Multipliers |
| `area:.../band/if:not_my_dxcc:...` | non scandinavi: prefisso e cifra dei paesi scandinavi per banda (`LA/G3XYZ` = LA0, `OZ150A` = OZ1) | Multipliers |
| `ammesso()` (script) | vale solo il QSO fra una scandinava e una non scandinava | Aim, Scoring |

Punteggio = punti x moltiplicatori, contando ogni banda a parte.

## Cosa il motore NON copre
- Segmenti di frequenza, categorie, 10 minuti dei multi-operatore, controllo dei log degli altri.
- Il paese `OH0` (Aland) e `OJ0` (Market Reef) sono dentro la lista scandinava: dipendono dal `cty.dat`.
