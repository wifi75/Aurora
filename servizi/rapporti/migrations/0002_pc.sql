-- Il computer (0.11.220): processore, processori logici e memoria delle installazioni; il computer dell'ultimo esempio di ogni errore.
-- Va eseguita PRIMA del deploy del Worker che li scrive.
ALTER TABLE installazioni ADD COLUMN cpu TEXT;
ALTER TABLE installazioni ADD COLUMN core INTEGER;
ALTER TABLE installazioni ADD COLUMN ram_gb INTEGER;
ALTER TABLE errori ADD COLUMN pc TEXT NOT NULL DEFAULT '';
