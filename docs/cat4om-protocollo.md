# CAT4OM — riferimento tecnico del protocollo per un client nativo in Aurora

Fonte: *CAT4OM Integration Manual*, document version 2026-10-08 (rev 22), wire protocol `1.0.0`.
Tutti i nomi di campi, azioni, eventi e codici sono copiati dal manuale. Dove il manuale tace è scritto
«non documentato». Obiettivo: client Rust (tokio + tungstenite) che affianca/sostituisce rigctld.

---

## 1. Trasporto

| Voce | Valore |
|---|---|
| Trasporto | WebSocket, messaggi **text** UTF-8 JSON |
| Path | `/` |
| URL Management | `ws://<host>:<managementPort>/` (default `17300`) |
| URL Control | `ws://<host>:<controlPort>/` (una porta per gruppo, scelta da Cat4OM: per esempio `5001`; si legge da `listGroups` sulla porta di gestione) |
| Nomi JSON | lower camel case, **case-sensitive** |
| Dispatch | leggere `type`, poi interpretare la forma concreta |
| Frame binari | sul Control Port arrivano **solo** dopo `startAudioStream`; altrimenti vanno ignorati in silenzio |
| TLS (`wss://`) | non documentato (il manuale parla solo di `ws://` e consiglia VPN/reverse proxy) |

- **Non cablare le porte**: leggerle da configurazione/utente; `welcome.managementPort` dà la porta Management
  del servizio raggiunto tramite Control (se assente: sconosciuta, **non** ripiegare su 17300).
- Raggiungibilità: per default il Service ascolta solo su loopback (`hub.bindAddress`; valori `lan`, `any`, IP).
  Una connessione TCP rifiutata verso un Service attivo è quasi sempre questa impostazione: il manuale
  suggerisce di dirlo nel messaggio d'errore ("check Settings → SERVER").
- Modello **push**: non fare polling di `getState`/`getAllState`; usarli solo per bootstrap, refresh manuale,
  recupero dopo riconnessione.

### 1.1 Management vs Control

| Canale | Scopo |
|---|---|
| Management (`17300`, uno per servizio) | configurazione server, ciclo di vita gruppi/radio, handbook, porte seriali, stato servizio (`serviceStatus`) |
| Control (uno per gruppo in esecuzione) | controllo in tempo reale delle radio del gruppo; le radio si indirizzano con `radioId` dentro le richieste |

Per Aurora (programma che controlla una radio) il Management è **opzionale**: serve per scoprire gruppi e
porte Control, o avviare un gruppo fermo.

### 1.2 Scoperta automatica (UDP, opzionale)

- Multicast `239.255.15.18`, beacon Service→client su UDP `17499`, richieste client→Service su UDP `17498`, TTL 1.
- Beacon ogni 10 s (e subito a ogni cambiamento). Ascolto: bind `0.0.0.0:17499` con `SO_REUSEADDR`, join del gruppo.
- Beacon:
```json
{ "type": "discovery", "v": 1, "serviceName": "Shack", "instanceId": "3f1a2b7e-...",
  "protocolVersion": "1.0.0", "address": "192.168.1.20", "managementPort": 17300,
  "passwordRequired": false,
  "groups": [ { "id": "main", "name": "Main Station", "address": "192.168.1.20",
                "port": 17301, "running": true, "passwordRequired": false } ],
  "groupsTruncated": false }
```
- Richiesta immediata: inviare `{"type":"discoveryRequest"}` al gruppo multicast porta `17498` (o a
  `127.0.0.1:17498`), la risposta (un beacon unicast) arriva sullo stesso socket. Max 1 risposta/s per richiedente.
- Raggruppare per `instanceId` (un Service manda un datagramma per interfaccia). Nessun messaggio d'addio:
  scadere un server dopo ~3 intervalli (30 s). Beacon ≤ 1400 byte. Su Windows, catturare `ConnectionReset`
  (ICMP port unreachable) sulla receive e continuare. Feature Management: `serverDiscovery`; disattivabile con
  `hub.discoveryEnabled`.

---

## 2. Handshake (identico su entrambi i canali)

1. Aprire il WebSocket.
2. Primo messaggio: `hello`.
3. Risposta: `managementWelcome` (Management) / `welcome` (Control), oppure `error` e chiusura.
4. Poi le richieste normali.

Il server attende l'`hello` per circa **10 s**. Il welcome (o l'error) è garantito come **primo** messaggio
scritto dal server: qualsiasi altro primo messaggio = porta sbagliata.

### 2.1 `hello`
```json
{ "type": "hello", "protocolVersion": "1.0.0", "appName": "Aurora", "appVersion": "x.y.z" }
```
| Campo | Obbl. | Note |
|---|---|---|
| `type` | sì | `"hello"` |
| `protocolVersion` | sì | `1.0.0`; major diverso → `PROTOCOL_VERSION_MISMATCH` |
| `appName` | sì | nome mostrato nei dashboard |
| `appVersion` | sì | |
| `clientInstanceId` | no | accettato, nessun effetto in questa versione |
| `requestMaster` | no | accettato, nessun effetto ("il primo connesso diventa master") |
| `receiveSignal` | no | `true` = ricevere eventi `signalUpdate` (solo Control) |
| `noRegister` | no | osservatore di sola lettura (non conta come client, niente ownership, ignora richieste) — **non** per Aurora |
| `user` | no | accettato e ignorato |
| `auth` | no | prova di autenticazione, obbligatoria se l'endpoint ha password |
| `password` | no | legacy, non usare |

