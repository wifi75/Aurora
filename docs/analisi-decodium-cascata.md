# Perché la cascata di Aurora è meno reattiva di quella di Decodium

Analisi del 2026-10-25, in sola lettura: nessun file di Aurora è stato modificato. Fonti: sorgente pubblico di Decodium 4.0 «Core Shannon»
(versione 1.0.705, GPL-3.0, clonato in una cartella temporanea fuori dal repository e letto senza eseguire nulla), copia installata (1.0.711,
non avviata: ne ho solo cercato le stringhe e letto il registro diagnostico locale `decodium_diagnostic.log`, che contiene metriche vere
della cascata) e il codice di Aurora (`core/crates/audio-spectrum`, `core/crates/remote-api`, `app/src/render`, `app/src/components/Waterfall.tsx`).
REA e Ghidra non sono serviti: il sorgente spiega tutto quello che serviva.

Le idee si riscrivono, non si copiano: il codice di Decodium è C++/QML e va solo capito (vedi `docs/confronto-decodium.md`).

## In breve

- **Causa principale**: il core di Aurora produce già 15 righe al secondo, ma `Waterfall.tsx` ne butta via tre su quattro mediandole
  (`ROWS_PER_STEP = 4`). Sullo schermo arrivano **3,8 righe al secondo**; Decodium ne mostra **10-14 al secondo**, senza nessuna media.
  La cascata scorre quindi 3-4 volte più piano, e ogni riga è una media di quattro trasformate (più sfocata e più in ritardo).
- Il resto del percorso non è il collo di bottiglia: FFT 0,1 ms per riga, WebSocket regolare, disegno a ogni fotogramma.
- Per i telefoni c'è un'aggravante: la cascata scorre di **una riga per pixel del dispositivo**, quindi a parità di righe al secondo sullo schermo
  a 2x o 3x lo scorrimento in pixel CSS è la metà o un terzo (calcolato dal codice, non misurato su telefono).
- Decodium ha inoltre cursori che cambiano subito l'aspetto di **tutta** la storia (nero, guadagno/gamma, contrasto) perché conserva i dB grezzi
  e li colora nello shader; Aurora manda byte già normalizzati dal core e non ha controlli.

## 1. Come fa Decodium

### 1.1 Parametri del calcolo

| Voce | Valore | Dove |
|---|---|---|
| Frequenza di campionamento | 12 kHz, 16 bit | `src/bridge/DecodiumBridge.h:3665` (`SAMPLE_RATE`) |
| FFT | **4096 punti**, 2,93 Hz per bin | `DecodiumBridge.h:4382` (`PANADAPTER_FFT_SIZE`), `PanadapterItem.cpp:225` |
| Finestra | **Blackman-Harris a 4 termini** (lobi laterali molto bassi: sfondo pulito) | `shaders/panadapter_fft_direct.comp` (`blackmanHarris`), `DecodiumBridge.cpp:56601` per il percorso CPU |
| Durata della finestra | 4096 / 12000 = 341 ms | — |
| Sovrapposizione | Ogni riga usa gli ultimi 4096 campioni dell'anello da 16384 (`WF_RING_SIZE`, 1,37 s): con una riga ogni 66-80 ms la sovrapposizione è circa 80-85 % | `DecodiumBridge.cpp:52136-52162`, `52307` |
| Bin mostrati | tutti i 2048 bin positivi (0-6 kHz) nella texture, il tratto visibile si seleziona nello shader | `PanadapterItem.cpp:226` (`kGpuDirectStableOutputBins`), `waterfall_palette.frag` (`wf_XParams`) |
| Media nel tempo | **nessuna**: `m_avgFrames = 1`, e nel percorso GPU non viene comunque applicata alla cascata | `PanadapterItem.hpp:424` |
| Valore per bin | potenza normalizzata, `10·log10`, soglia a −200 dB; calibrazione di display −40 dB sul percorso CPU | `panadapter_fft_direct.comp`, `DecodiumBridge.cpp:56720` |

### 1.2 Da dove viene «Vel: 20 ms»

Il cursore **Vel** (10-500, predefinito 20) scrive l'impostazione `spectrumInterval` (`Waterfall.qml:1205-1211`) che diventa l'intervallo del
timer che controlla se c'è una riga da produrre (`DecodiumBridge.cpp:11553-11561`, `44857-44866`). **Non è il tempo tra due righe**:
`onSpectrumTimer` (`DecodiumBridge.cpp:51973`) scarta i giri che arrivano prima di un intervallo minimo (`52298-52304`) calcolato così:

