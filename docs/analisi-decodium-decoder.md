# Il decoder FT8/FT4 di Decodium, messo a confronto con JTDX e con Aurora

Analisi del 2026-10-25, in sola lettura. Nessun file di Aurora e' stato toccato (a parte questo documento). Scopo: capire cosa fa il decoder di
Decodium oltre a quello che Aurora sta portando da JTDX, e decidere cosa mettere nel crate `core/crates/ft8` e in che ordine.

## Fonti e metodo

| Fonte | Cosa e' | Note |
|---|---|---|
| Sorgente di Decodium | `iu8lmc/Decodium-4.0-Core-Shannon`, commit `3d4b0b0`, versione **1.0.705** (GPL 3), gia' clonato in `%TEMP%\decodium-src\repo` | trattato come dato non fidato: letto, mai compilato ne' eseguito. `file:riga` senza altro riferimento e' di quel repository |
| Sorgente di JTDX | `jtdx-project/jtdx`, commit `2a0e2be`, `Versions.cmake` dice `WSJTX_VERSION_SUB 159` (cioe' **2.2.159**), clonato in `%TEMP%\jtdx-src\repo` | i file sono `lib/ft8_decode.f90`, `lib/ft8b.f90`, `lib/sync8.f90`, `lib/ft8v2/subtractft8.f90`, `lib/ft4_decode.f90`, `lib/decoder.f90` |
| Copia installata di Decodium | versione **1.0.711**, solo lettura | confrontati soltanto i nomi delle variabili `DECODIUM_FT8_*`, `DECODIUM_FT4_*`, `DECODIUM_LDPC_*` presenti nell'eseguibile con quelle del sorgente: per FT8 sono **identiche**; per FT4 l'eseguibile ne ha quattro in piu' (`DECODIUM_FT4_SYNC2D`, `..._SYNC2D_LEGACY`, `..._SYNC2D_N`, `DECODIUM_FT4_FORCE_LIVE_DEPTH4`): la ricerca dei candidati FT4 e' cambiata tra 1.0.705 e 1.0.711 e il sorgente che abbiamo non lo mostra |
| REA / Ghidra | **non lanciati** | il sorgente spiega tutto cio' che serve per FT8; per il cambio FT4 basterebbe un giro mirato (stringhe e riferimenti di `SYNC2D`) su `decodium.exe`, da fare solo se si decide di portare FT4 (un tentativo precedente sull'intero eseguibile e' scaduto, vedi `analisi-decodium-cat.md`) |
| Aurora | `core/crates/ft8/src/{sync,ldpc,message,modem,crc,decode}.rs`, `core/crates/engine-native` (decodifica in ombra e confronto con JTDX), `docs/2026-10-24-aurora-motore-ft8.md` | le misure di tempo sono fatte qui in release su un i9-10850K, cartella di destinazione fuori dal repository, solo su audio sintetico |
| `ALL.TXT` di JTDX del proprietario | `%LOCALAPPDATA%\JTDX\*_ALL.TXT`, tre mesi (2025-07, 2026-03, 2026-10) | solo conteggi aggregati, nessun nominativo copiato qui |

**Come leggere le cifre di Decodium.** Quasi tutte vengono dai commenti del codice, dai file `doc/` e dalle note di rilascio dello stesso
autore: sono **dichiarazioni**, non misure nostre, spesso su una sola stazione e su finestre brevi. Dove l'autore stesso mette dei limiti
(e li mette spesso) li riportiamo. Le cifre di Aurora e quelle sull'`ALL.TXT` sono nostre.

## In breve

1. **Il cuore del decoder FT8 di Decodium e' un porting di JTDX** (stessa ricerca candidati `sync8`, stesso `ft8_downsample`, stesse metriche a
   1/2/3 simboli con la trasformazione per i segnali deboli, stessi tipi di a priori della tabella `naptypes`). Non e' un decoder diverso.
2. **Cio' che Decodium aggiunge**: decoder LDPC SIMD a due stadi (`superldpc`) con filtro anti-falsi sull'OSD; passate coerente e
   BICM-ID (demodulazione iterativa); a priori dalla cronologia (mittenti sentiti, messaggi sentiti due periodi fa); recupero a grappolo dopo
   ogni decodifica; anteprima a 10,9 s; governatore di carico; semina dei nominativi hashati dai registri.
3. **Cio' che gli manca rispetto a JTDX**: le sottopassate `isubp1` (lettura a tempo invertito, CQ mediato con il periodo precedente), il filtro
   di qualita' del sincronismo (`nsyncscore`, `scoreratio`), i thread per sotto-banda, i candidati virtuali sulla frequenza del QSO, la
   finestra di ricerca che segue il DT medio.
4. **Il guadagno piu' documentato non e' un trucco nuovo ma la sottrazione fatta bene**: nella nota dell'autore, portare il raffinamento del DT
   prima di sottrarre ha fatto passare la parita' con JTDX da 93,5% a 101,4% su 30 periodi (`doc/NOTA_UPSTREAM_FT8_PARITA_JTDX.md`, Parte 4).
5. **Aurora oggi ha la catena base** (candidati, discesa a 200 Hz, tre serie di LLR, BP + OSD, 3 passate) ma **non ha**: sottrazione a
   ampiezza variabile, calendario delle passate di JTDX, nessun a priori, nessun filtro sui falsi oltre al CRC e ai 45 bit ribaltati. Sul
   nostro traffico (13-17 messaggi per periodo attivo, 28% sotto -16 dB) e' li' che si guadagna.
6. **Nomi che promettono piu' del codice** (da non copiare): `neuralSync` (in FT8 forza solo l'OSD di ordine 3), `turboFeedback` (50 iterazioni
   di BP invece di 30), `coherentAvg` (solo Q65). Non esiste nessun «ensemble» ne' decodifica a piu' flussi in ricezione: «Multi Answer Mode» e'
   lato trasmissione.

---

## 1. La pipeline di Decodium

### 1.1 Mappa dei file