### 2.2 Autenticazione `hello.auth`
```json
"auth": { "scheme": "cat4om-sha256-minute-v1", "timestampUtc": "202608111423", "proof": "base64-sha256-value" }
```
Prova: `timestampUtc` = ora UTC al minuto `yyyyMMddHHmm`; `proof` = Base64(SHA-256(UTF-8(password + "\n" + timestampUtc))).
Il server accetta minuto corrente, −1 e +1. Password Management e password di gruppo sono separate
(`hub.managementPassword`, `group.authentication.password`), nessun fallback reciproco.

### 2.3 `welcome` (Control)
```json
{ "type": "welcome", "endpoint": "control", "protocolVersion": "1.0.0", "groupId": "main",
  "clientId": "f5f02de4-7d1c-40a1-9de0-8ea536b6ea38", "role": "slave", "sessionToken": null,
  "radios": [ /* RadioState[] */ ], "seq": 0, "authRequired": false, "authSchemes": ["password"],
  "features": ["stateUpdateSeq","commandCompleted","txSafetyLock","diagnostics","cwKeyer","tune",
               "rawCommands","bridgeExclusiveControl","audioStreaming","receiveSignal"],
  "instanceId": "9f1c4e2a-77b1-4c0e-9a35-2f6d8c0b41e7", "serviceName": "SHACK-PC",
  "keepaliveIntervalMs": 10000, "managementPort": 17300 }
```
Validare `type == "welcome"`, `endpoint == "control"`, major di `protocolVersion`. Salvare `clientId`, `groupId`,
`role` (`master` | `slave` | `observer`), `radios[]`, `seq`, `instanceId`, `keepaliveIntervalMs`, `features[]`.
`sessionToken`: presente, non usato dai client in questa versione.

### 2.4 `managementWelcome`
```json
{ "type": "managementWelcome", "endpoint": "management", "protocolVersion": "1.0.0",
  "clientId": "2d724de1-0aa7-4f97-b785-5ea1d099c221", "serviceVersion": "0.12.6.0",
  "authRequired": false, "authSchemes": ["password"],
  "features": ["statusSubscription","audioLevelMonitor","configurationValidation","logLevelControl",
               "handbookCatalog","deviceDiscovery","connectionTest","lifecycle","supportBundle"],
  "instanceId": "9f1c4e2a-...", "serviceName": "SHACK-PC", "keepaliveIntervalMs": 15000,
  "supportedBridgeProfiles": ["tci"] }
```

### 2.5 Rifiuto dell'handshake
```json
{ "type": "error", "code": "PROTOCOL_VERSION_MISMATCH", "message": "..." }
```
Codici: `MISSING_HELLO`, `PROTOCOL_VERSION_MISMATCH`, `MISSING_PARAM` (manca appName/appVersion),
`AUTH_REQUIRED`, `AUTH_FAILED`. Nessun `id`; il server poi chiude.

### 2.6 Regole di compatibilità (normative)
Confrontare solo il major; `type`/evento/campo/codice sconosciuti → ignorare e continuare (mai chiudere);
stringhe enum (`mode`, `tuner`, `role`, `connectionStatus`, `status`, categorie `changes`) sono insiemi aperti;
campo opzionale assente = "non pubblicato", mai un default inventato; tollerare `stateUpdate` identici
consecutivi e non assumere 1 comando = 1 `stateUpdate`; estensioni proprie col prefisso `x-`.
In serde: **non** usare `deny_unknown_fields`.

---

## 3. Messaggi comuni

### 3.1 Richiesta
```json
{ "type": "request", "id": "uuid", "clientId": "server-assigned-client-id",
  "action": "actionName", "radioId": "optional-radio-id", "params": { } }
```
`id` obbligatorio su Control (usare UUID, mai riusare mentre pendente); `clientId` obbligatorio su Control;
`radioId` per le azioni su una radio; nessun identificativo di gruppo (lo dà la porta).

### 3.2 Risposta
```json
{ "type": "response", "id": "...", "success": true, "status": "optional", "radioId": "...", "result": { } }
```
```json
{ "type": "response", "id": "...", "success": false, "radioId": "...", "result": null,
  "error": { "code": "MISSING_PARAM", "message": "...", "retryAfterMs": 2380, "details": { } } }
```
- Decidere **solo** su `success`. `status` è secondario e aperto (`SUPERSEDED`, `DIAGNOSTICS_STARTED`,
  `FAILSAFE_UNKEY`, ...). `errors[]` solo per `setConfiguration`.
- `retryAfterMs` presente solo su errori temporanei (oggi `OWNERSHIP_COOLDOWN`).
- Le risposte possono arrivare **fuori ordine** (lo spegnimento TX scavalca la coda): correlare sempre per `id`.
- Timeout lato client: 10 s è il default suggerito; le operazioni radio possono durare di più.

