# EUDX Contest - spiegazione del punteggio

| Pezzo | Cosa fa | Regolamento |
|---|---|---|
| `bands=160,80,40,20,15,10` | sei bande senza WARC | 1 |
| `punti()` (script) | stati UE (con l'oltremare) 2/10/3/5, fuori UE 10/2/3/5 | 3 |
| `moltiplicatori()` (script) | regione dallo scambio (stato UE + numero valido), paese DXCC/WAE, per banda | 5 |
| `points`, `mult` | ripiego se lo script non va (senza i 10 punti UE) | - |

Punteggio = punti x (regioni + paesi), ogni banda a parte; i doppi sono per banda e modo.

## Cosa il motore NON copre
- Corrispondenza fra regione e nominativo; elenco con i nomi delle regioni.
