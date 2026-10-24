# Analisi della gestione CAT di Decodium

Data: 2026-10-25 · Stato: analisi in sola lettura, nessun codice di Aurora toccato · Scopo: capire come Decodium comanda la radio per
tutti i modelli e decidere come farlo in Aurora (sezione finale «Cosa fare in Aurora»).

## Fonti e limiti

| Fonte | Cosa e' | Note |
|---|---|---|
| Sorgente pubblico `iu8lmc/Decodium-4.0-Core-Shannon` | clonato con `git clone --depth 1` fuori dal repository, letto senza eseguire nulla | commit `3d4b0b0`, versione **1.0.705** (2026-10-20), GPL-3.0. Nelle citazioni i percorsi sono relativi a quel repository |
| Copia installata `...\Programs\Decodium` | solo lettura, mai avviata | `decodium.exe` **1.0.711** («Core Gallager», SHA-256 `1f3e400a...fdce`), sei versioni dopo il sorgente: le parti CAT lette qui sotto vanno prese come valide per la 1.0.711 salvo diversa indicazione |
| `libhamlib-4.dll` installata | SHA-256 `00c2d7b1...d108`, uguale a quello scritto in `hamlib-runtime-manifest.json` | vedi sezione 2 |
| Hamlib 4.7.3 (rc) di WSJT-X, gia' presente sul PC | usato solo per `rigctl -u`, `rigctld --help` e una prova con il modello «dummy» (nessuna porta, nessuna radio) | e' piu' recente della 4.7.0 di Decodium: le capacita' dei modelli possono differire di poco |

Come sono stati usati gli strumenti: il sorgente ha risposto alla maggior parte delle domande, con file e righe. Sull'installato si sono fatti
controlli di stringhe e di struttura (versione e data di costruzione di Hamlib, assenza di `rigctld.exe`, nomi dei file di impostazioni,
presenza delle stringhe di CAT4OM e TCI). Una analisi completa di `decodium.exe` con REA 6.7.0 e Ghidra 12.1.4 e' stata tentata: il primo giro
si e' fermato alla scadenza predefinita di 330 s (l'eseguibile pesa 46 MB) e il secondo, piu' lungo, e' stato interrotto: vedi in fondo la sezione «Esito di REA/Ghidra».

## Sintesi

1. Decodium **non lancia nessun processo** per il CAT: collega `libhamlib-4.dll` (Hamlib **4.7.0**, costruita il 2026-02-15, commit `554e02b39`)
   e la chiama da un thread dedicato. Non c'e' `rigctld.exe` nella cartella.
2. Le «backend» scelte dall'utente sono quattro: **Hamlib**, **TCI**, **OmniRig**, **Cat4OM**. Hamlib e TCI passano dallo stesso gestore (stesso
   elenco di parametri); OmniRig e Cat4OM hanno un gestore proprio.
3. Il «rilevamento della radio» e' **passivo**: legge dal sistema operativo identita' USB e nomi delle schede audio. Non apre porte e non
   interroga la radio. La FT-2000 del proprietario **non e' riconoscibile** da quelle tabelle (sezione 4).
4. I profili sono copie di gruppi di un file INI di Qt, non un formato proprio (sezione 5).
5. La robustezza viene da decine di accorgimenti accumulati (PTT rilasciato subito dopo l'apertura, tolleranza ai guasti di lettura,
   riconnessione a gradini, porte seriali prenotate), che vale la pena riprodurre come elenco di requisiti, non come codice.

---

## 1. Architettura del CAT

### 1.1 Strati

```
QML (SettingsTab1 / RigControlDialog)
  -> DecodiumBridge  (sceglie il backend: "hamlib" | "tci" | "omnirig" | "cat4om"; "decoport" = radio remota)
       -> DecodiumTransceiverManager   Hamlib + TCI (+ HRD, DX Lab Suite Commander, nel menu delle radio)
       -> DecodiumOmniRigManager       COM di OmniRig (QAxObject)
       -> DecodiumCat4OmManager        client JSON/WebSocket
```

- Elenco dei backend nella schermata: `qml/decodium/components/SettingsTab1.qml:179` (`hamlib`, `tci`, `omnirig`, `cat4om`). Un valore
  sconosciuto torna a `hamlib` (`src/bridge/DecodiumBridge.cpp:342-354`, `setCatBackend` a `:31864-31912`): cambiando backend si
  disconnettono **tutti** i gestori e si lascia girare il ciclo eventi 100 ms prima di dare la radio al nuovo.
- `isHamlibFamilyBackend` (`:6246`) tratta TCI come Hamlib: usa lo stesso gestore, con il «rig» `TCI Client RX1` o `RX2`.
- Il vecchio backend «native» (QSerialPort, 15 radio) e' stato **tolto**: chi lo usava viene migrato a Hamlib
  (`src/radio/CatNativeMigration.h:19-46`). Indizio utile: anche Decodium ha concluso che riscrivere i protocolli radio uno per uno non
  conviene.
- Funzioni libere `activeCatSetFreq/SetPtt/SetTxFreq/CanPtt` (`DecodiumBridge.cpp:6272-6458`) smistano ogni comando sul gestore attivo.

### 1.2 Thread e coda dei comandi (backend Hamlib/TCI)

- Per ogni connessione si crea un `QThread` senza genitore e vi si sposta il «transceiver»
  (`src/radio/DecodiumTransceiverManager.cpp:1911`, `Transceiver/TransceiverFactory.cpp:88-229`). Il transceiver ha tre ruoli: aprire,
  fare polling, eseguire comandi.
- **Non c'e' una coda esplicita**: la GUI tiene uno *stato desiderato* completo (`d->desired`: frequenza, modo, split, PTT...) e a ogni
  cambio ne manda una **copia** al thread con un numero di sequenza (`sendState`, `DecodiumTransceiverManager.cpp:2597-2619`,
  `QMetaObject::invokeMethod(..., Qt::QueuedConnection)`). La coda e' quella degli eventi Qt del thread.
- Il thread confronta lo stato richiesto con quello ottenuto e fa solo la differenza (`TransceiverBase::set`,
  `Transceiver/TransceiverBase.cpp:61-285`): se `s.sequence < last_sequence` lo scarta (`:65-70`); prima di tutto toglie il PTT se va tolto
  (`:190-197`), poi frequenza/modo (`:219-237`), poi split (`:238-256`), poi mette il PTT (`:257-264`).
  Dopo `PTT on` e dopo `PTT off` fa una pausa di 100 ms nel thread del transceiver ma non in quello della GUI
  (`sleep_non_gui_thread`, `:22-30, :194, :261`): «alcune radio non sanno elaborare comandi CAT durante il passaggio Tx/Rx».
- Le risposte tornano come segnali con `Qt::QueuedConnection`: `update(stato, seq)` e `failure(motivo)`
  (`DecodiumTransceiverManager.cpp:2173-2313, :2326-2403`).
- Ogni sessione porta un **numero di revisione della configurazione** (`catConfigurationSnapshot`, `:1082-1138`): se mentre `rig_open()`
  e' in corso l'utente cambia profilo, la vecchia sessione puo' chiudersi ma non diventa mai quella attiva (`:2179-2187, :2331-2339`).
- L'unico comando asincrono verso il bridge e' `setRigTxFrequencyAndPttAsync` (`:2812-2858`): frequenza TX e PTT nello stesso stato, per non
  serializzare due giri sulla seriale.

### 1.3 Polling e keep-alive

`Transceiver/PollingTransceiver.cpp`:
- Intervallo scelto dall'utente, **minimo 2 s per le seriali** (`buildParams`, `DecodiumTransceiverManager.cpp:1692-1693`; nella schermata
  `SpinBox 2..99`, `SettingsTab1.qml:1063`). In trasmissione scende a **un quarto, mai sotto 250 ms** (`:44-48`); dopo un cambio di PTT arrivano due
  letture rapide a **300 e 800 ms** (`:19-20, :187-199`).