- tetto di fotogrammi `UI/SpectrumFpsCap`: 15, 20 (predefinito) o 30 fps, cioè 66, 50 o 33 ms (`11641-11642`, `52216-52217`);
- sul percorso con anello PCM del motore incorporato, almeno 66 ms (`52250-52252`); sul percorso GPU «accelerato» l'intervallo richiesto
  resta tra 33 e 66 ms (`src/core/FtRuntimeAdaptivePolicy.hpp:91-118`);
- primi 30 s di ascolto: almeno 100 ms (`52262-52264`); sotto pressione di CPU 180-500 ms; in modalità Low CPU almeno 500 ms (`52265-52275`).

Misura reale dal registro della copia installata (macchina del proprietario, ultimo avvio, 3 finestre da 60 s):

| | |
|---|---|
| Intervallo minimo applicato (`last_min_interval_ms`) | 66 ms |
| Giri del timer | circa 44 al secondo (timer a 20 ms) |
| Righe consegnate (`delivered_frames`) | 618, 740, 816 al minuto, cioè **10,3 - 13,6 al secondo** |
| Percorso | `RHI_compute_gpu`, Direct3D 12, `single_pass_dft_recurrence_4096`, `gpu_frames` = `delivered_frames` |
| Fotogrammi del grafico (`qsg_frame`) | media 16,7 ms (60 fps), disegno della cascata in media 6-9 µs, render 2 ms |

Quindi «20 ms» è il passo di campionamento del timer: la cascata vera avanza di una riga ogni 66-80 ms. Il cursore serve soprattutto a
**rallentare** (sopra i 66 ms la velocità cala davvero); sotto non cambia nulla.

### 1.3 Dove si calcola e come si disegna

- **FFT sulla GPU** (predefinito): shader di calcolo del Qt RHI (Direct3D 12 su Windows), nel thread di rendering di Qt Quick. Non è una vera FFT:
  ogni thread della GPU calcola **una DFT per un bin** con la ricorrenza del rotore su 4096 campioni (`panadapter_fft_direct.comp`, dispatch a
  gruppi da 64 in `PanadapterItem.cpp:5651-5655`). Il lavoro è 4096 × 2048 moltiplicazioni per riga, banale per una GPU. Scrive il risultato
  direttamente nella riga corrente di una texture `R32F` (dB grezzi) e in una seconda texture con minimo e scala della riga
  (`rowParamsImage`): nessuna lettura indietro, nessuna copia sulla CPU (`readback=off` nel registro).
- **Ripiego CPU**: FFTW in precisione singola in un thread del `QThreadPool` (`DecodiumBridge.cpp:52519-52556`, `56594`), con lo spazio di lavoro
  per thread; se il lavoro precedente non è finito, la riga si salta. Si usa solo se la GPU non c'è, in Low CPU, o se serve la storia sulla CPU
  (cascata remota via web, spettro 3D).
- **Cascata**: una `QSGGeometryNode` con uno shader (`shaders/waterfall_palette.frag`) che legge la texture ad anello: **una riga della texture
  per pixel dello schermo**, campionamento **Nearest** (`PanadapterItem.cpp:6671`), la riga più recente in alto. Lo scorrimento è a passi interi
  di pixel, uno per riga arrivata, senza interpolazione. L'anello ha tante righe quante ne mostra la finestra, minimo 128, a scaglioni di 32
  (`waterfallHistoryRowsForVisibleHeight`, `PanadapterItem.cpp:325-333`); nel registro: texture 2048 × 128.
- **Nero, guadagno, contrasto nello shader**: lo shader porta il dB grezzo di ogni riga a 0-1 con il minimo e la scala di *quella riga*, poi applica
  soglia del nero e gamma (`waterfall_palette.frag`, ramo `levelParams.w > 1.5`), poi cerca il colore in una tabella da 256 voci. Cambiare un
  cursore ricolora subito anche tutte le righe già scorse, senza ricalcolare nulla.

### 1.4 Scala in dB, palette, contrasto, guadagno

