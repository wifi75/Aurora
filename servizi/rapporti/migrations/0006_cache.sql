-- Cache delle risposte pesanti dell'amministratore (/v1/utilizzo e /v1/attivi): una riga per risposta, con la scadenza.
-- Cosi' le richieste ripetute del portale leggono UNA riga invece di riscansionare battiti, attivi e installazioni
-- (D1 gratuito: tetto di 5.000.000 di righe LETTE al giorno). Va eseguita PRIMA del deploy del Worker che la usa
-- (senza la tabella il Worker funziona lo stesso, ma senza cache).
CREATE TABLE IF NOT EXISTS cache_risposte (
  chiave TEXT PRIMARY KEY,
  scade_ms INTEGER NOT NULL,
  corpo TEXT NOT NULL
);
