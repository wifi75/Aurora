# IARU HF World Championship - spiegazione del punteggio

| Riga | Cosa fa | Regolamento |
|---|---|---|
| `bands=160,80,40,20,15,10` | le sei bande senza WARC | 2.1 |
| `points=any=1` | ripiego se lo script non va: 1 punto per QSO | 5.1 |
| `mult=itu/band` | ripiego: zone ITU del database dei paesi per banda | 5.2 |
| `punti()` (script) | HQ e ufficiali 1; stessa zona ITU 1; stesso continente (altra zona) 3; altrimenti 5. La propria zona e' il numero dello scambio mandato, altrimenti quella del database | 5.1.1-5.1.5 |
| `moltiplicatori()` (script) | zona ITU ricevuta (1-90) per banda; una sigla (ARRL, DARC, R1, AC, IARU...) e' un moltiplicatore `hq:` a parte; senza scambio vale la zona del database | 5.2.1, 5.2.2 |

Punteggio = punti x (zone + sigle), contando ogni banda a parte (non il modo).

## Cosa il motore NON copre
- Penalita' NIL e nominativi sbagliati; regola dei 10 minuti dei multi-operatore.
- Il limite di quattro moltiplicatori degli ufficiali: una sigla qualsiasi diversa e' contata come moltiplicatore (un errore di battitura vale uno in piu').
- Una stazione HQ che manda la sigla sbagliata o una zona nello stesso continente: vale cio' che e' scritto nello scambio.
