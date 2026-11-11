use dxcc::{DxccDb, DxccError};

/// Frammento di fantasia nel formato di cty.dat (paesi e prefissi inventati).
const CTY: &str = "\
Atlantis:                 14:  27:  EU:   43.00:    -7.00:    -1.0:  AT:
    AT,AU,AT5(15)[30];
Mu Island:                26:  50:  AS:    9.88:  -114.23:    -8.0:  MU:
    MU0,MV,MW1,
    =MU9/AA1AA,=ZZ9Q(20);
Lemuria:                  05:  08:  NA:   10.00:    60.00:     5.0:  LM:
    LM,LMA,LMB;
Narnia:                   33:  37:  AF:   -5.00:    20.00:    -2.0:  NN:
    NN,AT7,=AT1X;
";

fn db() -> DxccDb {
    DxccDb::parse(CTY).unwrap()
}

fn name_of(db: &DxccDb, call: &str) -> Option<String> {
    db.lookup(call).map(|l| db.entity(l.entity).name.clone())
}

#[test]
fn parses_entities_and_aliases() {
    let db = db();
    assert_eq!(db.len(), 4);
    let l = db.lookup("AT3ABC").unwrap();
    let e = db.entity(l.entity);
    assert_eq!((e.name.as_str(), e.prefix.as_str(), e.continent.as_str()), ("Atlantis", "AT", "EU"));
    assert_eq!((l.cq_zone, l.itu_zone), (14, 27));
}

#[test]
fn longest_prefix_wins() {
    let db = db();
    assert_eq!(name_of(&db, "AT3XYZ").as_deref(), Some("Atlantis"));
    assert_eq!(name_of(&db, "AT7XYZ").as_deref(), Some("Narnia"), "AT7 e' piu' lungo di AT");
    assert_eq!(name_of(&db, "LMA1X").as_deref(), Some("Lemuria"));
}

#[test]
fn exact_call_beats_prefix() {
    let db = db();
    assert_eq!(name_of(&db, "AT1X").as_deref(), Some("Narnia"));
    assert_eq!(name_of(&db, "AT1Y").as_deref(), Some("Atlantis"));
}

#[test]
fn zone_overrides_cq_and_itu() {
    let db = db();
    let l = db.lookup("AT5ABC").unwrap();
    assert_eq!((l.cq_zone, l.itu_zone), (15, 30));
    let q = db.lookup("ZZ9Q").unwrap();
    assert_eq!((q.cq_zone, q.itu_zone), (20, 50), "solo la zona CQ e' sovrascritta");
}

#[test]
fn portable_and_prefixed_calls() {
    let db = db();
    assert_eq!(name_of(&db, "LM/AA1AA").as_deref(), Some("Lemuria"));
    assert_eq!(name_of(&db, "AA1AA/LM").as_deref(), Some("Lemuria"));
    assert_eq!(name_of(&db, "MU9/AA1AA").as_deref(), Some("Mu Island"), "chiamata esatta con barra");
}

#[test]
fn suffixes_are_ignored_but_maritime_mobile_has_no_country() {
    let db = db();
    for suffix in ["P", "M", "QRP", "A"] {
        assert_eq!(name_of(&db, &format!("AT3ABC/{suffix}")).as_deref(), Some("Atlantis"), "/{suffix}");
    }
    assert_eq!(name_of(&db, "AT3ABC/MM"), None);
    assert_eq!(name_of(&db, "AT3ABC/AM"), None);
}

#[test]
fn case_insensitive_and_trims_spaces() {
    let db = db();
    assert_eq!(name_of(&db, "at3abc").as_deref(), Some("Atlantis"));
    assert_eq!(name_of(&db, "  AT3ABC ").as_deref(), Some("Atlantis"));
}

#[test]
fn unknown_and_hashed_calls_are_none() {
    let db = db();
    assert_eq!(name_of(&db, ""), None);
    assert_eq!(name_of(&db, "<...>"), None);
    assert_eq!(name_of(&db, "ABCDEF"), None, "senza cifre non e' un nominativo");
    assert_eq!(name_of(&db, "123"), None);
    assert_eq!(name_of(&db, "QQ9QQ"), None);
}

