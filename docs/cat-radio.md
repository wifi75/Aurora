# Radio CAT: collegare la radio ad Aurora

Il CAT e' il cavo dei comandi: con lui Aurora legge la frequenza della radio, la cambia (cambio banda, spot del DX Cluster, programmazione),
legge il modo e comanda il PTT. Si configura in **Impostazioni → Radio CAT**. Il progetto e l'analisi di Decodium da cui nasce sono in
`docs/analisi-decodium-cat.md`; le rotte in `docs/api.md`.

## In breve

1. Apri Impostazioni → Radio CAT (dal PC dove gira Aurora: le modifiche non si fanno dal telefono).
2. Premi **Rileva la mia radio** (facoltativo): Aurora guarda quello che Windows sa gia' (porte, identita' USB, schede audio) e propone radio e porta.
   Non apre nessuna porta e non manda nessun comando alla radio; non cambia nulla finche' non premi **Usa** e poi **Applica**.
3. Scegli la **radio** dall'elenco (cerca per marca o modello), la **porta** e la **velocita'**. Per le Yaesu, come la FT-2000, di norma 8 bit, 2 stop e controllo
   di flusso hardware: nel dubbio lascia «predefinito del modello», che e' quello giusto per il modello scelto.
4. Premi **Applica** (salva) e poi **Collega**. Lo stato dice come va: «Collegata», con frequenza e modo letti dalla radio, oppure l'errore spiegato in parole.
5. Per non rifare tutto: **Salva** un profilo con un nome. Caricarlo porta le impostazioni nel modulo e **scollega** la radio, senza ricollegarla: la scelta di collegare e' sempre tua.

Nulla si collega da solo. Fa eccezione la casella «Collega la radio all'avvio di Aurora» (spenta di base) e la variabile `AURORA_RIG` (vedi sotto).

## Che cosa fa Aurora per te

| Cosa | Come |
|---|---|
| Collegamento | Aurora lancia `rigctld` (il demone di Hamlib) come processo nascosto e ci parla in locale. Se Hamlib si blocca, Aurora no; chiudendo il collegamento la porta seriale si libera |
| Lettura | Ogni `poll_secs` secondi (le seriali non scendono sotto 2 s) legge frequenza, modo e PTT. **In trasmissione** non legge frequenza e modo (competono con il PTT sullo stesso cavo) e il giro si accorcia a un quarto, mai sotto 250 ms |
| Guasti | Una lettura fallita non spegne il CAT: se ne tollerano 3 di fila (8 se sono timeout o errori della porta). Poi il collegamento si riapre a gradini: fino a 5 tentativi, `5 s × numero` (massimo 30 s). Dopo l'ultimo lo stato resta «Errore» e dice perche' |
| PTT | Subito dopo l'apertura il PTT viene **spento** (alcune radio trasmettono all'apertura della porta) e per 5 s non si accetta nessun PTT acceso. Spegnendo: 2 tentativi; se falliscono lo stato resta «acceso» (si dice la verita'). Alla chiusura, anche di Aurora, il PTT si spegne (3 tentativi) prima di liberare la radio. Non si cambia frequenza mentre si trasmette |
| DTR e RTS | All'apertura vengono tenuti **bassi**, perche' molte interfacce hanno una linea collegata al PTT e certi adattatori la alzano per un istante. Non si forzano se la linea e' il tuo PTT, ne' (RTS) con il controllo di flusso hardware |
| Split | **Radio**: split vero, l'audio resta tra 1500 e 2000 Hz (spostamento `(f/500)·500 − 1500`), il VFO di trasmissione si imposta prima e lo split si accende per ultimo; alla chiusura si toglie. **Simulato**: la radio resta in simplex e la frequenza si sposta solo durante la trasmissione. **Nessuno**: simplex |
| Modo | «Modo scritto nella radio»: a ogni cambio di frequenza Aurora riscrive USB o USB dati (se la radio ha richiamato quello memorizzato per la banda). «Non toccare» lascia il modo scelto sulla radio |
| Cambio frequenza | Dopo la scrittura controlla che la radio ci sia arrivata e ripete fino a 2 volte |

OmniRig resta disponibile come secondo sistema (il percorso storico di Aurora): con lui porta, velocita' e radio si impostano nel programma OmniRig e qui si scelgono solo split e modo.

## Variabili e file

| Cosa | Dove |
|---|---|
| `AURORA_RIG=hamlib` | all'avvio usa Hamlib e collega la radio con la configurazione salvata (come `omnirig` per OmniRig). Senza modello scelto il collegamento non parte e lo stato lo dice |
| `AURORA_RIGCTLD` | percorso di `rigctld` (il file o la cartella che lo contiene) se non sta nella cartella `hamlib` accanto ad Aurora |
| `data/radio.json` | configurazione viva e profili: modello, porta, velocita', PTT, split, modo, condivisione di `rigctld` (`share_enabled`, `share_port`). **Nessun segreto, nessun nominativo.** Se il file e' rovinato Aurora lo mette da parte come `radio.json.bad` e riparte dai valori di base |

### Dove si cerca `rigctld.exe`

1. nella cartella `hamlib` accanto al programma (e nella cartella del programma stessa): **e' dove lo mette l'installatore**, per esempio `%LOCALAPPDATA%\Aurora\hamlib\rigctld.exe`;
2. nella variabile `AURORA_RIGCTLD` (il file o la cartella che lo contiene);
3. **solo per lo sviluppo**, nelle installazioni gia' presenti: `C:\JTDX64\<versione>\bin\rigctld-*.exe` (la piu' recente) e `C:\WSJT\wsjtx\bin\rigctld-*.exe`.

