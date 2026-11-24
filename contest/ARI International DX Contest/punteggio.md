# ARI International DX Contest - spiegazione del punteggio

| Riga | Cosa fa | Regolamento |
|---|---|---|
| `bands=80,40,20,15,10` | 80-10 m senza WARC (anche nello script `ammesso`) | 5 |
| `ammesso()` (script) | le stazioni italiane (I, IS, IT9) non si lavorano fra loro: QSO non ammesso | 2 |
| `dxcc:I,IT9,IS=10` | con una stazione italiana: 10 punti | 8 |
| `same_dxcc=0` | proprio paese: 0 punti (conta per i moltiplicatori) | 8 |
| `same_cont=1; any=3` | stesso continente 1, altro continente 3 | 8 |
| `moltiplicatori()` (script) | una provincia italiana per banda (dallo scambio, con l'elenco delle 107) e un paese per banda (DXCC piu' elenco WAE; I, IS e IT9 esclusi) | 7 |

Punteggio = somma dei punti x somma dei moltiplicatori (province + paesi, per banda, sommati).

## Cosa il motore non copre
- Controllo dei log degli altri, categorie multi-operatore (regola dei 10 minuti), SWL, sovrapposizioni Rookie, Youth e ARI IQ (solo categorie del Cabrillo).
- Una stazione e' italiana se il suo paese DXCC e' I o IS o se e' un'entita' WAE IT9. L'Italia africana (IG9, IH9) non e' nell'elenco dei paesi italiani del regolamento: nel database DXCC di Aurora e' trattata come l'Italia e non l'ho provata (da verificare).
- L'elenco ufficiale scrive «OR SS» per la Sardegna: lo leggo come due province (OR e SS), 107 in tutto.
