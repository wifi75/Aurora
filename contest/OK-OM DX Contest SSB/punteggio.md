# OK-OM DX Contest SSB - spiegazione del punteggio

| Riga | Cosa fa | Regolamento |
|---|---|---|
| `bands=160,80,40,20,15,10` | 1,8-28 MHz senza WARC | 2 |
| `valid=not_dxcc:UA,UA9,UA2,R1FJ,EU` | Russia (europea, asiatica, Kaliningrad, Franz Josef) e Bielorussia: QSO annullati | 14 |
| `call:MM=5` | /MM: 5 punti a chiunque | 6.3 |
| `my_dxcc:OK,OM&same_dxcc=2` ... `=5` | stazioni OK/OM: stesso paese 2, stesso continente 3, altro continente 5 | 6.1 |
| `dxcc:OK,OM=10` | le altre stazioni con OK/OM: 10 punti | 6.2 |
| `same_dxcc=1; same_cont=3; other_cont=5` | le altre stazioni: proprio paese 1, stesso continente 3, altro continente 5 | 6.2 |
| `moltiplicatori()` (script) | un distretto OK/OM per banda (dallo scambio) e un paese per banda (DXCC piu' elenco WAE), /MM esclusi | 8 |

Punteggio = punti dei QSO x (distretti + paesi, per banda, sommati).

## Cosa il motore non copre
- Penalita' e controllo dei log degli altri (UBN), categorie multi-operatore (regola dei 10 minuti, 8 cambi di banda per ora) e SWL.
- Il paese del proprio nominativo conta come moltiplicatore (OK per un OK, OM per un OM): il regolamento dice «each different country contacted», senza escludere il proprio. Da verificare.
- Il motore usa il continente del paese DXCC per i punti; le entita' WAE senza voce DXCC (IT9, GM/s...) contano solo per i moltiplicatori.
- Un QSO OK-OM vale 3 punti perche' sono due entita' DXCC diverse (nota 6.1).
