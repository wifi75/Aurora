//! Le liste ufficiali per controllare lo scambio ricevuto: sezioni ARRL/RAC, stati USA, province e territori canadesi, province olandesi.
//! Uno scambio fuori lista resta nel log ma e' segnalato e non apre moltiplicatori.
//!
//! Le liste cambiano di rado (l'ultima volta l'Ontario diviso in GH/ONE/ONN/ONS e NT diventato TER): ricontrollarle sulle fonti prima
//! della gara.

/// Sezioni ARRL e RAC (85: 71 negli Stati Uniti, 14 in Canada), raggruppate per distretto come nella lista ufficiale.
/// Fonte: ARRL, «ARRL/RAC Section Abbreviations», <https://www.arrl.org/section-abbreviations> (le stesse del Field Day,
/// <https://www.arrl.org/field-day-rules>).
pub const SEZIONI_ARRL_RAC: &[&str] = &[
    // 1
    "CT", "EMA", "ME", "NH", "RI", "VT", "WMA",
    // 2
    "ENY", "NLI", "NNJ", "NNY", "SNJ", "WNY",
    // 3
    "DE", "EPA", "MDC", "WPA",
    // 4
    "AL", "GA", "KY", "NC", "NFL", "PR", "SC", "SFL", "TN", "VA", "VI", "WCF",
    // 5
    "AR", "LA", "MS", "NM", "NTX", "OK", "STX", "WTX",
    // 6
    "EB", "LAX", "ORG", "PAC", "SB", "SCV", "SDG", "SF", "SJV", "SV",
    // 7
    "AK", "AZ", "EWA", "ID", "MT", "NV", "OR", "UT", "WWA", "WY",
    // 8
    "MI", "OH", "WV",
    // 9
    "IL", "IN", "WI",
    // 0
    "CO", "IA", "KS", "MN", "MO", "ND", "NE", "SD",
    // Canada (RAC)
    "AB", "BC", "GH", "MB", "NB", "NL", "NS", "ONE", "ONN", "ONS", "PE", "QC", "SK", "TER",
];

/// I 50 stati USA piu' il District of Columbia (sigle postali USPS).
/// Fonte: ARRL RTTY Roundup, moltiplicatori «US states (plus DC)», <https://www.arrl.org/rtty-roundup>; sigle da USPS Publication 28,
/// appendice B, <https://pe.usps.com/text/pub28/28apb.htm>.
pub const STATI_USA: &[&str] = &[
    "AL", "AK", "AZ", "AR", "CA", "CO", "CT", "DE", "DC", "FL", "GA", "HI", "ID", "IL", "IN", "IA", "KS", "KY", "LA", "ME", "MD", "MA",
    "MI", "MN", "MS", "MO", "MT", "NE", "NV", "NH", "NJ", "NM", "NY", "NC", "ND", "OH", "OK", "OR", "PA", "RI", "SC", "SD", "TN", "TX",
    "UT", "VT", "VA", "WA", "WV", "WI", "WY",
];

/// Le 10 province e i 3 territori canadesi (sigle di Canada Post).
/// Fonte: ARRL RTTY Roundup, moltiplicatori «Canadian provinces/territories», <https://www.arrl.org/rtty-roundup>; sigle da Canada Post,
/// <https://www.canadapost-postescanada.ca/cpc/en/support/kb/sending/general-information/province-and-territory-symbols-and-abbreviations>.
pub const PROVINCE_CANADA: &[&str] = &["AB", "BC", "MB", "NB", "NL", "NS", "NT", "NU", "ON", "PE", "QC", "SK", "YT"];

/// Le 12 province olandesi con le sigle del PACC.
/// Fonte: VERON, regolamento PACCdigi, <https://veron.nl/vereniging/commissies-en-werkgroepen/traffic-bureau/hf-contesten/paccdigi-rules/>.
pub const PROVINCE_OLANDA: &[&str] = &["DR", "FL", "FR", "GD", "GR", "LB", "NB", "NH", "OV", "UT", "ZH", "ZL"];

fn in_lista(lista: &[&str], s: &str) -> bool {
    lista.iter().any(|v| v.eq_ignore_ascii_case(s.trim()))
}

/// Una sezione ARRL/RAC valida (senza `DX`).
pub fn sezione_valida(s: &str) -> bool {
    in_lista(SEZIONI_ARRL_RAC, s)
}

/// Uno stato USA o una provincia/territorio canadese (Roundup).
pub fn stato_valido(s: &str) -> bool {
    in_lista(STATI_USA, s) || in_lista(PROVINCE_CANADA, s)
}

/// Una provincia olandese (PACC).
pub fn provincia_olanda_valida(s: &str) -> bool {
    in_lista(PROVINCE_OLANDA, s)
}

/// La classe del Field Day: numero di trasmettitori (1-99) e lettera A-F (`1D`, `12A`).
pub fn classe_fd_valida(s: &str) -> bool {
    let s = s.trim().to_ascii_uppercase();
    let n: String = s.chars().take_while(char::is_ascii_digit).collect();
    let resto = &s[n.len()..];
    !n.is_empty() && n.len() <= 2 && n.parse::<u32>().is_ok_and(|v| v > 0) && resto.len() == 1 && resto.chars().all(|c| ('A'..='F').contains(&c))
}

/// Un numero progressivo (lo scambio delle stazioni DX nel Roundup e nel PACC).
pub fn seriale_valido(s: &str) -> bool {
    let s = s.trim();
    !s.is_empty() && s.len() <= 5 && s.chars().all(|c| c.is_ascii_digit())
}
