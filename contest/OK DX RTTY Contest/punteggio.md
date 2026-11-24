# OK DX RTTY Contest - spiegazione del punteggio

| Riga | Cosa fa | Regolamento |
|---|---|---|
| `bands=80,40,20,15,10` | le cinque bande | Bands |
| `points=band:80,40&same_cont=3; band:80,40=6; same_cont=1; any=2` | 3 / 6 punti su 80 e 40 m, 1 / 2 su 20, 15 e 10 m, secondo il continente | Points |
| `mult=dxcc/band` | paesi DXCC una volta per banda | Multipliers |
| `call/band/if:dxcc:OK&not_my_dxcc:OK` | ogni stazione OK una volta per banda, solo per chi non e' OK/OL | Multipliers |

Punteggio = punti x moltiplicatori (paesi + stazioni OK, sommati su tutte le bande). Nessuno script.

Cosa il motore non copre: il confronto con i log degli altri (QSO con chi non ha mandato il log), il limite di un cambio di banda ogni 5 minuti.