---

## 4. Gruppi, radio e selezione

### 4.1 Elenco gruppi (Management)
```json
{ "type": "request", "id": "status-1", "clientId": "<mgmt clientId>", "action": "getServiceStatus", "params": {} }
```
`result.groups[]`: `id`, `name`, `controlPort`, `autoStart`, `isRunning`, `radioCount`, `connectedClients`,
`clients[]`, `radios[]` (`radioId`, `radioName`, `connectionState`, `lastSeen`, `activeVfo`, `frequency`, `mode`).
Nota: qui il campo è `connectionState`, nel `RadioState` Control è `connectionStatus`.

Altre azioni Management utili: `subscribe` / `unsubscribe` (push `serviceStatus`, keepalive ~15 s con
`isKeepAlive`), `getGroupStatus` (`groupId`), `startGroup` / `stopGroup` / `restartGroup` (`groupId`),
`startRadio` / `stopRadio` (`groupId`, `radioId`), `getHandbooks`, `getHandbookDetails` (`handbookId`),
`powerOnRadio` / `powerOffRadio`. Un gruppo fermo non ha Control Port aperto: va avviato con `startGroup`.
Le radio Follower compaiono solo su Management (blocco `follow`) e non sono comandabili.

### 4.2 Elenco radio (Control)
- `welcome.radios[]` è già lo snapshot completo; `getAllState` → `result.radios[]`; `getState` (con `radioId`) → `result`.
```json
{ "type": "request", "id": "state-all-1", "clientId": "<clientId>", "action": "getAllState", "params": {} }
{ "type": "request", "id": "state-run-1", "clientId": "<clientId>", "action": "getState", "radioId": "run", "params": {} }
```
- "Selezionare" una radio = usare il suo `radioId` nelle richieste. Non esiste un'azione di selezione.
- Follower: assenti dal Control; un comando verso di essi → `RADIO_NOT_FOUND`.

### 4.3 Ownership (master/slave)
- Un solo **master per gruppo** (non per radio), con autorità di scrittura su tutte le radio del gruppo.
  Il primo client connesso è master, gli altri slave. Se il master si disconnette il server promuove un altro.
- Chi può scrivere: confrontare `stateUpdate.masterId` con il proprio `clientId`.
- **Richiesta**: `getOwnership` (nonostante il nome, *prende* l'ownership):
```json
{ "type": "request", "id": "own-1", "clientId": "<clientId>", "action": "getOwnership", "params": {} }
{ "type": "response", "id": "own-1", "success": true, "result": { "role": "master" } }
```
  Poi evento `ownershipChanged` a tutti e `stateUpdate` con `groupChanges: ["ownership"]`.
- **Rilascio**: azione esplicita «non documentata». L'ownership finisce chiudendo la connessione (è legata alla
  connessione: dopo riconnessione nuovo `clientId` e nuovo ruolo) o quando un altro la prende.
- **Osservare**: tutti i ruoli ricevono i push; `noRegister: true` = osservatore puro.
- Cooldown: tra due trasferimenti richiesti dallo stesso client (default 5 s, minimo 2 s,
  `hub.ownershipCooldownMs`) → `OWNERSHIP_COOLDOWN` con `retryAfterMs`. La prima richiesta di un client non è mai
  bloccata; la richiesta del master attuale è sempre successo no-op. Non riprovare in loop.
- Durante TX di **qualsiasi** radio del gruppo `getOwnership` è rifiutato (`TX_IN_PROGRESS`).
- Azioni solo master: `setFrequency`, `setMode`, `setPtt`, `setVfo`, `setSplit`, `setRit`, `setXit`, `vfoAtoB`,
  `vfoBtoA`, `vfoEqualize`, `vfoSwap`, `setRxVfos`, `sendRaw`, `sendRawCmdFireAndForget`, `sendCw`, `stopCw`,
  `setCwSpeed`, `cwKeyDown`, `runDiagnostics`, `cancelDiagnostics`, `powerOff`, `reconnectDisconnectedRadios`
  (+ `tune` e stream audio TX). Da slave → `NOT_MASTER`.
- Radio presa da un profilo di emulazione (TCI/Hamlib): `bridgeControlledBy` non vuoto, comandi nativi →
  `RADIO_CLAIMED`.

### 4.4 TX safety lock (con `ptt == true`)
1. Ownership congelata.
2. Sulla radio in TX sono accettati solo `setPtt` `enabled:false` e letture (più `stopCw`, `sendCw` con
   `append:true`, `cwKeyDown` `false`, `startAudioStream`/`stopAudioStream`).
3. Interlock: una sola radio del gruppo in TX (`txInterlock`, attivo di default).
Errore `TX_IN_PROGRESS` con `error.details.rule` = `radioTransmitting` | `txInterlock` e
`details.transmittingRadioId`. Failsafe server: unkey se il master si disconnette in TX; watchdog TX per radio
(default 120 s, massimo 180 s, `txTimeoutSeconds`).

---

## 5. Comandi necessari ad Aurora

