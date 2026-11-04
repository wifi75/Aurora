-- Indice giornaliero dei rapporti degli errori (nessun IP, nessun dato personale).
CREATE TABLE IF NOT EXISTS errori (
  giorno TEXT NOT NULL,
  versione TEXT NOT NULL,
  area TEXT NOT NULL,
  messaggio TEXT NOT NULL,
  conteggio INTEGER NOT NULL DEFAULT 0,
  rapporti INTEGER NOT NULL DEFAULT 0,
  installazioni TEXT NOT NULL DEFAULT '',
  PRIMARY KEY (giorno, versione, area, messaggio)
);
CREATE INDEX IF NOT EXISTS errori_versione ON errori (versione, giorno);

-- I pacchetti zip (dopo un crash o dalla scheda Assistenza), ritrovabili con il codice breve.
CREATE TABLE IF NOT EXISTS pacchetti (
  codice TEXT PRIMARY KEY,
  chiave TEXT NOT NULL,
  giorno TEXT NOT NULL,
  versione TEXT NOT NULL,
  motivo TEXT NOT NULL,
  byte INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS pacchetti_giorno ON pacchetti (giorno);

-- Installazioni attive: una riga per installazione e giorno (codice casuale, versione, sistema). Niente IP, niente dati personali.
CREATE TABLE IF NOT EXISTS attivi (
  giorno TEXT NOT NULL,
  installazione TEXT NOT NULL,
  versione TEXT NOT NULL,
  sistema TEXT NOT NULL,
  PRIMARY KEY (giorno, installazione)
);
CREATE INDEX IF NOT EXISTS attivi_versione ON attivi (versione, giorno);
