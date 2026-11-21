-- Il battito delle installazioni aperte (ogni circa 40 minuti, solo con il conteggio delle installazioni acceso): una riga per giorno e installazione.
-- Solo il codice anonimo, la versione, la nazione (da Cloudflare), quanti battiti, il primo e l'ultimo (ms UTC) e il livello d'uso
-- (0 aperto e fermo, 1 riceve, 2 trasmette). Nessun IP, nessun nominativo. La pulizia notturna toglie i giorni oltre i 90.
-- Va eseguita PRIMA del deploy del Worker che la usa.
CREATE TABLE IF NOT EXISTS battiti (
  giorno TEXT NOT NULL,
  installazione TEXT NOT NULL,
  versione TEXT NOT NULL DEFAULT '',
  paese TEXT NOT NULL DEFAULT 'XX',
  battiti INTEGER NOT NULL DEFAULT 0,
  primo_ms INTEGER NOT NULL,
  ultimo_ms INTEGER NOT NULL,
  uso INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (giorno, installazione)
);
CREATE INDEX IF NOT EXISTS battiti_ultimo ON battiti (ultimo_ms);
CREATE INDEX IF NOT EXISTS battiti_installazione ON battiti (installazione, giorno);