Lettura frequenza/modo/PTT/split/VFO: **non esistono azioni `get*` dedicate**; si leggono dal `RadioState`
(welcome, `stateUpdate`, `getState`). Dopo una scrittura riuscita il valore va preso dal `stateUpdate`
successivo (la radio può arrotondare/normalizzare).

### 5.1 Leggere la frequenza
`radios[i].vfos["A"].frequency` (Hz, intero), VFO attivo in `activeVfo`, VFO TX in `txVfo`.
I nomi VFO sono opachi (`A`, `B`, `MAIN`, `SUB`, ...): usare `availableVfos`.
Con `frequencyOffset` (transverter) la frequenza è già quella pubblica: non applicare offset lato client.

### 5.2 `setFrequency`
```json
{ "type": "request", "id": "freq-1", "clientId": "<clientId>", "action": "setFrequency",
  "radioId": "run", "params": { "frequency": 14074000, "vfo": "A" } }
{ "type": "response", "id": "freq-1", "success": true, "radioId": "run", "result": null }
```
`vfo` opzionale (default VFO attivo). `commandVfoScopes.SetFrequency == "activeOnly"` → per altro VFO
`INVALID_TARGET_VFO`. Fuori range offset → `FREQUENCY_OUT_OF_RANGE`. Più `setFrequency` consecutivi per stessa
radio e stesso VFO senza altre richieste in mezzo: i precedenti rispondono `success:true`, `status:"SUPERSEDED"`.
Coda richieste per connessione: max 256 in attesa.

### 5.3 Modo: lettura `vfos[v].mode`; scrittura `setMode`
```json
{ "type": "request", "id": "mode-1", "clientId": "<clientId>", "action": "setMode",
  "radioId": "run", "params": { "mode": "USB", "vfo": "A" } }
```
Il server mette in maiuscolo. Validi: `supportedModes`. Nomi comuni: `LSB, USB, CW, CW-R, AM, AM-N, FM, FM-N,
RTTY, RTTY-R, RTTY-LSB, RTTY-USB, DATA-USB, DATA-LSB, DATA-FM, DATA-AM, FT8, FT4, DV, C4FM`.
Possibile `"SetMode": "activeOnly"`. Radio senza `SetVfo` (es. FlexRadio a slice): indirizzare il VFO con `vfo`.
Larghezza filtro: solo lettura (`filterWidth`); impostarla «non documentato».

### 5.4 PTT: lettura `ptt`; scrittura `setPtt`
```json
{ "type": "request", "id": "ptt-1", "clientId": "<clientId>", "action": "setPtt",
  "radioId": "run", "params": { "enabled": true } }
```
`enabled:false` = RX, elaborato subito davanti alla coda. `enabled` mancante/non booleano → il server forza
PTT off e risponde con `status:"FAILSAFE_UNKEY"` (richiesta precedente da considerare non onorata).
Inviare sempre `setPtt false` prima di chiudere. Response di successo: forma esatta «non documentata» oltre
`success`.

### 5.5 Split: lettura `split`, `txVfo`; scrittura `setSplit`
```json
{ "type": "request", "id": "split-1", "clientId": "<clientId>", "action": "setSplit",
  "radioId": "run", "params": { "enabled": true, "txVfo": "B" } }
```
`txVfo` opzionale; alcune radio deducono da sole il VFO TX.

### 5.6 Selezione VFO: `setVfo`
```json
{ "type": "request", "id": "vfo-1", "clientId": "<clientId>", "action": "setVfo",
  "radioId": "run", "params": { "vfo": "B" } }
```
Solo se `availableCommands` contiene `SetVfo`. Operazioni: `vfoAtoB`, `vfoBtoA`, `vfoEqualize`, `vfoSwap`
(`params: {}`). Ricevitori: `setRxVfos` (`vfo`, `enabled`), combinazioni in `rxVfoSets`, stato in `rxVfos`.
RIT/XIT: `setRit`/`setXit` (`enabled`, `offset` opzionale ±9999 Hz).

### 5.7 Strumenti (`metering`, solo lettura)
| Campo | Significato |
|---|---|
| `signalDbm` | segnale in dBm (preferito); S1 = −121, S9 = −73 dBm, 6 dB/S-unit |
| `sMeterMeter` | deflessione 0.0–1.0 (non S-unit) |
| `sMeter` | legacy, scala dipendente dalla radio, ultimo ripiego |
| `sMeterSub`, `sMeterMeterSub`, `signalDbmSub` | ricevitore SUB (assenti se non riportati) |
| `reducedBandwidth` | `true` = segnale non letto (radio `receiveSignal` off o `hub.reduceBandwidth`) |
| `swr` | ROS reale, max 99.9, `null` in RX |
| `swrMeter` | deflessione ROS 0.0–1.0, mai convertire |
| `power` | **impostazione** potenza 0–100 %, valida anche in RX; nessuna azione per cambiarla |
| `powerWatts` | watt erogati (TCI), `null` in RX |
| `powerMeter` | deflessione potenza 0.0–1.0, `null` in RX |
| `alc` | ALC (int) |
| `ratedPowerWatts` | (per radio) potenza nominale, se dichiarata nell'handbook |

