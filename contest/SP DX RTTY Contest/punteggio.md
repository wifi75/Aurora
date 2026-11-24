# SP DX RTTY Contest - spiegazione del punteggio

| Riga | Cosa fa | Regolamento |
|---|---|---|
| `bands=80,40,20,15,10` | le cinque bande, senza WARC | Bands |
| `points=dxcc:UA,UA2,UA9,EU=0; same_dxcc=2; same_cont=5; any=10` | 0 punti con UA ed EW (in cty.dat: UA, UA2, UA9 e la Bielorussia, prefisso `EU`); 2 punti stesso paese, 5 stesso continente, 10 altri continenti | QSO points |
| `dxcc/band/except:UA,UA2,UA9,EU` | ogni paese DXCC una volta per banda, senza UA ed EW | Multipliers |
| `exch/band/if:dxcc:SP` | ogni powiat (sigla di due lettere) una volta per banda, solo per i QSO con la Polonia | Multipliers |
| `cont/product` | ogni continente lavorato una volta sola (massimo 6), moltiplica a parte | Multipliers, Final score |

Punteggio = punti x (paesi + powiat di tutte le bande) x continenti. Nessuno script.

Cosa il motore non copre: il continente di un QSO con UA/EW vale comunque come moltiplicatore (il regolamento esclude UA ed EW solo dai paesi: da verificare); il limite di 380 powiat per banda e' implicito; la verifica delle sigle dei powiat sull'elenco ufficiale (il PDF «Lista powiatow» non e' stato letto).
