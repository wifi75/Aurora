mod common;

use std::{net::SocketAddr, path::PathBuf, time::Duration};

use aurora::{run, Config, EngineKind, RunningServer};
use aurora_core::Event;
use common::*;
use tokio::{net::UdpSocket, sync::oneshot, time::timeout};
use udp_bridge::{decode, encode_halt_tx, encode_reply, Message};

const REPLY: u32 = 4;

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../udp-bridge/tests/fixtures").join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn kind_of(d: &[u8]) -> u32 {
    u32::from_be_bytes(d[8..12].try_into().unwrap())
}

struct Rig {
    server: RunningServer,
    jtdx: UdpSocket,
    hub_addr: SocketAddr,
    _stop: oneshot::Sender<()>,
}

async fn rig(allow_tx: bool, forward: Vec<SocketAddr>) -> Rig {
    let (tx, rx) = oneshot::channel::<()>();
    let config = Config {
        bind: "127.0.0.1:0".parse().unwrap(),
        data_dir: temp_dir("jtdx"),
        engine: EngineKind::Jtdx,
        udp_listen: "127.0.0.1:0".parse().unwrap(),
        udp_forward: forward,
        allow_tx,
        ..Config::defaults()
    };
    let server = run(config, async move {
        let _ = rx.await;
    })
    .await
    .unwrap();
    let hub_addr = server.hub.as_ref().expect("con il motore JTDX l'hub esiste").local_addr();
    let jtdx = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    Rig { server, jtdx, hub_addr, _stop: tx }
}

impl Rig {
    async fn send(&self, bytes: &[u8]) {
        self.jtdx.send_to(bytes, self.hub_addr).await.unwrap();
    }

    async fn received(&self, wait_ms: u64) -> Vec<Vec<u8>> {
        let mut out = Vec::new();
        let mut buf = vec![0u8; 65_535];
        while let Ok(Ok((n, _))) = timeout(Duration::from_millis(wait_ms), self.jtdx.recv_from(&mut buf)).await {
            out.push(buf[..n].to_vec());
        }
        out
    }
}