`signalVfo` dice a quale VFO appartiene la lettura. Il solo cambio del segnale non genera `stateUpdate`: per
l'S-meter dal vivo chiedere `signalUpdate` (`hello.receiveSignal: true` o `setReceiveSignal`):
```json
{ "type": "request", "id": "sig-1", "clientId": "<clientId>", "action": "setReceiveSignal", "params": { "enabled": true } }
{ "type": "response", "id": "sig-1", "success": true, "result": { "receiveSignal": true, "signalRead": { "run": true } } }
```
Impostare livelli (RF power, guadagni): «non documentato» / non previsto.

### 5.8 Accordatore
- Stato: `tuner` (`bypass`, `active`, `tuning`, può essere `null`).
- `tune` (`params: {}`): avvia il ciclo dell'accordatore interno, **trasmette**, solo master, solo se
  `availableCommands` contiene `Tune` (altrimenti `NOT_SUPPORTED`); `ptt` true durante il ciclo, unkey forzato dal
  server dopo 20 s. Non mandare `setPtt false` subito dopo, se non per interrompere.
- Portante manuale: `cwKeyDown` `{ "enabled": true|false }` (richiede `CwSend`; errore `CW_NOT_SUPPORTED`).
- Attivare/disattivare l'accordatore (bypass on/off): «non documentato».

### 5.9 Scoperta delle capacità
- Server: `features[]` del welcome.
- Radio: `availableCommands` (es. `GetFrequencyA`, `SetFrequency`, `SetMode`, `SetVfo`, `SetSplit`, `SetPtt`,
  `VfoAtoB`, `VfoBtoA`, `VfoEqualize`, `VfoSwap`, `PowerOff`, `SetRxVfos`, `Tune`, `CwSend`, `SendRaw`; confronto
  case-insensitive), `commandVfoScopes`, `supportedModes`, `availableVfos`, `rxVfoSets`, `availableStateFields`,
  `isPushBased` (TCI: `availableStateFields` vuoto ma campi popolati), `frequencyResolution` (Hz),
  `frequencyGroups` (`hf`, `vhf`, `uhf`, `shf`), `audioCaptureAvailable`, `audioTransmitAvailable`.
- Prima di inviare: feature → `connectionStatus == "connected"` → master → `availableCommands` →
  `commandVfoScopes` → `supportedModes`/`availableVfos`.
- Limiti di banda per radio: solo `frequencyGroups` (grossolano); intervalli precisi «non documentati».

### 5.10 Stato connessione radio (`connectionStatus`)
`unknown`, `disconnected`, `connecting`, `reconnecting`, `connected`, `no_response`, `stopped`.
Ritentare le radio scollegate: `reconnectDisconnectedRadios` (master, senza `radioId`).

---

## 6. Push, keepalive, riconnessione

### 6.1 `stateUpdate`
```json
{ "type": "stateUpdate", "timestamp": "2026-06-02T20:30:00Z", "masterId": "<clientId master>",
  "seq": 148, "full": true, "groupChanges": [], "changes": { "run": ["data"] },
  "radios": [ /* RadioState completi */ ] }
```
- `full`: oggi sempre `true` (sostituire); implementare comunque `false` come merge. Assente = completo.
- `seq`: contatore **per connessione**, parte dal `seq` del welcome. `+1` normale; salto → applicare e poi
  `getAllState`; minore → gruppo riavviato: scartare lo stato e ripartire.
- `changes` (radioId → categorie): `data` (frequenza, modo, filtro, VFO attivo/TX/RX, split, PTT, stato
  connessione, RIT/XIT), `metering`, `exclusiveControl`. `groupChanges`: `ownership`, `radios`. Keepalive:
  `changes: {}`, `groupChanges: []`. Insiemi aperti: testare con "contiene".
- Rate limit per radio `hub.stateUpdateInterval` (default 250 ms): primo cambio subito, i successivi accorpati.
- Keepalive Control ≈ ogni 10 s (`keepaliveIntervalMs`).

### 6.2 `event`
Forma: `{ "type": "event", "event": "<nome>", "timestamp": "...", "radioId": null|"...", "details": { } }`.
- `ownershipChanged` → `details.masterId`.
- `commandCompleted` → `details.requestId`, `action`, `clientId`, `success`, `error`, `result` (oggi solo `runDiagnostics`).
- `diagnosticsCompleted`.
- `audioStreamStopped` → `details.streamId`, `radioId`, `direction`, `reason` (solo al proprietario).
- `signalUpdate` → `details.radios` = mappa radioId → blocco segnale (`sMeter` sempre, poi `signalDbm`,
  `sMeterMeter`, `sMeterSub`, `sMeterMeterSub`, `signalDbmSub`); finestra 200 ms, niente `seq`, sostituisce
  solo il blocco segnale.
Eventi dedicati per frequenza o PTT: non esistono, arrivano nello `stateUpdate` (categoria `data`).

### 6.3 Timeout e riconnessione
- Timeout di inattività = 3 × `keepaliveIntervalMs` (non cablare).
- Alla caduta: segnare disconnesso, svuotare le richieste pendenti, riconnettere con backoff (valori «non
  documentati»), rimandare `hello`, attendere il welcome, confrontare `instanceId` (se cambiato scartare
  `clientId`, `sessionToken`, snapshot, `seq`, configurazione), ricaricare lo stato da welcome/`getAllState`,
  **non** presumere di essere ancora master.
