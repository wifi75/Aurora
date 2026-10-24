# Aurora - istruzioni per chi sviluppa

Stazione digitale FT8/FT4 con automazione dei QSO e controllo remoto; JTDX e' il motore.

## Comandi
- Test: `cd core && cargo test --workspace`
- Build: `cd core && cargo build --workspace`

## Regole
- Rust edition 2024. Niente `unwrap()`/`expect()` fuori dai test; errori come `Result`.
- Nessun segreto nel repository; usare `.env.example`.
- Nessun riferimento a strumenti di assistenza nel repository, ne' nei messaggi di commit.
- Le prove manuali con radio e telefono le fa il proprietario; qui si eseguono i test automatici.
- Specifica e piani in `docs/`.