- **Auto-range** (predefinito): il pavimento si stima dal valore efficace dei campioni nel tempo, mediato con 0,18 per riga
  (`PanadapterItem.cpp:5311-5335`, `estimateGpuDirectNoiseFloorDb` a `287`); lo sfondo sta 6 dB sotto il pavimento (`gpuDirectAutoRangeFloorMarginDb`)
  più la soglia del nero; l'ampiezza della scala dipende dal contrasto: `70 − (contrasto − 10)/140 × 50` dB, quindi **45 dB** a contrasto 80
  (predefinito), 70 dB a 10, 20 dB a 150 (`gpuDirectAutoRangeSpanDb`, `269-278`). Nel registro: `span_db=45 contrast=80 floor_margin_db=6`.
- **Nero** 0-100 (predefinito 15): soglia `nero × 0,0022`, al massimo 0,30 della scala (`waterfallBlackThresholdForLevel`, `280-285`); a 15 è 0,033.
- **Guadagno** 0-100 (predefinito 50): non è un guadagno ma un **gamma** `2,5 − 0,02 × guadagno` (1,5 a 50) applicato al livello normalizzato
  (`PanadapterItem.cpp:4749`, `6535`): comprime i bassi verso il nero, quindi sfondo pulito e segnali netti.
- **Palette**: 12 tabelle da 256 colori; la predefinita (indice 3) è «SmartSDR»: nero, blu notte, blu, ciano, verde, giallo, arancio, rosso
  (`PanadapterItem.cpp:1516`, `2184-2263`); c'è anche la chiara per il tema chiaro.
- Controlli in `Waterfall.qml:1155-1215`: Black, Gain, Contrasto, Vel, più Zoom, Peak hold, auto-range, taglio del rumore (percentile 10).

## 2. Il ritardo tra audio e schermo in Decodium

Non c'è compensazione: la riga mostrata è la trasformata degli ultimi 4096 campioni nel momento del giro del timer. Il ritardo è la somma di:
blocco di cattura (Qt `QAudioSource`, 2048 campioni richiesti, `Audio/soundin.cpp:514`) + metà finestra (≈170 ms: il centro della finestra è
indietro di metà durata) + attesa del prossimo giro (0-80 ms) + un fotogramma di disegno. Non si fa nessuna media che aggiunga ritardo.
Quando manca l'audio per 6 s un controllo di guardia riavvia la cattura (`DecodiumBridge.cpp:52167-52178`); sotto carico (decodifica pesante,
Low CPU) la cascata rallenta di proposito a 100-500 ms per riga (`52265-52275`): il proprietario, con 16 thread e Low CPU spento, non la vede.

## 3. Il percorso di Aurora, misurato

Percorso: scheda audio (`cpal`, WASAPI condiviso) → thread di cattura → `SpectrumAnalyzer` → canale `broadcast` (4096 posti,
`aurora/src/lib.rs:538`) → WebSocket (`remote-api/src/ws.rs`) → `stream.ts` → `spectrumFeed.ts` → `Waterfall.tsx` (`RowAverager`) → WebGPU o WebGL2.

| Tratto | Cosa fa | Numeri |
|---|---|---|
| Cattura | Un blocco per callback (circa 10 ms), canale `mpsc` verso il thread di analisi, un solo canale (primo canale se stereo) | `capture.rs:130`, `166-176` |
| FFT | `rustfft` su finestra Hann; 44,1/48 kHz → **16384 punti** (341-371 ms), 2,93 Hz per bin; una riga ogni `rate/15` campioni (66,7 ms): **15 righe/s**, sovrapposizione 80 % | `analyzer.rs:5-7`, `60-64` |
| Riduzione | 8192 bin FFT → **1024 bin** su 0-3000 Hz prendendo il **massimo** per gruppo | `analyzer.rs:67-74`, `116-119` |
| Scala | dBFS (seno a fondo scala = 0 dB), rumore = **mediana dei 1024 bin**, mediata con 0,1 per riga (circa 0,7 s); il byte vale 0 a «rumore − 5 dB» e 255 a «rumore + 40 dB»: finestra fissa da **45 dB**, 5,7 livelli per dB | `analyzer.rs:9-16`, `122-133` |
| Formato sul filo | un messaggio binario per riga: `[0x01][seq u64][f_min u32][f_max u32][1024 byte]` = **1041 byte**, 15,4 KiB/s, nessuna compressione | `frames.rs` |
| Invio | stesso ciclo `select!` degli eventi; ogni riga è un `send` atteso; se la coda va in ritardo le righe vecchie si perdono (`Lagged`) | `ws.rs:53-77` |
| Client | `onmessage` → copia dei byte → emette ai registrati senza passare da React | `stream.ts:64-76`, `spectrumFeed.ts` |
| **Media** | `RowAverager(4)`: somma 4 righe e ne emette una; **3,75 righe/s** | `Waterfall.tsx:7`, `54-58`, `smooth.ts:1-26` |
| Anello e disegno | texture `R8` 1024 × **512** righe; una riga per pixel del canvas (in pixel del dispositivo); `RowClock` stima l'intervallo con una media mobile e dà la frazione di riga già scorsa, per uno scorrimento continuo; ogni fotogramma (`requestAnimationFrame`) si ridisegna: 4 letture orizzontali (media e massimo) + una nella tabella colori | `webgl2.ts:6-52`, `webgpu.ts:7-43`, `115-183`, `smooth.ts:32-55` |
| Palette | 7 punti: blu notte → blu → azzurro → giallo → arancio → bianco; nessun cursore di nero, gamma o contrasto | `colormap.ts:15-23` |