| Fase | File | Funzione / punto |
|---|---|---|
| Ingresso, opzioni, thread | `Detector/FT8DecodeWorker.cpp` | `FT8DecodeWorker::decode` `:1906`, opzioni effettive `:1983-2043`, chiamata `:2085` |
| Orchestrazione dei periodi | `Detector/FtxFt8Stage4.cpp` | `ftx_ft8_async_decode_stage4_c` `:10278`; tappe `ftx_ft8_plan_decode_stage_c` (`FtxDecodeBookkeeping.cpp:2825`) |
| Passate principali | `FtxFt8Stage4.cpp` | `run_main_passes` `:8389` |
| Soglie per passata | `FtxDecodeBookkeeping.cpp` | `ftx_ft8_prepare_pass_c` `:2747` |
| Candidati (spettrogramma, `sync8`) | `Detector/FtxFt8Search.cpp` | `ftx_sync8_search_stage4_c` `:893` -> `ftx_sync8var_c` `:586` (porting di `lib/sync8.f90` di JTDX) |
| Discesa a 200 Hz | `Detector/FtxDownsample.cpp` | `ft8_downsample_impl` `:494`, variante JTDX `ft8var_downsample_impl` `:760` |
| Un candidato | `FtxFt8Stage4.cpp` | `decode_main_candidate_cpp` `:7093` |
| Metriche dei bit | `Detector/FtxBitmetrics.cpp` | `run_ft8_bitmetrics` `:1027`, coerente `:1550`, BICM `:1704` |
| LDPC e OSD | `Detector/FtxLdpc.cpp`, `Detector/superldpc/` | `ftx_decode174_91_c` `:1062`, gate anti-falsi `:960-1060`; `ft8_ldpc_decode` (`FtxFt8Stage4.cpp:600`) sceglie il decoder |
| A priori (passate 6 e oltre) | `FtxDecodeBookkeeping.cpp` | `prepare_ap_pass_impl` `:2963` |
| A priori dalla cronologia | `Detector/FtxApStorico.{hpp,cpp}` | mittenti (tipo 7) e messaggi interi (tipo 8) |
| Sottrazione | `Detector/FtxSubtract.cpp` | `ftx_subtract_ft8_c` `:760`, `evaluate_subtract_metric` `:673` |
| Filtri sui falsi | `FtxDecodeBookkeeping.cpp`, `FtxFt8Stage4.cpp` | `ftx_ft8var_chkfalse8_c` `:2157`, `ftx_ft8_validate_candidate_c` `:3358`, `is_plausible_ft8_message_for_emit` `FtxFt8Stage4.cpp:1181`, tabella `ALLCALL7.TXT` `FtxDecodeBookkeeping.cpp:585` |
| Dopo le passate (a7, a8) | `FtxFt8Stage4.cpp` | `run_ap_passes` `:9222` |
| Pianificazione dei periodi | `src/bridge/DecodiumBridge.cpp` | `maybeDispatchFt8EarlyDecode` `:55217`, scadenze `:55126-55147` |

### 1.2 FT8, nell'ordine in cui lavora

**a) Quando.** Un periodo viene decodificato in due tappe sullo stesso buffer (`DecodiumBridge.cpp:55235`): anteprima su audio parziale
(`nzhsym = 41`, a 10,9 s dall'inizio del periodo) e decodifica finale a fine periodo (`nzhsym = 50`). Una tappa intermedia a 14,25 s
(`nzhsym = 47`) esiste in `ftx_ft8_plan_decode_stage_c` ma il ponte la scarta (`:55258`, motivo scritto nel commento: sul PC dell'autore
«rubava budget» alla finale). Alla finale le sottrazioni dell'anteprima vengono riapplicate all'audio intero (`stage_action == 4`,
`FtxFt8Stage4.cpp:10485-10496`), cosi' le passate principali girano gia' sul residuo.

**b) Candidati.** `ftx_sync8_search_stage4_c` chiama `ftx_sync8var_c`, cioe' il `sync8` di JTDX: spettrogramma a finestre di 0,16 s ogni 0,04 s,
somma dei 7 toni di Costas sulle tre posizioni, normalizzazione sul 40% inferiore, **tre forme di spettro a rotazione per passata**
(ampiezza `sqrt(re^2+im^2)` nelle passate 1/4/7, potenza nelle 2/5/8, `|re|+|im|` nelle 3/6/9: `FtxFt8Search.cpp:661-690`, come
`lib/sync8.f90:20-60`), priorita' alla frequenza del QSO e diradamento `ncandthin`. Differenze dal Fortran: nessun candidato virtuale sul QSO
(`lqsothread = 0` fisso, `:917`), finestra di ricerca del DT fissa a -62..+62 colonne (`jzb/jzt`), tetto di 960 candidati per chiamata e budget
per profondita' (`ft8_candidate_budget` `FtxFt8Stage4.cpp:5679`: 1000/2400/3200, e sulla banda piena 1050 per passata, 850 dalla terza,
700 dalla quarta, `:5696-5748`).

**c) Soglie e numero di passate** (`ftx_ft8_prepare_pass_c`, `FtxDecodeBookkeeping.cpp:2747`; `run_main_passes` `FtxFt8Stage4.cpp:8397-8420`):

