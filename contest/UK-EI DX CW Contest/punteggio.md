# UK-EI DX CW Contest - spiegazione del punteggio

| Riga | Cosa fa | Regolamento |
|---|---|---|
| `bands=80,40,20,15,10` | le cinque bande senza WARC | 3 |
| `points=my_dxcc:G,GM,GW,GI,GD,GJ,GU,EI&hour:0100-0459&...` | stazione UK/EI dalle 01:00 alle 04:59 UTC: doppio dei punti sotto (8, 4, 16, 8 secondo banda e controparte) | 6 |
| `my_dxcc:G,GM,GW,GI,GD,GJ,GU,EI&cont:EU...` | stazione UK/EI con UK/EI o Europa: 4 su 80 e 40 m, 2 sulle altre; con DX: 8 e 4 | 6 |
| `my_cont:EU&dxcc:G,GM,GW,GI,GD,GJ,GU,EI...` | stazione europea (non UK/EI) con UK/EI: 4 e 2; con un europeo 2 e 1; con un DX 4 e 2 | 6 |
| `dxcc:G,GM,GW,GI,GD,GJ,GU,EI&band:80,40=8; ...` | stazione DX con UK/EI: 8 e 4; con un europeo 4 e 2; con un DX 2 e 1 | 6 |
| `mult=dxcc/band/except:G,GM,GW,GI,GD,GJ,GU,EI` | paesi DXCC per banda, UK/EI esclusi | 7 |
| `exch/band/if:dxcc:G,GM,GW,GI,GD,GJ,GU,EI` | codice di distretto dello scambio ricevuto (elenco `dati/distretti.dom`, 155 voci) per banda, solo dai QSO con UK/EI | 7, 9 |

Punteggio = punti x moltiplicatori, contando ogni banda a parte.

## Cosa il motore NON copre
- Penalita' di nominativo e numero (2 QSO equivalenti) e NIL, perdita del moltiplicatore con un QSO perso.
- Segmenti di frequenza, categorie, squadre.
- Per «Europa» vale il continente del `cty.dat`.
