# Macro dei contest: versione scelta

Per ogni contest dei modi testuali si e' confrontata la macro di Aurora (`desktop/src-tauri/contest/macros/`) con una versione nuova,
con questi criteri: tutti i passi del QSO di contest (CQ, scambio, TU con salvataggio del QSO, AGN, QSO gia' fatto), scambio giusto per il
regolamento (`docs/contest-regole.md`), solo comandi che Aurora conosce (`app/src/lib/macroLang.ts`), tasti in ordine, nessun `<EXEC:...>`
e nessun dato personale.

Esito: le macro di Aurora vincono quasi sempre, perche' le versioni nuove si appoggiano a comandi che Aurora non esegue (richiami di altri
tasti, scorciatoie del nominativo precedente) e spesso mancano del TU con salvataggio o del QSO gia' fatto. Due sono unite, i 14 contest
nuovi hanno un file `.contest` e una macro scritta da capo con l'ordine dei tasti della versione nuova e i comandi di Aurora.

| Contest | Versione | Motivo |
|---|---|---|
| 10-Meter RTTY Contest | Aurora | completa e con i tasti di contest di Aurora; la nuova usa 11 comandi che Aurora non conosce |
| Africa All Mode International DX Contest | Aurora | completa e con i tasti di contest di Aurora; la nuova usa 13 comandi che Aurora non conosce |
| Alesandro Volta RTTY Contest | Aurora | completa e con i tasti di contest di Aurora; la nuova usa 9 comandi che Aurora non conosce |
| All Asian DX Contest | Aurora | completa e con i tasti di contest di Aurora; la nuova non ha TU con salvataggio, QSO B4 e usa 9 comandi che Aurora non conosce |
| ARI International DX Contest | Aurora | completa e con i tasti di contest di Aurora; la nuova usa 10 comandi che Aurora non conosce |
| Arktika Cup Digital | Aurora | completa e con i tasti di contest di Aurora; la nuova usa 13 comandi che Aurora non conosce |
| ARRL 10 meter Contest | Aurora | completa e con i tasti di contest di Aurora; la nuova non ha TU con salvataggio, QSO B4 e usa 9 comandi che Aurora non conosce |
| ARRL International DX-CW Contest | Aurora | completa e con i tasti di contest di Aurora; la nuova non ha TU con salvataggio, QSO B4 e usa 9 comandi che Aurora non conosce |
| ARRL International DX-SSB Contest | Aurora | completa e con i tasti di contest di Aurora; la nuova non ha CQ, scambio, TU con salvataggio, QSO B4 e usa 2 comandi che Aurora non conosce |
| ARRL RTTY ROUNDUP | Aurora | completa e con i tasti di contest di Aurora; la nuova usa 14 comandi che Aurora non conosce |
| Baltic Contest | Aurora | completa e con i tasti di contest di Aurora; la nuova non ha TU con salvataggio, QSO B4 e usa 9 comandi che Aurora non conosce |
| BARTG HF RTTY | Aurora | completa e con i tasti di contest di Aurora; la nuova usa 9 comandi che Aurora non conosce |
| BARTG Sprint 75 RTTY | Aurora | completa e con i tasti di contest di Aurora; la nuova usa 9 comandi che Aurora non conosce |
| BARTG Sprint RTTY | Aurora | completa e con i tasti di contest di Aurora; la nuova usa 9 comandi che Aurora non conosce |
| CBNR_CW (Contest Riachuelo Naval Battle) | Aurora | completa e con i tasti di contest di Aurora; la nuova non ha TU con salvataggio, QSO B4 e usa 10 comandi che Aurora non conosce |
| CBNR_SSB (Contest Riachuelo Naval Battle) | Aurora | completa e con i tasti di contest di Aurora; la nuova non ha CQ, scambio, TU con salvataggio, QSO B4 e usa 2 comandi che Aurora non conosce |
| CDX Test Independence Day | Aurora | completa e con i tasti di contest di Aurora; la nuova usa 7 comandi che Aurora non conosce |
| CQ BDM RTTY WW Contest | Aurora | completa e con i tasti di contest di Aurora; la nuova usa 9 comandi che Aurora non conosce |
| CQ WPX RTTY Contest | Aurora | completa e con i tasti di contest di Aurora; la nuova usa 9 comandi che Aurora non conosce |
| CQ WW 160 meter contest | Aurora | completa e con i tasti di contest di Aurora; la nuova non ha TU con salvataggio, QSO B4 e usa 9 comandi che Aurora non conosce |
| CQ WW DX Contest | Aurora | completa e con i tasti di contest di Aurora; la nuova non ha TU con salvataggio, QSO B4 e usa 9 comandi che Aurora non conosce |
| CQ WW RTTY DX | Aurora | completa e con i tasti di contest di Aurora; la nuova usa 11 comandi che Aurora non conosce |
| CQ WW WPX CW Contest | Aurora | completa e con i tasti di contest di Aurora; la nuova non ha TU con salvataggio, QSO B4 e usa 9 comandi che Aurora non conosce |
| CQ-M Contest | Aurora | completa e con i tasti di contest di Aurora; la nuova non ha TU con salvataggio, QSO B4 e usa 9 comandi che Aurora non conosce |
| CQ-MM-DX | Aurora | completa e con i tasti di contest di Aurora; la nuova non ha TU con salvataggio, QSO B4 e usa 9 comandi che Aurora non conosce |
| DigiFest Contest | Aurora | completa e con i tasti di contest di Aurora; la nuova usa 9 comandi che Aurora non conosce |
| DMC RTTY Contest | Aurora | completa e con i tasti di contest di Aurora; la nuova usa 9 comandi che Aurora non conosce |
| DX Expedition | Aurora | completa e con i tasti di contest di Aurora; la nuova non ha TU con salvataggio, QSO B4 e usa 10 comandi che Aurora non conosce |
| EA PSK63 Contest | Aurora | completa e con i tasti di contest di Aurora; la nuova usa 9 comandi che Aurora non conosce |
| EA RTTY Contest | Aurora | completa e con i tasti di contest di Aurora; la nuova usa 9 comandi che Aurora non conosce |
| EPC Ukraine DX Contest | Aurora | completa e con i tasti di contest di Aurora; la nuova usa 14 comandi che Aurora non conosce |
| EU-HF Contest | Aurora | completa e con i tasti di contest di Aurora; la nuova non ha TU con salvataggio, QSO B4 e usa 9 comandi che Aurora non conosce |
| Eurasia Championship | unione (contest nuovo) | contest nuovo: tasti in ordine (F1 CQ, F2 scambio, F3 TU) dalla nuova, scritti solo con comandi di Aurora; date e `[Score]` da fare |
| Fieldday Contest | Aurora | completa e con i tasti di contest di Aurora; la nuova non ha QSO B4 e usa 9 comandi che Aurora non conosce |
| French BPSK Contest | Aurora | completa e con i tasti di contest di Aurora; la nuova usa 8 comandi che Aurora non conosce |
| French HF Championship | Aurora | completa e con i tasti di contest di Aurora; la nuova non ha TU con salvataggio, QSO B4 e usa 9 comandi che Aurora non conosce |
| GACW WWSA CW DX Contest | unione (contest nuovo) | contest nuovo: tasti in ordine (F1 CQ, F2 scambio, F3 TU) dalla nuova, scritti solo con comandi di Aurora; date e `[Score]` da fare |
| HA-DX Contest  SSB-CW Contest | Aurora | completa e con i tasti di contest di Aurora; la nuova non ha TU con salvataggio, QSO B4 e usa 9 comandi che Aurora non conosce |
| Helvetia Contest | Aurora | completa e con i tasti di contest di Aurora; la nuova usa 9 comandi che Aurora non conosce |
| Holyland-DX Contest | Aurora | completa e con i tasti di contest di Aurora; la nuova usa 9 comandi che Aurora non conosce |
| HPC World Wide DX Contest | Aurora | completa e con i tasti di contest di Aurora; la nuova usa 9 comandi che Aurora non conosce |
| HSC-CW Contest | Aurora | completa e con i tasti di contest di Aurora; la nuova non ha TU con salvataggio, QSO B4 e usa 10 comandi che Aurora non conosce |
| IAFA Contest | unione (contest nuovo) | contest nuovo: tasti in ordine (F1 CQ, F2 scambio, F3 TU) dalla nuova, scritti solo con comandi di Aurora; date e `[Score]` da fare |
| IARU HF World-Championship-CW-SSB Contest | Aurora | completa e con i tasti di contest di Aurora; la nuova non ha TU con salvataggio, QSO B4 e usa 9 comandi che Aurora non conosce |
| IGRY WW RTTY Contest | unione (contest nuovo) | contest nuovo: tasti in ordine (F1 CQ, F2 scambio, F3 TU) dalla nuova, scritti solo con comandi di Aurora; date e `[Score]` da fare |
| International  Bucuresti Contest | Aurora | completa e con i tasti di contest di Aurora; la nuova usa 9 comandi che Aurora non conosce |
| Jakarta RTTY Contest | unione (contest nuovo) | contest nuovo: tasti in ordine (F1 CQ, F2 scambio, F3 TU) dalla nuova, scritti solo con comandi di Aurora; date e `[Score]` da fare |
| Japan International DX CW Contest | Aurora | completa e con i tasti di contest di Aurora; la nuova non ha TU con salvataggio, QSO B4 e usa 9 comandi che Aurora non conosce |
| Japan International DX SSB Contest | Aurora | completa e con i tasti di contest di Aurora; la nuova non ha CQ, scambio, TU con salvataggio, QSO B4 e usa 2 comandi che Aurora non conosce |
| JARTS RTTY Contest | Aurora | completa e con i tasti di contest di Aurora; la nuova usa 10 comandi che Aurora non conosce |
| KCJ Contest | Aurora | completa e con i tasti di contest di Aurora; la nuova non ha TU con salvataggio, QSO B4 e usa 9 comandi che Aurora non conosce |
| King of Spain CW Contest | Aurora | completa e con i tasti di contest di Aurora; la nuova non ha TU con salvataggio, QSO B4 e usa 9 comandi che Aurora non conosce |
| Makrothen Contest  | Aurora | completa e con i tasti di contest di Aurora; la nuova usa 10 comandi che Aurora non conosce |
| Marconi Memorial Contest | Aurora | completa e con i tasti di contest di Aurora; la nuova non ha TU con salvataggio, QSO B4 e usa 9 comandi che Aurora non conosce |
| Mexican RTTY Contest | Aurora | completa e con i tasti di contest di Aurora; la nuova usa 14 comandi che Aurora non conosce |
| NAURYZ DX Contest | Aurora | completa e con i tasti di contest di Aurora; la nuova non ha TU con salvataggio, QSO B4 e usa 9 comandi che Aurora non conosce |
| NDG Contest | Aurora | completa e con i tasti di contest di Aurora; la nuova usa 9 comandi che Aurora non conosce |
| OK DX RTTY Contest | Aurora | completa e con i tasti di contest di Aurora; la nuova usa 10 comandi che Aurora non conosce |
| OK-OM-DX-CW Contest | Aurora | completa e con i tasti di contest di Aurora; la nuova non ha TU con salvataggio, QSO B4 e usa 10 comandi che Aurora non conosce |
| OK-OM-DX-SSB Contest | Aurora | completa e con i tasti di contest di Aurora; la nuova non ha CQ, scambio, TU con salvataggio, QSO B4 e usa 2 comandi che Aurora non conosce |
| ON 10 Meter Contest | Aurora | completa e con i tasti di contest di Aurora; la nuova non ha TU con salvataggio, QSO B4 e usa 9 comandi che Aurora non conosce |
| OPEN UKRAINE RTTY CHAMPIONSHIP | Aurora | completa e con i tasti di contest di Aurora; la nuova usa 13 comandi che Aurora non conosce |
| PDC Contest | unione (contest nuovo) | contest nuovo: tasti in ordine (F1 CQ, F2 scambio, F3 TU) dalla nuova, scritti solo con comandi di Aurora; date e `[Score]` da fare |
| PGA-DIGI Contest | Aurora | completa e con i tasti di contest di Aurora; la nuova usa 11 comandi che Aurora non conosce |
| PSK-63 Marathon contest | Aurora | completa e con i tasti di contest di Aurora; la nuova usa 10 comandi che Aurora non conosce |
| R3A-CUP-DIGI | Aurora | completa e con i tasti di contest di Aurora; la nuova usa 10 comandi che Aurora non conosce |
| RAEM contest | unione | la struttura di Aurora con le coordinate `<MYQTH>` aggiunte allo scambio, come vuole il regolamento |
| RDA Contest | Aurora | completa e con i tasti di contest di Aurora; la nuova non ha TU con salvataggio, QSO B4 e usa 9 comandi che Aurora non conosce |
| RTTYOPS WW DX Contest | unione (contest nuovo) | contest nuovo: tasti in ordine (F1 CQ, F2 scambio, F3 TU) dalla nuova, scritti solo con comandi di Aurora; date e `[Score]` da fare |
| RUSSIAN DX Contest | Aurora | completa e con i tasti di contest di Aurora; la nuova non ha TU con salvataggio, QSO B4 e usa 9 comandi che Aurora non conosce |
| RUSSIAN RADIO WW RTTY CONTEST | Aurora | completa e con i tasti di contest di Aurora; la nuova usa 10 comandi che Aurora non conosce |
| Russian WW MultiMode Contest | Aurora | completa e con i tasti di contest di Aurora; la nuova usa 9 comandi che Aurora non conosce |
| Russian WW PSK Contest | Aurora | completa e con i tasti di contest di Aurora; la nuova usa 9 comandi che Aurora non conosce |
| SARTG New Year RTTY Contest | unione | la struttura di Aurora con `<MYNAME> HNY` al posto di un nome scritto a mano nello scambio (numero, nome, auguri) |
| SARTG WW RTTY Contest | Aurora | completa e con i tasti di contest di Aurora; la nuova usa 9 comandi che Aurora non conosce |
| Scandinavian Activity Contest | Aurora | completa e con i tasti di contest di Aurora; la nuova non ha TU con salvataggio, QSO B4 e usa 10 comandi che Aurora non conosce |
| SPDX RTTY Contest | Aurora | completa e con i tasti di contest di Aurora; la nuova usa 9 comandi che Aurora non conosce |
| Stew Perry Topband Distance Challenge | Aurora | completa e con i tasti di contest di Aurora; la nuova non ha TU con salvataggio, QSO B4 e usa 9 comandi che Aurora non conosce |
| TRC DIGI Contest | unione (contest nuovo) | contest nuovo: tasti in ordine (F1 CQ, F2 scambio, F3 TU) dalla nuova, scritti solo con comandi di Aurora; date e `[Score]` da fare |
| TRC DX Contest | Aurora | completa e con i tasti di contest di Aurora; la nuova non ha TU con salvataggio, QSO B4 e usa 9 comandi che Aurora non conosce |
| UA2 QSO Party | Aurora | completa e con i tasti di contest di Aurora; la nuova non ha TU con salvataggio, QSO B4 e usa 9 comandi che Aurora non conosce |
| UBA CW Contest | Aurora | completa e con i tasti di contest di Aurora; la nuova non ha TU con salvataggio, QSO B4 e usa 9 comandi che Aurora non conosce |
| UBA PSK63 Prefix Contest | Aurora | completa e con i tasti di contest di Aurora; la nuova usa 9 comandi che Aurora non conosce |
| UBA Spring 80M CW Contest | Aurora | completa e con i tasti di contest di Aurora; la nuova non ha TU con salvataggio, QSO B4 e usa 9 comandi che Aurora non conosce |
| UK_EI DX Contest | Aurora | completa e con i tasti di contest di Aurora; la nuova non ha TU con salvataggio, QSO B4 e usa 9 comandi che Aurora non conosce |
| Ukrainian DX Contest | Aurora | completa e con i tasti di contest di Aurora; la nuova non ha TU con salvataggio, QSO B4 e usa 10 comandi che Aurora non conosce |
| Ukrainian DX DIGI Contest | Aurora | completa e con i tasti di contest di Aurora; la nuova usa 9 comandi che Aurora non conosce |
| Venezuelan Independence Contest | Aurora | completa e con i tasti di contest di Aurora; la nuova usa 9 comandi che Aurora non conosce |
| WAEDC RTTY Contest | Aurora | completa e con i tasti di contest di Aurora; la nuova usa 10 comandi che Aurora non conosce |
| YB DX Contest | unione (contest nuovo) | contest nuovo: tasti in ordine (F1 CQ, F2 scambio, F3 TU) dalla nuova, scritti solo con comandi di Aurora; date e `[Score]` da fare |
| YB DX RTTY Contest | unione (contest nuovo) | contest nuovo: tasti in ordine (F1 CQ, F2 scambio, F3 TU) dalla nuova, scritti solo con comandi di Aurora; date e `[Score]` da fare |
| YO DX HF Contest | Aurora | completa e con i tasti di contest di Aurora; la nuova non ha TU con salvataggio, QSO B4 e usa 9 comandi che Aurora non conosce |
| YU-DX-HF Contest | Aurora | completa e con i tasti di contest di Aurora; la nuova non ha TU con salvataggio, QSO B4 e usa 9 comandi che Aurora non conosce |