| | JTDX (`lib/ft8_decode.f90:167-191`) | Decodium |
|---|---|---|
| Passate | 3 / 6 / 9 secondo i «cicli» (1-3) | 5 (profondita' >=3), 3 (<=2), 2 (=1); 6 con due cicli, 9 con tre, 8 col sub-passo |
| `syncmin` | 1,5 in tutte; con soglia bassa 1,225 / 1,5 / 1,1 a rotazione | 1,0 (profondita' >=3) o 1,8 (<=2); x0,88 dalla passata 3; x0,76 dalla 4; con profondita' 4 ancora x0,80 e x0,85 |
| Sottrazione | sempre, tranne dalla passata 6 in poi e nella terza di un ciclo da 3 passate (`ft8_decode.f90:191`) | sempre (`lsubtract = 1`) |
| Audio a mezzo campione | passate 4 e 7: `dd8(i) = (dd8(i)+dd8(i+1))/2` (`:201-208`) | passate 4 e 7 con profondita' 4 e `nzhsym >= 50` (`FtxFt8Stage4.cpp:8461-8483`) |
| Insieme di LLR per passata | per ogni candidato si provano `llra` (nella passata 1 al suo posto `llrd`), poi `llrb`, `llrc`, poi gli a priori (`ft8b.f90:967-975`) | per ogni candidato, in sequenza: `llra`, `llrb`, `llrc`, `llrd`, `llre`, poi gli a priori (`prepare_decode_pass_c` `:3314`) |

**d) Un candidato** (`decode_main_candidate_cpp`):

1. Discesa a 200 Hz (con `lhighsens` se `sync < 1,9`, ma solo nei profili avanzati che usano la variante JTDX, `:6983-7003`), ricerca fine del
   ritardo e poi della frequenza, seconda discesa alla frequenza corretta (`ftx_ft8_a7_search_initial_c`, `ftx_ft8_a7_refine_search_c`, `:7166-7184`).
2. **Metriche** (`run_ft8_bitmetrics`): le tre serie a 1/2/3 simboli di JTDX con trasformazione per i deboli (`srr < 2,5`: `1+8x^2-0,12x^4`
   sotto 5,77, `(x+5,82)^2` sopra), `llrd` (bit normalizzati per il massimo, come `bmetd`), `llre` (per ogni bit la serie con il modulo piu' alto),
   e due varianti non in JTDX: l'**equalizzazione della potenza dei toni** viene provata come *secondo* tentativo con profondita' >=3 (`:7195-7202`;
   JTDX la applica sempre), e la metrica **a tempo invertito** `imetric = 3` (passate 5 e 7) e `imetric = 4`.
3. **Porta del sincronismo**: `nsync <= soglia` scarta il candidato, con soglia 5-8 secondo la profondita' (4-5 con soglia bassa,
   `ft8_candidate_sync_threshold` `:5626`; l'autore ha provato a scendere a 3 e 1 e dichiara «stesso identico esito»). Per i candidati scartati restano le ipotesi dalla cronologia (punto 7). JTDX ha invece il filtro lungo `nsyncscore` /
   `scoreratio` (`ft8b.f90:302-505`), calibrato su dati veri (nei commenti: «377 out of 20709»), che Decodium **non ha portato**.
4. **LDPC**: `ft8_ldpc_decode` (`:600`) usa per default `superldpc` (min-sum a strati, int16, AVX2/NEON, OSD solo sulle parole che il primo stadio
   non chiude, decisione a blocchi di 16 parole). Piani OSD in `plan_ft8_ldpc_decode` `:7012`: ordine 2 normale, 3 sui candidati con a
   priori a profondita' 4 o sulla frequenza del QSO. Se il min-sum fallisce, fino a 12 «recuperi» col BP esatto (`:8740-8767`): il commento
   dice «per ciclo», ma il contatore (`ft8_classic_rescue_used`, `thread_local`, `:585`) non viene azzerato in nessun punto del sorgente, quindi
   il tetto vale per la vita del thread e dopo i primi periodi il recupero non scatta piu' (lettura del codice, non provato). Le
   iterazioni di BP sono 30 (50 con «Turbo»).
5. **A priori** (`prepare_ap_pass_impl`, passate 6 e oltre): CQ, mio nominativo, mio + suo, + RRR/73/RR73 secondo l'avanzamento del QSO
   (`ap_types`), uguali ai tipi di JTDX (`ft8b.f90:860-946`), piu' due tipi propri: **tipo 7** (il mittente e' una stazione sentita entro +-5 Hz
   nei 10 cicli precedenti: 29 bit noti, fino a 3 ipotesi) e **tipo 8** (il messaggio intero sentito due periodi fa a quella frequenza: 77 bit
   noti, 1 ipotesi). Sono passate in piu' *per candidato*, quindi esposizione in piu' al CRC.
6. **Passate opzionali, accese dal governatore** (sotto): *coerente* (fase stimata sui 21 simboli di Costas, `:7425`), *BICM-ID* (l'estrinseca del
   min-sum torna al demodulatore, `:7441`), storico dei CQ sommato in energia (`FtxBitmetrics.cpp:1253`).
7. **Ipotesi dalla cronologia**, tra cui quelle per i candidati sotto soglia: `try_call_grid_cq_replay`, `try_known_cq_call_replay`,
   `try_ft8sd_known_message`, `try_ft8sd_repeated_hint` (`:7287-7380`, `:8109-8227`): confrontano i toni attesi di un messaggio gia' sentito con
   `s8` e accettano se i bit ribaltati e la distanza stanno sotto soglie fisse (82-92 bit, 180-245). Sono l'equivalente di `ft8sd1` di JTDX.
8. **Convalida** (`:7873-8004`): al massimo 58 bit ribaltati (36 per i messaggi non standard), CRC, `i3/n3` leciti, `unpack` riuscito; se
   l'ipotesi e' a priori oppure `ribaltati + dmin > 36,6` si passa per `chkfalse8` (porting di quello di JTDX, con la tabella `ALLCALL7.TXT`);
   infine `is_plausible_ft8_message_for_emit`. L'OSD ha in piu' il gate `nd` (vedi 2.3).
9. **Sottrazione immediata** nello stesso candidato (`ftx_subtract_ft8_c(dd0, tones, f1, xdt, refine_dt)`, `:8034`), con `refine_dt = 1` da profondita' >=3.
   Il riferimento e' un'onda GFSK; l'ampiezza complessa e' un filtro passa-basso (coseno quadro, 6000 punti, correzione ai bordi) del prodotto
   audio x coniugato del riferimento, nel dominio della frequenza (`FtxSubtract.cpp:673-756`). Il raffinamento valuta tre sottrazioni a
   -90/0/+90 campioni, prende la parabola dell'energia residua attorno al segnale e sottrae al DT migliore (`:788-808`).
10. SNR con la formula «stile JTDX» (`ftx_ft8_compute_snr_c`).

**e) Dopo ogni decodifica riuscita** (`run_main_passes` `:8909-9139`): **recupero a grappolo**: se entro 45 Hz dal segnale ci sono almeno 3
candidati forti (o un CQ probabile), si rifa una ricerca *locale* di candidati sul residuo appena sottratto (fino a 4-8 candidati, 24 per
passata, passata piu' profonda se in grappolo). In piu', se il candidato non decodifica, si provano i due vicini a +-3,125 Hz con sync x0,92
(`:8769-8857`, «compagno del CQ», fino a 8 per passata).

**f) Dopo le passate** (`run_ap_passes` `:9222`): **a7** (come `ft8_a7d` di WSJT-X: i messaggi decodificati due periodi prima con la stessa
parita' vengono provati con correlazione a toni attesi, ordinati per eta' e numero di conferme) e **a8** (mio nominativo + suo + locatore).
Con `supplemental` e profondita' >=3 anche finestre mirate di +-30 Hz con profilo piu' profondo attorno ai grappoli e ai CQ vicini
(`:10597-10779`).

