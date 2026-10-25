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
| `AURORA_RIGCTLD` | percorso di `rigctld` (il file o la cartella che lo contiene) se non sta accanto ad Aurora |
| `data/radio.json` | configurazione viva e profili: modello, porta, velocita', PTT, split, modo. **Nessun segreto, nessun nominativo.** Se il file e' rovinato Aurora lo mette da parte come `radio.json.bad` e riparte dai valori di base |

### Dove si cerca `rigctld.exe`

1. nella cartella accanto al programma (e nella sua sottocartella `hamlib`);
2. nella variabile `AURORA_RIGCTLD`;
3. **solo per lo sviluppo**, nelle installazioni gia' presenti: `C:\JTDX64\<versione>\bin\rigctld-*.exe` (la piu' recente) e `C:\WSJT\wsjtx\bin\rigctld-*.exe`.

Se non lo trova la scheda lo scrive in alto e **Collega** risponde «Manca rigctld».
L'elenco dei modelli viene da `rigctl -l` della stessa cartella.

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
- Collegare JTDX allo stesso `rigctld` (modello «Hamlib NET rigctl»): Aurora e' un'applicazione a se' e non lo prevede.
- Inseguire la radio se Windows rinumera la porta (la chiave dell'apparato fisico per questo c'e' gia' nel rilevamento).

## Per chi distribuisce Aurora: Hamlib e la licenza

Aurora non porta con se' Hamlib: usa `rigctld.exe` come programma a parte. Per la distribuzione va **incorporato accanto al programma**
(cartella `hamlib` o la stessa del programma): `rigctld.exe`, `rigctl.exe`, `libhamlib-4.dll`, `libusb-1.0.dll` e le DLL del compilatore che lo accompagnano.
Versione consigliata: **Hamlib 4.7.x** (come Decodium); prendere il pacchetto Windows ufficiale del progetto Hamlib e registrare nelle note di rilascio la versione e lo SHA-256 dei file.

Licenze (dalle intestazioni dei sorgenti, **non e' una consulenza legale**):

- la **libreria** Hamlib (`libhamlib-4.dll`, i backend delle radio) e' **LGPL-2.1 o successiva**;
- i **programmi** come `rigctld` e `rigctl` sono **GPL-2.0 o successiva**, compatibile con la GPL-3.0 di Aurora;
- lanciare `rigctld` come processo separato e parlargli su TCP non pone problemi di collegamento: e' un protocollo, non un'unione di programmi.

Da fare alla consegna: inserire i testi `COPYING` (GPL) e `COPYING.LIB` (LGPL) dell'Hamlib scelto accanto ai file, indicare dove si prende il sorgente
(https://github.com/Hamlib/Hamlib, stessa versione) e aggiungere Hamlib e le DLL di terzi che porta (libusb e librerie del compilatore, ognuna con la sua licenza)
a `THIRD-PARTY-NOTICES.md`. `docs/licenze-terze-parti.md` e' generato da `scripts/licenze-terze-parti.py` dalle dipendenze di Rust e npm: non copre i file binari, che vanno in `THIRD-PARTY-NOTICES.md`.
Rileggere il testo della `COPYING.LIB` della versione scelta prima del rilascio.

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
