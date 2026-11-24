# CQ WW 160-Meter Contest CW - spiegazione del punteggio

| Riga | Cosa fa | Regolamento |
|---|---|---|
| `bands=160` | solo 160 m (1810-2000 kHz in Regione 1, 1800-2000 kHz nelle Regioni 2 e 3) | «Bands» |
| `call:MM=5` | marittimo mobile: 5 punti (e nessun moltiplicatore, perche' non ha paese) | VI |
| `same_dxcc=2` | stesso paese: 2 punti | VI |
| `same_cont=5` | stesso continente, altro paese: 5 punti | VI |
| `any=10` | altro continente: 10 punti | VI |
| `exch/if:dxcc:K,VE` | stato USA (48 + DC) o provincia canadese dello scambio ricevuto, solo se il corrispondente e' K o VE | «Multipliers» |
| `dxcc/except:K,VE` | paese DXCC/WAE, esclusi K e VE (che valgono per stato e provincia); Alaska (KL) e Hawaii (KH6) sono paesi | «Multipliers» |

Punteggio = punti dei QSO x (stati e province + paesi). Ogni moltiplicatore conta una volta sola (una banda sola). Le zone CQ dei DX sono solo un'indicazione.

## Cosa il motore non copre
- Penalita' per QSO sbagliati (due QSO equivalenti) e controllo dei log degli altri.
- Il regolamento letto non definisce i doppi: si tiene una stazione una sola volta.
- Paesi WAE (IT9, GM/s, 4U1V...): il motore li tratta come paesi DXCC, quindi IT9 conta separato da I; le entita' WAE senza voce DXCC non sono distinte.
- Alaska e Hawaii come paesi e non come stati: dedotto dal fatto che il regolamento elenca come stati solo i 48 contigui e DC; la pagina non li nomina.
- Ore operabili (30 su 48, multi-op 40), pause minime di 30 minuti, un'unica frequenza di CQ: promemoria, non calcolati.