#[test]
fn multiline_alias_lists() {
    let db = db();
    assert_eq!(name_of(&db, "MW1AAA").as_deref(), Some("Mu Island"));
    assert_eq!(name_of(&db, "MV5X").as_deref(), Some("Mu Island"));
}

#[test]
fn rejects_a_file_without_entities() {
    assert_eq!(DxccDb::parse("").err(), Some(DxccError::NoEntities));
    assert_eq!(DxccDb::parse("questo non e' un cty.dat\n").err(), Some(DxccError::NoEntities));
}

#[test]
fn reading_a_missing_file_is_an_error_not_a_panic() {
    assert!(matches!(DxccDb::from_file(std::path::Path::new("non/esiste/cty.dat")), Err(DxccError::Io(_))));
}

/// Le entita' solo WAE (prefisso con `*`) non sono paesi DXCC: i loro nominativi vanno al paese genitore, come la vista DXCC di WSJT-X.
#[test]
fn wae_only_entities_resolve_to_their_dxcc_parent() {
    let cty = "\
Italy:                    15:  28:  EU:   42.82:   -12.58:    -1.0:  I:
    I,IK,IU,IW,IZ,=IU0ABC;
Sicily:                   15:  28:  EU:   37.50:   -14.00:    -1.0:  *IT9:
    IT9,IU9,IW9,=IW0HBY/9;
";
    let db = DxccDb::parse(cty).unwrap();
    assert_eq!(name_of(&db, "IU9ABC").as_deref(), Some("Italy"));
    assert_eq!(name_of(&db, "IT9XYZ").as_deref(), Some("Italy"));
    assert_eq!(name_of(&db, "IW0HBY/9").as_deref(), Some("Italy"));
    assert_eq!(name_of(&db, "IU3ABC").as_deref(), Some("Italy"));
    assert_eq!((db.len(), db.dxcc_count()), (2, 1));
}

/// I suffissi operativi di WSJT-X non diventano prefissi di paese.
#[test]
fn operating_suffixes_do_not_change_the_country() {
    let cty = "\
Italy:                    15:  28:  EU:   42.82:   -12.58:    -1.0:  I:
    I,IU;
European Russia:          16:  29:  EU:   53.65:   -41.37:    -4.0:  R:
    R,UA;
France:                   14:  27:  EU:   46.00:    -2.00:    -1.0:  F:
    F;
Norway:                   14:  18:  EU:   61.00:    -9.00:    -1.0:  LA:
    LA,LG;
";
    let db = DxccDb::parse(cty).unwrap();
    for call in ["IU3ABC/R", "IU3ABC/FD", "IU3ABC/FF", "IU3ABC/LGT", "IU3ABC/LT", "IU3ABC/Q", "IU3ABC/QRP"] {
        assert_eq!(name_of(&db, call).as_deref(), Some("Italy"), "{call}");
    }
}

#[test]
fn a_new_suffix_in_front_is_still_a_country_prefix() {
    let cty = "\
Italy:                    15:  28:  EU:   42.82:   -12.58:    -1.0:  I:
    I,IU;
Argentina:                13:  14:  SA:  -34.80:    65.92:     3.0:  LU:
    LU,LT;
";
    let db = DxccDb::parse(cty).unwrap();
    assert_eq!(name_of(&db, "LT/IU3ABC").as_deref(), Some("Argentina"));
    assert_eq!(name_of(&db, "IU3ABC/LT").as_deref(), Some("Italy"));
}

/// Con il `cty.dat` vero (quello di riserva dell'installatore): le entita' WAE italiane, turche e scozzesi vanno al paese DXCC.
#[test]
fn the_real_cty_dat_resolves_wae_entities_to_dxcc_countries() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../desktop/cty-riserva/cty.dat");
    let Ok(db) = DxccDb::from_file(&path) else { return };
    assert_eq!(name_of(&db, "IU9ABC").as_deref(), Some("Italy"));
    assert_eq!(name_of(&db, "IT9XYZ").as_deref(), Some("Italy"));
    assert_eq!(name_of(&db, "TA1ABC").as_deref(), Some("Asiatic Turkey"));
    assert!(db.dxcc_count() < db.len(), "le entita' WAE non contano tra i paesi DXCC");
}
