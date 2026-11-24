# Holyland DX Contest - spiegazione del punteggio

| Riga | Cosa fa | Regolamento |
|---|---|---|
| `bands=80,40,20,15,10` | le cinque bande senza WARC | Bands |
| `points=call:MM=4` | mobili marittimi: 4 punti | Scoring |
| `my_dxcc:4X&dxcc:4X=1` | fra due stazioni 4X/4Z: 1 punto | Scoring (4X/4Z) |
| `dxcc:4X=8` | fuori da Israele con una stazione 4X/4Z: 8 punti | Scoring |
| `my_dxcc:4X&other_cont=8` | stazione 4X/4Z con il DX: 8 punti | Scoring (4X/4Z) |
| `same_dxcc=1; same_cont=2; other_cont=4` | stesso paese 1, stesso continente 2, altro continente 4 | Scoring |
| `exch/band/if:dxcc:4X&not_my_dxcc:4X` | Area dello scambio (elenco `dati/aree.dom`) per banda, fuori da Israele | Multipliers |
| `exch/if:dxcc:4X&my_dxcc:4X` | Area dello scambio, una volta sola, per le stazioni 4X/4Z | Multipliers |
| `dxcc/band` | paese DXCC per banda, CW e SSB insieme (Israele compresa) | Multipliers |

Punteggio = somma dei punti x somma dei moltiplicatori di tutte le bande. Doppi: stessa stazione, stessa banda e stesso modo.

## Cosa il motore NON copre
- Segmenti di frequenza (CW contro SSB), penalita' e QSO annullati dal controllo dei log, regole del MOST, categorie.
- Il moltiplicatore del paese dei `/MM` (il regolamento non lo conta): il motore non riconosce `/MM` come eccezione ai moltiplicatori.
- Un'Area che non e' nell'elenco ufficiale `dati/aree.dom` non conta come moltiplicatore.