- Chiusura con radio in TX: il server toglie la portante (anche per il master nativo).

---

## 7. Errori

Codici Control rilevanti (insieme **aperto**, mai `match` esaustivo senza ramo di default):
`INTERNAL_ERROR`, `INVALID_REQUEST`, `MISSING_PARAM`, `INVALID_PARAM`, `UNKNOWN_ACTION`, `NOT_FOUND`, `NOT_READY`,
`NOT_SUPPORTED`, `NOT_MASTER`, `RADIO_CLAIMED`, `NOT_CONNECTED`, `RADIO_NOT_CONNECTED`, `RADIO_NOT_FOUND`,
`TIMEOUT`, `CANCELED`, `TRANSPORT_ERROR`, `BUILD_FAILED`, `SEND_FAILED`, `RADIO_REJECTED`, `PARSE_FAILED`,
`STEP_REJECTED`, `COMMAND_FAILED`, `INVALID_MODE`, `INVALID_VFO`, `INVALID_TARGET_VFO`, `FREQUENCY_OUT_OF_RANGE`,
`TX_IN_PROGRESS`, `OWNERSHIP_COOLDOWN`, `CW_NOT_SUPPORTED`, `AUDIO_NOT_AVAILABLE`, `AUDIO_DEVICE_OPEN_FAILED`,
`AUDIO_DEVICE_NOT_PRESENT`, `AUDIO_TRANSMIT_IN_USE`, `AUDIO_RATE_UNAVAILABLE`, `AUDIO_STREAM_NOT_FOUND`,
`AUDIO_STREAM_ALREADY_OPEN` (riservato, non emesso).
Management/config: `START_FAILED`, `STOP_FAILED`, `RESTART_FAILED`, `CONNECTION_FAILED`, `TEST_FAILED`,
`INVALID_CONFIG`, `SAVE_ERROR`, `INVALID_LOG_LEVEL`, `LOG_LEVEL_SAVE_ERROR`, `FORBIDDEN`,
`AUTH_CHANGE_REQUIRES_LOCAL_CLIENT`, codici di validazione configurazione (`DUPLICATE_RADIO_ID`, ...).
Handshake: vedi §2.5. `error.message` è solo testo per l'operatore.

---

## 8. Audio in rete

### 8.1 Prerequisiti
- `audioStreaming` in `welcome.features[]`.
- Radio con `audioCaptureAvailable` (RX) / `audioTransmitAvailable` (TX): indicano configurazione, non un
  dispositivo vivo.

### 8.2 Apertura / chiusura
```json
{ "type": "request", "id": "req-1", "clientId": "<clientId>", "action": "startAudioStream",
  "radioId": "ftdx101d", "params": { "direction": "rx", "sampleRateHz": 48000, "channels": 2 } }
{ "type": "response", "id": "req-1", "success": true, "radioId": "ftdx101d",
  "result": { "streamId": 1, "radioId": "ftdx101d", "direction": "rx", "sampleRateHz": 48000,
              "channels": 2, "codec": "pcm16" } }
{ "type": "request", "id": "req-2", "clientId": "<clientId>", "action": "stopAudioStream", "params": { "streamId": 1 } }
```
(Negli esempi del manuale `clientId` manca in queste richieste; la regola generale lo vuole su Control.)
- `direction`: `"rx"` | `"tx"` (obbligatorio); `sampleRateHz` default `48000`; `channels` `1` | `2`, default `2`.
- Usare i valori **restituiti**, non quelli chiesti.
- RX: nessuna ownership richiesta, disponibile anche se la radio è presa da un profilo. TX: master e radio non
  presa (`NOT_MASTER`, `RADIO_CLAIMED`); un solo detentore TX (`AUDIO_TRANSMIT_IN_USE`).
- Uno stream per client/radio/direzione: un secondo `startAudioStream` chiude il vecchio e dà un nuovo `streamId`.
- Stream chiusi automaticamente alla chiusura della connessione. `stopAudioStream` accettato anche in TX.
- Stream TX senza audio per 60 s a radio non in TX → chiuso con `audioStreamStopped`
  (`reason`: `idle for too long; the transmit lease was reclaimed`). Altro motivo: `the radio is being reconfigured`.
- Più client possono ascoltare la stessa radio a frequenze di campionamento diverse (es. 12 kHz per un decoder).
- Tassi supportati: elenco «non documentato»; per `AUDIO_RATE_UNAVAILABLE` il manuale dice di riprovare con 48000.

### 8.3 Frame binario (un messaggio WebSocket binario = header 32 byte + payload, little-endian)
| Offset | Dim. | Campo | Note |
|---|---|---|---|
| 0 | 4 | `magic` | ASCII `C4OA` |
| 4 | 1 | `version` | `1` |
| 5 | 1 | `streamKind` | `1` = rx, `2` = tx (mai `0`) |
| 6 | 1 | `codec` | `0` = `pcm16` |
| 7 | 1 | `channels` | `1` o `2` |
| 8 | 4 | `streamId` | da `startAudioStream` |
| 12 | 4 | `sampleRateHz` | |
| 16 | 4 | `sampleCount` | campioni **per canale** |
| 20 | 4 | `sequence` | contatore per stream, wrap a 2^32 |
| 24 | 8 | `timestampUs` | µs dal clock monotono del mittente |

