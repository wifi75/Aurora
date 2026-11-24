# UBA Spring 2M Contest - spiegazione del punteggio

| Riga | Cosa fa | Regolamento |
|---|---|---|
| `bands=2` | solo 2 m (anche nello script `ammesso`) | 1 |
| `ammesso()` (script) | una stazione estera vale solo con un belga: un QSO fra due stazioni non ON e' non ammesso; un belga vale con tutti | 3, 7 |
| `points=any=3` | 3 punti per ogni QSO valido | 3 |
| `mult=exch/if:dxcc:ON` | ogni sezione UBA dello scambio di un belga (XXX e UBA compresi) | 5 |
| `dxcc/except:ON/if:my_dxcc:ON` | per le stazioni belghe: ogni paese DXCC tranne ON | 5 |

Punteggio = punti x moltiplicatori (la banda e' una, quindi una volta sola).

## Cosa il motore non copre
- Classifica dei club, SWL, controllo dei log (oltre il 5 % di errori: squalifica).
- Segmenti di frequenza (`[[modi.limite]]` e' solo una dichiarazione).
- L'elenco delle sezioni (`dati/sezioni.dom`) e' quello della pagina del regolamento: se ne nascono di nuove, una sezione fuori elenco conta lo stesso come ultima parola non numerica dello scambio.