Misure fatte in questa analisi (nessuna usa radio o audio vero):

| Prova | Risultato |
|---|---|
| `SpectrumAnalyzer` in release, 20 s di audio sintetico in blocchi da 10 ms | 14,8 righe/s; **37 µs per riga a 12 kHz, 115 µs a 48 kHz, 134 µs a 44,1 kHz**; caso peggiore di un blocco 0,1-3,5 ms. Meno dello 0,3 % di un core: **la CPU non è un limite, la GPU per la FFT non serve** |
| Core simulato (`AURORA_ENGINE=sim`, build debug, circa 15 righe/s come il reale) + client WebSocket, 30 s su loopback | 15,16 righe/s, 1041 byte, 15,4 KiB/s; intervallo tra righe p50 63 ms, p90 78, p99 94, massimo 110; **nessuna raffica**. Il core non ritarda né accumula. (Il simulatore usa un timer di Windows con granularità di 15,6 ms, quindi il vero è più regolare.) Non è stato provato su Wi-Fi o telefono |
| Interfaccia vera nel pannello del browser (WebGPU, canvas 483 × 589, DPR 1), 30 s | scritture nella texture: **3,80 righe/s** (intervallo p50 264,5 ms, massimo 366); disegni 28,8/s (il pannello va a circa 30 fps: `requestAnimationFrame` p50 33 ms); righe per pixel 0,87 (canvas più alto di 510) |
| Test | `audio-spectrum` e `remote-api`: 32 suite, 194 test verdi, 0 falliti; interfaccia `src/render`: 22 verdi |

Lettura: il core fa 15 righe/s e il filo le porta tutte e con regolarità; il disegno è leggero. **Il client scarta tre righe su quattro
(le fonde in una)** e lo scorrimento è un lento 3,8 pixel al secondo (nel pannello di prova; a DPR 2 sono 1,9 pixel CSS al secondo).

## 4. Differenza di fondo

| | Aurora oggi | Decodium |
|---|---|---|
| Righe al secondo sullo schermo | **3,8** (15 dal core, mediate 4 a 1) | **10-14** (una per FFT) |
| Pixel al secondo | 3,8 in pixel del dispositivo | 10-14 |
| Un ciclo FT8 di 15 s occupa | circa 57 pixel | circa 190 pixel |
| Media tra righe | 4 righe (ritardo medio circa 130 ms, fino a 267) | nessuna |
| Finestra FFT | Hann, 341-371 ms | Blackman-Harris, 341 ms |
| Valori | byte già normalizzati dal core, finestra fissa da 45 dB | dB grezzi (`R32F`) + scala per riga |
| Controlli | nessuno | nero, guadagno (gamma), contrasto, velocità, palette, auto-range |
| Ricolorare la storia | non si può | immediato (shader) |
| Storia | 512 righe = 136 s a 3,8/s (ma 34 s se salisse a 15/s) | quante ne mostra la finestra (min. 128) |
| Rete | WebSocket (telefono incluso) | nessuna, tutto nello stesso processo |

Ritardo dall'audio allo schermo (**stime** dai numeri sopra, non misure): Aurora circa 350 ms in media (10 di cattura + 171 di metà finestra + 33 di
attesa della riga + 133 della media a 4 + disegno), Decodium circa 225 ms. La metà finestra è identica; la differenza è tutta nella media a 4.

Due altre differenze che contano meno: Decodium stima il rumore dal valore efficace del segnale (si alza con i segnali forti), Aurora dalla
mediana dei bin (più robusta: da tenere); e Decodium scorre a passi interi di pixel, mentre Aurora interpola lo scorrimento tra una riga e l'altra,
cosa utile solo quando le righe sono lente e che a 15 righe/s si può togliere (anche perché aggiunge fino a una riga di ritardo).

