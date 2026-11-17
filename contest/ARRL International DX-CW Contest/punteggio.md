# ARRL International DX-CW Contest - spiegazione del punteggio

Confronto riga per riga fra `punteggio.regole`, `punteggio.rhai`, la vecchia sezione `[Score]`/`[Script]` di `desktop/src-tauri/contest/ARRL International DX-CW Contest.contest` e il regolamento (versione 2.0).

| Riga | Cosa fa | Regolamento |
|---|---|---|
| `bands=160,80,40,20,15,10` | le sei bande senza WARC (anche nello script, che sostituisce `bands=`) | 2.1 |
| `points=any=3` | 3 punti a ogni QSO valido | 5.1 |
| `mult=exch/band/if:dxcc:K,VE` | stato o provincia dello scambio ricevuto, una volta per banda, solo se il corrispondente e' K o VE: lo contano i DX | 5.2.1, 5.2.3 |
| `dxcc/band/except:K,VE` | entita' DXCC una volta per banda, esclusi K e VE: la contano i W/VE (KH6, KL, CY9, CY0 contano come paesi) | 5.2.1, 5.2.2, 5.2.3.1 |
| `ammesso()` (script) | ammesso solo se esattamente uno dei due, la propria stazione o il corrispondente, e' K/VE; controlla anche le bande | 2.1, 2.3 |

**Una regola per due punti di vista.** Il motore conosce la stazione propria (`my_dxcc`) e il corrispondente (`dxcc`). Per chi e' K o VE ogni QSO ammesso e' con un DX: vale il secondo pezzo di `mult` (il primo non e' mai vero, perche' il corrispondente non e' K/VE). Per un DX ogni QSO ammesso e' con K o VE: vale il primo pezzo (il secondo e' escluso da `except:K,VE`). Le regole non hanno condizioni sulla stazione propria: le prove in `prove.toml` coprono i due casi (stazione K1ABC e stazione IU3ABC, `[[altra_stazione]]`).

**Hawaii e Alaska.** `K` nel database dei paesi sono solo gli Stati Uniti continentali: KH6 (`KH6`) e Alaska (`KL`, non `KL7`) sono altre entita', quindi DX, fuori dall'elenco dello script e contate come paesi (5.2.3.1). Lo stesso vale per KP4, KG4, CY9, CY0.

Punteggio = punti dei QSO x numero totale dei moltiplicatori (somma di quelli di tutte le bande).

## Cosa il motore attuale NON copre

1. **Mobili marittimi e aeronautici fuori da USA e Canada** (OPRG.5.1): per questi il QSO vale solo come contatto (nessun moltiplicatore); il motore non riconosce `/MM` e `/AM` come eccezione ai moltiplicatori.
2. **Penalita' per nominativo errato o NIL** (LGCK.2, 3 punti per QSO): richiede il confronto con i log degli altri.
3. **Limiti di cambio banda** delle categorie MS e M2 (10 e 6 per ora d'orologio) e **trasmettitore** di ogni QSO per M2: non controllati.
4. **Categorie Single Band**: il motore calcola il punteggio su tutte le bande; nelle categorie Single Band contano solo i QSO della banda scelta (HCAT.3.5).
5. Un W/VE che opera da un'entita' diversa dal suo nominativo (6.2), cerchio di 500 m, un solo segnale per banda: regole operative, non di punteggio.
6. Stazione propria sconosciuta (nessun `my_dxcc`): lo script la tratta come DX.

Vedi «Da fare nel motore» in `docs/contest-formato.md`.