- «Stabilizzazione»: dopo un comando lo stato nuovo viene annunciato solo quando la radio lo conferma o dopo **3 letture** (`:16, :249-257`).
- **Tolleranza ai guasti**: una lettura fallita non spegne il CAT. Si tollerano **3 guasti consecutivi**, **8** se il messaggio dice
  timeout/I-O (`:17-18, :50-65, :285-302`); poi `offline(motivo)`.
- Le letture di frequenza/modo/VFO si saltano **in trasmissione** (`HamlibTransceiver.cpp:1832-1964`): competono con PTT e misuratori sullo
  stesso bus.
- Per gli Icom seriali (CI-V) il polling passivo di split, modo e PTT e' spento di default; la frequenza si legge con **backoff adattivo**
  (salta 2, poi 4, 8, 10 giri dopo 2 guasti, `:1603-1649`) e le scritture si sospendono mentre la lettura e' instabile (`:1651-1674`).
  Variabili d'ambiente per forzare il comportamento: `DECODIUM_HAMLIB_POLL_PASSIVE_STATE`, `..._DISABLE_FREQ_POLL`, `..._POLL_PTT` ecc.
  (`:568-577`).
- **Keep-alive** (casella «CAT keep-alive», solo Icom seriale con polling spento): una `rig_get_freq` ogni **300 ms**, si spegne da sola dopo
  3 guasti o se la radio dice «non disponibile» (`HamlibTransceiver.cpp:2012-2110`). Serve ad adattatori con LED di attivita'.
- Strumenti (potenza, ROS, ALC, S-meter, tensione, corrente, temperatura, compressione): si leggono **solo se la radio li dichiara** nelle
  capacita'; niente sonde «per vedere se rispondono», perche' un'ALC interrogata a ogni giro aveva lasciato una radio in trasmissione
  (`:869-967, :897-908`). In RX si salta 3 giri su 4 (`:1987-2007`); a inizio TX si fanno letture a **120, 350, 700, 1100 ms** (`:2529`).

### 1.4 Errori, riconnessioni, radio che non risponde

Percorso di un guasto:
1. `TransceiverBase::offline` emette `failure` e chiude (`TransceiverBase.cpp:482-494`).
2. Il gestore decide (`DecodiumTransceiverManager.cpp:2340-2402`):
   - guasto **in apertura**, porta mancante: messaggio «ritento a breve» e riprova (`:2385`);
   - guasto in apertura per timeout/I-O: messaggio, **nessun nuovo tentativo** (`:2387`);
   - guasto a sessione aperta di tipo transitorio (timeout, I/O, `returning(-6)`...), e non un errore di PTT: **riconnessione automatica a
     gradini**, massimo **5 tentativi**, attesa `5 s x numero` con tetto 30 s (`scheduleTransientReconnect`, `:2539-2576`;
     riconoscimento in `isTransientCatIoFailure`, `:591-616`);
   - altrimenti errore all'utente, dopo averlo **tradotto** (`sanitizeHamlibFailure`, `:59-165`): Hamlib 4 mette in `rigerror()` l'ultima riga di
     debug invece del testo dell'errore, quindi si riconoscono «porta occupata» (`already open`, `Access denied`), «porta inesistente», timeout
     della NewCAT Yaesu e si sostituisce un testo comprensibile.
3. Apertura difficile: se `rig_open()` fallisce si fa comunque `rig_close()`, altrimenti la COM resta occupata al tentativo successivo
   (`HamlibTransceiver.cpp:795-801`). Hamlib 4.7.0 puo' restituire 0 da `kenwood_open` anche se la radio tace, lasciando `caps` nullo: c'e' un
   controllo esplicito prima del polling (`:803-810`).
4. **Porte seriali prenotate dentro il processo** (`SerialPortRegistry::acquire`, `DecodiumTransceiverManager.cpp:1977-2013`): CAT, PTT,
   CW e condivisione non aprono la stessa COM insieme. La prenotazione si rilascia solo dopo `rig_close()` (`:2155-2157`).
5. Un nuovo `connectRig()` mentre il vecchio thread sta ancora chiudendo viene **accodato** e parte dal segnale `finished` del thread
   (`:1819-1826, :2134-2170`); un tentativo «vecchio» di oltre 20 s (75 s per HRD) viene ripreso da capo (`:1828-1852`).
6. Se la COM **sparisce** (Windows rinumera le porte dopo lo spegnimento) il gestore cerca la stessa radio fisica con `deviceKey` e la segue
   sulla nuova porta (`followRadioToNewPort`, `:3124-3159`).
7. Radio accesa solo come interfaccia (NewCAT spenta): si mantiene aperta la sessione, si sonda `get_powerstat` ogni 2 s e si riprende
   dopo 5 s dall'accensione dal pannello (`HamlibTransceiver.cpp:839-858, :1788-1826`).
8. Avvio sempre in **solo ricezione**: dopo la connessione si blocca ogni richiesta di PTT per **5 s** (`m_startupPttBlocked`,
   `DecodiumTransceiverManager.cpp:2098-2104, :2862-2865`) e subito dopo `rig_open()` si manda `PTT off` (`HamlibTransceiver.cpp:816-833`).

Se la radio non risponde: i timeout dipendono da Hamlib. Il modello FT-2000 dichiara *timeout 2000 ms, 3 tentativi, pausa dopo scrittura 5 ms*
(`rigctl -m 1029 -u`, Hamlib 4.7.3; confermare sulla 4.7.0). Per gli Icom seriali Decodium li **accorcia**: `timeout=700`, `retry=1`,
`timeout_retry=0` (`HamlibTransceiver.cpp:612-621`), e lascia che sia il backoff a assorbire il guasto.

---

## 2. Hamlib

### 2.1 Come e' integrato

- **Libreria collegata**, non processo: `find_package(Hamlib REQUIRED)` e `target_link_libraries(wsjt_qt Hamlib::Hamlib ...)`
  (`CMakeLists.txt:1168, :1822`). Nella cartella installata c'e' solo `libhamlib-4.dll` (29.378.990 byte, `hamlib-runtime-manifest.json`:
  «Hamlib 4.7.0, windows») e `libusb-1.0.dll`; **nessun `rigctld.exe` ne' `rigctl.exe`**. Nel codice di `src/radio` il solo
  `QProcess` e' l'avvio di `OmniRig.exe` (`DecodiumOmniRigManager.cpp:187`).
- Dalla DLL: stringa di versione `Hamlib 4.7.0 2026-02-15T21:14:25Z SHA=554e02b39 64-bit`; contiene i nomi `auto_power_on`, `ptt_share`, `no_xchg`,
  `newcat_*` (126 simboli), `rigctld_version`, `hamlib_settings.json` e' invece nell'eseguibile.
- Un test senza radio controlla la DLL di rilascio (`tests/test_hamlib_release_contract.cpp`): carica i backend e verifica le capacita'
  dell'IC-7300 (`set_powerstat`, tuner).
- `rig_load_all_backends()` va chiamato **una sola volta per processo**: la 4.7.0 si schianta con «Hash collision» alla seconda chiamata
  (`HamlibTransceiver.cpp:330-340`).
- Il debug di Hamlib e' disattivato (`RIG_DEBUG_NONE`) e le righe residue passano per un callback verso il registro (`:299-327`).

### 2.2 Elenco dei modelli

- `rig_list_foreach_model(register_callback, registry)` (`HamlibTransceiver.cpp:80-133, :339`). Chiave mostrata: `"<Costruttore> <Modello>"`
  (`rig_get_caps_cptr(..., MFG_NAME / MODEL_NAME)`); il modello `RIG_MODEL_DUMMY` diventa **«None»** (radio senza CAT: serve solo per il
  PTT); `RIG_MODEL_DUMMY_NOVFO` e' **nascosto** (`:86-91`).
- Per ogni modello si memorizzano: numero, tipo di porta (seriale / rete / USB / nessuna da `RIG_CAPS_PORT_TYPE`), se sa fare PTT via CAT
  (`RIG_PTT_RIG` o `RIG_PTT_RIG_MICDATA`), se ha il PTT sul connettore dati (`MICDATA`) (`:107-130`).