Perché Aurora fa così: il registro delle modifiche dice che la media a 4 e il filtro orizzontale sono nati per eliminare l'«effetto serpente»
(`CHANGELOG.md`, voce sulla cascata a una riga per pixel). Va quindi ridotto con una prova dell'occhio, non solo con i numeri (vedi sotto).

## 5. Cosa fare in Aurora

Difficoltà: **S** piccola (meno di mezza giornata), **M** media, **L** grande. In ordine di resa.

1. **Mostrare tutte le righe: `ROWS_PER_STEP` da 4 a 1** (S, in `Waterfall.tsx`). Da 3,8 a 15 righe/s, 4 volte più veloce, ritardo medio
   della media tolto (circa −130 ms). Per non tornare al serpente: al posto della decimazione, una **media mobile leggera** per riga
   (`uscita = 0,6 × nuova + 0,4 × precedente`, ritardo medio circa 30 ms, nessuna riduzione di righe) e tenere il filtro orizzontale dello
   shader. Offrire una scelta «Velocità» con 1, 2, 3, 4 righe per passo (15, 7,5, 5, 3,75 al secondo) salvata in `app/src/settings`, predefinito 1
   o 2; il numero di Decodium è 12,5 al secondo, quindi 1 lo supera.
2. **Alzare l'anello da 512 a 1024 righe** (S, `webgl2.ts:7`, usato anche da WebGPU). A 15 righe/s 512 righe sono 34 s; e con canvas oltre i
   510 pixel di altezza oggi `rowsPerPixel` scende sotto 1 e il disegno comprime in verticale (il commento nel codice dice che fa tremolare).
   Con 1024 righe resta 1 riga per pixel fino a circa 1000 pixel del dispositivo (1 MB di texture).
3. **Stessa velocità apparente su ogni schermo** (S-M): oggi una riga vale un pixel del dispositivo, quindi uno schermo a DPR 2-3 scorre a metà o a
   un terzo della velocità in pixel CSS. Far valere ogni riga `dpr` pixel del dispositivo (campionamento verticale «più vicino», quindi
   `rowsPerPixel = 1/dpr`, e anello dimensionato di conseguenza) dà al telefono la stessa velocità del desktop. Da provare a occhio sul telefono.
4. **Cursori Nero, Guadagno (gamma) e Contrasto nello shader** (M, `webgl2.ts`/`webgpu.ts`/`colormap.ts`, impostazioni e interfaccia). Tre
   uniform in più: `soglia`, `gamma` (a 50: 1,5), `ampiezza`; la tabella colori resta. Agisce su tutta la storia come in Decodium, perché la texture
   conserva i byte. Aggiungere almeno una palette «SmartSDR» (nero → blu → ciano → verde → giallo → rosso) e lasciare l'attuale. Valori di partenza da
   Decodium: nero 15 (0,033), gamma 1,5, finestra 45 dB.
5. **Più dinamica nel dato sul filo** (M, `analyzer.rs`, `frames.rs`, `stream.ts`): oggi la finestra è fissa a 45 dB intorno al rumore, quindi
   un contrasto più «morbido» (finestra 70 dB) non si può avere. Nuovo tipo di frame `0x02`: `[0x02][seq][f_min][f_max][rumore dB i16 ×10][1024 byte
   in dBFS assoluti a 0,4 dB per livello da −120 a −20 dBFS]` (il tipo `0x01` resta per i client vecchi, la PWA in cache compresa); il client
   ricostruisce il riferimento dal rumore e applica la finestra voluta dal cursore. Costo sul filo: +2 byte per riga.
6. **Finestra Blackman-Harris al posto di Hann** (S, `analyzer.rs:62-64` e `full_scale`: da `n/4` a `0,35875·n/2`). Lobi laterali molto più bassi
   (circa −92 dB contro −31 dB): sfondo più pulito attorno ai segnali forti, con il lobo principale più largo (poco, a 2,93 Hz per bin). Provare a vista.
7. **Frequenza righe del core configurabile** (S-M, `ROWS_PER_SECOND`: 15 di default, 20-30 opzionali). A 30 al secondo la FFT costa ancora meno
   dello 0,6 % di un core e il filo 31 KiB/s; non serve per pareggiare Decodium (12,5), conviene solo se dopo la prova 1 lo scorrimento sembra a scatti
   su schermi a 120 Hz.
