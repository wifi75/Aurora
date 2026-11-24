# HA-DX Contest - spiegazione del punteggio

| Riga | Cosa fa | Regolamento |
|---|---|---|
| `bands=160,80,40,20,15,10` | le sei bande senza WARC | Bands |
| `points=call:MM,AM=2` | mobili marittimi e aeronautici: sempre 2 punti | Points |
| `dxcc:HA=10` | stazione ungherese: 10 punti | Points |
| `same_cont=2; other_cont=5` | stesso continente 2, altro continente 5 | Points |
| `moltiplicatori()` (script) | paese per banda: l'entita' WAE se c'e' (IT9, GM/s, TA1...), altrimenti il paese DXCC; per HA la contea (20 sigle) per banda; niente per /MM e /AM; CW e SSB insieme | Multipliers |
| `mult=dxcc/band/except:HA; exch/band/if:dxcc:HA` | ripiego equivalente senza WAE e senza /MM, usato solo se lo script manca | Multipliers |

Punteggio = somma dei punti x somma dei moltiplicatori di tutte le bande. Doppi: stessa stazione, stessa banda e stesso modo (nelle categorie MIX un QSO in CW e uno in SSB con la stessa stazione sulla stessa banda sono entrambi validi).

## Cosa il motore NON copre
- Penalita' dei QSO sbagliati (il doppio dei punti), controllo dei log, regole del multi-operatore (10 minuti, stazione MULT), categorie e limite delle 12 ore.
- I moltiplicatori dello script non hanno un elenco chiuso di paesi: la scheda dei paesi non mostra quelli che mancano; le contee si controllano contro l'elenco scritto nello script.