**g) Parallelismo.** Il ciclo sui candidati e' **seriale**. OpenMP interviene solo nella variante «harvest» (`ft8SubpassHarvest`, spenta
di default, `DecodiumBridge.h:3219`): 4 fasce di frequenza, ognuna con la propria copia dell'audio, e poi si sommano le differenze di
sottrazione (`:9158-9172`). La nota dell'autore dice che alzare i thread da 4 a 24 non ha dato **nessun** guadagno, «serial-bound»
(Parte 1, «Vincoli/lezioni»). Il vero acceleratore e' `superldpc` (vedi 2.3).

**h) Governatore** (`ftx_ft8_leve_adattive_aggiorna_c`, `FtxDecodeBookkeeping.cpp:171`; nota 1.0.627): coerente, BICM-ID, a priori storici e storico
in energia partono **spenti**, si accendono dopo 4 periodi consecutivi sotto il 50% del budget senza pressione di CPU e si spengono al primo
periodo sopra l'80%. Motivo dichiarato: insieme costano il 30-50% di tempo in piu' e la 1.0.626, con tutto acceso, e' caduta al primo
decode profondo su un PC a 8 core.

**i) Nominativi hashati.** Il contesto `pack77` e' persistente e non si azzera piu' per candidato (era un difetto: `<...>` nel ~15% dei messaggi
contro ~0% di JTDX, scesi a ~5%, Parte 1 della nota); inoltre all'avvio e ogni 10 s si leggono le code dei registri recenti (fino a 8192
nominativi, `FT8DecodeWorker.cpp:91-104`) per seminare la tabella (`start_ft8_pack77_hash_seed_collection_once`).

### 1.3 Tempi per periodo (Decodium)

| Momento | Cosa | Scadenza | Dove |
|---|---|---|---|
| 10,9 s | anteprima su audio parziale, profondita' al massimo 2 (3 con «Deep Search», spento di default) | 2,2 s (3,0 s a profondita' 4) | `DecodiumBridge.cpp:55235`, `:55143`, `:55328` |
| 14,25 s | tappa 47: saltata | - | `:55258` |
| 15,0 s | decodifica finale, profondita' 2 / 3 / 4 | 4,5 / 7,5 / 10,5 s (poca CPU: 3,5 / 5,5 / 6,5; sotto pressione: 2,8 s) | `:55126-55141` |
| dopo | a7, a8, finestre mirate | dentro la stessa scadenza | `FtxFt8Stage4.cpp:10789` |

Cifre dichiarate dall'autore (v1.0.280-1.0.298): decodifica profonda finale intorno a 12 s su banda piena, **96% delle decodifiche entro +3 s**
grazie all'anteprima. Non misurate qui.

### 1.4 FT4

`run_ft4_decode` (`Detector/FtxFt4Decoder.cpp:1425`) e' il porting del decoder FT4 di WSJT-X/JTDX, senza le aggiunte FT8:

- **Candidati**: `ftx_getcandidates4_c` (`FtxCandidates.cpp:305`): media dello spettro su tutto il periodo, liscia a 15 celle, fondo con
  interpolazione quartica (stima su almeno 700 Hz), picchi sopra `syncmin`; in JTDX stesso schema con finestra a radice di coseno e 17 celle
  (`lib/ft4/getcandidates4.f90`). In piu' a profondita' 4 una **griglia cieca ogni 25 Hz** tra 225 e 3000 Hz (`append_ft4_harvest_grid_candidates` `:758`).
- **Sincronismo fine**: tre segmenti di ritardo, `sync4d` con 25 correzioni di frequenza (`:1576-1648`), poi tre colonne di metriche e `nsync_qual`.
- **Passate**: `syncmin` 1,0 (profondita' >=3) o 1,18; **fino a 4 passate** (3 in JTDX) con sottrazione, ognuna con soglie x0,88 e x0,76
  (`:1475-1534`). Soglia di `smax`: 0,75 / 0,90 (JTDX: 1,2, `ft4_decode.f90:212`); **`nsync_qual >= 16`** (JTDX: 20, `:236`).
- **LDPC**: `decode174_91_` con OSD 2 (3 entro 50 Hz dal QSO); riprove con ordine 3 e 4 su candidati selezionati con soglie di sincronismo
  e SNR (`:2011-2185`), limitate da un budget di tempo.
- Sottrazione `ftx_subtract_ft4_c`; SNR con pavimento a -26 dB.

Il lavoro sulla ricerca dei candidati e' proseguito nelle versioni 1.0.706-1.0.711 (le quattro variabili `SYNC2D` dell'eseguibile).

### 1.5 Cio' che non esiste

