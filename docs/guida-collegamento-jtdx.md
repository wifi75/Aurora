# Collegare Aurora a JTDX (guida per Tiziano)

Obiettivo: Aurora riceve stato e decode da JTDX **senza togliere nulla a GridTracker2 e a Log4OM2**. Si fa in 3 passi e si può tornare indietro in 30 secondi.

> **Prima regola:** con `AURORA_ALLOW_TX=0` (predefinito) Aurora **non invia mai Reply**. Un Reply fa trasmettere JTDX davvero, anche con Enable Tx spento (provato il 2026-10-24). Lo si abilita solo con antenna scollegata o carico fittizio.

## Come funziona il giro

```
JTDX --(server UDP primario, porta 2334)--> Aurora (hub) --inoltra identico--> GridTracker2 (porta 2333)
JTDX --(server UDP secondario, porta 2237)--> Log4OM2          <- NON cambia
```

Aurora fa da "ripetitore": quello che riceve da JTDX lo gira uguale a GridTracker2, e i comandi che GridTracker2 manda indietro li gira a JTDX. Il collegamento diretto JTDX → Log4OM2 sulla 2237 non passa da Aurora.

## Passo 1 — Avvia Aurora (JTDX ancora com'è)

In un terminale PowerShell:

```powershell
cd D:\progetti\Aurora\core
$env:AURORA_ENGINE = 'jtdx'
$env:AURORA_UDP_LISTEN = '127.0.0.1:2334'
$env:AURORA_UDP_FORWARD = '127.0.0.1:2333'
$env:AURORA_DATA_DIR = '..\data'
cargo run -p aurora
```

Deve comparire "in ascolto su http://127.0.0.1:8787". Lascia GridTracker2 e Log4OM2 accesi come sempre. Per avere anche la cascata dall'audio aggiungi, prima di `cargo run`, `$env:AURORA_AUDIO_DEVICE = 'USB Audio'`.

**Modo più semplice (consigliato):** un solo comando avvia Aurora con JTDX, l'inoltro a GridTracker2 e l'interfaccia, in una sola tab di Windows Terminal:

```powershell
.\scripts\avvia-sviluppo.ps1 -Jtdx
```

Con la cascata dall'audio: `.\scripts\avvia-sviluppo.ps1 -Jtdx -Audio "USB Audio"`.

L'interfaccia è una **pagina web**: si apre nel browser su `http://localhost:5173` (Aurora non è ancora un programma Windows a sé; quello arriverà con il guscio desktop). Associa il browser con il codice che dà `.\scripts\associa-telefono.ps1`.

**Importante:** dopo il Passo 2 JTDX manda i dati ad Aurora, quindi **Aurora deve restare acceso** finché vuoi che GridTracker2 riceva i decode. Se Aurora è spento, GridTracker2 non riceve nulla (Log4OM2 sì, perché usa il collegamento diretto). Per tornare com'era basta rimettere la porta di JTDX a 2333.

## Passo 2 — Punta JTDX ad Aurora (una sola modifica)

In JTDX: **Impostazioni → Reporting → Primary UDP Server**
- indirizzo: `127.0.0.1` (com'è)
- porta: da **2333** a **2334**

**Non toccare il secondo server UDP (2237):** è quello che porta i QSO a Log4OM2.

## Passo 3 — Controlla che tutto arrivi

- In Aurora (interfaccia) compaiono banda, frequenza e i decode che scorrono.
- **GridTracker2 continua a mostrare i decode** (gli arrivano da Aurora, identici).
- Chiudi un QSO come fai di solito: deve arrivare in Log4OM2 (sempre dal collegamento diretto).
- Provando a toccare un decode in Aurora deve comparire «La trasmissione è disattivata in Aurora»: è il comportamento voluto. Il pulsante **STOP TX** invece funziona sempre.

Se qualcosa non va: rimetti la porta di JTDX a **2333** e tutto torna com'era. Aurora si può chiudere con Ctrl+C.

## Abilitare le risposte da Aurora (solo con carico fittizio)

1. Scollega l'antenna (o usa un carico fittizio / potenza azzerata).
2. Chiudi Aurora e riavviala con `$env:AURORA_ALLOW_TX = '1'`.
3. Tocca un CQ: JTDX **accende Enable Tx e trasmette** la risposta (nella tua configurazione `Tx2QSO=true`).
4. **STOP TX** ferma subito la trasmissione.

Il decode su cui rispondere deve essere ancora nella finestra di JTDX (al massimo 45 secondi) e JTDX deve avere "Accept UDP Reply messages" su una modalità che lo accetta (oggi: solo CQ).

## Opzioni utili

| Variabile | Valore | A cosa serve |
|---|---|---|
| `AURORA_UDP_FORWARD` | `127.0.0.1:2333,127.0.0.1:2335` | più destinazioni, separate da virgole |
| `AURORA_REPLAY_ON_CONNECT` | `0` | `1` chiede a JTDX di rimandare i decode correnti all'avvio (si rivedono le stazioni di prima e GridTracker2 le riceve di nuovo, marcate "non nuove"); spento per impostazione predefinita |
| `AURORA_AUDIO_DEVICE` | `USB Audio` | parte del nome dell'ingresso audio per la cascata |
| `AURORA_BIND` | `0.0.0.0:8787` | rende Aurora raggiungibile dal telefono sulla rete locale |

## Cosa Aurora NON fa (ancora)

- Non cambia le impostazioni di JTDX: JTDX 159 non le accetta via UDP.
- Non scrive il log: i QSO continuano ad andare a Log4OM2 direttamente da JTDX.
- Non comanda la radio (frequenza e PTT) né genera CQ da solo: sono le prossime fasi.

## Sicurezza: cosa il cancello di Aurora NON copre

- **I comandi di GridTracker2 passano dall'hub come passavano prima.** Se fai doppio clic su una stazione in GridTracker2, il Reply arriva a JTDX e JTDX trasmette, anche con `AURORA_ALLOW_TX=0`. Il cancello di Aurora governa solo i comandi che partono da Aurora (interfaccia e telefono). L'hub non cambia il comportamento che hai oggi.
- **L'hub si fida di chiunque scriva sulla sua porta.** Un datagramma valido da un altro programma diventerebbe "JTDX" e riceverebbe i comandi. Per questo l'ascolto resta su `127.0.0.1`: non metterlo su `0.0.0.0` e non aprire la porta 2334 nel firewall.
- **STOP TX non dipende da nessuna impostazione**: arriva sempre a JTDX, anche con la trasmissione chiusa.