8. **Rete: più regolare sul telefono** (S + M, da verificare prima): impostare `TCP_NODELAY` sul listener (`axum::serve` con `ListenerExt::tap_io`,
   `aurora/src/lib.rs:795`) e, nel client, un piccolo buffer di riproduzione (1-2 righe) invece di far dipendere lo scorrimento dall'istante di
   arrivo (`RowClock`). In loopback il filo è regolare; su Wi-Fi o tramite tunnel **non è stato misurato**: va fatto con la prova qui sotto.
9. **Non fare**: FFT sulla GPU (la CPU spende 0,1 ms per riga: Decodium la usa perché fa una DFT per bin, noi usiamo una vera FFT); tornare alla cattura
   a 12 kHz (la FFT a 48 kHz ha la stessa risoluzione e costa niente); cambiare il motore di disegno (WebGPU/WebGL2 vanno bene).

### Come verificare il miglioramento

Misurare prima e dopo ogni modifica, con la stessa sorgente.

- **Contatore di righe al secondo** nel client (S): in `Waterfall.tsx`, un oggetto `stats` (non in React) con le marche di tempo di `pushRow`,
  `submit`/`drawArrays` e `requestAnimationFrame`; mostrarlo con `#/?wfstats` come una riga sotto la cascata: righe/s ricevute, righe/s scritte, fps
  reali, intervallo p50 e p99 tra le righe, profondità della coda. Obiettivo: ≥ 12 righe/s scritte, intervallo p99 sotto 120 ms, fps uguali
  alla frequenza dello schermo.
- **Latenza** (M): mettere nel frame `0x02` un orologio di cattura (millisecondi dall'ultima mezzanotte UTC del centro della finestra, u32) e
  mostrare `ora − orologio` (p50 e p99), con l'orologio del client sincronizzato dall'evento di stato; in locale basta così. Obiettivo: 250 ms o meno
  in media (oggi stimati circa 350). In alternativa, senza toccare il filo: il simulatore (`engine-sim`) fa comparire a un istante noto una riga con
  un picco e un test dell'interfaccia (con orologio finto) misura quanto passa fino alla scrittura in texture.
- **Core** (S): trasformare la prova di questa analisi in un test (`audio-spectrum/tests`): 15 righe al secondo (c'è già, `row_rate_is_about_15_per_second`)
  e **costo per riga sotto 1 ms** in release, per non peggiorare l'FFT.
- **Rete** (S): `node` con `WebSocket` e lo script usato qui (intervalli di arrivo, p50/p90/p99, raffiche) contro il core simulato e poi
  contro il core vero via Wi-Fi o via tunnel; confronto con la prova in loopback (p99 94 ms).
- **A occhio** (prova del proprietario con la radio): una trasmissione FT8 vera, 15 s: devono vedersi circa 190 righe per ciclo e i segnali devono
  restare stretti e senza «serpente» (provare con 1 e con 2 righe per passo). Il giudizio sul colore, contrasto e velocità spetta a lui.

## Appendice: punti di partenza nel sorgente di Decodium

| Cosa | File e riga |
|---|---|
| Cursori Black/Gain/Contrasto/Vel e proprietà del pannello | `qml/decodium/components/Waterfall.qml:1155-1215`, `1272-1323` |
| Timer e intervallo effettivo | `src/bridge/DecodiumBridge.cpp:11553-11561`, `44857-44866`, `51973-52310` |
| Criteri di limitazione | `src/core/FtRuntimeAdaptivePolicy.hpp:91-118` |
| Anello di campioni e dimensioni | `src/bridge/DecodiumBridge.h:4370-4382`, `DecodiumBridge.cpp:52136-52201` |
| FFT sulla GPU | `shaders/panadapter_fft_direct.comp`, `src/ui/PanadapterItem.cpp:3002-3138`, `5289-5700` |
| FFT sulla CPU (ripiego) | `DecodiumBridge.cpp:56594-56730`, avvio a `52498-52556` |
| Cascata e colori | `shaders/waterfall_palette.frag`, `PanadapterItem.cpp:4717-4859`, `6490-6560` |
| Scala, nero, contrasto | `PanadapterItem.cpp:246-312`, `4747-4750` |
| Palette | `PanadapterItem.cpp:1516`, `2184-2263`, `src/ui/WFPalette.cpp` |

Clone temporaneo: `%TEMP%\decodium-src\repo` (fuori dal repository, da cancellare quando non serve più).