- Si aggiungono **pseudo-modelli** con numero oltre 99899 (`TransceiverFactory.cpp:25-48`): `TCI Client RX1/RX2`, `Ham Radio Deluxe`,
  `DX Lab Suite Commander`, `OmniRig Rig 1/2`.
- In QML l'elenco e' ordinato e filtrabile con ricerca per parole (`SettingsTab1.qml:387-535`).
- **Quante radio**: Decodium porta anche una tabella ricavata dai sorgenti Hamlib 4.7.0, `data/decoport/radios-hamlib.json` (318 modelli):
  271 seriali, 11 di rete, 9 USB, 20 «altro», 7 senza porta; stato 245 Stable, 60 Beta, 3 Alpha, 2 New (8 senza stato). Marche principali:
  Icom 86, Kenwood 56, Yaesu 46. La schermata dice «Hamlib (300+ radios)». Con la Hamlib 4.7.3 di WSJT-X `rigctl -l` stampa 315 righe.
- Capacita' per modello (porta, velocita' minima e massima, bit, stop, handshake predefiniti): nella stessa tabella, per esempio
  **Yaesu FT-2000 = modello 1029, protocollo `newcat`, 4800-38400 baud, 8N2, handshake hardware**.

### 2.3 Parametri passati a Hamlib

Con `rig_set_conf(token, valore)` prima di `rig_open` (`HamlibTransceiver.cpp:550-732`). Un token non valido per il modello viene saltato
(`set_conf`, `:399-406`), quindi lo stesso codice vale per tutti.

| Parametro Hamlib | Da dove | Note |
|---|---|---|
| `rig_pathname` | porta seriale / `host:porta` / USB | su Windows si antepone `\\.\` (COM10 e oltre), ma `host:porta` resta com'e' (`:40-62`) |
| `serial_speed` | baud | liste della schermata: 1200..115200 (`DecodiumTransceiverManager.h:180`) |
| `data_bits`, `stop_bits` | 7/8, 1/2 oppure «Default» | «Default» = non si imposta: vale quello del modello |
| `serial_handshake` | `None`, `XONXOFF`, `Hardware`, «Default» | idem |
| `civaddr` | indirizzo CI-V (0x00..0xFF) | l'indirizzo di default si chiede a Hamlib con `rig_init` + `rig_get_conf(civaddr)` e c'e' una tabella di riserva (IC-7300 0x94, IC-7300MK2 0xB6, IC-7600 0x7A, IC-7610 0x98, IC-9700 0xA2, IC-705 0xA4; `DecodiumTransceiverManager.cpp:521-589`) |
| `dtr_state`, `rts_state` | `ON`/`OFF` quando «Force DTR/RTS» | regole sotto |
| `ptt_type` | `None` (VOX), `DTR`, `RTS`; con CAT non si imposta | usa il tipo predefinito del modello |
| `ptt_pathname` | porta del PTT | se coincide con la porta CAT non si imposta |
| `ptt_share` | `1` per DTR/RTS | permette di usare la stessa COM |
| `no_xchg` | `1` sempre | «Make Icom CAT split commands less glitchy» (`:711-712`) |
| `client` | `WSJTX` | (`:789-793`) |
| `timeout`, `retry`, `timeout_retry` | Icom seriali: `700`, `1`, `0` | |
| `auto_power_on` | `1` solo per l'azione «Accendi» a CAT chiuso | |
| qualunque `config.*` | da `hamlib_settings.json` in `AppConfigLocation` | ultimo, quindi vince su tutto (`:736-782`) |

**DTR/RTS all'apertura** (`src/radio/CatSerialOpenSafety.h:36-61`): all'apertura di una COM alcuni adattatori alzano DTR o RTS per un istante, e
molte interfacce hanno uno dei due collegato al PTT. Decodium **forza DTR e RTS bassi** se la porta e' un CAT seriale, l'utente non ha chiesto
altro, la linea non e' usata come PTT sulla stessa porta e (per RTS) non c'e' handshake hardware. Per poter forzare le linee serve un handshake
**esplicito**: con «Default» Hamlib potrebbe scegliere hardware e rifiutare con «cannot set RTS with hardware handshake»
(`DecodiumTransceiverManager.cpp:618-646`).

### 2.4 Licenza: LGPL va bene con un progetto GPL-3.0?

- Dalle intestazioni dei sorgenti Hamlib: la **libreria** (`src/rig.c`, backend) e' `LGPL-2.1-or-later`; i programmi come `tests/rigctld.c`
  sono `GPL-2.0-or-later`. Il repository contiene `COPYING` (GPL-2.0) e `COPYING.LIB` (LGPL-2.1).
- **Collegare** la DLL (anche dinamicamente) a un programma GPL-3.0-only e' permesso: «or later» consente di usare la libreria con i
  termini della LGPL 3, che si combina con la GPL 3. Per la DLL si distribuiscono i testi di licenza e si indica dove prendere il sorgente.
- **Lanciare `rigctld.exe`** come programma separato e parlargli per TCP non pone il problema: il collegamento e' un protocollo, e anche la
  GPL-2.0-or-later e' compatibile con la GPL-3.0.
- Da non dimenticare: il pacchetto Hamlib per Windows porta con se' altre DLL (libusb e le librerie del compilatore) con licenze proprie;
  vanno elencate in `licenze-terze-parti.md` allo stesso modo di quelle gia' presenti.
- Aurora e' GPL-3.0-only (`core/Cargo.toml`): in entrambi i casi la strada e' aperta. **Non e' una consulenza legale**: va riletto il
  testo della `COPYING.LIB` della versione scelta prima del rilascio.

---

## 3. TCI, Cat4OM, OmniRig

### 3.1 TCI (protocollo WebSocket di Expert Electronics)

Implementato in `Transceiver/TCITransceiver.cpp` (89 KB): Decodium e' il **client**, il server e' il programma SDR (ExpertSDR3, SunSDR,
Thetis, AetherSDR, ColibriNANO ecc.).

- **Collegamento**: `ws://host:50001` (porta predefinita `50001`; `http`/`https` diventano `ws`/`wss`) (`make_tci_ws_url`, `:378-413`). Il
  socket vive in un thread proprio; i messaggi in arrivo si accodano (tetto 2048 testi e 256 binari) e si elaborano a gruppi per non bloccare
  (`:646-797`).
- **Formato testo**: comandi `nome:arg1,arg2;`, si possono mandare piu' comandi nello stesso messaggio, `\` come escape; nomi in minuscolo.
- **Avvio** (`onConnected`, `:706-727`; `do_poll`, `:1959-2036`): `iq_stop:0;` e `iq_stop:1;`, `split_enable:<rx>,false;`; se richiesto `start;`
  (accende il server). Attesa del server: `device`, `protocol:esdr,1.2` (il primo argomento riconosce **ExpertSDR3** e **Thetis**, che usano
  forme diverse per modo e sensori), `ready`, `start`. Poi `tx_enable:<rx>,true;`, `rx_enable:<rx>,true;`, `rx_mute:<rx>,false;`.
