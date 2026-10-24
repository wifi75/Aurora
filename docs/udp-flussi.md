# Mappa dei flussi UDP intorno a JTDX (stazione di Tiziano)

Fonti: configurazione di JTDX (`JTDX.ini`), configurazione di Log4OM2 (backup del 2026-10-22), reverse di `jtdx.exe` con REA, pacchetti reali catturati il 2026-10-24.
Le righe marcate **(dedotto)** non sono ancora state osservate dal vivo.

## Chi parla con chi

```
                       +--------------------------------------+
                       |  JTDX 2.2.159 (id "JTDX", schema 2)   |
                       |  decodifica, codifica, CAT, PTT, TX   |
                       +------+-----------------------+-------+
        server UDP PRIMARIO   |                       |  server UDP SECONDARIO
        127.0.0.1:2333        |                       |  127.0.0.1:2237
   Heartbeat, Status, Decode, |                       |  QSO in formato ADIF
   Clear, QsoLogged, ADIF     |                       |  al termine del contatto
                              v                       v
                  +-------------------+     +------------------------------+
                  |  GridTracker2     |     |  Log4OM2 "WSJT-X DIRECT"     |
                  |  (mappa, avvisi)  |     |  ascolta su UDP 2237         |
                  +---------+---------+     |  -> salva il QSO nel log     |
              inoltra (dedotto) |           +------------------------------+
        2236 "GRIDTRAKER LOG"   |
        2238 "GRIDTRAKER CALL"  v
                  +------------------------------+
                  |  Log4OM2 (ascoltatori 2236,  |
                  |  2238: salvataggio e upload) |
                  +------------------------------+

   Comandi di ritorno verso JTDX (AcceptUDPRequests = true): Reply, Replay, HaltTx,
   FreeText, HighlightCallsign (evidenziare i nominativi gia' lavorati), tipi 50 e 51.
```

## Dati osservati

| Flusso | Direzione | Dove | Contenuto | Stato |
|---|---|---|---|---|
| Primario | JTDX → GridTracker2 | `127.0.0.1:2333` | Heartbeat, Clear, Status, Decode (catturati dal vivo); QsoLogged e LoggedADIF alla fine del QSO (`EnableUDP1adifSending=true`) | catturato (tranne QSO) |
| Secondario | JTDX → Log4OM2 | `127.0.0.1:2237` | QSO in ADIF (`EnableUDP2adifBroadcast=true`) | da dettagliare con il reverse (in corso) |
| Inoltro | GridTracker2 → Log4OM2 | `2236` (tipo 1) e `2238` (tipo 3: `SAVE_NEW_QSO`, `UPLOAD_QSO`) | log e chiamate | dedotto dalla configurazione di Log4OM |
| Ritorno | GridTracker2 → JTDX | indirizzo mittente di JTDX (porta effimera) | Reply (doppio clic su una stazione), evidenziazione dei nominativi | dedotto |

Impostazioni di JTDX che contano: `UDPServer=127.0.0.1`, `UDPServerPort=2333`, `UDP2Server=127.0.0.1`, `UDP2ServerPort=2237`,
`AcceptUDPRequests=true`, `AcceptUDPReplyMessages=1` (solo CQ), `ApplyFiltersToUDPmessages=true`, `preventFalseUDPspots=true`,
`udpWindowToFront=true`, `udpWindowRestore=true`, `Tx2QSO=true` (un Reply avvia la trasmissione).

## Il QSO a fine contatto

Quando il contatto si chiude JTDX registra il QSO e lo manda in due modi: messaggi `QsoLogged` (tipo 5) e `LoggedADIF` (tipo 12) sul server primario, e l'ADIF sul secondario verso Log4OM, che lo scrive nel proprio log. **Il log "ufficiale" di Tiziano e' Log4OM2.** Aurora non deve duplicarlo: deve leggere gli stessi eventi per le proprie statistiche, ma lasciare a Log4OM la scrittura del log principale (o, in alternativa, inoltrargli il QSO).

## Come puo' inserirsi Aurora (da decidere)

Un solo programma alla volta puo' ascoltare la porta 2333. Opzioni:

1. **Aurora come hub (consigliata).** In JTDX il server primario passa a una porta di Aurora (per esempio 2334). Aurora ascolta, **inoltra identico il flusso a GridTracker2 su 2333** e rimanda a JTDX i comandi che arrivano da GridTracker. GridTracker2 e Log4OM2 non cambiano; il secondario di JTDX verso Log4OM resta com'e'. Richiede una sola modifica a JTDX.
2. **Multicast.** JTDX supporta come server un indirizzo multicast (lo dice il suo stesso aiuto): Aurora e GridTracker2 si iscrivono allo stesso gruppo. Pulito, ma richiede di cambiare l'impostazione di entrambi.
3. **Aurora al posto di GridTracker2** (a lungo termine, quando avra' mappa e avvisi propri).

In ogni caso Aurora deve copiare l'**identificativo** dei pacchetti di JTDX nei comandi, accettare che **piu' client** mandino comandi a JTDX e non occupare la porta 2333 mentre GridTracker2 e' acceso.

## Cose da non rompere

- Il collegamento diretto JTDX → Log4OM2 sulla 2237: e' quello che salva i QSO.
- La porta 2333 di GridTracker2: durante la prova del 2026-10-24 l'ho tenuta occupata io per alcuni minuti (GridTracker2 poteva non ricevere dati); ora e' libera.
- Le salvaguardie di trasmissione: un Reply UDP fa trasmettere (vedi `fase0-rapporto.md`).
