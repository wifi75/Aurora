-- I clic sul pulsante di un messaggio dell'autore (0.11.320): una riga per messaggio e giorno, solo il conto. Nessun IP, nessun codice.
CREATE TABLE IF NOT EXISTS messaggi_clic (
  messaggio TEXT NOT NULL,
  giorno TEXT NOT NULL,
  n INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (messaggio, giorno)
);
