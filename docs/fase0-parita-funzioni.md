# Inventario delle funzioni di JTDX 159 (parità funzionale)

**Stato: bozza.** Ricavata da (a) le stringhe dell'interfaccia dentro `jtdx.exe`, (b) la struttura del manuale installato
(`share/doc/JTDX/wsjtx-main-2.2.159.html`, che è la vecchia guida WSJT-X 1.6: i menu vi compaiono solo come immagini),
(c) i file in `share/jtdx` e `bin/sounds`. **Va completata e confermata sfogliando le finestre di JTDX** (menu, Impostazioni, filtri),
perché le stringhe non dicono dove stia ogni voce.

Colonna **Via**: *UDP* = il protocollo UDP la espone (già nota: Heartbeat, Status, Decode, Reply, HaltTx, FreeText, QsoLogged);
*JTDX* = per ora resta nella finestra di JTDX; *Aurora* = da riscrivere nell'interfaccia di Aurora.
Colonna **Pri.**: A = indispensabile per sostituire JTDX nell'uso quotidiano, M = utile, B = rinviabile.

## Modi e decodifica

| Funzione | Via | Pri. |
|---|---|---|
| FT8 (modo principale), FT4 | UDP (decode, stato) + JTDX (motore) | A |
| JT65, JT9, JT65+JT9, T10 | JTDX | M |
| WSPR (`wsprd_jtdx.exe`) | JTDX | B |
| Profondità/filtri di decodifica, soglie, sottrazione del segnale | JTDX (verificare Configure UDP) | A |
| Decode multipli per ciclo, indicatore bassa confidenza, fuori onda | UDP (campi di Decode) | A |
| Modo SWL (solo ascolto) | JTDX | B |

## Operatività

| Funzione | Via | Pri. |
|---|---|---|
| Monitor on/off, Enable Tx, Halt Tx, Tune | UDP (HaltTx) + JTDX | A |
| Risposta con doppio clic su una stazione | UDP (Reply) | A |
| Messaggi Tx 1–6, messaggio libero, macro di trasmissione | UDP (FreeText) + JTDX | A |
| Frequenze Rx/Tx, Tx = Rx, split, spostamento della frequenza durante la Tx | JTDX / rigctld | A |
| Messaggi Tx nella finestra della frequenza Rx; linea vuota tra i periodi; nascondi i messaggi parziali | Aurora (visualizzazione) | M |
| ID in CW (periodico e dopo il 73) | JTDX | B |
| Pulizia di DX Call e griglia dopo il log | Aurora | M |

## Radio e audio

| Funzione | Via | Pri. |
|---|---|---|
| Controllo CAT via Hamlib (`libhamlib`, `rigctld`, `rigctlcom`) | rigctld (da verificare, H3) | A |
| Ham Radio Deluxe come controllo radio | JTDX | B |
| TCI per SDR | JTDX | B |
| PTT (CAT, RTS/DTR, VOX), porta seriale, velocità | JTDX / rigctld | A |
| Dispositivi audio di ingresso/uscita, canale (mono/L/R), livelli | JTDX (Aurora legge l'ingresso per la cascata, H2) | A |
| Tabella frequenze per banda e modo | JTDX → Aurora | A |

## Log e servizi online

| Funzione | Via | Pri. |
|---|---|---|
| Registrazione del QSO con conferma, log ADIF | UDP (QsoLogged) + Aurora (logbook) | A |
| Chiedere conferma prima di registrare ("Prompt me to log") | Aurora | M |
| dB dei rapporti nei commenti, conversione del modo in RTTY | Aurora | B |
| Invio a PSK Reporter (spot) | JTDX → Aurora | M |
| Invio a eQSL | JTDX → Aurora | B |
| DXSummit / cluster | JTDX → Aurora | M |
| Secondo server UDP (inoltro a software esterni) | JTDX | M |
| Cooperazione con programmi esterni (log, rotori, ecc.) | UDP | M |

## Dati e avvisi

| Funzione | Via | Pri. |
|---|---|---|
| Nomi DXCC dai decode (`cty.dat`), prefissi, paese accanto al messaggio | Aurora | A |
| Distanza in miglia o km; griglia propria/locatore | Aurora | M |
| Colori dei messaggi per tipo (CQ, DXCC, griglia, zona CQ, zona ITU, "on band") | Aurora (tema) | A |
| **Avvisi sonori** per CQ, DXCC, griglia, zone ITU/CQ, DX call (e varianti "sulla banda") — file in `bin/sounds` | Aurora | A |
| Elenchi di chiamate (`ALLCALL7.TXT`, `CALL3.TXT`), stati/province (`state_data.bin`, `grid_data.bin`) | Aurora | M |
| Effemeridi lunari (`JPLEPH`, per EME) | JTDX | B |
| Regione IARU | JTDX → Aurora | B |

## Interfaccia

| Funzione | Via | Pri. |
|---|---|---|
| Cascata (Wide Graph) con zoom, paletta, rapporto banda | Aurora (nuova, WebGPU) | A |
| Finestre dei messaggi Rx (banda e frequenza Rx), scorrimento | Aurora | A |
| Stile scuro | Aurora (tema Notte radio) | A |
| Scorciatoie da tastiera e comandi speciali del mouse | Aurora | A |
| Salvataggio audio (`Save all`, `Save decoded`), riproduzione di file | JTDX | M |
| Scaricamento di campioni audio (`Download Samples`) | JTDX | B |
| Lingue dell'interfaccia (sono presenti traduzioni multiple) | Aurora (italiano per primo) | M |

## Funzioni nuove proposte (da scegliere con Tiziano)

1. **Automazione dei QSO** con regole, prova a vuoto e salvaguardie (già in specifica §6).
2. **Controllo da telefono** su LAN, Tailscale e Cloudflare (già in specifica §7).
3. Avvisi push sul telefono per stazioni/paesi cercati ("wanted list").
4. Pannello premi con avanzamento (DXCC, WAZ, griglie, stati) e mancanti per banda/modo.
5. Mappa globale in tempo reale con terminatore giorno/notte e archi luminosi tra le stazioni.
6. Riproduzione dei cicli decodificati (come registratore) e confronto tra due bande.
7. Statistiche di propagazione per banda (SNR medio, paesi sentiti per ora) e suggerimento della banda migliore.
8. Temi di colore multipli (oltre a "Notte radio"), incluso uno chiaro leggibile al sole.

## Da fare per chiudere l'inventario

- [ ] Tiziano scorre le finestre di JTDX (menu File/Vista/Modo/Decodifica/Salva/Aiuto, tutte le schede di Impostazioni, il pannello dei filtri) e segnala le voci mancanti o sbagliate qui sopra.
- [ ] Verificare nei sorgenti pubblici di JTDX quali impostazioni passano da `Configure` (UDP tipo 15) e quali no.
- [ ] Assegnare a ogni riga il valore finale di *Via* e *Pri.*
