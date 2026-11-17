# ARRL 10 Meter Contest - spiegazione del punteggio

Confronto riga per riga fra `punteggio.regole`, la sezione `[Score]` di `desktop/src-tauri/contest/ARRL 10 meter Contest.contest` (le tre righe sono identiche) e il regolamento 2026 (versione 2.0).

| Riga | Cosa fa | Regolamento |
|---|---|---|
| `bands=10` | ammette solo i 10 m | 2.3 |
| `points=mode:CW=4; any=2` | CW 4 punti, fonia 2 | 5.1.1, 5.1.2 |
| `mult=exch/mode/if:dxcc:K,VE,XE,KH6,KL7` | stato o provincia dallo scambio, per modo, solo per quei paesi | 5.2.2, 5.2.2.1, 5.2.1 |
| `dxcc/mode/except:K,VE,XE,KH6,KL7` | paese DXCC per modo, tolti K, VE, XE, KH6, KL7 | 5.2.3, 5.2.1 |

Punteggio = punti dei QSO x numero totale dei moltiplicatori (somma di quelli CW e di quelli in fonia), come l'esempio del regolamento (6330 x (83+57) = 886 200).

## Cosa il motore attuale NON copre

1. **Regioni ITU dei mobili marittimi** (moltiplicatore 5.2.4): lo scambio del `/MM` e' 1, 2 o 3; il tipo `exch` guarda l'elenco `.dom` o l'ultima parola non numerica, quindi non le conta.
2. **QSO CW a 28300 kHz o piu'** (6.2): `bands` guarda solo la banda, non la frequenza; il QSO resta ammesso.
3. **Penalita' per nominativo errato o NIL** (LGCK.2, pari ai punti del QSO): richiede il confronto con i log degli altri, lo fanno gli organizzatori.
4. Nominativo che indica l'entita' DXCC (6.3), cerchio di 500 m, un solo segnale per volta: regole operative, non di punteggio.
5. Maritime mobile fuori da USA e Canada: l'art. OPRG.5 generico dice «solo credito di contatto» per i contest dove i W/VE lavorano i DX; resta da chiarire con ARRL se vale per il 10 m, dove le regioni ITU sono moltiplicatori.

Vedi «Da fare nel motore» in `docs/contest-formato.md`.