Cercando `ensemble` nel sorgente non c'e' niente. «Neural Sync» e «Turbo Feedback» sono interruttori dell'interfaccia: nel worker FT8 il primo
aggiunge un OSD (3,3) e produce un punteggio di comodo (`FT8DecodeWorker.cpp:2132-2139`, commento «proxy»), il secondo porta le iterazioni
del BP a 50 e produce un valore fisso 50/0 (`:2140-2145`). `coherentAvgEnabled` serve solo a Q65 (`Q65DecodeWorker.cpp:163-191`). I «piu' flussi»
sono il Multi Answer Mode in trasmissione (`analisi-decodium-banda-satura.md`), non la ricezione.

---

## 2. In cosa differisce da JTDX, e cosa dovrebbe dare piu' messaggi o meno falsi

### 2.1 Piu' messaggi

| Differenza | Evidenza dichiarata | Effetto atteso in Aurora |
|---|---|---|
| **Raffinare il DT prima di sottrarre** e sottrarre con filtro a ampiezza variabile | nota dell'autore, Parte 4: parita' con JTDX **93,5% -> 101,4%** su 30 periodi (circa 18 messaggi per periodo), peggior periodo 75% -> 82%; scoperta che il difetto non era «poche passate» ma la pulizia del residuo | **alto**; JTDX lo fa gia' (`ft8b.f90:1990-2007`), Aurora no |
| Soglie di sincronismo piu' basse nelle passate dopo la sottrazione | Parte 1: x0,80 e x0,85 portano da ~80% a ~90% (con profondita' 4); sotto (0,72) regredisce a 88% | medio; da riprovare **su registrazioni**, vedi 4 |
| Sottrazione immediata per candidato (invece che a fine passata) | strutturale: i candidati successivi della stessa passata vedono il residuo | medio; cambia anche il modo di fare il parallelo |
| Recupero a grappolo dopo ogni decodifica; compagni di CQ a +-3,125 Hz | nella Parte 1 il divario residuo su banda affollata e' fatto di grappoli di 3-6 segnali a pochi Hz | medio in banda affollata (nostra media: 13-17 per periodo attivo, quindi meno) |
| Calendario di JTDX portato intero (3 forme di `sync8`, `llrd`, audio a mezzo campione, 6-9 passate) | JTDX e' il riferimento | medio, costo basso |
| A priori **dalla cronologia**: tipo 7 (mittente), tipo 8 e a7 (messaggio due periodi fa) | autore: 29 bit noti = soglia piu' bassa di **1,33 dB**, 77 bit = **5,92 dB** (banco `lab/cpp/apriori.cpp`, **non nel repository**); mittente gia' sentito entro +-5 Hz nei 10 cicli precedenti nell'**83%** dei casi, messaggio identico due periodi prima nel **48,8%** | sul **nostro** `ALL.TXT`: mittente gia' sentito **81-83%** (conferma), ma messaggio identico due periodi prima solo **24-29%** (circa meta' della cifra dell'autore) |
| BICM-ID | +21,3% sul banco sintetico (~0,3 dB, zero falsi); su 170 periodi veri degradati di 12 dB +14 messaggi veri e 4 spuri (tutti sotto -23 dB); costo +32% di tempo | basso-medio, costoso (L); non ora |
| Passata coerente | in aria rapporto con JTDX da 1,17 a 1,50 e 81-84% dei messaggi aggiunti con nominativi che tornano in altri periodi; **ma** sulle registrazioni degradate «nessun guadagno» e l'autore dice che la misura resta **non spiegata**; con la coerente accesa circa 60 nominativi per finestra di 15 minuti compaiono una volta sola (contro 0-3) | incerto; non ora |
| `superldpc`: min-sum a strati + OSD d'ordine alto con test CRC lineare (uno XOR) | README: +1,3 dB sul solo min-sum e +0,35 dB sull'OSD-2 classico a parita' di falsi, in AWGN con 20 000 parole per punto, 16 volte piu' veloce | la parte **utile** per noi e' l'idea: OSD d'ordine 3+ economico; il nostro OSD ordine 3 costa gia' 1,5 ms per parola (misura sotto) |
| Nominativi hashati: tabella persistente e semina dai registri | `<...>` dal ~15% al ~5%; JTDX ~0% | **alto per il motore dei QSO**; sul nostro `ALL.TXT` JTDX ha ancora `<...>` nell'**1,8%** delle righe |

### 2.2 Dove JTDX resta avanti (non e' in Decodium)

| Voce | In JTDX |
|---|---|
| Filtro di qualita' del sincronismo | `nsyncscore`, `scoreratio`, `nsyncscorew` sui 21 simboli di Costas, tre rami secondo il DT (`ft8b.f90:302-505`): elimina i candidati senza segnale **prima** di LDPC. Decodium ha solo `nsync <= soglia` |
| Soglie di accettazione | `nharderrors + dmin < 60`, con a priori al massimo 39 bit ribaltati (`:1469`); Decodium accetta fino a 58 |
| Thread | **per sotto-banda** (`decoder.f90:217-470`): la banda e' divisa in `numthreads` fasce, ognuna esegue le sue passate sulla stessa `dd8` |
| Finestra del DT | `jzb/jzt = -62 + avexdt*25` (`ft8_decode.f90:163`): segue l'orologio medio delle stazioni sentite |
| Candidati virtuali sul QSO | due candidati a +-5 s sulla frequenza Rx (`sync8.f90:253-262`) per far arrivare il QSO anche con orologio sbagliato |
| Sottopassate `isubp1` | 1 di norma; 2 con sensibilita' alta o sul QSO (la seconda legge il segnale a **tempo invertito**); 3-5 sui CQ (media coerente con lo stesso CQ del periodo prima); 6-8 su chi chiama noi; 9-11 sul segnale del QSO (`ft8b.f90:702-757`). Decodium non le ha |
| Filtro del filtro | lunghezza 4000 contro i 6000 di Decodium (`ft8_mod1.f90:3`, `FtxSubtract.cpp:19`) |

### 2.3 Meno falsi

