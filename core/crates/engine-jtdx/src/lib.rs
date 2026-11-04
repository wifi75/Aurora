//! I dati della stazione condivisi dal motore di Aurora: paesi e novita' dei nominativi (`enrich`), QSO gia' fatti (`worked`),
//! scheda DXCC (`matrix`), lettura dei messaggi FT8 (`text`), bande (`map::band_of`) e lo schema delle impostazioni (`ini`, `ini_edit`).
//! Nessun legame con altri programmi: qui non si legge ne' si comanda JTDX.

pub mod enrich;
pub mod ini;
pub mod ini_edit;
pub mod map;
pub mod matrix;
pub mod text;
pub mod worked;
