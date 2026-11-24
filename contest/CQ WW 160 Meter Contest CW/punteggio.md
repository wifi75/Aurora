# CQ WW 160-Meter Contest CW - spiegazione del punteggio

| Riga | Cosa fa | Regolamento |
|---|---|---|
| `bands=160` | solo 160 m (1810-2000 kHz in Regione 1, 1800-2000 kHz nelle Regioni 2 e 3) | «Bands» |
| `call:MM=5` | marittimo mobile: 5 punti (e nessun moltiplicatore, perche' non ha paese) | VI |
| `same_dxcc=2` | stesso paese: 2 punti | VI |
| `same_cont=5` | stesso continente, altro paese: 5 punti | VI |
| `any=10` | altro continente: 10 punti | VI |
| `punteggio.rhai` | moltiplicatori per K e VE: stato USA (48 + DC) o provincia canadese dello scambio ricevuto (NWT vale NT); per tutti gli altri l'entita' WAE (IT9, GM/s, 4U1V...) se il database la riconosce, altrimenti il paese DXCC. K e VE non sono paesi; i /MM non sono moltiplicatori; Alaska (KL) e Hawaii (KH6) sono paesi | «Multipliers» |
| `mult=` | la riga di `punteggio.regole` (stato/provincia + paese DXCC) vale solo se lo script manca o fallisce: in quel caso IT9 conta come I | - |

Punteggio = punti dei QSO x (stati e province + paesi). Ogni moltiplicatore conta una volta sola (una banda sola); un paese WAE come IT9 vale a parte dal suo paese DXCC (I). Le zone CQ dei DX sono solo un'indicazione.

## Cosa il motore non copre
- Penalita' per QSO sbagliati (due QSO equivalenti) e controllo dei log degli altri.
- Il regolamento letto non definisce i doppi: si tiene una stazione una sola volta.
- Paesi WAE: contano quelli che il database dei paesi di Aurora porta come entita' WAE; l'elenco del regolamento (IT9 non e' nominato esplicitamente, ma e' la voce «IT» senza IG9/IH9) non e' stato confrontato voce per voce con il database: un'entita' che il database non porta come WAE vale come il suo paese DXCC.
- Alaska e Hawaii come paesi e non come stati: dedotto dal fatto che il regolamento elenca come stati solo i 48 contigui e DC; la pagina non li nomina.
- Ore operabili (30 su 48, multi-op 40), pause minime di 30 minuti, un'unica frequenza di CQ: promemoria, non calcolati.