Payload: PCM 16 bit con segno little-endian interlacciato, `sampleCount × channels × 2` byte
(20 ms stereo a 48 kHz = 960 campioni, 3840 byte). Verificare magic e versione, scartare frame illeggibili,
rifiutare codec sconosciuti. Salti di `sequence` = audio perso (spesso consumer lento: il server scarta il più
vecchio).

### 8.4 Ricezione
Nessun ack. Con 1 canale da radio stereo arriva il canale configurato per quella radio (non un mix).

### 8.5 Trasmissione
- Aprire lo stream TX **prima** di andare in trasmissione (indispensabile per radio TCI che scelgono la sorgente
  al momento del key). Lo stream non manda in TX: serve `setPtt true`.
- Frame scartati se: radio presa da profilo, non master, radio non in TX, `sampleRateHz` diverso dal
  negoziato (nessun ricampionamento). Negli esempi: frame da 20 ms (`rate / 50` campioni), `sequence` da 1,
  `timestampUs` 0.
- Banda: PCM16 48 kHz stereo ≈ 1,5 Mbit/s; 12 kHz mono ≈ 190 kbit/s.
- Latenza: valori «non documentati» (il manuale dice solo che l'audio in ritardo viene scartato e che 60 s di
  silenzio coprono più cicli FT8/FT4).

---

## 9. Emulazione Hamlib NET rigctl (sottoinsieme)

Profilo per gruppo su TCP (porta proposta `4532`). Sessione **esclusiva**: un secondo client viene rifiutato
(anche mentre la radio trasmette). Fuori dal sottoinsieme: `RPRT -11`. Rifiuti: codici `RPRT` negativi.
Uscita a sessione chiusa in TX: unkey automatico. Mentre la sessione è attiva i client nativi ricevono
`RADIO_CLAIMED` e `bridgeControlledBy = "hamlib"`.

