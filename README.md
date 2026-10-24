# Aurora

Stazione digitale FT8/FT4 di nuova concezione: interfaccia grafica d'impatto, automazione dei QSO e controllo remoto da telefono. Usa JTDX come motore di decodifica e trasmissione.

Progettazione e sviluppo: Tiziano Cassone

## Stato

Versione 0.4.0: core e interfaccia funzionanti su una **stazione simulata** oppure su **JTDX reale** (decode, stato, STOP TX, associazione del telefono, cascata dall'audio, paese e novita' dei decode, avvisi sonori).
Esiti delle prove con JTDX 159 in `docs/fase0-rapporto.md`.

## Avvio rapido (Windows)

```
.\scripts\avvia-sviluppo.ps1
```

Avvia il core (porta 8787) e l'interfaccia (porta 5173) in una sola tab di Windows Terminal. Poi apri `http://127.0.0.1:5173`.

### Associare un telefono

Con Aurora acceso sul PC:

```
.\scripts\associa-telefono.ps1
```

Mostra un codice di 8 cifre e un QR (valgono 2 minuti). Inquadra il QR con il telefono (stessa rete Wi-Fi), dai un nome al dispositivo e conferma.
Nessuna password: l'accesso è per dispositivo associato; si revoca dall'elenco dei dispositivi.

### Con JTDX reale

Vedi `docs/guida-collegamento-jtdx.md`: Aurora si mette tra JTDX e GridTracker2 (hub UDP) senza togliere nulla a nessuno dei due,
e la trasmissione da Aurora e' chiusa finche' non la abiliti (`AURORA_ALLOW_TX=1`, solo con carico fittizio).

## Documentazione

- Specifica: `docs/2026-10-24-aurora-design.md`
- API: `docs/api.md`
- Piano 1 (Fase 0): `docs/2026-10-24-aurora-fase0-spike-plan.md` · Piano 2 (core e interfaccia): `docs/2026-10-24-aurora-piano2-core-ui.md`
- Rapporti: `docs/fase0-rapporto.md`, `docs/piano2-rapporto.md`

## Sviluppo

```
cd core && cargo test --workspace      # test del core
cd app && npm install && npm test      # test dell'interfaccia
cd app && npm run build                # build di produzione (tsc + vite)
```