#[test]
fn config_reads_jtdx_variables_and_rejects_bad_ones() {
    let none = |_: &str| None::<String>;
    let c = Config::from_lookup(none).unwrap();
    assert_eq!(c.udp_listen, "127.0.0.1:2334".parse::<SocketAddr>().unwrap());
    assert!(c.udp_forward.is_empty());
    assert!(!c.allow_tx, "la trasmissione e' chiusa per impostazione predefinita");
    assert!(c.audio_device.is_none());
    assert!(c.replay_on_connect, "il Replay all'avvio e' attivo per impostazione predefinita");
    assert!(c.cty_path.is_none() && c.worked_adif.is_none());
    assert_eq!((c.auto_limits.max_consecutive_cycles, c.auto_limits.max_per_day, c.auto_limits.dead_man_secs), (6, 40, 120));
    assert!(!c.auto_limits.run_unattended);
    assert!(c.export_udp.is_empty() && c.export_file.is_none());
    assert!(!c.rig_omnirig, "il comando radio e' spento per impostazione predefinita");

    let env = |k: &str| match k {
        "AURORA_ENGINE" => Some("jtdx".to_string()),
        "AURORA_UDP_LISTEN" => Some("127.0.0.1:2340".to_string()),
        "AURORA_UDP_FORWARD" => Some("127.0.0.1:2333, 127.0.0.1:2335".to_string()),
        "AURORA_ALLOW_TX" => Some("1".to_string()),
        "AURORA_AUDIO_DEVICE" => Some("USB Audio".to_string()),
        "AURORA_REPLAY_ON_CONNECT" => Some("0".to_string()),
        "AURORA_CTY_PATH" => Some("dati/cty.dat".to_string()),
        "AURORA_WORKED_ADIF" => Some("dati/log.adi".to_string()),
        "AURORA_AUTO_MAX_CYCLES" => Some("3".to_string()),
        "AURORA_AUTO_MAX_PER_DAY" => Some("10".to_string()),
        "AURORA_AUTO_DEAD_MAN_SECS" => Some("30".to_string()),
        "AURORA_AUTO_UNATTENDED" => Some("1".to_string()),
        "AURORA_EXPORT_UDP" => Some("127.0.0.1:2239, 127.0.0.1:2240".to_string()),
        "AURORA_EXPORT_FILE" => Some("dati/export.adi".to_string()),
        "AURORA_RIG" => Some("omnirig".to_string()),
        _ => None,
    };
    let c = Config::from_lookup(env).unwrap();
    assert_eq!(c.engine, EngineKind::Jtdx);
    assert_eq!(c.udp_listen.port(), 2340);
    assert_eq!(c.udp_forward.len(), 2);
    assert_eq!(c.udp_forward[1].port(), 2335);
    assert!(c.allow_tx);
    assert_eq!(c.audio_device.as_deref(), Some("USB Audio"));
    assert!(!c.replay_on_connect);
    assert_eq!(c.cty_path, Some(PathBuf::from("dati/cty.dat")));
    assert_eq!(c.worked_adif, Some(PathBuf::from("dati/log.adi")));
    assert_eq!((c.auto_limits.max_consecutive_cycles, c.auto_limits.max_per_day, c.auto_limits.dead_man_secs), (3, 10, 30));
    assert!(c.auto_limits.run_unattended);
    assert_eq!(c.export_udp.len(), 2);
    assert!(c.rig_omnirig);
    assert_eq!(c.export_file, Some(PathBuf::from("dati/export.adi")));

    let bad = |name: &'static str, value: &'static str| move |k: &str| (k == name).then(|| value.to_string());
    assert!(Config::from_lookup(bad("AURORA_UDP_LISTEN", "non-un-indirizzo")).is_err());
    assert!(Config::from_lookup(bad("AURORA_UDP_FORWARD", "127.0.0.1:2333,xyz")).is_err());
    assert!(Config::from_lookup(bad("AURORA_ALLOW_TX", "2")).is_err());
    assert!(Config::from_lookup(bad("AURORA_ALLOW_TX", "forse")).is_err());
    assert!(Config::from_lookup(bad("AURORA_REPLAY_ON_CONNECT", "boh")).is_err());
    assert!(Config::from_lookup(bad("AURORA_AUTO_MAX_CYCLES", "0")).is_err(), "un tetto a zero bloccherebbe tutto: si rifiuta");
    assert!(Config::from_lookup(bad("AURORA_AUTO_MAX_PER_DAY", "molte")).is_err());
    assert!(Config::from_lookup(bad("AURORA_AUTO_DEAD_MAN_SECS", "-5")).is_err());
    assert!(Config::from_lookup(bad("AURORA_AUTO_UNATTENDED", "si")).is_err());
    assert!(Config::from_lookup(bad("AURORA_EXPORT_UDP", "127.0.0.1:2239,xyz")).is_err());
    assert!(Config::from_lookup(bad("AURORA_RIG", "hamlib")).is_err());
    assert!(!Config::from_lookup(bad("AURORA_RIG", "none")).unwrap().rig_omnirig);
}

#[tokio::test]
async fn e2e_automation_starts_off_and_dry_run_and_live_is_closed_by_default() {
    let r = rig(false, vec![]).await;
    let cookie = cookie_for(&r.server);
    let (code, body) = http(r.server.addr, "GET", "/api/v1/automation", Some(&cookie), None).await;
    assert_eq!(code, 200, "{body}");
    assert!(body.contains("\"enabled\":false") && body.contains("\"mode\":\"dry_run\"") && body.contains("\"live_allowed\":false"), "{body}");
}

#[tokio::test]
async fn e2e_jtdx_status_and_decode_arrive_over_websocket() {
    let r = rig(false, vec![]).await;
    let mut ws = open_ws(r.server.addr, &cookie_for(&r.server)).await;
    r.send(&fixture("status-decoding.bin")).await;
    r.send(&fixture("decode-cq.bin")).await;
    let (mut status, mut decode_seen) = (false, false);
    for _ in 0..10 {
        match next_event(&mut ws, 1).await {
            Some(Event::Status(s)) if s.dial_hz == 21_074_000 => status = true,
            Some(Event::Decode(d)) if d.text == "CQ AA1AA FN42" => decode_seen = true,
            _ => {}
        }
        if status && decode_seen {
            break;
        }
    }
    assert!(status && decode_seen, "status: {status}, decode: {decode_seen}");
}

const CTY: &str = "Utopia: 14: 27: EU: 43.00: -7.00: -1.0: AA:
    AA;
Lemuria: 05: 08: NA: 10.00: 60.00: 5.0: LM:
    LM;
";