## Varianti per modo non aggiunte

Le varianti CW, SSB e DIGI dei contest gia' presenti non sono state aggiunte: la macro di Aurora del contest vale in tutti i modi testuali
(PSK, RTTY, CW) ed e' piu' completa; le varianti SSB non hanno testo da trasmettere (CQ e scambio sono registrazioni vocali) e le CW
mancano del TU con salvataggio e del QSO gia' fatto. Lo stesso vale per le varianti dei contest nuovi (soci/non soci di PDC e TRC DIGI:
il suffisso si mette nello scambio, `m_sNrs`). File non aggiunti: 44.

| File | |
|---|---|
| ARI International DX Contest CW | non aggiunta |
| ARI International DX Contest SSB | non aggiunta |
| ARRL 10 meter Contest SSB | non aggiunta |
| Africa All Mode International DX Contest CW | non aggiunta |
| Africa All Mode International DX Contest SSB | non aggiunta |
| Baltic Contest SSB | non aggiunta |
| CQ WW 160 meter contest SSB | non aggiunta |
| CQ WW DX Contest SSB | non aggiunta |
| CQ-M Contest SSB | non aggiunta |
| EU-HF Contest SSB | non aggiunta |
| Fieldday Contest SSB | non aggiunta |
| French HF Championship SSB | non aggiunta |
| HA-DX Contest  SSB-CW Contest SSB | non aggiunta |
| Helvetia Contest CW | non aggiunta |
| Helvetia Contest SSB | non aggiunta |
| Holyland-DX Contest CW | non aggiunta |
| Holyland-DX Contest SSB | non aggiunta |
| IARU HF World-Championship-CW-SSB Contest SSB | non aggiunta |
| International  Bucuresti Contest CW | non aggiunta |
| International  Bucuresti Contest SSB | non aggiunta |
| NAURYZ DX Contest SSB | non aggiunta |
| ON 10 Meter Contest UBA | non aggiunta |
| RDA Contest SSB | non aggiunta |
| RUSSIAN DX Contest SSB | non aggiunta |
| Russian WW MultiMode Contest CW | non aggiunta |
| Russian WW MultiMode Contest SSB | non aggiunta |
| Russian WW PSK Contest (PSK) | non aggiunta |
| Scandinavian Activity Contest SSB | non aggiunta |
| TRC DX Contest TRC | non aggiunta |
| UBA Spring 80M CW Contest CQ | non aggiunta |
| UK_EI DX Contest SSB | non aggiunta |
| Ukrainian DX Contest SSB | non aggiunta |
| Venezuelan Independence Contest CW | non aggiunta |
| Venezuelan Independence Contest SSB | non aggiunta |
| YO DX HF Contest SSB | non aggiunta |
| YU-DX-HF Contest SSB | non aggiunta |