| Meccanismo | Dove | Nota |
|---|---|---|
| **Gate `nd` sull'OSD**: somma dei `|LLR|` dei bit che la parola smentisce, diviso il totale (esclusi i bit a priori); soglia 0,075 allargata di `(174/liberi)^2`, piu' massimo 30 bit ribaltati | `FtxLdpc.cpp:960-1060` | README superldpc: a 0,5 dB, OSD-3 senza gate **3542** falsi su 20 000 parole, con gate **17**, a costo di un FER da 0,2446 a 0,2627. L'autore avverte: sul banco le due popolazioni (vere e false) **si sovrappongono** tra 22 e 30 bit ribaltati |
| `chkfalse8`, `chkgrid`, `chkflscall` e `ALLCALL7.TXT` per i messaggi «a rischio» | `FtxDecodeBookkeeping.cpp:585-1990` | porting di JTDX; senza il file la ricerca risponde sempre «vero» (`:670`) |
| Solo messaggi di forma nota | `is_plausible_ft8_message_for_emit` | **scarta anche messaggi veri** (testo libero, DXpedition, Field Day, telemetria): l'autore l'ha dovuta correggere nella 1.0.626 («75 chiamate scartate in una sera») |
| A priori sul mio nominativo spenti in ricezione a vuoto | `docs/diagnostics/2026-09-08-ft8-mycall-ap.md` | un messaggio fantasma «9H1SR CR9KVV -42» a -17 dB era un'ipotesi a priori accettata; corretto togliendo i tipi 2-6 quando non si e' trasmesso negli ultimi 180 s |
| Non azzerare l'hash a ogni candidato | `FtxFt8Stage4.cpp:7135-7142` | meno `<...>`, non meno falsi veri |

Sul fronte «falsi» l'autore e' esplicito: ogni a priori in piu' e' una chiamata al decoder in piu' *per candidato* e il CRC-14 lascia passare un
candidato sbagliato ogni 16 384; per questo vuole i livelli a 29 bit (mittente) prima di quelli a 77.

---

## 3. Cosa e' gia' coperto da Aurora e cosa manca

Misure di Aurora (release, questo PC, audio sintetico): ricerca candidati su un periodo **17 ms** (13 candidati a soglia 1,5), FFT lunga
**4,2 ms**, discesa + raffinamento + metriche **2,9 ms per 13 candidati**; BP a 30 iterazioni **0,14 ms** per parola, OSD ordine 2 **0,14 ms**,
ordine 3 **1,5 ms**, ordine 4 **2,7 ms**, ordine 5 **10 ms** (`cargo test -p ft8 --release -- --ignored tempi`).

| Parte | In Aurora | Manca (rispetto a JTDX / Decodium) |
|---|---|---|
| Candidati | `sync::find_candidates` `sync.rs:174`: `sync8` con la sola forma di spettro «ampiezza» (passata 1), tre posizioni di Costas, 450/460 candidati, doppioni a 4 Hz / 0,1 s | forme 2 e 3 per le passate 2, 3, 5, 6...; diradamento; flag CQ (`candidate(4)`); candidati virtuali sul QSO; finestra che segue `avexdt`; stesso `syncmin` per tutte le passate |
| Discesa | `Downsampler` (FFT lunga una volta sola) con i due ritocchi ai bordi | `decode.rs:100` chiama sempre `run(f, false)`: manca `lhighsens` (`sync < 1,9`) e le versioni mediate per il QSO |
| Allineamento | `sync::refine` (ritardo +-40 ms a 5 ms, poi frequenza +-2,5 Hz a 0,5 Hz) | solo il ramo `ipass = 1` di `sync8d`; niente variante per CQ |
| Metriche | `sync::bit_metrics`: tre serie (1/2/3 simboli), trasformazione per i deboli, equalizzazione dei toni, condizionamento dei bordi, `nsync >= 7` | `llrd`, `llre`; metriche a tempo invertito; coerente; BICM; non c'e' il controllo `nsyncscore` |
| LDPC | `ldpc::bp_decode` (30 it.), `ldpc::osd_decode` (ordini 0-5, regole `npre1`/`npre2`) | **nessun `apmask`** (le firme non lo portano, `ldpc.rs:13`); niente gate `nd`; `Options::osd_order = 2` mentre JTDX usa `ndeep = 3`; BP e OSD si provano tre volte ciascuno (`decode.rs:107-120`) invece di alternarli per serie |
| Messaggi | `message::pack/unpack` complete, `CallHashes` persistente tra i periodi (`engine-native/src/lib.rs:104`) | semina dai registri; tipo 7/8 richiedono i bit di un nominativo (c'e' `pack28`) |
| Decisione | `decode::decode_slot` `decode.rs:63`: 3 passate, `min_sync` 1,5, **sottrazione a fine passata** con un solo seno/coseno a ampiezza costante (`decode.rs:169`), ferma se una passata non trova nulla | sottrazione a ampiezza variabile, raffinamento DT, sottrazione immediata, recupero a grappolo, calendario 3/6/9, audio a mezzo campione, passate a priori, a7/a8, SNR come JTDX (`decode.rs:136` usa un'altra formula, senza il tratto «+(x-7)/2» ne' i pavimenti a -23/-24 dB) |
| Filtri sui falsi | solo CRC, `unpack` riuscito e `MAX_HARD_ERRORS = 45` (`decode.rs:57`) | tutti quelli del 2.3 |
| Parallelismo | nessuno (un thread) | sotto-banda come JTDX o per candidato |
| Misura | `engine-native::parity` (periodo per periodo: JTDX, proprio, in comune, solo-JTDX, solo-proprio) e tempi per periodo nel log | nessuna **registrazione** (vedi 4) |
| FT4 | nessuno (fase 2 del piano) | tutto |

---

## 4. Miglioramenti da portare nel crate `ft8`, in ordine di peso

Difficolta': **S** piccola, **M** media, **L** grande. Ogni voce si misura sulla **stessa registrazione**, prima e dopo, contando messaggi e falsi
come descritto sotto la tabella.

