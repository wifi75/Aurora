# CQ WW WPX CW Contest - spiegazione del punteggio

| Riga | Cosa fa | Regolamento |
|---|---|---|
| `bands=160,80,40,20,15,10` | le sei bande da 1,8 a 28 MHz, niente WARC | «Bands» |
| `same_dxcc=1` | stesso paese: 1 punto su qualsiasi banda | V.B |
| `other_cont&band:160,80,40=6; other_cont=3` | altro continente: 6 punti su 1,8/3,5/7 MHz, 3 su 14/21/28 MHz | V.B |
| `my_cont:NA&cont:NA&band:160,80,40=4; ...=2` | due stazioni del Nord America (paesi diversi): 4 sulle bande basse, 2 sulle alte | V.B |
| `band:160,80,40=2; any=1` | stesso continente, paesi diversi: 2 sulle bande basse, 1 sulle alte | V.B |
| `mult=wpx` | prefissi WPX, una volta sola in tutto il contest | V.C |

Punteggio = punti dei QSO x numero di prefissi diversi. Ogni stazione si lavora una volta per banda; il prefisso conta una volta, qualunque sia la banda.

## Cosa il motore non copre
- Il prefisso dei nominativi portatili lo calcola il motore (`PA/N8BJQ` = PA0, `KH6/W1AW` = KH6). Il regolamento dice che mobile, marittimo mobile, `/A`, `/E`, `/J`, `/P` e simili non sono prefissi: `DL1ABC/MM` o `DL1ABC/P` contano il prefisso base (DL1), che il regolamento non attribuisce; va controllato sul log (vedi `[[prova_futura]]` in `prove.toml`).
- «Stesso paese» e' il paese DXCC del database di Aurora (il regolamento non cita l'elenco WAE). Il database conta la Sicilia (IT9) come Italia: per un italiano un QSO con IT9 vale come stesso paese, mentre per il DXCC sarebbe un altro paese.
- Ore operabili (36 su 48), pause di 60 minuti, cambi di banda dei multi-operatore, sequenza di numeri per banda, penalita' sui nominativi sbagliati (due volte i punti del QSO): promemoria, non calcolati.
