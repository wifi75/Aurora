//! Regione IARU e tabella iniziale (JTDX 2.2.159, letta con REA: `docs/analisi-jtdx-reverse-rea.md`, sezione «Regione IARU»).
use rig_control::table::{Entry, FrequencyTable, Region};

fn rows(t: &FrequencyTable, region: Region, band: &str, mode: &str) -> Vec<u64> {
    t.visible(region).filter(|x| x.band == band && x.mode == mode).map(|x| x.hz).collect()
}

#[test]
fn the_default_table_is_the_jtdx_one_for_ft8_and_ft4() {
    let t = FrequencyTable::default();
    assert!(t.is_valid());
    // 160 m: 1,810 e 1,908 solo regione 1, 1,840 per tutti
    assert_eq!(rows(&t, Region::All, "160m", "FT8"), vec![1_810_000, 1_840_000, 1_908_000]);
    // 80 m FT4: 3,568 solo regione 3, 3,575 per tutti
    assert_eq!(rows(&t, Region::All, "80m", "FT4"), vec![3_568_000, 3_575_000]);
    // 20 m FT8: 14,071 e 14,090 non sono predefinite
    let alt: Vec<bool> = t.visible(Region::All).filter(|x| x.band == "20m" && x.mode == "FT8").map(|x| x.preferred).collect();
    assert_eq!(alt, vec![false, true, false]);
    // JTDX non ha FT4 su 160 m e 60 m
    assert_eq!(t.frequency("160m", "FT4"), None);
    assert_eq!(t.frequency("60m", "FT4"), None);
}

#[test]
fn a_region_hides_the_rows_of_the_other_regions_and_all_shows_everything() {
    let t = FrequencyTable::default();
    assert_eq!(rows(&t, Region::R1, "160m", "FT8"), vec![1_810_000, 1_840_000, 1_908_000]);
    assert_eq!(rows(&t, Region::R2, "160m", "FT8"), vec![1_840_000], "la regione 2 non vede le righe della regione 1");
    assert_eq!(rows(&t, Region::R3, "160m", "FT8"), vec![1_840_000]);
    assert_eq!(rows(&t, Region::R3, "80m", "FT4"), vec![3_568_000, 3_575_000]);
    assert_eq!(rows(&t, Region::R1, "80m", "FT4"), vec![3_575_000]);
    assert!(Region::All.sees(Region::R3) && Region::R2.sees(Region::All) && Region::R2.sees(Region::R2) && !Region::R2.sees(Region::R1));
}

#[test]
fn the_band_picks_the_preferred_row_and_the_chosen_region_wins_over_the_common_one() {
    let t = FrequencyTable::default();
    // tutte le regioni: la riga valida ovunque, come prima delle regioni
    assert_eq!(t.frequency("160m", "FT8"), Some(1_840_000));
    assert_eq!(t.frequency("80m", "FT4"), Some(3_575_000));
    assert_eq!(t.frequency("20m", "FT8"), Some(14_074_000), "14,071 e 14,090 non sono predefinite");
    assert_eq!(t.frequency("6m", "FT8"), Some(50_313_000), "tra 50,313 e 50,323, entrambe predefinite, la piu' bassa");
    // regione scelta: la sua riga predefinita passa avanti
    assert_eq!(t.frequency_in(Region::R1, "160m", "FT8"), Some(1_810_000));
    assert_eq!(t.frequency_in(Region::R2, "160m", "FT8"), Some(1_840_000));
    assert_eq!(t.frequency_in(Region::R3, "80m", "FT4"), Some(3_568_000));
    assert_eq!(t.frequency_in(Region::R1, "80m", "FT4"), Some(3_575_000));
}