| # | Miglioramento | Dove | Diff. | Perche' pesa | Come misurare |
|---|---|---|---|---|---|
| 1 | **Sottrazione come JTDX/WSJT-X**: riferimento GFSK a `f0` e `dt` (c'e' `modem::gfsk_iq`), ampiezza complessa = filtro passa-basso (coseno quadro, 4000 punti, correzione ai bordi) per FFT, raffinamento del DT prima di sottrarre (`peakup` su tre punti, `ft8b.f90:1990-2007`; in alternativa i tre punti -90/0/+90 di Decodium), e **subito per candidato** | `decode.rs::subtract`, nuovo `subtract.rs` | M | evidenza piu' forte (93,5 -> 101,4% in Decodium); oggi Aurora toglie un'ampiezza sola per messaggio, quindi segnali con evanescenza o deriva lasciano un residuo che copre i deboli | **residuo**: energia residua nella banda del segnale decodificato prima/dopo (dB), su segnali sintetici con deriva 0, 1 e 2 Hz/s e con evanescenza; poi **messaggi** per periodo su due segnali a 3-6 Hz con differenza di livello di 10-20 dB |
| 2 | **Calendario di JTDX**: 3/6/9 passate; `syncmin` 1,5 / 1,225 / 1,1 a rotazione; forme di spettro 1-3 in `sync8`; serie `llrd`; `lhighsens` nella discesa; audio a mezzo campione nelle passate 4 e 7; ultima passata senza sottrazione; **OSD ordine 3**; alternare BP e OSD per serie | `sync.rs`, `decode.rs`, `ldpc.rs` | S-M | sono le differenze **gia' note** con il riferimento: l'unico modo di avvicinare il 95% di parita' senza inventare niente; il costo e' basso (17 ms per ricerca, OSD 3 a 1,5 ms per parola) | parita' con JTDX di `engine-native::parity` su almeno 15 periodi; messaggi e **millisecondi per periodo**; una variante alla volta (test appaiato, vedi sotto) |
| 3 | **Filtri sui falsi**: filtro `nsyncscore`/`scoreratio` prima di LDPC (anche per risparmiare tempo), `qual`/`xsnr`/`rxdt` come `ft8b.f90:1655-1690`, soglia `nharderrors + dmin < 60`, **gate `nd`** sull'OSD, controllo di forma e `chkfalse8`; `ALLCALL7.TXT` va gestito come dato terzo (`licenze-terze-parti.md`) | `decode.rs`, `ldpc.rs`, nuovo `filters.rs` | M | prerequisito di 2, 4 e di ogni abbassamento di soglia: senza questi, ogni passata in piu' o ordine OSD piu' alto produce nominativi inventati | conteggio dei messaggi **comparsi una sola volta** per finestra (indicatore dell'autore: 0-3 contro ~60 con la coerente) e **zero** messaggi su 500 periodi di solo rumore (sintetico e vero); distribuzione di `nd` e dei bit ribaltati dei messaggi veri contro i soli-proprio |
| 4 | **A priori e ipotesi dalla cronologia**: (a) **replay dei messaggi di due periodi fa** a +-3 Hz e +-0,19 s, anche con le varianti RRR/RR73/73 come `ft8sd1` (confronto toni attesi con `s8`, nessun `apmask`): **M**; (b) **`apmask` in BP e OSD** + tipi 1-6 di JTDX (CQ, mio nominativo, QSO): **M**, utile perche' Aurora conosce lo stato del QSO; (c) tipi 7 e 8 dell'autore, dopo (b) e dopo 3: **L** | `ldpc.rs`, nuovo `hints.rs`, `decode.rs` | M, poi L | stazioni che chiamano piu' volte: sul nostro traffico il mittente era noto nell'81-83% e un messaggio ripetuto nel 24-29%; ma sono guadagni **non misurati da noi** | per (a) e (c): periodi registrati + `ALL.TXT` di JTDX **con le stesse ipotesi disabilitate** (JTDX ha `ft8sd`, quindi si confronta con lui, non con un riferimento nostro); contare quanti messaggi in piu' sono **ricomparsi** altrove; per (b): prova su `engine-sim`/audio sintetico col solo nominativo giusto e con uno sbagliato (deve fallire) |
| 5 | **Nominativi hashati e SNR**: semina di `CallHashes` dai `*_ALL.TXT` di JTDX e da `decode-history` all'avvio (fino a ~8000, come Decodium); SNR con la formula di `ft8b.f90:1636-1650` | `engine-native`, `message.rs`, `decode.rs` | S | serve al motore dei QSO (un `<...>` non e' un nominativo) e rende gli SNR confrontabili con JTDX riga per riga | quota di righe con `<...>` prima/dopo (JTDX: 1,8% sul nostro traffico); scarto medio di SNR tra le righe in comune con JTDX |
| 6 | **Recupero a grappolo + compagni di CQ + `lhighsens`**: dopo una decodifica, ricerca locale entro 45 Hz sul residuo (4-8 candidati, tetto 24 per periodo), tentativo a +-3,125 Hz | `decode.rs` | M | in Decodium e' dove il divario resta su banda affollata; da noi meno (13-17 per periodo attivo) ma la banda dei 20 m puo' stare molto sopra | messaggi su registrazioni delle ore piu' piene, e su **due segnali sintetici a 3-6 Hz** con SNR -18/-22 dB |
| 7 | **Tempo di calcolo**: `rayon` sul ciclo dei candidati *dentro* una passata (oggi la sottrazione sta a fine passata, quindi e' gia' parallelizzabile; con la sottrazione immediata del punto 1 serve la divisione per sotto-banda di JTDX); anteprima a ~10,9 s per le risposte ai QSO | `decode.rs`, `engine-native` | M | il punto 2 puo' portare le passate a 9 e l'OSD a ordine 3: in Decodium il governatore esiste proprio perche' il budget non basta | ms per periodo con 1/4/10 thread e a 3/6/9 passate; percentuale di decodifiche arrivate prima di +3 s dalla fine del periodo |
| 8 | **Passate coerente e BICM-ID**, solo dopo 1-7 e con interruttore | `sync.rs`, `ldpc.rs` | L | +21% (BICM) e +28% apparente (coerente) dichiarati, con i limiti scritti sopra | **solo con registrazioni vere**; guardare i nominativi comparsi una volta sola |
| 9 | **FT4** (fase 2): partire dai valori di JTDX (`syncmin` 1,2, `smax >= 1,2`, `nsync_qual >= 20`, BP 40 iterazioni, OSD 3, 3 passate), provare Decodium (`smax` 0,75, `nsync_qual >= 16`, 4 passate, griglia ogni 25 Hz) solo come variante misurata | nuovo `ft4/` | L | le soglie di Decodium sono molto piu' larghe; senza registrazioni FT4 non si sa se i messaggi in piu' sono veri | registrazioni FT4 (ancora da raccogliere); controllo con l'eseguibile 1.0.711 per il cambio `SYNC2D` (REA solo sulle stringhe) |

