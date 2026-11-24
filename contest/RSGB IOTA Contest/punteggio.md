# RSGB IOTA Contest - spiegazione del punteggio

| Riga | Cosa fa | Regolamento |
|---|---|---|
| `bands=80,40,20,15,10` | le cinque bande senza WARC | Bands |
| `points=any=2` | ripiego se lo script non va: 2 punti per QSO | Scoring |
| `punti()` (script) | il proprio riferimento IOTA e' quello nello scambio inviato: se c'e' (stazione Island) World 5, stesso riferimento 5, altra isola 15; se manca (World) World 2, isola 15 | Scoring |
| `moltiplicatori()` (script) | il riferimento IOTA ricevuto (EU115 o EU-115, il trattino si toglie), una volta per banda e per modo | Scoring |

Punteggio = punti x moltiplicatori, contando ogni banda e ogni modo a parte.

## Cosa il motore NON copre
- Controllo dei riferimenti sull'elenco ufficiale IOTA: vale la parola dello scambio fatta da due lettere del continente (EU, AF, AN, AS, NA, OC, SA) e tre cifre.
- Penalita' di 5 punti per gli errori e perdita del moltiplicatore con un QSO perso.
- Categorie, ore (12/24), regole dei multi-operatore.
