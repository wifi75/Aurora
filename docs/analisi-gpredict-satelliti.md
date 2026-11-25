# Gpredict: come comanda una radio satellitare via rigctld

Gpredict e' un programma aperto (GPL, come Aurora): se se ne riprende qualcosa di preciso, vanno dati i crediti (`THIRD-PARTY-NOTICES.md`). Quello che segue descrive solo la sua logica, per confrontarla con `mode-sat` e `rig-control/src/ic820h_civ.rs` di Aurora (IC-820H, caso #83 di Vladimir).

Sorgente analizzato: `D:\progetti\pc\gpredict-analisi\sorgente\src\` (sola lettura). Righe riferite ai file di quella copia.

## 1. Modalità di controllo ("Radio type")

Scelta nell'editor dell'apparecchio, menu a tendina "Radio type" (`sat-pref-rig-editor.c:309-356`); enum in `radio-conf.h:37-42`:

| Voce | Enum | Ciclo eseguito | Comandi |
|---|---|---|---|
| RX only | RIG_TYPE_RX | `exec_rx_cycle` (`gtk-rig-ctrl.c:1693`) | solo `F`/`f` sul VFO corrente; sospeso se PTT=ON |
| TX only | RIG_TYPE_TX | `exec_tx_cycle` (`:1816`) | solo `F`/`f` (uplink) quando PTT=ON |
| Half-duplex | RIG_TYPE_TRX | rx + tx (`:1942`) | sempre `F`/`f`: frequenza RX o TX secondo il PTT letto; il PTT è obbligatorio (`sat-pref-rig-editor.c:221-227`) |
| Full-duplex | RIG_TYPE_DUPLEX | `setup_split` all'avvio (`:3006-3010`) poi `exec_duplex_cycle` (`:2136`) | downlink con `F`/`f`, uplink con `I`/`i` (split TX freq) |
| FT817/857/897 auto | TOGGLE_AUTO | `exec_toggle_cycle` (`:1948`) | `S 1 <vfo>` all'avvio, uplink con `I` ogni 10 s solo in RX |
| FT817/857/897 manuale | TOGGLE_MAN | idem, uplink scritto quando l'utente preme SPAZIO (`:2788-2815`), poi `T 1` |

Non esiste una voce "Set freq and mode": Gpredict non tocca mai il modo (vedi §4). Esiste poi "PTT status" (None / Read PTT `t` / Read DCD, `sat-pref-rig-editor.c:358-378`) e "VFO Up/Down" (solo per Full-duplex, `:380-405`).

## 2. Sequenza dei comandi

All'apertura (`rigctrl_open`, `gtk-rig-ctrl.c:2963`):
1. `\set_vfo_opt 1` poi `\chk_vfo`: se la risposta è `1` tutti i comandi successivi portano l'argomento VFO (`get_vfo_opt`, `:2673-2701`).
2. Solo Full-duplex: `setup_split` (`:1619-1667`) manda **una volta** `S 1 Main` (oppure con vfo_opt `S Sub 1 Main`) se l'uplink è MAIN; `S 1 Sub` / `S Main 1 Sub` se l'uplink è SUB; analogo per VFOA/VFOB. Il commento a `:3007` dice che serve a mettere la radio "in SAT" lato Hamlib anche se lo è già.

Ad ogni ciclo, Full-duplex:
- Downlink (`exec_rx_cycle`): eventuale `t` per il PTT; `f` (o `f currVFO`, `:2637-2650`) per leggere; se diverso dall'ultimo impostato di ≥1 Hz è sintonia manuale (§5); altrimenti calcola `freq_sat + doppler_down − LO` (`:1754-1758`) e se differisce di ≥1 Hz manda `F <hz>` / `F currVFO <hz>` (`:2547-2560`), aspetta 5 ms (`WR_DEL`, `:68`) e rilegge con `f` salvando il valore letto come riferimento (`:1783-1790`).
- Uplink (`exec_duplex_tx_cycle`, `:2029-2134`): legge `i` / `i currVFO` (`:2708-2726`), confronto ≥1 Hz; poi `freq_sat_up + doppler_up − LOup` (`:2098`) e manda `I <hz>` (con vfo_opt **`I VFOA <hz>`**, fisso, `:2568-2580`), 5 ms, rilegge `i`.

Quindi: con UNA radio full-duplex usa `S` una volta e poi sempre `F` (VFO corrente = RX) e `I` (TX split). Non usa mai `V` (set_vfo) né due `F` su VFO diversi. Due VFO "veri" ci sono solo con due radio (`exec_dual_rig_cycle`, `:2142`: `F` su due socket distinti). In Half-duplex usa un solo `F`, ripartendo da zero il riferimento dell'altro lato al cambio PTT (`:1792-1802`, `:1917-1928`).
Uscita: per i modi toggle `S 0 <vfo>` (`unset_toggle`, `:2615-2625`, chiamato a `:2950-2954`); per Full-duplex nessun comando di uscita.

Nota: `get_vfos` (`:1569-1616`) assegna variabili locali e non restituisce niente: codice morto/bacato, i nomi Main/Sub servono solo dentro `setup_split`.

## 3. Una o due connessioni

Una "Radio" è **un solo rigctld** (`ctrl->sock`, `:2971`). Un secondo socket (`sock2`) esiste solo se si sceglie un secondo apparecchio per l'uplink nel pannello (configurazione dual rig, `:2979-2987`). In `sat-pref-rig-data.h` non ci sono "Duplex TRX"/"Simplex": contiene solo le colonne della tabella (`:36-38` PTT, VFO Up, VFO Down). Le voci equivalenti sono il "Radio type" di §1.

## 4. Modo satellite e modo operativo

- SATMODE: nessun `\set_func SATMODE` né `U SATMODE`. L'unico modo di entrare "in satellite" è il comando split `S` (`:3007-3009`); per l'IC-910 Hamlib interpreta lo split su Main/Sub. L'accensione del SAT sulla radio resta a mano.
- Modo USB/LSB/FM: **mai** inviato (`M`/`m` non compaiono in `gtk-rig-ctrl.c`). Il file dei transponder ha una chiave `MODE` (`trsp-conf.c:41,176`) ma è solo descrittiva; inoltre in scrittura salva il nome al posto del modo (`trsp-conf.c:256`, difetto). Usa invece `INVERT` per calcolare l'uplink invertito (`gtk-rig-ctrl.c:355-358`, `381-384`).
- Passaggio MAIN/SUB: non lo fa; si affida a split (`S`) + `I`. Non manda `V Main`/`V Sub`.

## 5. Periodo, soglie, sintonia manuale

- Periodo: timer `delay`, predefinito 1000 ms (`gtk-rig-ctrl.c:174`, `radio-conf.c:53`), regolabile 10-30000 ms nel campo "Cycle" (`gtk-rig-ctrl.c:1412`). Se il ciclo precedente non è finito salta (`:1681-1685`).
- Soglia: nuovo comando solo se la frequenza calcolata differisce di ≥1 Hz dall'ultima letta (`:1775`, `:2112`); toggle ≥10 Hz e al massimo ogni 10 s (`:1963-1964`, `:2012`).
- Sintonia manuale ("dial feedback"): a ogni ciclo legge `f`/`i`; se diverso di ≥1 Hz dall'ultimo valore scritto-e-riletto, considera che l'utente ha girato la manopola: ricava la nuova frequenza al satellite togliendo il Doppler, aggiorna la manopola del transponder e (se "lock") ricalcola l'altro lato, poi **salta** la scrittura in quel ciclo (`:1702-1743`, `:2034-2084`). Riferimento 0 = sincronia annullata (p.es. pulsante tune, cambio PTT). Il punto chiave è rileggere dopo ogni scrittura (`:1785-1790`) così l'arrotondamento del passo della radio non viene preso per manopola.
- In RX/Half-duplex col PTT letto: niente scritture RX durante TX e viceversa (`:1710`, `:1774`, `:1835`, `:1899`).

## 6. Particolari Icom

Solo nei tooltip: IC-910H = Full-duplex con MAIN↑/SUB↓ (`sat-pref-rig-editor.c:338-342`, `401-404`); FT-847 SUB↑/MAIN↓; TS-2000 B↑/A↓. Nessun riferimento a IC-820H/821H/9700, nessun comando CI-V diretto.

## Proposta: sequenza minima per l'IC-820H in Aurora

- Una sola connessione a rigctld; all'avvio `\chk_vfo` e lavorare in modo coerente (con vfo_opt: indicare sempre il VFO). Come `get_vfo_opt`, `gtk-rig-ctrl.c:2673`.
- SAT acceso sulla radio (con l'IC-820H Hamlib non ha un SATMODE affidabile: verificare con `\get_func SATMODE`; Gpredict non lo tocca).
- Evitare di affidarsi al "VFO corrente": in SAT l'820H scambia MAIN/SUB e `f` senza VFO legge quello attivo, che cambia (spiega il salto 145/435). Gpredict assume che il VFO corrente sia il downlink (`F currVFO`, `:2554`); in Aurora meglio indirizzare esplicitamente `F Sub <down>` / `F Main <up>` (vfo_opt) oppure, come Gpredict, `S 1 Main` una volta (`setup_split`, `:1645-1647`) poi `F` per RX e `I` per TX (`:2577-2579`), da provare quale delle due vie l'820H esegue senza scambiare.
- Modo: impostarlo noi una volta all'inizio del passaggio dal transponder (`M Sub USB 0` / `M Main LSB 0`, o `X` split mode), cosa che Gpredict non fa (§4).
- Ciclo ~1 s (`:174`): calcolo Doppler, scrittura solo se Δ≥1 Hz (meglio 5-10 Hz per l'820H, passo 10 Hz) (`:1775`, `:2112`); dopo ogni scrittura attesa breve e rilettura dello stesso VFO come riferimento (`:1783-1790`, `:2120-2127`).
- Manopola: se la lettura differisce dal riferimento, aggiornare la frequenza al satellite (togliendo il Doppler), ricalcolare l'altro lato con il transponder (diretto/invertito, `:342-388`) e non scrivere in quel ciclo (`:1717-1743`).
- In TX (PTT letto con `t`) non riscrivere il downlink se la radio non lo consente (`:1774`).
- Alla chiusura togliere lo split (`S 0`, come `unset_toggle` `:2615`) e ripristinare frequenze/modi salvati.

## Confronto con Aurora (2026-11-25)

- Aurora ha gia' un modulo satelliti (`mode-sat`) e, per l'IC-820H (Hamlib 3032), la scrittura dell'uplink sulla banda secondaria con trame CI-V grezze (`ic820h_civ.rs`), con rilettura prima di scrivere.
- Il difetto riferito nei pacchetti 7A547090 e B8024BF8 e' nella lettura ordinaria `f` del VFO principale (stato della radio): con SAT acceso salta fra 145 e 435 MHz e il motore la prende per cambi di banda (212 nel pacchetto di Vladimir). Gpredict da' per scontato che il VFO corrente sia sempre quello di ricezione e non risolve questo caso.
- Da fare dopo la prova con Vladimir: indicare sempre il VFO nei comandi di lettura con SAT acceso, ignorare come cambi di banda le letture che saltano fra le due bande del satellite attivo, impostare il modo (RX USB, TX LSB) dal transponder.