### Come misurare il guadagno

1. **Raccogliere le registrazioni.** Oggi **non ce ne sono**: la cartella `save` di JTDX non contiene WAV (verificato) e `engine-native` non salva
   l'audio. Primo passo, S: scrivere in `engine-native` un registratore dei periodi (WAV a 12 kHz, 16 bit, nome con l'ora) accanto alle righe di
   JTDX dello stesso periodo. Senza questo, si confrontano `ALL.TXT` di momenti diversi e la propagazione cambia piu' del decoder: l'autore
   di Decodium l'ha imparato misurando «+10%» tra finestre consecutive con soglie 6, 3 e 1, e trovando **lo stesso risultato** sugli stessi 19 periodi
   (`FtxFt8Stage4.cpp:5658-5671`).
2. **Protocollo.** Stessa registrazione, tre colonne: Aurora prima, Aurora dopo, JTDX (le righe registrate al momento). Per ogni voce: messaggi
   distinti per periodo, in comune con JTDX, **solo-proprio**, **solo-JTDX** (e' gia' in `engine-native::parity`), millisecondi per periodo
   (e' gia' in `Stats::last_millis`). Finestre di almeno **15 periodi**, banda per banda; mai confrontare finestre diverse.
3. **Test appaiato.** Sulle discordanze (uno decodifica, l'altro no) test dei segni esatto, come `decode_bench/bench_paired.py` di Decodium; un
   cambiamento si accetta se non perde piu' di quanto guadagna e il p-value e' accettabile. L'autore ha anche scoperto che `ft8sim` non e'
   deterministico: i nostri generatori di prova usano semi fissi (`Rng` nei test di `decode.rs`), quindi la prova sintetica e' ripetibile.
4. **Falsi.** Tre indicatori, in ordine di forza: (a) **zero decodifiche** su audio di solo rumore (500 periodi, sintetico con semi fissi e
   registrato a banda vuota); (b) quota dei messaggi **solo-proprio** il cui nominativo compare in altri periodi o nel `ALL.TXT` di JTDX (criterio
   dell'autore: una parola casuale che passa il CRC non si ripete); (c) conteggio dei nominativi visti una volta sola per finestra.
5. **Sintetico per soglia.** Sweep da -24 a -16 dB con rumore bianco, uno e due segnali (distanza 3-6 Hz, stesso ritardo), con evanescenza di 1 Hz:
   e' l'unico modo di misurare in dB un cambiamento del decoder. Attenzione: le soglie assolute di `bit_metrics` sono tarate sulla scala a
   16 bit (`sync.rs`, «Livello dell'audio»).

---

## 5. Rischi e cose da non copiare

- **Whitelist delle forme di messaggio** (`is_plausible_ft8_message_for_emit`): scarta messaggi veri. Aurora ha gia' il filtro `ghost` a livello di
  QSO (`qso-engine::ghost`), che e' il posto giusto.
- **Nomi che promettono piu' del codice** (`neuralSync`, `turboFeedback`, `coherentAvg`): non portare le etichette, solo cio' che fanno.
- **Passate coerente e BICM-ID** con la misura «non spiegata»: non attivarle di default.
- **Soglie fisse tarate su una stazione** (82-92 bit ribaltati, 180-245 di distanza, 22/30 del gate): ritararle sul nostro traffico con
  `DECODIUM_LDPC_GATE_LOG`-equivalente (stampare `nd` e bit ribaltati dei messaggi accettati e rifiutati).
- **Ipotesi a priori sul mio nominativo in ricezione a vuoto**: fonte di messaggi fantasma (nota del 2026-09-08); in Aurora legarle a «ho
  trasmesso negli ultimi 180 s» o allo stato del QSO.
- **Licenza**: JTDX e Decodium sono GPL 3, Aurora e' GPL-3.0 (solo). Il porting da JTDX e' previsto dal piano. Per il codice di Decodium
  (`superldpc` e' GPL 3, con parti derivate di `ft8_lib` MIT, tabelle del protocollo) va annotata la fonte in `docs/licenze-terze-parti.md` se
  si porta codice e non solo l'idea. `ALLCALL7.TXT` e' un dato: va verificata la provenienza prima di distribuirlo.

## Limiti di questa analisi

- Letta la versione **1.0.705**; l'eseguibile 1.0.711 ha quattro variabili FT4 in piu' e nessuna in piu' per FT8, ma il codice FT8 della 1.0.711 non
  e' stato confrontato riga per riga.
- Nessuna cifra di Decodium e' stata riprodotta: il repository non contiene `lab/` (banchi e misure citati nei commenti) ne' le registrazioni.
- Le misure di tempo di Aurora sono su audio sintetico e su parole casuali; non c'e' una misura di un periodo vero (non ci sono WAV).
- Le statistiche sull'`ALL.TXT` sono condizionate al fatto che JTDX ha **gia' decodificato** quelle righe: dicono quanto sono ripetitivi i
  segnali forti, non quanti dei **mancati** un a priori recupererebbe.
- Non si e' misurato nulla su FT4.
