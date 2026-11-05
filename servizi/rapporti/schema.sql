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

-- Totale di tutte le installazioni che hanno usato Aurora: una riga per identificativo anonimo, con il primo e l'ultimo giorno.
-- Non si cancella mai: e' lo storico di quante installazioni hanno usato Aurora dall'inizio.
CREATE TABLE IF NOT EXISTS installazioni (
  installazione TEXT PRIMARY KEY,
  prima TEXT NOT NULL,
  ultima TEXT NOT NULL,
  -- la nazione (sigla di due lettere data da Cloudflare, `XX` se non si sa): niente IP, niente citta'
  paese TEXT
);
-- Conteggi di ogni giorno (per versione e sistema, senza identificativi), salvati prima che la pulizia tolga le righe di `attivi`.
CREATE TABLE IF NOT EXISTS storico (
  giorno TEXT NOT NULL,
  versione TEXT NOT NULL,
  sistema TEXT NOT NULL,
  attivi INTEGER NOT NULL,
  PRIMARY KEY (giorno, versione, sistema)
);