| Setter | Lungo | Note |
|---|---|---|
| `F` | `\set_freq` | indirizzabile a VFO se negoziato il VFO mode |
| `M` | `\set_mode` | passband accettato, filtro deciso da radio/handbook |
| `T` | `\set_ptt` | `0`,`1`,`2`,`3`; ≠0 = key; `3` tune portante |
| `V` | `\set_vfo` | |
| `S` | `\set_split_vfo` | |
| `I` | `\set_split_freq` | |
| `X` | `\set_split_mode` | |
| `G` | `\vfo_op` | solo `TUNE` (accordatore, se `Tune` nell'handbook) |

Getter: `f` `\get_freq`, `m` `\get_mode`, `t` `\get_ptt`, `v` `\get_vfo`, `s` `\get_split_vfo`,
`i` `\get_split_freq`, `x` `\get_split_mode`.
Utility: `\dump_state`, `\chk_vfo` (risponde `1`), `\get_powerstat` (`1`), `\set_powerstat` (`RPRT 0`),
`\get_rig_info`, `\get_lock_mode` (`0` + `RPRT 0`).
`targetable_vfo` annunciato solo se la radio indirizza davvero il VFO non attivo. Prefisso VFO opzionale
(`m VFOB` e `m`). Modo non mappabile → `None`; VFO inesistente → `RPRT -6`.
`get_level` (`l`): citato nel changelog come consumatore di `availableStateFields` ma **assente** dalla tabella
dei comandi supportati: da verificare. Livelli, `set_level`, `get_split`, RIT/XIT via Hamlib: non elencati.
Audio via Hamlib: non previsto (l'audio emulato esiste solo nel profilo TCI).

---

## 10. Cosa serve ad Aurora

| Esigenza | Chiamata CAT4OM nativa |
|---|---|
| Connessione | WS al Control Port → `hello` (`appName: "Aurora"`, `receiveSignal` se serve l'S-meter, `auth` se `authRequired`) → `welcome` |
| Scelta radio dall'interfaccia | elenco da `welcome.radios[]` (`radioId`, `radioName`, `model`, `connectionStatus`); più gruppi/server: Management `getServiceStatus` (`groups[].controlPort`) o beacon UDP |
| Diventare controllore | se `role == "slave"` → `getOwnership`; seguire `masterId` |
| Frequenza (lettura) | `vfos[activeVfo].frequency` da `stateUpdate` con `changes[radioId]` contenente `data` |
| Frequenza (scrittura) | `setFrequency` `{frequency, vfo}`; rispettare `commandVfoScopes` |
| Modo | `setMode` `{mode, vfo}` — per FT8/FT4 di norma `DATA-USB`/`USB` o `FT8`/`FT4` se in `supportedModes` |
| PTT | `setPtt {enabled}`; lettura `ptt`; gestire `TX_IN_PROGRESS` (mostrare `transmittingRadioId`) e `FAILSAFE_UNKEY`; `setPtt false` alla chiusura |
| Split | `setSplit {enabled, txVfo}` + `setFrequency` col `vfo` TX; lettura `split`/`txVfo` |
| VFO | `setVfo {vfo}` se `SetVfo` presente |
| Strumenti | `metering`: `signalDbm` → `sMeterMeter` → `sMeter`; `swr` → `swrMeter`; `powerWatts` → `powerMeter`; `alc`; S-meter live via `signalUpdate` |
| Audio RX di rete | `startAudioStream {direction:"rx", sampleRateHz:12000, channels:1}` (il decoder lavora a 12 kHz; usare i valori restituiti) → frame `C4OA` |
| Audio TX di rete | `getOwnership` → `startAudioStream {direction:"tx"}` **prima** del key → `setPtt true` → frame PCM16 ogni ~20 ms → `setPtt false`; gestire `audioStreamStopped` e riaprire a richiesta |
| Salute del collegamento | timeout 3 × `keepaliveIntervalMs`, `seq`, `instanceId` |
| Alternativa a costo zero | profilo Hamlib (porta 4532) col client rigctld già esistente: una radio, niente audio, niente multi-client |

### Punti poco chiari o mancanti nel manuale
- Nessuna azione per **rilasciare** l'ownership senza chiudere la connessione.
- Forma della `response` di successo di `setPtt`, `setMode`, `setSplit`, `setVfo`: mostrata solo per
  `setFrequency` (`result: null`).
- Elenco delle frequenze di campionamento audio accettate e latenza tipica: assenti.
- Nessuna azione per impostare potenza RF, larghezza filtro, attivare/disattivare l'accordatore.
- Intervalli di frequenza per banda della radio: assenti (solo `frequencyGroups`).
- Politica di backoff per la riconnessione: non specificata.
- TLS/`wss://`: non menzionato.
- `ratedPowerWatts` è citato ma non compare nella tabella dei campi `RadioState`.
- Gli esempi audio omettono `clientId` nelle richieste, la regola generale lo richiede su Control.
- `RADIO_NOT_CONNECTED` è usato da `powerOff` ma non è in tabella errori; `NOT_CONNECTED` sì.
- Hamlib `get_level`: citato nel changelog ma non nella tabella dei comandi supportati.
- La tabella dell'header binario nel PDF è impaginata male; l'ordine degli offset sopra è ricostruito dagli
  esempi di codice C#/Python/JS del manuale (coerenti tra loro).
- Le sezioni 3.2 e 3.3 non esistono nel documento (numerazione salta da 3.1 a 3.4).

---

## 11. Come lo usa Aurora (codice)

- Collegamento, scoperta del gruppo, comandi: `core/crates/rig-control/src/cat4om.rs`.
- «Cerca» della pagina Radio (`POST /api/v1/cat/cat4om/search`, `cat4om::search`): `getServiceStatus` sulla gestione (senza avviare
  gruppi) e, per ogni gruppo in funzione, `hello` con `noRegister: true` sul Control (osservatore: non conta come client, non prende
  l'ownership) per leggere `welcome.radios[]` (`connectionStatus`, `bridgeControlledBy`, `audioCaptureAvailable`,
  `audioTransmitAvailable`) e `features` (`audioStreaming`). Se il `welcome` dell'osservatore non avesse le radio restano quelle della
  gestione. Attese di 3 s.
- Audio in rete (`core/crates/rig-control/src/cat4om_audio.rs` per la parte pura, `cat4om.rs` per la rete): dispositivo «Cat4OM (audio in
  rete)» nella scheda Audio, presente con il CAT su Cat4OM (`aurora/src/audio_control.rs`).
  - RX: aperto dal thread del collegamento quando l'ingresso e' scelto, `{direction:"rx", sampleRateHz:12000, channels:1}`; con
    `AUDIO_RATE_UNAVAILABLE` si riprova a 48000; la frequenza concessa si ricorda per le riaperture. I blocchi vanno alla stessa strada
    dell'audio TCI (cascata, livello, decodificatore che porta a 12 kHz). Blocchi illeggibili, d'altri stream, vecchi o ripetuti si
    scartano; fino a 50 blocchi persi diventano silenzio. `audioStreamStopped` → nuova apertura dopo 2 s; un rifiuto → nuovo tentativo
    dopo 10 s.
  - TX: `getOwnership` se serve, poi `{direction:"tx", sampleRateHz:48000, channels:1}` **prima** di `setPtt true`; se non si apre il PTT
    non parte e l'errore lo dice (`NOT_MASTER`, `RADIO_CLAIMED`, `AUDIO_TRANSMIT_IN_USE`, `AUDIO_RATE_UNAVAILABLE`, `AUDIO_NOT_AVAILABLE`,
    `AUDIO_DEVICE_*`). Blocchi da 20 ms (`rate/50` campioni) con `sequence` da 1 per stream, `timestampUs` = µs dall'apertura, mandati
    a ritmo con 40 ms di anticipo. Lo stream resta aperto tra una trasmissione e l'altra; dopo 50 s senza audio (o dopo
    `audioStreamStopped`) si riapre al PTT successivo.
