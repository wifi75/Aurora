-- L'antivirus dei PC (0.11.276): nome dei prodotti registrati in Windows, per le installazioni, e gli errori dell'area `antivirus`
-- (file di Aurora tolti dall'antivirus) contati per antivirus. Va eseguita PRIMA del deploy del Worker che li scrive.
ALTER TABLE installazioni ADD COLUMN antivirus TEXT;
CREATE TABLE IF NOT EXISTS antivirus_errori (
  giorno TEXT NOT NULL,
  antivirus TEXT NOT NULL,
  installazione TEXT NOT NULL,
  versione TEXT NOT NULL DEFAULT '',
  rapporti INTEGER NOT NULL DEFAULT 0,
  esempio TEXT NOT NULL DEFAULT '',
  PRIMARY KEY (giorno, antivirus, installazione)
);
CREATE INDEX IF NOT EXISTS antivirus_errori_giorno ON antivirus_errori (giorno);
