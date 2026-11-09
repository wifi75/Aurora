-- Conteggio delle persone (0.11.214): codice della persona, HMAC del codice ricavato dal nominativo sul PC.
ALTER TABLE attivi ADD COLUMN persona TEXT;
ALTER TABLE installazioni ADD COLUMN persona TEXT;
