# Alessandro Volta RTTY DX Contest - spiegazione del punteggio

| Riga | Cosa fa | Regolamento |
|---|---|---|
| `bands=80,40,20,15,10` | le cinque bande | Bands |
| `valid=other_dxcc` | ripiego se lo script non va: stesso paese non valido | Contacts |
| `ammesso()` (script) | banda e QSO fra stazioni dello stesso paese non validi; JA, K, VE, VK e ZL sono lasciati validi perche' conta l'area | Contacts |
| `points=other_cont&band:80,10=cqtab*2; any=cqtab` | tabella delle zone (`cq1`...`cq40`), doppio fuori continente su 80 e 10 m | Exchange Points Table |
| `mult=dxcc/band/except:JA,VE,VK,K,ZL; area:JA,VE,VK,K,ZL/band` | paesi per banda e, per i 5 paesi con aree, solo l'area | Countries Multipliers |
| `punteggio()` (script) | QSO x punti x (moltiplicatori + paesi di altro continente lavorati su 4 bande) | Final score |
| `times_qso=1` | ripiego dello script: moltiplica anche per i QSO validi | Final score |

Cosa il motore non copre:
- **Stessa area** di JA, K, VE, VK, ZL: il regolamento la vieta (JA2 con JA2), ma il motore non conosce l'area della stazione propria e non la controlla.
- **«3 altri log»**: un QSO conta solo se la stazione compare in almeno 3 altri log o ha mandato il proprio: serve il confronto con i log degli altri, fuori portata.
- Le zone dei punti sono quelle del database dei paesi (cty.dat), non quelle scritte nello scambio.
- La tabella dei punti e' quella «REV. 2004» del sito; nella cella zona 21 - zona 38 il valore e' 19 (il vecchio file `.contest` aveva 10).

## Cosa il motore non copre
- Il Cabrillo di Aurora non scrive i punti del QSO in fondo alla riga come chiede il sito del contest, e il controllo di conformita' e' quello del sito (caricamento web).
