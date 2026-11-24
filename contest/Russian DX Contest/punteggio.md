# Russian DX Contest - spiegazione del punteggio

| Riga | Cosa fa | Regolamento |
|---|---|---|
| `bands=160,80,40,20,15,10` | le sei bande senza WARC | 2 |
| `points=same_dxcc=2; same_cont=3; any=5` | ripiego se lo script non va: proprio paese 2, proprio continente 3, altro 5 | 7 |
| `mult=dxcc/band; exch/band/if:dxcc:UA,UA9,UA2,R1FJ` | ripiego: paesi per banda e regioni russe dallo scambio per banda | 9 |
| `punti()` (script) | russi: Russia del proprio continente 2, dell'altro 5, altro paese del continente 3, altro continente 5; altri: Russia 10, proprio paese 2, continente 3, altro 5; /MM sempre 5; Franz Josef Land, Kaliningrad e Antartide russa contano come Russia europea | 7.1-7.4 |
| `moltiplicatori()` (script) | paese (WAE se c'e', altrimenti DXCC) e regione russa (sigla dello scambio controllata sull'elenco ufficiale; senza scambio valido KA, FJ o AN per le tre entita' a parte) per banda; i /MM nessuno | 7.3, 7.4, 9 |

Punteggio = punti x (paesi + regioni), contando ogni banda a parte (non il modo).

## Cosa il motore NON copre
- Penalita' (nominativo o numero sbagliato) e controllo con i log degli altri.
- Una stazione russa in Antartide che si usa come propria: il database la vede come Antartide (CE9).
- Una sigla non ufficiale mandata davvero dal corrispondente (accettata dal sito): qui non conta come regione.
- Regola dei 10 minuti, cambi di banda e numerazione per banda dei multi-operatore.
