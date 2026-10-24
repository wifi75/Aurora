# Aurora Piano 5 - Automazioni: piano di lavoro

Specifica: `2026-10-24-aurora-design.md` §6. Voci 6 e 7 di `mancante.md`.

**Scopo:** rispondere da soli ai CQ che interessano, con tutte le salvaguardie della specifica, senza poter trasmettere per errore.

## Decisioni (ruling)
- **Parte spento e in prova a vuoto.** Passare a "reale" richiede: richiesta dal PC (loopback, senza proxy), trasmissione abilitata (`AURORA_ALLOW_TX=1`) e conferma esplicita dell'avviso sulla licenza. Dal telefono si puo' solo accendere in prova a vuoto o spegnere.
- **Doppio cancello invariato:** l'automazione esegue un `Command::Reply`, che il motore JTDX rifiuta comunque senza `allow_tx`.
- **Limiti impostati sul PC** (variabili d'ambiente, non modificabili da API): `AURORA_AUTO_MAX_CYCLES` (6 risposte di fila senza QSO), `AURORA_AUTO_MAX_PER_DAY` (40), `AURORA_AUTO_DEAD_MAN_SECS` (120), `AURORA_AUTO_UNATTENDED` (0).
- **Dead-man:** senza nessun client collegato per N secondi l'automazione si spegne da sola, salvo `AURORA_AUTO_UNATTENDED=1`. Una risposta manuale singola dal telefono non e' continua (la trasmissione la chiude JTDX a fine ciclo): il dead-man vale per l'automazione.
- **Tetto di cicli raggiunto = spegnimento**, da riaccendere a mano. Tetto giornaliero = blocco fino al giorno UTC dopo.
- **STOP TX** (`TxHalted`) spegne sempre l'automazione.
- **Un solo QSO attivo**: dopo una risposta, nessun'altra per 60 s o fino al QSO registrato.
- In prova a vuoto i contatori giornalieri/di fila non si consumano; "un solo QSO attivo" e il limite di tentativi per stazione si simulano comunque.
- Regole in memoria (si perdono al riavvio): rinviato.

## Compiti
1. `qso-engine::Brain` (logica pura, tempo passato dal chiamante): regole, limiti, decisioni con motivo. Test per ogni salvaguardia.
2. `qso-engine::Automation` (task Tokio): ascolta il bus, esegue i `Reply`, tick di 1 s, contatore client (guardia RAII), istantanea per l'API.
3. API `GET/PUT /api/v1/automation`, WebSocket che conta i client, collegamento in `aurora`, variabili d'ambiente.
4. Pagina Automazioni (interruttore, modo, regole, registro delle decisioni).
5. Documenti, verifica completa, commit.

## Review Focus
- Un `Reply` mai eseguito senza `allow_tx`, in prova a vuoto o dopo STOP.
- Cambio di giorno UTC, client che cade e torna, messaggi senza nominativo leggibile, nominativi in minuscolo nell'esclusione.