- **Ricevitore**: «TCI Client RX1» = `rx` `0`, «RX2» = `1`; RX2 si abilita con `rx_enable:1,true`.
- **Comandi usati** (inviati): `vfo:<rx>,0,<hz>` (frequenza RX), `vfo:<rx>,1,<hz>` (altra, per split), `split_enable:<rx>,<bool>`, `modulation:<rx>,<modo>`
  (`am cw usb lsb fsk fsk-r digl digu wfm nfm`, `:38-57`), `trx:<rx>,<bool>[,tci]` (PTT; `tci` = l'audio parte dal flusso TCI), `rx_smeter`, `rx_sensors_enable`, `tx_sensors_enable`, `audio_samplerate`, `audio_stream_sample_type`, `audio_stream_channels`,
  `audio_stream_samples`, `tx_stream_audio_buffering`, `audio_start`/`audio_stop`, `start`/`stop`. Ricevuti: oltre ai precedenti `drive`, `rx_sensors`, `tx_sensors`,
  `rx_channel_sensors`, `tx_power`, `tx_swr`, `app_focus`.
- **Livello TX**: non si usa `drive`: la potenza si regola con l'**ampiezza dell'audio** generato, da un'attenuazione 0-45 dB come il cursore Pwr
  (`readAudioData`, `:2281-2283`); `tune` del server non viene mai inviato.
- **Limiti di sicurezza**: frequenze valide solo tra 100 kHz e 250 GHz (`:1886, :1921`); PTT con timer di 1 s per la conferma (`:1860`);
  `stop` dal server mette il PTT a falso (`:1262-1280`). S-meter: dB + 73 per portarlo alla scala di Hamlib (`:1046-1054`); potenza e ROS arrivano
  in decimi (`parse_decimal_tenths`).
- **Flusso audio** (frame binari, intestazione di **64 byte = 16 interi a 32 bit**, poi i campioni; `Data_Stream`, `TCITransceiver.hpp:26-38`):
  `receiver, sampleRate, format, codec, crc, length, type, channels, reserv[8]`. `type`: 0 IQ, 1 audio RX, 2 audio TX, 3 «TxChrono»
  (richiesta di audio TX), 4 uscita linea. `format`: 0 int16, 1 int24, 2 int32, 3 float32.
  - **RX**: audio float32, 2 canali, **48000 Hz**, blocchi da 2048 campioni; Decodium prende un canale, applica un guadagno (predefinito **-20 dB**,
    perche' i server arrivano a fondo scala; `m_tciRxGainDb`) e scende a **12000 Hz** con un fattore 4 (`writeAudioData`, `:1603-1720`).
  - **TX**: il server manda un frame `TxChrono` (`:1330-1363`) e il client risponde con un frame `type=2` pieno di audio generato
    (`send_tx_audio_frame`, `:1495-1580`). Se per mezzo secondo non arriva nessun `TxChrono` si passa a **spingere** un frame ogni 40 ms
    seguendo il tempo reale con 120 ms di anticipo (`:1405-1491`; aggiunta della 1.0.542 per i server che non scandiscono).
  - L'audio parte dopo `audio_start`; `trx:...,tci` dice al server di prendere l'audio da qui invece che dalla scheda.
- **PTT**: con TCI il metodo e' sempre CAT (`DecodiumTransceiverManager.cpp:1431-1435`, `buildParams:1663-1666`).

### 3.2 Cat4OM

- **Cos'e'**: un servizio di rete per il CAT di piu' radio con **gruppi** e **master/slave** (solo il master comanda). Decodium lo
  parla con **JSON su WebSocket** (`src/radio/DecodiumCat4OmManager.cpp`, 51 KB). Il sorgente **non dice chi lo produce**; un simulatore in
  Python, `tools/cat4om_mock.py`, documenta il protocollo 1.0.0 ed e' la miglior specifica disponibile.
- **Due porte**: gestione `127.0.0.1:5000` (scoperta) e controllo `127.0.0.1:5001` (comandi). Se la gestione non risponde si prova il
  controllo scritto a mano (`failConnection`, `:474-493`).
- **Sequenza** (`:495-845`): `hello {protocolVersion "1.0.0", appName, appVersion}` -> `managementWelcome {clientId}` -> richiesta `getServiceStatus`
  -> elenco di gruppi (`id`, `isRunning`, `controlPort`, `radios[radioId]`) -> `startGroup` se fermo -> si apre il controllo sulla porta del
  gruppo -> `welcome {role, groupId, radios[...]}`; poi **solo notifiche push**: `stateUpdate` con per ogni radio `ptt`, `split`, `activeVfo`, `txVfo`,
  `vfos{A:{frequency,mode}}`, `metering{power,swr,alc}`, `connectionStatus`.
- **Comandi**: richieste `{type:"request", id, clientId, action, params, radioId}` con `setFrequency`, `setMode`, `setPtt`, `setVfo`, `setSplit`
  (+ `getOwnership`, rit/xit); risposta `{type:"response", id, success, result|error{code,message}}`. Le capacita' vengono dalle azioni dichiarate.
- **Sicurezza**: scadenza 10 s per ogni richiesta, handshake 10 s, riconnessione a **1, 2,5, 5, 10, 20, 30 s** (`:457-472`); alla chiusura si chiede
  il ritorno in ricezione e si aspetta **800 ms** prima di chiudere i socket; le scritture si accodano finche' non si e' master.
- **Valore per Aurora**: basso. Serve a chi ha gia' quel servizio. Decodium stesso lo ha aggiunto alla 1.0.521 come «quinto backend».

### 3.3 OmniRig

- COM `OmniRig.OmniRigX` via `QAxObject`; se manca il server si avvia `OmniRig.exe` da `Program Files (x86)\Afreet\OmniRig` o `Program Files`
  e si riprova per 10 x 500 ms (`DecodiumOmniRigManager.cpp:160-190`). I «rig» sono `OmniRig Rig 1/2`; il PTT puo' passare da CAT o da RTS/DTR
  comandati da OmniRig (`has_CAT_indirect_serial_PTT`, `OmniRigTransceiver.cpp:79-99`).
- Modi: `PM_CW_U`->`CW-R`, `PM_CW_L`->`CW`, `PM_DIG_U`->`DATA-U`, `PM_DIG_L`->`DATA-L`, `PM_SSB_U`->`USB`, `PM_SSB_L`->`LSB` (`:20-77`).
- Per Aurora e' il percorso gia' in uso (`core/crates/rig-control/src/omnirig.rs` via PowerShell, con le prove dal vivo sulla FT-2000 nel suo
  commento). Decodium conferma la stessa cosa: **OmniRig non da' ALC/ROS** (`SettingsDialog.qml:1017-1026`).

### 3.4 Altri backend presenti

Ham Radio Deluxe (TCP 7809) e DX Lab Suite Commander (TCP 52002) sono «rig» del gestore Hamlib (`TransceiverFactory.cpp:119-161`,
`DecodiumTransceiverManager.cpp:286-293`). **CAT condivisa**: Decodium tiene la seriale e la rivende in rete **con il protocollo di `rigctld`**
(`src/radio/DecodiumCatShare.cpp`, porta predefinita 4533) per altri programmi, e in piu' c'e' «DecoPort» (`src/net/DecoPort*`: COM virtuali
con com0com, multiplexer CAT, collegamento a radio remota). Sono funzioni di contorno: non servono a replicare le schermate richieste.
La cosa utile per Aurora e' l'idea: **se la seriale e' di Aurora, JTDX deve potersi collegare ad Aurora**, vedi sezione finale.

---

## 4. Rilevamento automatico («Detect my radio»)

Il tooltip lo dice: «legge quello che il sistema gia' sa: non apre nessuna porta e non invia nessun comando»
(`SettingsTab1.qml:351-381`). Il sorgente `src/radio/DecodiumRigDetector.h` spiega perche': **una sonda attiva e' rimandata a una «Fase 2»**
«perche' in passato una sonda troppo insistente ha saturato il bus CI-V lasciando il PTT incollato». Quella Fase 2 **non esiste nel codice**:
`modelsForKenwoodId`, `modelsForYaesuId`, `modelsForCivAddress` sono caricate dalla libreria ma nessun altro file le usa.

### 4.1 Algoritmo (`DecodiumRigDetector.cpp:317-472`)

1. Elenco porte: `QSerialPortInfo::availablePorts()` piu', su Windows, **SetupAPI** su tutte le classi (`DIGCF_ALLCLASSES | DIGCF_PRESENT`) leggendo
   `PortName`, nome descrittivo e **istanza PnP** (`:125-167`): e' li' che compare «Enhanced»/«Standard» per le due porte di un CP2105. Si scartano le
   parallele e le porte virtuali (enumeratori `ROOT`, `COM0COM`, `VSP*`, `VIRTUAL*`, `:190-195`).
2. Raggruppamento per **apparato fisico**: `deviceKey` = `vid:pid:serie` se c'e' il numero di serie, altrimenti l'istanza PnP privata del numero di
   interfaccia (`&MI_xx`), altrimenti `porta:<nome>` (`:241-258`). La chiave non cambia se Windows rinumera la COM.
3. **Riconoscimento in tre passi**, il primo che scatta vince:
   - *nome per esteso* nel testo del sistema (nome descrittivo, descrizione, costruttore, **numero di serie**): fiducia **90**;
   - *firma USB* (VID/PID): fiducia dalla firma (**75** per il CP2105 Yaesu, **60** per Icom `0C26`), piu' **15** se la porta marcata come CAT
     (`catHint`, per esempio «enhanced») e' proprio la prima trovata;
   - *interfaccia* (CH340, CP210, FT232, Prolific, Digirig, RIGblaster, SignaLink): fiducia **35**, con la scritta «il modello va scelto a mano»;
   - nient'altro: «Porta seriale», fiducia **15**.
4. Se piu' porte non riconosciute come radio hanno lo stesso apparato (CP2108, FT4232...), **ognuna e' un candidato a se'** (`:450-462`).
5. Schede audio abbinate: primo ingresso/uscita il cui nome contiene `USB Audio CODEC`, `USB AUDIO CODEC`, `USB PnP Sound Device`, `USB Audio Device`,
   `SignaLink`, `Digirig`, **escludendo** `VB-Audio`, `CABLE`, `Virtual Audio Cable`, `Voicemeeter`, `WO Mic`... (`:60-93, :280-311`).
6. Ordinamento per fiducia; per ogni candidato: `deviceKey`, `isRadio`, `protocol`, `rigLabel`, `rigToken`, `catPort`, `otherPorts`, `baudRate`,
   `civAddress`, `audioInput`, `audioOutput`, `confidence`, `evidence` (`DecodiumRigDetector.h:16-37`).

### 4.2 Firme riconosciute (`data/decoport/radios-decoport.json`, nel programma)

- USB: `10C4:EA70` (CP2105 doppio) -> «Yaesu FT-991 / FT-991A / FT-DX10», 38400 baud, `newcat`; `0C26:*` -> «Icom (interfaccia USB propria)», 19200, `civ`.
- Nomi: IC-7300, IC-7610, IC-9700, IC-705, IC-7600 (con indirizzo CI-V e 19200 baud), FT-991, FT-DX10/FTDX10, FTDX101/FT-DX101 (38400), TS-590 (57600),
  TS-890 (115200), TS-2000 (57600). **Sono 13 voci in tutto.**
- Tabelle di identificazione di Hamlib (`radios-hamlib.json`, chiave `identify`): 22 risposte `ID;` Kenwood, 20 Yaesu (per esempio `0251`/`0252` =
  FT-2000/FT-2000D, modello 1029), 70 indirizzi CI-V; e una nota «IC-705 manca dalla tabella `icom_addr_list` di Hamlib 4.7.0».

### 4.3 Cosa succede con la FT-2000

La FT-2000 non ha un nome nelle 13 voci, non e' un CP2105 e di solito arriva da un adattatore seriale/USB generico: il rilevatore la
mostrerebbe come «adattatore USB-seriale ...» (fiducia 35) o «Porta seriale» (15), **senza proporre il modello**. Lo stesso vale per molte altre radio:
il «Detect» di Decodium funziona bene per poche famiglie USB e per il resto si limita a elencare le porte.

### 4.4 Applicazione del risultato (`RigDetectResults.qml:58-92`)

Mostra «Detected radios», *non cambia nulla* finche' l'utente non accetta; applicando: cerca nella lista Hamlib la voce che contiene `rigToken`
(ignorando maiuscole e `-_ `, a parita' vince la voce piu' corta: «FT-991» invece di «FT-991A»), imposta porta e baud, e con «con audio» anche i dispositivi
audio. Il riquadro compare anche alla **prima accensione**.

---

## 5. Profili salvati

- **Posizione**: `QSettings(IniFormat, UserScope, "Decodium", "Decodium3")`, cioe' `%APPDATA%\Decodium\Decodium3.ini` (il file esiste su questo PC).
  C'e' anche un livello superiore di «configurazioni multiple» (`CurrentMultiSettingsConfiguration`, `DecodiumProfileSettings.h:13-30`) che
  sposta *tutte* le chiavi in un gruppo; i profili CAT stanno dentro.
- **Impostazioni vive**: gruppo `[Transceiver]` con `rigName, serialPort, baudRate, dataBits, stopBits, handshake, forceDtr, dtrHigh, forceRts, rtsHigh,
  networkPort, tciPort, pttMethod, pttPort, splitMode, civAddress, catKeepAlive, pollInterval, catAutoConnect, audioAutoStart, tciAudioEnabled,
  tciRxGainDb, hrdStrictRadioMatch` (`DecodiumTransceiverManager.cpp:3345-3380`); `[CAT_OmniRig]` e `[CAT_Cat4OM]` per gli altri due; chiavi
  radice `catBackend`, `CATMode` (USB / Data/Pkt / None), `TXAudioSource` (Rear/Data / Front/Mic), dispositivi audio, `PWRandSWR`, `CheckSWR`,
  `SWRStopThreshold` (`DecodiumBridge.cpp:385-409`).
  Salvataggio **automatico a 500 ms** da ogni modifica (`DecodiumTransceiverManager.cpp:943-981`). Nel file di questo PC i valori sono ancora quelli
  predefiniti (`rigName=None`, `catBackend=hamlib`, `pollInterval=1`, nessun profilo salvato).
- **Profilo salvato** = *istantanea*: `CAT_Profiles/p_<primi 24 caratteri esadecimali di SHA-1(nome in minuscolo)>/` con `name`, `catBackend`, `rigName`,
  `updatedUtc`, sottogruppo `Root/<chiavi radice>` e `Groups/<Transceiver | CAT_OmniRig | CAT_Cat4OM | CAT_Native>/<chiavi>`; il nome attivo sta in
  `CAT_ProfileActive`; nomi con spazi compressi, massimo **80** caratteri (`DecodiumBridge.cpp:335-454, :31559-31845`).
- **Operazioni**: Salva (copia le impostazioni vive nell'istantanea), Carica (le riversa, **disconnette tutto** e dice «collegarsi a mano»: non si
  connette da solo), Nuovo (nome suggerito `"<rig> <backend>"`), Elimina, Rinomina.
- Lezione scritta nel codice (`:31683-31731`): il profilo **non** si riapplica all'avvio, perche' cancellava in silenzio le scelte fatte dopo; si
  applica solo quando l'utente lo carica.

---

## 6. PTT: metodi e tempi

Metodi (`TransceiverFactory.hpp:75`, lista della schermata `CAT / DTR / RTS / VOX`):

| Metodo | Cosa fa | Note |
|---|---|---|
| **CAT** | `rig_set_ptt(ON)`; con «TX Audio Src = Rear/Data» e radio con PTT `MICDATA` usa `RIG_PTT_ON_DATA` (`HamlibTransceiver.cpp:2546-2548`) | predefinito; forzato con TCI |
| **DTR** / **RTS** | `ptt_type=DTR|RTS`, `ptt_pathname`, `ptt_share=1`; porta uguale a quella CAT o altra (`resolvedPttPort`, `:1461-1477`) | senza porta fisica diventa VOX con avviso; RTS-PTT sulla porta CAT con handshake hardware e' vietato |
| **VOX** | niente comandi: `setRigPtt` non fa nulla (`:2866-2874`); solo audio | |

Tempi e salvaguardie:

- **Inizio TX**: `setRigTxFrequencyAndPttAsync` mette in un solo stato frequenza TX (se split) e PTT; l'audio parte **125 ms dopo** il PTT
  quando questo e' asincrono (`kLegacyBridgeAudioAfterAsyncPttDelayMs`, `DecodiumBridge.cpp:332, :29239`). Nel thread, dopo `PTT on` si aspetta 100 ms
  prima di altri comandi.
- **Fine TX**: `PTT off` per primo; **2 tentativi** (3 alla chiusura; **1** per gli Icom seriali, perche' ogni timeout costa secondi); se tutti falliscono
  `ptt_on_` resta vero e **si dice la verita'** (`:2537-2617, :1306-1316`). Alla chiusura del CAT: tentativi di `PTT off` e solo dopo `rig_close()`
  (`:1247-1304`; nota 1.0.365: `rig_close()` non toglie la trasmissione sugli Icom).
- Errori di **PTT-on** si propagano (deve vedersi), quelli di **PTT-off** non interrompono lo smontaggio.
- **Radio che trasmette all'apertura**: `PTT off` subito dopo `rig_open()` e blocco 5 s (sezione 1.4).
- **ROS**: se `CheckSWR` e' acceso e il ROS misurato in trasmissione supera `SWRStopThreshold` (1,5-5,0, predefinita 2,5) il bridge **ferma la
  trasmissione** (`DecodiumBridge.cpp:23746-23802`); serve che la radio dichiari il livello `SWR` (con OmniRig non c'e').

---

## 7. Split e sorgente audio TX

### 7.1 Split (voce «Split», valori `none / rig / emulate`)

- **none**: la radio resta in simplex. `do_tx_frequency` esce subito (`HamlibTransceiver.cpp:1394-1399`).
- **rig** («Rig»): split vero della radio. Sequenza (`:1390-1513`): `rig_set_split_vfo` una sola volta per sessione per far sapere a Hamlib quale e' il
  VFO TX («tickle»), poi `rig_set_split_freq_mode` (o `..._freq`) con il VFO TX temporaneamente forzato, **per ultimo** `rig_set_split_vfo(ON)`, perche'
  alcune radio (Kenwood) escono dallo split quando si cambia il VFO RX e la K3 rifiuta certe combinazioni. Con indirizzamento a VFO unico (Icom) si
  cambia il VFO corrente solo durante la trasmissione. Alla chiusura (o se lo split si spegne) `rig_set_split_vfo(OFF)`; un errore `ENAVAIL/ENIMPL` quando
  si spegne lo split **non e' un errore** (radio senza split comandabile).
- **emulate** («Fake It»): `EmulateSplitTransceiver` (`Transceiver/EmulateSplitTransceiver.cpp`) tiene la radio in simplex e al PTT sposta il VFO sulla frequenza
  TX; a fine trasmissione torna. Se la radio riferisce split attivo il decoratore dice «l'emulazione richiede la radio in simplex» (`:51-53`).
- **Quale frequenza TX**: lo spostamento (XIT) e' `(f_TX / 500) * 500 - 1500`, cosi' l'audio resta tra 1500 e 2000 Hz
  (`catSplitXitHzForTxFrequency`, `DecodiumBridge.cpp:17922-17931`); frequenza TX dial = frequenza dial + XIT (`:17939-17956`). La frequenza TX
  viene **limitata a 1 MHz** dalla RX: se un valore sporco sembra un salto di banda si ripiega su RX + 4 kHz (`sanitizedCatTxFrequencyHz`, `:324-347`).
- Dopo un salto di banda: si bloccano le scritture di modo e split per **8 s**, si controlla che la radio abbia raggiunto la frequenza e, se non e' vero,
  si **ripete il QSY fino a 2 volte** a 1 s di distanza; solo dopo si rimette il modo e si sincronizza lo split (`schedulePostQsyCatSettledSync`,
  `DecodiumBridge.cpp:17767-17883`). Per l'RTTY il modo si riapplica anche dopo 500 ms perche' certe radio richiamano il modo memorizzato per banda
  (`:17799-17809`): e' lo stesso problema del modo che Aurora risolve oggi con OmniRig.

### 7.2 Sorgente audio TX

Impostazione `TXAudioSource` = «Rear/Data» o «Front/Mic» (`DecodiumTransceiverManager.cpp:1566-1587`; schermata `SettingsTab1.qml:1267-1279`).
Non e' un comando separato: modifica solo il PTT scelto con CAT (`RIG_PTT_ON_DATA` se il modello ha PTT `MICDATA` e si e' scelto «Rear/Data»). Con TCI la voce e'
sostituita da «TCI Audio» (non modificabile). Il **modo della radio** («Mode»: `USB`, `Data/Pkt`, `None`, chiave `CATMode`) decide se Decodium
riscrive il modo (`USB` o `DATA-U`) a ogni QSY; `None` = non toccare.

---

## 8. Radio particolari

| Radio | Modello/protocollo in Hamlib | Cosa e' scritto nel codice o ricavato dalle capacita' |
|---|---|---|
| **Yaesu FT-2000** (radio del proprietario) | **1029**, backend `newcat`, 4800-38400, **8N2**, handshake hardware (CTS/RTS) | Nessuna regola dedicata nel codice. Dal `rigctl -m 1029 -u` (4.7.3): timeout 2000 ms, 3 tentativi, pausa dopo scrittura 5 ms; **VFO indirizzabili**, `set_split_vfo` si, **frequenza e modo di split «E»** (emulati da Hamlib), PTT impostabile e leggibile, accensione/spegnimento comandabile, `send_morse`; livelli letti: `STRENGTH`, `SWR`, `ALC`, `RFPOWER_METER(_WATTS)`, `COMP_METER`, `VD_METER`, `ID_METER`. Il commento di Decodium sulle Yaesu FTdx («possono riferire il modo sbagliato quando si trasmette in split con modi diversi per VFO») **potrebbe** valere anche per lui; per questo in TX il modo non si rilegge (`:1944-1953`). Il suo difetto di OmniRig (`Freq` non scrivibile, `SetSimplexMode` che lo mette in split) **non esiste** con Hamlib, ma va provato dal vivo |
| **Yaesu FT-991/991A, FTdx** | newcat, 38400 baud tipico | la piu' citata nei commenti: 150 ms a lettura di ROS/potenza a 38400 baud, polling rallentato; NewCAT spenta rifiuta le letture VFO ma accetta `PS`: sessione tenuta per l'accensione (`HamlibTransceiver.hpp:83-92, :132-137`); accoppiata CP2105 `10C4:EA70` |
| **Icom (CI-V)** | `civ`, 86 modelli; indirizzo CI-V per modello | il caso piu' trattato: timeout 700/1/0, polling passivo spento, backoff adattivo, keep-alive, un solo tentativo di PTT-off, `no_xchg=1`, lettura del solo VFO selezionato senza scambiarli (sennò si vede un QSY, `:988-1023`); VFO esplicito `MAIN` per IC-7300; l'IC-705 manca dalla tabella indirizzi di Hamlib (nota 0xA4) |
| **Kenwood** | `kenwood`, 56 modelli (TS-590/890/2000...) | a 57600/115200; Hamlib 4.7 chiede il blocco VFO prima di ogni cambio di modo; TS-2000 «imposta il modo secondo la frequenza» -> si ripete `set_mode` dopo `set_freq` (`:1374-1377`); TS-480/850/870: senza ROS (`SettingsDialog.qml:1023`) |
| **Elecraft** | `kenwood`: K2, K3/K3S, KX2, KX3, K4 (2021-2047) | K3: «si rifiuta di andare in split con certe combinazioni di modo A/B» -> `set_split_vfo` si fa una volta e lo split si abilita per ultimo (`:1420-1426, :1484-1489`); la risposta `ID;` `017` e' condivisa con Elecraft |
| **FlexRadio** | `FlexRadio 6xxx` (2036, protocollo Kenwood su un collegamento di rete) e `FlexRadio/Apache PowerSDR` (2048) | la lettura dell'«altro VFO» solo in split esiste per far funzionare Flex in modo TS-2000 senza il comando `FB;` (`:1924-1927`) |
| **SDR** (Expert/ExpertSDR3, Thetis/HPSDR, SunSDR, SDR#, SDRuno, SDRConsole, GQRX, Quisk) | in Hamlib come rig kenwood/di rete (2040-2056, 9-11); **meglio TCI** | TCI da' audio, S-meter, potenza, ROS e accensione; per `SunSDR2DX/PRO` l'audio TX usa una scala di ampiezza diversa (`tx_top_`, `:1288`, `:217-229`); ColibriNANO e' solo ricevitore |
| **QRP Labs QMX** | `RIG_MODEL_QRPLABS_QMX` | potenza e ROS letti con `rig_send_raw` (`PC;`, `SW;`) quando Hamlib non li espone, con filtro del ROS e soglia (`:910-930`) |
| **Radio senza CAT** | modello «None» (dummy) | si usa solo il PTT su DTR/RTS o VOX; `dummy_frequency_` ricorda l'ultima frequenza |

Controlli opzionali visti nel codice, non richiesti ora: accordatore interno (`RIG_FUNC_TUNER`), accensione/spegnimento (`rig_set_powerstat`), CW con
`send_morse` (FT-991A: da 1 a 49 caratteri, 1 carattere = comando speciale), voce (`voice_controls`).

---

## Cosa fare in Aurora

### Scelta consigliata

**Hamlib tramite `rigctld` incorporato** (processo figlio, protocollo di testo su TCP in locale), con **TCI come secondo backend nativo in
Rust** e OmniRig mantenuto cosi' com'e' finche' il nuovo percorso non e' provato dal vivo. Cat4OM e FFI: **no**, per ora.

Perche':

| Opzione | Pro | Contro | Giudizio |
|---|---|---|---|
| **`rigctld` figlio + TCP** | stessa Hamlib e stessi 300+ modelli di Decodium; un guasto o un blocco di Hamlib non butta giu' Aurora (Decodium ha dovuto scrivere molto codice difensivo per le chiamate bloccanti e le `access violation` della 4.7.0); si accorda con l'architettura di Aurora (tokio, un processo, nessun codice non `safe`); **piu' client**: JTDX si collega ad Aurora come «Hamlib NET rigctl» sullo stesso rigctld; prove senza radio con il modello 1 | si porta dietro `rigctld.exe` e DLL (circa 30 MB) nell'installatore; ciclo di vita del processo da curare (Windows non uccide da solo i figli); Aurora deve **tradurre** le sue regole (polling, riconnessione) in comandi di testo invece che chiamate dirette; alcune letture costose via rete | **consigliata** |
| FFI verso `libhamlib-4.dll` | nessun processo, la stessa strada di Decodium | `unsafe`, nessun isolamento (Hamlib non e' pensata per chiamate concorrenti, la 4.7.0 ha difetti noti), `bindgen` e DLL per versione, la regola «niente `unwrap`» non protegge da UB | no |
| Backend **TCI** nativo | `tokio-tungstenite` e' gia' una dipendenza; copre gli SDR, e' l'unico che porta anche l'audio e i misuratori | non e' «tutte le radio» | si', come secondo |
| **Cat4OM** | gia' documentato dal sorgente di Decodium | serve il servizio di terzi, pochi utenti | no, dopo |
| Protocolli nativi in Rust | nessuna dipendenza | Decodium ha **tolto** il suo (15 radio): troppo lavoro per troppo poco | no |

**Un punto che cambia le cose**: oggi la radio la comanda OmniRig e la usa anche JTDX (OmniRig e' multi-client). Con `rigctld` **la seriale la tiene
Aurora** e JTDX deve collegarsi a lei: in JTDX si sceglie il modello «Hamlib NET rigctl» con `localhost:<porta>`. Va verificato su JTDX 2.2.159 che
quel modello sia presente e che accetti la nostra impostazione (le chiavi `Rig`, `PTTport` sono gia' lette da `engine-jtdx/src/ini.rs`), e **va
scritto nella guida di collegamento**. Se non funzionasse, il piano B e' lasciare OmniRig per JTDX e usare `rigctld` solo con il motore proprio
(fase 4 di `2026-10-24-aurora-motore-ft8.md`).

### Passi, nell'ordine

Difficolta': **S** piccola, **M** media, **L** grande.

| # | Passo | Diff. | Dopo il passo |
|---|---|---|---|
| 1 | Tipo `RigConfig` (backend, rig, porta, baud, bit, handshake, DTR/RTS forzati, PTT, porta PTT, split, CI-V, polling, keep-alive, modo CAT, sorgente audio TX) con `serde`, e **`RigProfile`** (nome, backend, `RigConfig`) salvati in un file JSON nella cartella dati di Aurora; nome massimo 80 caratteri come Decodium; **nessun segreto, nessun nominativo** | S | test di scrittura/lettura e di migrazione |
| 2 | Separare nel crate `rig-control` il tratto `Rig` in due: **`RigControl`** (frequenza, modo, PTT, split, stato) e i comandi del motore JTDX (oggi nello stesso `trait Rig`, `lib.rs:29-66`) | M | test esistenti verdi |
| 3 | **Elenco dei modelli**: file `radios-hamlib.json` generato al momento della costruzione da `rigctld -l` / `-u` della versione che si distribuisce (numero, marca, nome, stato, tipo di porta, baud min/max, bit, stop, handshake), con ricerca per parole come in `SettingsTab1.qml:387-535` | S | test su un elenco di esempio |
| 4 | **Supervisore di `rigctld`**: avvio con `-m -r -s -t <porta libera> -T 127.0.0.1 -C ...` (i token di §2.3), lettura dell'errore di avvio, **Job Object** di Windows perche' il figlio muoia con Aurora (il problema dell'aggiornamento automatico del 0.10.5 insegna), nessuna finestra nera, riavvio a gradini (5 tentativi, `5 s x n` fino a 30 s), registro dei messaggi | M | prova con il modello «dummy»; prova di `kill` del figlio |
| 5 | **Client del protocollo di `rigctld`** (tokio TCP, timeout per comando): `f/F`, `m/M`, `t/T`, `s/S`, `i/I`, `l` (STRENGTH, SWR, ALC, RFPOWER_METER_WATTS), `\get_powerstat`, `\chk_vfo`; risposte `RPRT n` -> `RigError`; **traduzione dei messaggi** come `sanitizeHamlibFailure` (§1.4) | M | prove contro `rigctld -m 1` |
| 6 | **Regole di sicurezza del PTT**: PTT off all'avvio e a ogni chiusura; blocco 5 s dopo la connessione; 2 tentativi di off, e **stato reale** se falliscono; pausa 100 ms dopo i cambi di PTT; audio 125 ms dopo il PTT; mai PTT con il backend non connesso; interruttore di emergenza che passa per qualunque stato | M | prove con il dummy e con un finto rigctld che fallisce |
| 7 | **Polling e riconnessione**: intervallo (minimo 2 s per seriali), 3 guasti tollerati (8 se timeout), letture sospese in TX con cadenza a 1/4 (min 250 ms), letture rapide a 300/800 ms dopo il PTT, strumenti solo se dichiarati (`\dump_caps` -> `Get level`), riconnessione a 5 gradini | M | prove sul dummy e con un finto rigctld che tace |
| 8 | **Split** `none/rig/emulate` con la regola `(f_TX/500)*500 - 1500`, TX limitato a 1 MHz dalla RX, blocco 8 s dopo il QSY, due ritentativi del QSY e riapplicazione del modo dopo 500 ms (Aurora lo fa gia' con OmniRig in `session.rs`) | M | test di funzioni pure |
| 9 | **Rilevamento passivo**: crate `serialport` (VID, PID, numero di serie, costruttore, prodotto) + SetupAPI per il nome «Enhanced/Standard» e l'istanza PnP; tabella di firme in un JSON (le 13 voci di Decodium **piu' quelle che servono a noi**: **FT-2000**, SCU-17, adattatori comuni); schede audio gia' viste dalla cattura (`cpal`); stessa fiducia 90/75/60/35/15 e **mai** apertura porta. «Fase 2» (interrogare la radio) **solo come opzione spenta** e solo con le sonde che Hamlib stessa usa (`ID;` Kenwood/Yaesu, CI-V) | M | prove su dati registrati |
| 10 | **API** (`remote-api`): `GET /rig/models`, `GET /rig/ports`, `GET /rig/detect`, `GET/PUT /rig/config`, `GET/POST/DELETE /rig/profiles`, `POST /rig/connect`, `POST /rig/disconnect`, evento `rig_status` con stato reale (collegata, errore tradotto, frequenza, modo, PTT, split, S-meter, potenza, ROS) | M | `docs/api.md` aggiornato |
| 11 | **Schermata Impostazioni -> Radio** (React): backend (Hamlib/TCI/OmniRig), profilo (Carica/Salva/Nuovo/Elimina), stato con «Connetti», **«Rileva la mia radio»** con riquadro «Radio rilevate» e conferma, rig con ricerca, porta e baud, metodo PTT (+ porta PTT se DTR/RTS), intervallo di polling, keep-alive, parametri seriali (bit, stop, handshake, DTR/RTS forzati con le regole di §2.3), split, modo, sorgente audio TX; campi che appaiono/spariscono come in Decodium (`SettingsDialog.qml:903-1015`); **nessuna connessione automatica** al caricamento di un profilo | M | prova nel browser |
| 12 | **TCI** (`tokio-tungstenite`): handshake (`iq_stop`, `split_enable`, `start`...), comandi di §3.1, S-meter e misure; **solo controllo** nella prima versione; audio RX/TX **non ora** (serve al motore proprio) | M | prove contro un finto server TCI (da scrivere: il formato e' in §3.1) |
| 13 | **Installatore**: `rigctld.exe` + `libhamlib-4.dll` + `libusb-1.0.dll` e testi `COPYING`/`COPYING.LIB` in `licenze-terze-parti.md`; versione di Hamlib scelta e bloccata (**4.7.x**, la stessa di Decodium), con il suo SHA-256 | S | verifica in installazione |
| 14 | **JTDX come client** di `rigctld` (modello «Hamlib NET rigctl»): impostazione scritta da `engine-jtdx` e istruzione nella guida | M | prova manuale con il proprietario |
| 15 | Migrazione: tenere OmniRig come backend finche' non e' provato; poi decidere | S | |
| 16 | **Cat4OM** | L | solo su richiesta di un utente |
| 17 | Audio TX/RX via TCI per il motore proprio | L | con la fase 3 del motore |

### Cosa si puo' provare senza radio (automatico)

- **`rigctld -m 1` («Dummy»)**: provato qui in locale, risponde come segue (`+` = forma estesa):
  `f` -> `145000000`; `F 14074000` -> `RPRT 0`; `m` -> `FM\n15000`; `M PKTUSB 0` -> `RPRT 0`; `t` -> `0`; `T 1` -> `RPRT 0`;
  `s` -> `0\nNone`; `S 1 VFOB` -> `RPRT 0`, poi `s` -> `1\nVFOB`; `I 14075000` / `i`; `l STRENGTH` -> `-5`; `l SWR` -> `0`; `\get_powerstat` -> `1`;
  `\chk_vfo` -> `0`; `+f` -> `get_freq:\nFrequency: 14074000\nRPRT 0`; `F abc` -> `RPRT -1`. Quindi protocollo, timeout, traduzione degli errori, supervisore
  (avvio, `kill`, riavvio), coda dei comandi e regole di PTT si provano **tutti** senza hardware.
- **Un finto `rigctld`** in Rust (TcpListener) che tace, risponde con `RPRT -5` (timeout) o chiude: per riconnessione, tolleranza ai guasti e PTT-off fallito.
- **Un finto server TCI** in Rust o Python (formato di §3.1), e per Cat4OM `tools/cat4om_mock.py` se un giorno servisse.
- **Una coppia di porte virtuali** (com0com, la stessa che Decodium porta con se') con un emulatore minimo del protocollo `newcat` («FA;», «IF;», «TX;»...) per
  provare apertura della COM, 8N2, handshake e DTR/RTS **senza radio** ma con una vera `rigctld -m 1029 -r COMx`. Va scritto: M.
- Funzioni pure: split (XIT, limiti), sanificazione dei messaggi, scelta del candidato nel rilevamento, profili, tabella dei modelli, normalizzazione di porte e indirizzi.
- I test esistenti di `rig-control` (`tests/`) devono restare verdi dopo il passo 2.

### Cosa solo con la radio vera (prove del proprietario)

1. **FT-2000 con Hamlib 1029**: velocita' e 8N2 con handshake hardware o nessuno; la radio risponde ai comandi NewCAT; polling a 2 s senza disturbare il VFO manuale.
2. **PTT**: da CAT (anche `RIG_PTT_ON_DATA` con «Rear/Data», se la 1029 lo prevede), poi RTS e DTR se l'interfaccia ne ha; **PTT-off dopo errore** e sotto carico di polling; niente trasmissione
   all'apertura, alla chiusura di Aurora e al `kill` di `rigctld` (**il rischio piu' serio**: provare con carico fittizio).
3. **Split** sulla FT-2000 (la sua radio lo usa gia' con OmniRig): `rig` ed `emulate`, XIT di 1500-2000 Hz, ritorno in simplex dopo la trasmissione, cambio banda con memoria del modo per banda (modo dati),
   confronto con quanto fa OmniRig oggi (script in `omnirig.rs`).
4. **JTDX come client di `rigctld`** sulla stessa seriale: frequenza, PTT e split da JTDX mentre Aurora legge; nessuna lite per la porta; blocco di JTDX con Aurora aperta.
5. **Rilevamento passivo** con l'adattatore del proprietario (e con una seconda radio o interfaccia se disponibile) per mettere nella tabella le firme vere.
6. Misuratori (S-meter, potenza, ROS, ALC) e il loro effetto sul bus a 4800-38400 baud.
7. USB che sparisce e ritorna (cavo staccato, radio spenta e riaccesa): la riconnessione automatica e il cambio del numero COM.
8. Prove con altre radio solo se qualcuno le ha (Icom CI-V: il caso piu' delicato; Kenwood/Elecraft; TCI con un SDR vero).

### Rischi

1. **Spartizione della seriale con JTDX**: se non si riesce a far usare a JTDX il nostro `rigctld`, il CAT di Aurora e quello di JTDX non possono coesistere sulla stessa COM.
2. **Processo figlio**: se `rigctld` resta vivo dopo la chiusura di Aurora tiene occupata la COM; Aurora gia' ha avuto problemi di figli che sopravvivono all'aggiornamento automatico.
3. **PTT incollato**: tutti i casi in cui il PTT-off non arriva (rete locale, processo morto, bus saturo). Le regole del §6 sono il minimo.
4. **Versione di Hamlib**: la 4.7.0 ha difetti noti scritti nel codice di Decodium (`kenwood_open` che dice «ok» senza radio; «Hash collision» alla seconda inizializzazione; messaggi di errore con tracce di debug); con `rigctld` il secondo non conta, gli altri vanno gestiti.
5. **Rilevamento**: promettere piu' di quanto si sa (una firma sbagliata e' peggio di nessuna) e **toccare il bus** con sonde attive (storia di Decodium).
6. **Licenze**: la DLL e' LGPL-2.1+, il programma `rigctld` GPL-2.0+: bastano i testi e il sorgente, ma va fatto.
7. Cio' che qui e' stato letto sul sorgente 1.0.705 puo' essere diverso nell'installata 1.0.711, e le capacita' dei modelli sono state lette dalla Hamlib 4.7.3 di WSJT-X, non dalla 4.7.0.

---

## Esito di REA/Ghidra

Due giri su `decodium.exe` (46 MB, SHA-256 `1f3e400a...fdce`) con REA 6.7.0 e Ghidra 12.1.4 (`GHIDRA_INSTALL_DIR=C:	ools\ghidra_12.1.4_PUBLIC`):

1. scadenza predefinita di 330 s, **provider_timeout** (importazione e analisi di Ghidra non finite);
2. secondo giro con `REA_GHIDRA_STARTUP_TIMEOUT_MS=2400000`, **interrotto a mano** dopo alcuni minuti senza risultato, per non tenere Ghidra in memoria.

Quindi **non c'e' nessun risultato di REA in questo documento**: cio' che si dice dell'installata viene dal sorgente, dal file
`hamlib-runtime-manifest.json`, dai due SHA-256 e da un controllo di stringhe (versione Hamlib `4.7.0 2026-02-15 SHA=554e02b39`, nomi `auto_power_on`/`ptt_share`/`no_xchg`/`newcat_*`
nella DLL; stringhe `CAT4OM`, `TCI Client RX1`, `hamlib_settings.json`, `Qt6WebSockets` e assenza di `Log4OM` nell'eseguibile). Se serve confermare che 1.0.711 coincide con 1.0.705
nelle parti CAT, il giro va rifatto lasciando Ghidra libero per piu' tempo (o analizzando solo `libhamlib-4.dll`).
