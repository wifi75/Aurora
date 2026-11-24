# All Asian DX Contest CW - spiegazione del punteggio

| Riga | Cosa fa | Regolamento |
|---|---|---|
| `bands=160,80,40,20,15,10` | le sei bande senza WARC | Bands |
| `points=...` | asiatico con asiatico 3 (160 m), 2 (80 e 10 m), 1 (altre); asiatico con non asiatico 9, 6, 3; non asiatico con asiatico 3, 2, 1 (vince la prima regola vera) | QSO Points |
| `mult=dxcc/band/if:my_cont:AS; wpx/band/if:my_cont:EU,AF,NA,SA,OC` | asiatici: paesi DXCC per banda; gli altri: prefissi WPX per banda | Multipliers |
| `ammesso()` (script) | gli asiatici lavorano tutti tranne il proprio paese; gli altri solo i paesi asiatici (la Turchia europea TA1 non e' Asia); solo le sei bande | Contacts |
| `punti()` (script) | gli stessi punti della riga `points=`, con il continente WAE (TA1 in Europa) | QSO Points |

Punteggio: categorie tutte le bande = (punti di tutte le bande) x (moltiplicatori di tutte le bande); categorie monobanda = punti x moltiplicatori di quella banda.

## Cosa il motore NON copre
- I mobili marittimi in alto mare (solo QSO con l'Asia, punti come fra asiatici ma niente moltiplicatore, classifica a parte): le stazioni /MM non sono riconosciute.
- Le stazioni ausiliarie militari statunitensi in Estremo Oriente (non contano per i non asiatici) e le parti asiatiche di Turchia e Maldive: contano come il database dei paesi le assegna.
- Le categorie JA SOJR e SOSV e il limite delle 24 ore.
