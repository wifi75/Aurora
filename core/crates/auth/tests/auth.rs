use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

use auth::{AuthError, Clock, DeviceStore};

#[derive(Clone)]
struct FakeClock(Arc<AtomicU64>);

impl FakeClock {
    fn new() -> Self {
        FakeClock(Arc::new(AtomicU64::new(1_000)))
    }
    fn advance(&self, secs: u64) {
        self.0.fetch_add(secs, Ordering::SeqCst);
    }
}

impl Clock for FakeClock {
    fn now_secs(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}

fn store() -> (DeviceStore, FakeClock) {
    let clock = FakeClock::new();
    (DeviceStore::open_in_memory(Arc::new(clock.clone())).unwrap(), clock)
}

#[test]
fn pairing_returns_64_hex_token_that_verifies() {
    let (s, _) = store();
    let code = s.start_pairing().unwrap();
    assert_eq!(code.code.len(), 8);
    assert!(code.code.chars().all(|c| c.is_ascii_digit()));
    assert_eq!(code.expires_in_secs, 600, "il codice dura 10 minuti per impostazione predefinita");
    let token = s.finish_pairing(&code.code, "iPhone di Tiziano").unwrap();
    assert_eq!(token.0.len(), 64);
    assert!(token.0.chars().all(|c| c.is_ascii_hexdigit()));
    assert!(s.verify(&token.0).is_some());
}

#[test]
fn wrong_code_is_bad_code() {
    let (s, _) = store();
    let code = s.start_pairing().unwrap();
    let wrong = if code.code == "00000000" { "00000001" } else { "00000000" };
    assert_eq!(s.finish_pairing(wrong, "x").unwrap_err(), AuthError::BadCode);
}

#[test]
fn the_code_lifetime_can_be_changed_and_is_kept_within_sane_limits() {
    let clock = FakeClock::new();
    let short = DeviceStore::open_in_memory(Arc::new(clock.clone())).unwrap().with_code_ttl(30);
    assert_eq!(short.start_pairing().unwrap().expires_in_secs, 60, "mai meno di un minuto");
    let long = DeviceStore::open_in_memory(Arc::new(clock.clone())).unwrap().with_code_ttl(999_999);
    assert_eq!(long.start_pairing().unwrap().expires_in_secs, 3600, "mai piu' di un'ora");
    let custom = DeviceStore::open_in_memory(Arc::new(clock)).unwrap().with_code_ttl(300);
    assert_eq!(custom.start_pairing().unwrap().expires_in_secs, 300);
}

#[test]
fn the_lifetime_can_be_changed_while_running_and_applies_to_the_next_code() {
    let (s, _) = store();
    assert_eq!(s.code_ttl_secs(), 600);
    s.set_code_ttl(300);
    assert_eq!(s.code_ttl_secs(), 300);
    assert_eq!(s.start_pairing().unwrap().expires_in_secs, 300);
    s.set_code_ttl(1);
    assert_eq!(s.code_ttl_secs(), 60, "sempre dentro i limiti");
}

#[test]
fn code_survives_the_old_two_minutes_and_expires_after_ten() {
    let (s, clock) = store();
    let code = s.start_pairing().unwrap();
    clock.advance(300);
    assert!(s.finish_pairing(&code.code, "ancora valido").is_ok());
}

#[test]
fn code_expires_after_the_lifetime() {
    let (s, clock) = store();
    let code = s.start_pairing().unwrap();
    clock.advance(601);
    assert_eq!(s.finish_pairing(&code.code, "x").unwrap_err(), AuthError::Expired);
}

#[test]
fn code_is_single_use() {
    let (s, _) = store();
    let code = s.start_pairing().unwrap();
    s.finish_pairing(&code.code, "uno").unwrap();
    assert_eq!(s.finish_pairing(&code.code, "due").unwrap_err(), AuthError::BadCode);
}

#[test]
fn five_wrong_codes_lock_for_60s_even_with_right_code() {
    let (s, clock) = store();
    let code = s.start_pairing().unwrap();
    let wrong = if code.code == "00000000" { "00000001" } else { "00000000" };
    for _ in 0..5 {
        assert_eq!(s.finish_pairing(wrong, "x").unwrap_err(), AuthError::BadCode);
    }
    match s.finish_pairing(&code.code, "x").unwrap_err() {
        AuthError::Locked { retry_after_secs } => assert!((1..=60).contains(&retry_after_secs)),
        other => panic!("atteso Locked, trovato {other:?}"),
    }
    clock.advance(61);
    assert!(s.finish_pairing(&code.code, "x").is_ok(), "dopo 61 s il codice valido torna utilizzabile");
}

#[test]
fn database_stores_only_hash() {
    let path = std::env::temp_dir().join(format!("aurora-auth-test-{}.db", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let token = {
        let s = DeviceStore::open(&path, Arc::new(FakeClock::new())).unwrap();
        let code = s.start_pairing().unwrap();
        s.finish_pairing(&code.code, "telefono").unwrap()
    };
    let raw = std::fs::read(&path).unwrap();
    let hay = String::from_utf8_lossy(&raw);
    assert!(!hay.contains(&token.0), "il token in chiaro non deve stare nel file");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn revoked_token_no_longer_verifies() {
    let (s, _) = store();
    let code = s.start_pairing().unwrap();
    let token = s.finish_pairing(&code.code, "telefono").unwrap();
    let id = s.verify(&token.0).unwrap();
    assert!(s.revoke(id).unwrap());
    assert!(s.verify(&token.0).is_none());
    assert!(!s.revoke(id).unwrap(), "seconda revoca: nulla da revocare");
}

#[test]
fn unknown_token_is_none() {
    let (s, _) = store();
    assert!(s.verify(&"a".repeat(64)).is_none());
    assert!(s.verify("").is_none());
}

#[test]
fn list_never_returns_token_or_hash() {
    let (s, _) = store();
    let code = s.start_pairing().unwrap();
    let token = s.finish_pairing(&code.code, "telefono").unwrap();
    let list = s.list().unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].name, "telefono");
    let dump = format!("{list:?}");
    assert!(!dump.contains(&token.0));
}

#[test]
fn empty_device_name_is_rejected() {
    let (s, _) = store();
    let code = s.start_pairing().unwrap();
    assert_eq!(s.finish_pairing(&code.code, "   ").unwrap_err(), AuthError::BadName);
}

#[test]
fn exists_is_false_after_revoke() {
    let (s, _) = store();
    let code = s.start_pairing().unwrap();
    let token = s.finish_pairing(&code.code, "telefono").unwrap();
    let id = s.verify(&token.0).unwrap();
    assert!(s.exists(id));
    s.revoke(id).unwrap();
    assert!(!s.exists(id));
}