Va bene qualunque nome che inizia per `rigctld` e finisce in `.exe` (anche `rigctld-jtdx.exe`). Se non lo trova la scheda lo scrive in alto e **Collega** risponde «Manca rigctld».
L'elenco dei modelli viene da `rigctl -l` della stessa cartella.

## Hamlib incluso nell'installatore

L'installatore di Aurora porta con se' Hamlib: la scheda Radio CAT funziona su qualsiasi PC **senza installare altro** (niente JTDX, WSJT-X o OmniRig).
Nella cartella `hamlib` accanto ad `aurora-desktop.exe` ci sono:

| File | A cosa serve |
|---|---|
| `rigctld.exe` | il demone Hamlib: Aurora lo lancia nascosto e ci parla in locale |
| `rigctl.exe` | serve solo per l'elenco dei modelli (`rigctl -l`) |
| `libhamlib-4.dll` | la libreria con i driver delle radio |
| `libusb-1.0.dll`, `libwinpthread-1.dll` | le DLL di cui Hamlib ha bisogno (nient'altro: niente runtime GCC, ne' Qt) |
| `LEGGIMI-HAMLIB.txt`, `VERSIONE.txt`, `LGPL-2.1.txt`, `GPL-3.0.txt`, `libwinpthread-licenza.txt` | avviso, versione e SHA-256 dei file, testi delle licenze |