#[test]
fn switching_mode_stays_on_a_row_or_goes_to_the_nearest_preferred_one() {
    let t = FrequencyTable::default();
    // gia' su una riga del nuovo modo: si resta li' (anche non predefinita)
    assert_eq!(t.best_working(Region::All, "FT8", 14_090_000), Some(14_090_000));
    // FT8 -> FT4 su 20 m: una sola riga
    assert_eq!(t.best_working(Region::All, "FT4", 14_074_000), Some(14_080_000));
    // su una frequenza che non e' una riga vince la predefinita piu' vicina
    assert_eq!(t.best_working(Region::All, "FT8", 14_072_500), Some(14_074_000));
    assert_eq!(t.best_working(Region::All, "FT8", 14_089_000), Some(14_074_000), "14,090 e' piu' vicina ma non e' predefinita");
    // fuori banda o senza righe: niente
    assert_eq!(t.best_working(Region::All, "FT8", 12_345_678), None);
    assert_eq!(t.best_working(Region::All, "FT4", 1_840_000), None);
    // la regione toglie le righe che non vede
    assert_eq!(t.best_working(Region::R2, "FT8", 1_810_000), Some(1_840_000));
    assert_eq!(t.best_working(Region::R1, "FT8", 1_810_000), Some(1_810_000));
}

#[test]
fn region_names_follow_jtdx() {
    assert_eq!(Region::parse("ALL"), Region::All);
    assert_eq!(Region::parse("R1"), Region::R1);
    assert_eq!(Region::parse("r3"), Region::R3);
    assert_eq!(Region::parse("Region 2"), Region::All, "un nome sconosciuto vale ALL come in JTDX");
    assert_eq!([Region::All, Region::R1, Region::R2, Region::R3].map(Region::as_str), ["ALL", "R1", "R2", "R3"]);
}

#[test]
fn rows_with_the_same_band_and_mode_are_allowed_but_not_the_same_frequency_twice() {
    let mut t = FrequencyTable::default();
    assert!(t.set(vec![Entry::new("20m", "FT8", 14_074_000), Entry::new("20m", "FT8", 14_090_000)]).is_ok());
    assert_eq!(t.entries().len(), 2);
    assert!(t.set(vec![Entry::new("20m", "FT8", 14_074_000), Entry::new("20M", "ft8", 14_074_000)]).is_err());
}

#[test]
fn an_old_saved_table_moves_to_the_jtdx_one_and_a_custom_one_stays() {
    let legacy: Vec<Entry> = [
        ("160m", "FT8", 1_840_000), ("160m", "FT4", 1_840_000), ("80m", "FT8", 3_573_000), ("80m", "FT4", 3_575_000), ("60m", "FT8", 5_357_000),
        ("40m", "FT8", 7_074_000), ("40m", "FT4", 7_047_500), ("30m", "FT8", 10_136_000), ("30m", "FT4", 10_140_000), ("20m", "FT8", 14_074_000),
        ("20m", "FT4", 14_080_000), ("17m", "FT8", 18_100_000), ("17m", "FT4", 18_104_000), ("15m", "FT8", 21_074_000), ("15m", "FT4", 21_140_000),
        ("12m", "FT8", 24_915_000), ("12m", "FT4", 24_919_000), ("10m", "FT8", 28_074_000), ("10m", "FT4", 28_180_000), ("6m", "FT8", 50_313_000),
        ("6m", "FT4", 50_318_000), ("2m", "FT8", 144_174_000), ("2m", "FT4", 144_170_000),
    ]
    .iter()
    .map(|(b, m, h)| Entry::new(b, m, *h))
    .collect();
    assert_eq!(FrequencyTable::upgraded(legacy.clone()), FrequencyTable::default().entries().to_vec());
    let mut custom = legacy;
    custom[9] = Entry::new("20m", "FT8", 14_075_000);
    assert_eq!(FrequencyTable::upgraded(custom.clone()), custom, "una tabella modificata dall'utente non si tocca");
}

#[test]
fn rows_saved_without_region_read_as_all_and_preferred() {
    let rows: Vec<Entry> = serde_json::from_str(r#"[{"band":"20m","mode":"FT8","hz":14074000},{"band":"80m","mode":"FT4","hz":3568000,"region":"R3","preferred":false}]"#).unwrap();
    assert_eq!((rows[0].region, rows[0].preferred), (Region::All, true));
    assert_eq!((rows[1].region, rows[1].preferred), (Region::R3, false));
    assert_eq!(serde_json::to_value(&rows[1]).unwrap()["region"], "R3");
}
