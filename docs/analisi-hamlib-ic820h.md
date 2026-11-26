# IC-820H e IC-821H in Hamlib: cosa fa il codice (letto il 2026-11-26)

Fonte: `rigs/icom/ic820h.c`, `ic821h.c` e `icom.c` del repository Hamlib (LGPL). Non provato con la radio: serve la prova di Vladimir (#83).

## IC-820H (modello Hamlib 3032, quello che usa Aurora oggi)
- Indirizzo CI-V di fabbrica `0x42`, «modo 731» (frequenze a 4 byte BCD), nessun `XCHG`.
- Ha solo `set_freq`, `get_freq`, `set_mode`, `get_mode`, `set_vfo`, memorie e scansione. **Non ha `set_split_vfo`, ne' `set_split_freq`, ne' la banda «Sub»**: per questo `V Sub` risponde `RPRT -9` e Aurora scrive la frequenza di trasmissione con trame CI-V grezze (`rig-control/src/ic820h_civ.rs`).

## IC-821H (stessa famiglia, indirizzo `0x4C`)
- Ha `ic821h_set_split_vfo`: lo split Main/Sub e' considerato modo satellite. Commento nel codice: la radio non permette di leggere se il modo satellite e' acceso; in modo satellite **«l'IC-821 sembra al contrario: impostare Main seleziona Sub e viceversa»** (lo stesso scambio che la prova di Aurora rileva a mano).
- Hamlib tiene in memoria lo stato del modo satellite (`rig_set_cache_satmode`) e tratta Main/Sub nel codice comune Icom (`icom.c`: selezione `07 D0`/`07 D1`, banda principale e secondaria).

## Cosa se ne ricava per Aurora
- L'IC-820H e l'IC-821H parlano lo stesso CI-V per le due bande. **Idea da provare**: per l'IC-820H usare il modello IC-821H di Hamlib con l'indirizzo CI-V `0x42` per avere Main/Sub e split gia' fatti, invece delle trame grezze. Va provata con la radio di Vladimir prima di cambiare qualsiasi cosa.
- SatPC32 (programma di Vladimir) imposta ricezione e trasmissione da se', cambia banda da solo con IC-820/821 e lascia accendere a mano il modo satellite; sondaggio radio consigliato 40-90 ms a 9600 baud.
- Segnalazione analoga in Hamlib (issue 693): l'IC-821H non passa tra Main e Sub con `set_freq` di Gpredict.