**Origine.** Non si scarica nulla: lo script `scripts/prepara-hamlib.ps1` (chiamato da `scripts/rilascio-windows.ps1`, che con `-SoloHamlib` fa solo questo passo) copia i file da una cartella locale
(parametro `-Hamlib`, variabile `AURORA_HAMLIB_SRC`, altrimenti l'ultima versione di JTDX in `C:\JTDX64\<versione>\bin`, poi WSJT-X in `C:\WSJT\wsjtx\bin`),
legge le importazioni di `rigctld` e `rigctl` per ricavare **solo le DLL che servono davvero** (se ne manca una si ferma con un messaggio chiaro), controlla che il pacchetto parta e che
l'elenco contenga FT-2000, FTDX-101D, FTDX-5000 e Dummy, e scrive `VERSIONE.txt`. I file finiscono in `desktop/src-tauri/hamlib-bundle/` (esclusa da git: i binari non si committano)
e `tauri.conf.json` li porta nell'installatore come risorse nella cartella `hamlib`. I testi delle licenze stanno in `desktop/src-tauri/hamlib-licenze/` (questi si committano).

Verificato sul PC il 2026-10-25 (lettura delle importazioni dei file PE):

| Cartella | Hamlib | Dipendenze di `rigctld`/`rigctl` | Modelli |
|---|---|---|---|
| JTDX 159 (`C:\JTDX64\159\bin`) | 4.6 (git 2024-01-23, SHA 75079d) | `libhamlib-4.dll` -> `libusb-1.0.dll`, `libwinpthread-1.dll` | 287, con FT-2000 (1029), FTDX-101D/5000/3000/1200/10 |
| WSJT-X (`C:\WSJT\wsjtx\bin`) | 4.7.3~rc (2026-09-22, SHA d034b2365) | le stesse tre | 314 |

Nella cartella di JTDX c'e' anche `libhamlib-5.dll`: la usa `jtdx.exe`, non `rigctld`/`rigctl` (che importano `libhamlib-4.dll`), quindi non si include.
Di base si usa JTDX 159 (Hamlib 4.6). Per l'Hamlib piu' recente (4.7.3, con le correzioni per le Yaesu): `.\scripts\prepara-hamlib.ps1 -Hamlib C:\WSJT\wsjtx\bin` (provato: parte e conosce FT-2000 e FT-DX).
Provato anche con il modello 1 (Dummy): `rigctld` del pacchetto risponde a frequenza, modo e `l STRENGTH`.

**Aggiornamenti.** L'installatore (`installer/hooks.nsh`) prima di copiare ferma il `rigctld` lanciato da quella cartella e cancella la cartella `hamlib`: un aggiornamento non lascia file di versioni vecchie.
Non tocca gli altri `rigctld` che usi per altri programmi.

**Licenza.** Hamlib e' software libero: la **libreria** (`libhamlib-4.dll`, i driver) e' **LGPL-2.1 o successiva**, i **programmi** (`rigctld`, `rigctl`) sono **GPL-2.0 o successiva**
(compatibile con la GPL-3.0 di Aurora), libusb e' LGPL-2.1 o successiva, libwinpthread (MinGW-w64) ha una licenza permissiva in stile MIT. I testi stanno accanto ai file e in `THIRD-PARTY-NOTICES.md`.
Aurora lancia `rigctld` come programma separato e gli parla su TCP: nessun collegamento tra codice. I file sono quelli di JTDX/WSJT-X, non modificati, e si possono sostituire con altri
(requisito della LGPL). **Sorgenti**: https://github.com/Hamlib/Hamlib (stessa versione e commit scritti in `VERSIONE.txt`); libusb: https://github.com/libusb/libusb; MinGW-w64: https://www.mingw-w64.org.
Rileggere i testi delle licenze della versione scelta prima del rilascio. Questa non e' una consulenza legale.

## Collegare la radio e gli altri programmi (Hamlib, OmniRig, JTDX, Log4OM2, GridTracker)

**Perche' serve scegliere.** Una porta COM (quella del cavo CAT della radio, per esempio la COM3 della FT-2000) la puo' usare **un solo programma alla volta**.
Se Aurora, OmniRig, JTDX e Log4OM2 provano ad aprirla insieme, il secondo che arriva trova «porta occupata» e non si collega. Quindi: **uno solo parla con la radio**,
gli altri parlano con lui (o con una copia della porta). Hamlib ha un modo comodo per farlo: `rigctld` ascolta sul tuo PC e piu' programmi si collegano insieme.

### A) Tutto su OmniRig, come oggi
1. In Impostazioni -> Radio CAT scegli **OmniRig**.
2. Lascia JTDX, Log4OM2 e gli altri come sono: la COM la tiene OmniRig.
3. Limite: con OmniRig Aurora non ha misuratori veri (lo S-meter e' **stimato dall'audio**).

### B) Consigliato: Aurora con Hamlib, gli altri via rete (nessun driver, nessuna porta in piu')
1. **Libera la COM**: chiudi OmniRig e in JTDX, Log4OM2 ecc. togli la radio dalla COM (passo 5 per il collegamento di rete).
2. In Radio CAT: sistema **Hamlib**, radio **Yaesu FT-2000** (cerca «2000»), porta **la COM della radio**, velocita' come sulla radio (per la FT-2000 di solito 38400; nel dubbio «predefinito del modello»),
   **2 bit di stop** e controllo di flusso **hardware** (nel dubbio, «predefinito»). Premi **Applica**.
3. Nel riquadro **Condividi la radio con altri programmi** metti la spunta (porta di base **4532**). Premi **Applica** e **Collega**.
4. Il riquadro mostra l'indirizzo da usare: **127.0.0.1** e **4532**. Vale solo su questo PC: da casa, dalla rete, non si raggiunge.
5. Negli altri programmi scegli come radio **Hamlib «NET rigctl»** (modello 2 nell'elenco di Hamlib) e come indirizzo `127.0.0.1:4532`:
   - **JTDX / WSJT-X**: Impostazioni -> Radio -> Radio: «Hamlib NET rigctl»; server di rete CAT: `127.0.0.1:4532`; PTT: CAT. (Il modello «NET rigctl» e' nell'elenco di Hamlib incluso, numero 2.)
   - **Log4OM2**: Impostazioni -> Configurazione hardware -> radio: Hamlib, modello «NET rigctl», indirizzo `127.0.0.1:4532`. (Non verificato su un'installazione: i nomi delle voci cambiano con la versione.)
   - **GridTracker**: di norma **non apre la radio**: prende frequenza e modo dai messaggi UDP di JTDX/WSJT-X (o di Aurora). Se non ti serve cambiare frequenza con i suoi clic, non devi fare nulla. Se la tua versione ha il controllo radio, vale lo stesso indirizzo. (Non verificato.)
6. Se un programma dice «connessione rifiutata»: Aurora deve essere **collegata** alla radio (il riquadro compare solo allora).
7. Se la porta 4532 e' gia' usata (per esempio da un altro `rigctld`) Aurora lo dice: cambia il numero (da 1024 a 65535) e usalo anche negli altri programmi.

Vantaggi: nessun software in piu'; `rigctld` mette in fila i comandi di tutti, quindi le risposte non si mescolano.

### C) Due porte: CAT diretto per Aurora e OmniRig per gli altri
Solo se hai **due porte CAT vere**: certe radio ne hanno due (le Yaesu con il chip CP2105 hanno una porta «Enhanced» e una «Standard»), oppure hai un secondo adattatore.
La FT-2000 con un adattatore USB-seriale ne ha **una sola**: lì usa B.
1. Aurora: Hamlib sulla porta A. OmniRig/JTDX/Log4OM2: sulla porta B.
2. Le due porte devono arrivare davvero alla radio (due prese CAT diverse). Se e' la stessa presa torni al problema della porta unica.

### D) Con VSPE (porte virtuali)
VSPE (Virtual Serial Ports Emulator, di Eterlogic) e' un programma di terzi a parte, con la sua licenza: **Aurora non lo include ne' lo modifica**. Serve a «moltiplicare» la porta della radio:
1. In VSPE crea un dispositivo **Splitter** sulla porta fisica della radio (per esempio COM3) e le porte virtuali che ti servono, per esempio COM11 (Aurora), COM12 (OmniRig o JTDX), COM13 (Log4OM2 o GridTracker). La velocita' vera della radio (38400 per la FT-2000) si imposta sulla porta fisica. (I nomi delle voci possono cambiare con la versione: vedi la guida di VSPE.)
2. Premi «Avvia emulazione» in VSPE. Per averle anche dopo il riavvio di Windows: «Save as autostart config» (la configurazione parte con il servizio di VSPE, senza aprire la finestra).
3. In Aurora: Hamlib, radio **Yaesu FT-2000**, porta **COM11** (la virtuale, non la fisica), velocita' 38400, 2 bit di stop. Negli altri programmi: ciascuno la sua virtuale.
4. Nel menu delle porte di Aurora le porte virtuali di VSPE sono segnate «virtuale (VSPE)».

Confronto con B: con VSPE ogni programma crede di avere la radio tutta per se' e **i comandi si possono mescolare** (due programmi che chiedono la frequenza insieme leggono le risposte l'uno dell'altro); serve un driver di terzi (con la sua licenza) e vanno rifatte le impostazioni di ogni programma.
Con B non serve nessun driver e **VSPE diventa inutile per Aurora**. VSPE resta utile per i programmi che non sanno usare `rigctld` e vogliono una COM.

### Come capisco che funziona
1. Stato «**Collegata**» e la frequenza in Aurora e' quella della radio.
2. **Gira la manopola**: dopo 2 secondi la frequenza in Aurora segue.
3. Con la radio in ricezione cambia il segnale (o stacca l'antenna): lo **S-meter** si muove (con Hamlib e' quello della radio; con OmniRig e' stimato dall'audio).
4. Con la condivisione accesa apri JTDX o Log4OM2 sullo stesso indirizzo: la frequenza deve coincidere. Se uno dice «porta occupata», riparti dal punto 1 dello scenario B.

### Cosa si e' verificato su VSPE (analisi statica, 2026-10-25) e cosa no
Verificato sull'installazione del proprietario (VSPE 1.5.8, driver 4.61):
- **Le porte virtuali si riconoscono**: in Windows hanno identificatori `ETERLOGIC_VSPE\ETERLOGIC_VSPE_PORT\COMxx` (costruttore «Eterlogic Software») o `VSBC7\DEVICES\nnnn` (nome «ELTIMA Virtual Serial Port», il motore di VSPE). Aurora le elenca con l'etichetta «virtuale (VSPE)» leggendo i dispositivi di Windows (sola lettura, come per il rilevamento); non apre nessuna porta. Prova automatica in `rig-control/src/detect.rs`.
- **Il file di configurazione** (`*.vspe`, salvato dove vuole l'utente: Desktop, Documenti) e' un formato proprietario non documentato: righe a larghezza fissa con parametri separati da `;` che non si interpretano in modo affidabile; contiene anche percorsi e nomi dell'utente. **Aurora non lo legge e non lo scrive**, quindi non puo' suggerire la porta dal file: la suggerisce dall'elenco delle porte.
- Il servizio `EterlogicVspeDeviceManagerService.exe` scrive un registro di diagnostica in `C:\ProgramData\Eterlogic\VSPE\` con righe come «Success: [Splitter: COMa => COMb]» e il file caricato: e' leggibile ma non e' un'interfaccia, quindi **non lo si usa**. In `%APPDATA%` non c'e' niente di VSPE.
- Il programma si chiama `VSPEmulator.exe` (non `VSPE.exe`); dalle stringhe risulta l'opzione «Save as autostart config (on VSPE service startup)»: le porte nascono all'avvio di Windows. Non ho trovato opzioni documentate da riga di comando (la guida `VSPEmulator.chm` non e' stata letta).
- **Non verificato**: lo Splitter con piu' programmi insieme sulla FT-2000 vera (serve la radio), l'API di VSPE (`VSPE_API.dll`), le differenze tra versioni.
- Non si ridistribuisce VSPE ne' i suoi driver; Aurora non lo avvia e non lo cambia. Dalla guida si puo' solo ricordare dove si trova.

## Provare con la FT-2000 (Hamlib incluso)
1. Impostazioni -> Radio CAT: **Hamlib**, radio **Yaesu FT-2000**, la **porta COM** della radio, **38400 baud** (o quella impostata nel menu CAT della radio), **2 bit di stop**, controllo di flusso **hardware** (o «predefinito»). **Applica**, **Collega**.
2. «Collegata»: frequenza e modo come sulla radio. Gira la manopola: Aurora segue.
3. **Misuratori** (S-meter, potenza, ROS, ALC): la FT-2000 li fornisce a Hamlib; in Aurora la lettura nella scheda e' ancora da fare (vedi «Da fare ancora»). Per una prova a mano, con la condivisione accesa: `rigctl.exe -m 2 -r 127.0.0.1:4532 l STRENGTH` (e `l RFPOWER`, `l SWR`, `l ALC`).
4. Se la COM e' occupata da OmniRig, JTDX, Log4OM2 o GridTracker: scenario B o D sopra (la porta la usa un solo programma).

Prova a mano dal pacchetto, senza Aurora: `rigctld.exe -m 1029 -r COM3 -s 38400 --set-conf=stop_bits=2 -t 4532`.
Senza radio si prova con il modello **1 (Dummy)**: `rigctld.exe -m 1 -t 4532`.

## Rilevamento della radio

E' **passivo**: legge da Windows il nome delle porte («Silicon Labs Dual CP210x ... Enhanced COM Port (COM5)»), l'identita' USB (VID, PID, numero di serie)
e i nomi delle schede audio. Non apre porte e non interroga la radio: una sonda attiva troppo insistente puo' lasciare il PTT incollato.
La fiducia e' 90 se il nome della porta contiene il modello, 60-89 per un'identita' USB nota (per esempio il CP2105 doppio delle Yaesu, dove la porta «Enhanced» e' quella del CAT),
35 per un adattatore generico (CH340, CP210x, FTDI, Prolific: il modello va scelto a mano), 15 per una porta qualunque.
Le porte virtuali (com0com, Bluetooth) non sono mai proposte.

Radio riconosciute dal nome: Yaesu FT-2000, FT-991/991A, FT-DX10/101/3000/5000/1200, FT-710, FT-891, FT-450, FT-897, FT-817; Icom IC-7300 (e MK2), IC-7610, IC-9700, IC-705, IC-7600, IC-7851, IC-7100, IC-7410;
Kenwood TS-590, TS-890, TS-2000, TS-480; Elecraft K3, KX3, KX2, K4; FlexRadio (CAT di SmartSDR); Xiegu G90 e X6100; QRP Labs QMX e QDX.
La **FT-2000** non ha un'identita' USB propria (di solito arriva da un adattatore seriale): la riconosce solo se il nome della porta o il numero di serie dell'interfaccia lo contiene,
e il CP2105 doppio la elenca tra le Yaesu possibili. In tutti gli altri casi basta sceglierla nell'elenco (modello 1029).

## Da fare ancora

- **TCI** (SDR Expert Electronics e compatibili) come secondo sistema nativo: non fatto. Il protocollo e' descritto in `docs/analisi-decodium-cat.md`, sezione 3.1.
- Rilevamento **attivo** (interrogare la radio con le sonde di Hamlib): non fatto, volutamente. Se si fara' sara' un'opzione spenta.
- Misuratori (S-meter, potenza, ROS, ALC) e limite di ROS; keep-alive e backoff speciali per le Icom con CI-V; sorgente audio TX (Rear/Data, Front/Mic).
- Rendere disponibili i **sorgenti** di Hamlib (e di libusb) con la stessa versione dei file inclusi, oltre al rimando a https://github.com/Hamlib/Hamlib gia' in `VERSIONE.txt` e nei testi di licenza.
- Inseguire la radio se Windows rinumera la porta (la chiave dell'apparato fisico per questo c'e' gia' nel rilevamento).

## Prove

Automatiche (senza radio): un finto `rigctld` (`rig_control::fake_rigctld`, attivo con la funzione `testing` e nelle prove) per lettura, guasti, riconnessione, PTT e split;
`rigctld` vero con il modello 1 «Dummy» se trovato (avvio del processo, lettura, comandi, processo ucciso dall'esterno, chiusura); le rotte con le prove di `remote-api/tests/cat.rs`;
la scheda con le prove di componente (`app/src/settings/CatTab.test.tsx`).

Con la radio vera (le fa il proprietario), nell'ordine:

1. **Carico fittizio o antenna scollegata.** Impostazioni → Radio CAT → «Rileva la mia radio»: controlla che la FT-2000 compaia (o scegli «Yaesu FT-2000» dall'elenco), porta COM giusta, 4800-38400 baud, 8 bit, 2 stop, controllo hardware. **Applica**, **Collega**.
2. Lo stato diventa «Collegata» con la frequenza giusta. Gira la manopola della radio: dopo 2 s la frequenza in Aurora segue, senza che il VFO si muova da solo.
3. Cambio banda dall'interfaccia (e da uno spot del cluster): la radio ci va e, con «Modo scritto» = USB dati, torna nel modo dati anche se ha richiamato quello della banda.
4. **PTT**: controlla che alla connessione la radio **non** trasmetta; prova il PTT dal CAT (poi RTS e DTR se l'interfaccia li ha). Durante i primi 5 s dopo «Collega» il PTT deve essere rifiutato.
5. **Spegnimento**: con la radio in trasmissione su carico fittizio, premi **Scollega**, poi chiudi Aurora, poi termina il processo `rigctld.exe` dal Task Manager: in ogni caso la radio deve tornare in ricezione.
6. **Split** (la FT-2000 lo usa gia' con OmniRig): «Radio» e «Simulato», spostamento di 1500-2000 Hz, ritorno in simplex dopo la trasmissione; confronto con quello che fa OmniRig oggi.
7. **Guasti**: stacca il cavo USB o spegni la radio: lo stato passa a «Collegamento perso: nuovo tentativo» e al ritorno la radio si ricollega da sola. Se Windows cambia il numero della COM, aggiornalo e premi Applica.
8. Altre radio solo se ne hai (Icom CI-V e' il caso piu' delicato; Kenwood/Elecraft).