#[tokio::test]
async fn e2e_decode_carries_country_and_news_when_data_files_are_given() {
    let dir = temp_dir("enrich");
    let (cty, adif) = (dir.join("cty.dat"), dir.join("log.adi"));
    std::fs::write(&cty, CTY).unwrap();
    std::fs::write(&adif, "<CALL:5>AA9ZZ<BAND:3>15m<GRIDSQUARE:4>FN42<EOR>
").unwrap();
    let (tx, rx) = oneshot::channel::<()>();
    let config = Config {
        bind: "127.0.0.1:0".parse().unwrap(),
        data_dir: temp_dir("enrich-data"),
        engine: EngineKind::Jtdx,
        udp_listen: "127.0.0.1:0".parse().unwrap(),
        cty_path: Some(cty),
        worked_adif: Some(adif),
        ..Config::defaults()
    };
    let server = run(config, async move {
        let _ = rx.await;
    })
    .await
    .unwrap();
    let jtdx = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let hub = server.hub.as_ref().unwrap().local_addr();
    let mut ws = open_ws(server.addr, &cookie_for(&server)).await;
    jtdx.send_to(&fixture("status-decoding.bin"), hub).await.unwrap();
    jtdx.send_to(&fixture("decode-cq.bin"), hub).await.unwrap();
    let mut seen = None;
    for _ in 0..10 {
        if let Some(Event::Decode(d)) = next_event(&mut ws, 1).await {
            seen = Some(d);
            break;
        }
    }
    let d = seen.expect("decode non ricevuto");
    assert_eq!(d.country.as_deref(), Some("Utopia"));
    assert_eq!(d.tag, Some(aurora_core::DecodeTag::NewPx), "AA1AA: paese, zona e griglia gia' fatti su 15m, ma il prefisso AA1 no");
    drop(tx);
}

#[tokio::test]
async fn automation_rules_are_loaded_at_start_and_saved_when_changed() {
    let data = temp_dir("rules");
    std::fs::write(
        data.join("automation-rules.json"),
        r#"{"reply_to":["new_dxcc"],"min_snr":-11,"excluded_calls":["AT1AA"],"max_attempts_per_call":2}"#,
    )
    .unwrap();
    let (tx, rx) = oneshot::channel::<()>();
    let config = Config {
        bind: "127.0.0.1:0".parse().unwrap(),
        data_dir: data.clone(),
        engine: EngineKind::Jtdx,
        udp_listen: "127.0.0.1:0".parse().unwrap(),
        ..Config::defaults()
    };
    let server = run(config, async move {
        let _ = rx.await;
    })
    .await
    .unwrap();
    let cookie = cookie_for(&server);
    let (_, body) = http(server.addr, "GET", "/api/v1/automation", Some(&cookie), None).await;
    assert!(body.contains("\"min_snr\":-11") && body.contains("AT1AA"), "{body}");
    let change = r#"{"rules":{"reply_to":["new_zone"],"min_snr":-7,"excluded_calls":[],"max_attempts_per_call":3}}"#;
    let (code, _) = http(server.addr, "PUT", "/api/v1/automation", Some(&cookie), Some(change)).await;
    assert_eq!(code, 200);
    let saved = std::fs::read_to_string(data.join("automation-rules.json")).unwrap();
    assert!(saved.contains("\"min_snr\":-7") && saved.contains("new_zone"), "{saved}");
    drop(tx);
}

#[tokio::test]
async fn an_invalid_rules_file_falls_back_to_the_defaults() {
    let data = temp_dir("rules-bad");
    std::fs::write(data.join("automation-rules.json"), r#"{"reply_to":["cq"],"min_snr":-11,"excluded_calls":[],"max_attempts_per_call":2}"#).unwrap();
    let (tx, rx) = oneshot::channel::<()>();
    let config = Config {
        bind: "127.0.0.1:0".parse().unwrap(),
        data_dir: data,
        engine: EngineKind::Jtdx,
        udp_listen: "127.0.0.1:0".parse().unwrap(),
        ..Config::defaults()
    };
    let server = run(config, async move {
        let _ = rx.await;
    })
    .await
    .unwrap();
    let cookie = cookie_for(&server);
    let (_, body) = http(server.addr, "GET", "/api/v1/automation", Some(&cookie), None).await;
    assert!(body.contains("\"min_snr\":-18"), "{body}");
    drop(tx);
}

#[tokio::test]
async fn e2e_stats_come_from_the_log_and_list_missing_countries() {
    let dir = temp_dir("stats");
    let (cty, adif) = (dir.join("cty.dat"), dir.join("log.adi"));
    std::fs::write(&cty, CTY).unwrap();
    std::fs::write(&adif, "<CALL:5>AA9ZZ<BAND:3>15m<GRIDSQUARE:4>FN42<EOR>
").unwrap();
    let (tx, rx) = oneshot::channel::<()>();
    let config = Config {
        bind: "127.0.0.1:0".parse().unwrap(),
        data_dir: temp_dir("stats-data"),
        engine: EngineKind::Jtdx,
        udp_listen: "127.0.0.1:0".parse().unwrap(),
        cty_path: Some(cty),
        worked_adif: Some(adif.clone()),
        ..Config::defaults()
    };
    let server = run(config, async move {
        let _ = rx.await;
    })
    .await
    .unwrap();
    let cookie = cookie_for(&server);
    let (code, body) = http(server.addr, "GET", "/api/v1/stats", Some(&cookie), None).await;
    assert_eq!(code, 200, "{body}");
    assert!(body.contains("\"dxcc_worked\":1") && body.contains("Lemuria"), "{body}");
    // un QSO nuovo nel file si vede alla richiesta successiva, senza riavvio
    std::fs::write(&adif, "<CALL:5>AA9ZZ<BAND:3>15m<GRIDSQUARE:4>FN42<EOR>
<CALL:5>LM1XY<BAND:3>20m<EOR>
").unwrap();
    let (_, again) = http(server.addr, "GET", "/api/v1/stats", Some(&cookie), None).await;
    assert!(again.contains("\"dxcc_worked\":2"), "{again}");
    drop(tx);
}

#[tokio::test]
async fn frequencies_and_schedule_are_loaded_at_start_and_saved_when_changed() {
    let data = temp_dir("settings");
    std::fs::write(data.join("frequencies.json"), r#"[{"band":"20m","mode":"FT8","hz":14075000}]"#).unwrap();
    std::fs::write(data.join("schedule.json"), r#"{"enabled":true,"entries":[{"hour":6,"minute":30,"band":"20m"}]}"#).unwrap();
    let (tx, rx) = oneshot::channel::<()>();
    let config = Config {
        bind: "127.0.0.1:0".parse().unwrap(),
        data_dir: data.clone(),
        engine: EngineKind::Jtdx,
        udp_listen: "127.0.0.1:0".parse().unwrap(),
        ..Config::defaults()
    };
    let server = run(config, async move {
        let _ = rx.await;
    })
    .await
    .unwrap();
    let cookie = cookie_for(&server);
    let (_, freq) = http(server.addr, "GET", "/api/v1/frequencies", Some(&cookie), None).await;
    assert!(freq.contains("14075000") && !freq.contains("7074000"), "{freq}");
    let (_, sched) = http(server.addr, "GET", "/api/v1/schedule", Some(&cookie), None).await;
    assert!(sched.contains("\"enabled\":true") && sched.contains("06") || sched.contains("\"hour\":6"), "{sched}");

    let change = r#"[{"band":"40m","mode":"FT8","hz":7074000}]"#;
    let (code, _) = http(server.addr, "PUT", "/api/v1/frequencies", Some(&cookie), Some(change)).await;
    assert_eq!(code, 200);
    let saved = std::fs::read_to_string(data.join("frequencies.json")).unwrap();
    assert!(saved.contains("40m") && saved.contains("7074000"), "{saved}");
    drop(tx);
}

#[tokio::test]
async fn a_broken_settings_file_falls_back_to_the_defaults() {
    let data = temp_dir("settings-bad");
    std::fs::write(data.join("frequencies.json"), "non json").unwrap();
    std::fs::write(data.join("schedule.json"), r#"{"enabled":true,"entries":[{"hour":99,"minute":0,"band":"20m"}]}"#).unwrap();
    let (tx, rx) = oneshot::channel::<()>();
    let config = Config {
        bind: "127.0.0.1:0".parse().unwrap(),
        data_dir: data,
        engine: EngineKind::Jtdx,
        udp_listen: "127.0.0.1:0".parse().unwrap(),
        ..Config::defaults()
    };
    let server = run(config, async move {
        let _ = rx.await;
    })
    .await
    .unwrap();
    let cookie = cookie_for(&server);
    let (_, freq) = http(server.addr, "GET", "/api/v1/frequencies", Some(&cookie), None).await;
    assert!(freq.contains("14074000"), "{freq}");
    let (_, sched) = http(server.addr, "GET", "/api/v1/schedule", Some(&cookie), None).await;
    assert!(sched.contains("\"enabled\":false"), "{sched}");
    drop(tx);
}

#[tokio::test]
async fn missing_data_files_do_not_stop_aurora() {
    let (tx, rx) = oneshot::channel::<()>();
    let config = Config {
        bind: "127.0.0.1:0".parse().unwrap(),
        data_dir: temp_dir("enrich-missing"),
        engine: EngineKind::Jtdx,
        udp_listen: "127.0.0.1:0".parse().unwrap(),
        cty_path: Some(PathBuf::from("/non/esiste/cty.dat")),
        worked_adif: Some(PathBuf::from("/non/esiste/log.adi")),
        ..Config::defaults()
    };
    let server = run(config, async move {
        let _ = rx.await;
    })
    .await;
    assert!(server.is_ok());
    drop(tx);
}

#[tokio::test]
async fn e2e_forward_target_receives_identical_datagrams() {
    let target = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let r = rig(false, vec![target.local_addr().unwrap()]).await;
    let status = fixture("status-decoding.bin");
    r.send(&status).await;
    let mut buf = vec![0u8; 65_535];
    let (n, _) = timeout(Duration::from_secs(1), target.recv_from(&mut buf)).await.unwrap().unwrap();
    assert_eq!(&buf[..n], status.as_slice(), "GridTracker2 deve ricevere i byte identici");
}

#[tokio::test]
async fn e2e_reply_is_403_and_nothing_reaches_jtdx_by_default() {
    let r = rig(false, vec![]).await;
    let cookie = cookie_for(&r.server);
    let mut ws = open_ws(r.server.addr, &cookie).await;
    r.send(&fixture("decode-cq.bin")).await;
    let id = loop {
        match next_event(&mut ws, 2).await {
            Some(Event::Decode(d)) => break d.id,
            Some(_) => {}
            None => panic!("nessun decode ricevuto"),
        }
    };
    tokio::time::sleep(Duration::from_millis(200)).await;
    let body = format!(r#"{{"decode_id":{id}}}"#);
    let (status, text) = http(r.server.addr, "POST", "/api/v1/commands/reply", Some(&cookie), Some(&body)).await;
    assert_eq!(status, 403);
    assert!(text.contains("tx_disabled"));
    let got = r.received(400).await;
    assert!(got.iter().all(|d| kind_of(d) != REPLY), "nessun Reply deve arrivare a JTDX");
}

#[tokio::test]
async fn e2e_reply_reaches_jtdx_only_with_allow_tx() {
    let r = rig(true, vec![]).await;
    let cookie = cookie_for(&r.server);
    let mut ws = open_ws(r.server.addr, &cookie).await;
    let packet = fixture("decode-cq.bin");
    r.send(&packet).await;
    let id = loop {
        match next_event(&mut ws, 2).await {
            Some(Event::Decode(d)) => break d.id,
            Some(_) => {}
            None => panic!("nessun decode ricevuto"),
        }
    };
    tokio::time::sleep(Duration::from_millis(200)).await;
    let body = format!(r#"{{"decode_id":{id}}}"#);
    let (status, _) = http(r.server.addr, "POST", "/api/v1/commands/reply", Some(&cookie), Some(&body)).await;
    assert_eq!(status, 200);
    let original = match decode(&packet).unwrap().message {
        Message::Decode(d) => d,
        other => panic!("{other:?}"),
    };
    let got = r.received(500).await;
    assert!(got.contains(&encode_reply("JTDX", 2, &original, 0)), "il Reply deve riportare il decode originale");
}

#[tokio::test]
async fn e2e_halt_reaches_jtdx_by_default() {
    let r = rig(false, vec![]).await;
    let cookie = cookie_for(&r.server);
    r.send(&fixture("status-decoding.bin")).await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    let (status, _) = http(r.server.addr, "POST", "/api/v1/commands/halt-tx", Some(&cookie), None).await;
    assert_eq!(status, 200);
    let got = r.received(500).await;
    assert!(got.contains(&encode_halt_tx("JTDX", 2, false)));
}

#[tokio::test]
async fn e2e_command_from_forward_target_reaches_jtdx() {
    let target = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let r = rig(false, vec![target.local_addr().unwrap()]).await;
    r.send(&fixture("status-decoding.bin")).await;
    let mut buf = vec![0u8; 65_535];
    let (_, hub_side) = timeout(Duration::from_secs(1), target.recv_from(&mut buf)).await.unwrap().unwrap();
    let command = encode_halt_tx("JTDX", 2, true);
    target.send_to(&command, hub_side).await.unwrap();
    let got = r.received(500).await;
    assert!(got.contains(&command), "il comando di GridTracker2 deve arrivare a JTDX");
}
