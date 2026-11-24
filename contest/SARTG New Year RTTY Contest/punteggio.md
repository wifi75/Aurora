# SARTG New Year RTTY Contest - spiegazione del punteggio

| Riga | Cosa fa | Regolamento |
|---|---|---|
| `bands=80,40` | solo 80 e 40 m | Bands |
| `points=any=1` | un punto per QSO | QSO points |
| `dxcc/band/except:SM,LA,OH,OZ,TF,OY,OX,JW,JX,OH0,OJ0` | ogni paese DXCC una volta per banda, tranne i paesi scandinavi | Multipliers |
| `area:SM,LA,OH,OZ,TF,OY,OX,JW,JX,OH0,OJ0/band` | in Scandinavia conta ogni prefisso con cifra una volta per banda (`SI3`, `SK3`, `SL3`, `SM3`, `7S3` e `8S3` sono la stessa area SM3; `OZ150A` e' OZ1; `LA/G3XYZ` e' LA0) | Multipliers |

Punteggio = punti x moltiplicatori (somma su tutte le bande). Nessuno script.

Cosa il motore non copre: il limite di un cambio di banda ogni cinque minuti (classe A) e il passaggio alla classe B; il controllo del nome ricevuto (senza nome o numero il log e' un check log).
