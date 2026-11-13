//! L'invio diretto dei QSO contro server locali (QRZ, Wavelog, ClubLog): nessuna prova esce dal PC.

use std::{sync::Arc, time::Duration};

use aurora_core::{Bus, Event, QsoRecord};
use reporting::{
    secret::{MemorySecrets, SecretStore},
    status::ReportStatus,
    upload::Target,
    ClublogAccount, Config, Endpoints, Service, WavelogAccount, CLUBLOG_API_SECRET, CLUBLOG_SECRET, QRZ_SECRET, WAVELOG_SECRET,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::mpsc,
    time::timeout,
};

// 2026-10-24 16:46:45 UTC
const NOW: u64 = 1_792_860_405;

fn qso(call: &str) -> QsoRecord {
    QsoRecord {
        call: call.into(),
        grid: "FN42".into(),
        band: "20m".into(),
        mode: "FT8".into(),
        freq_hz: 14_075_084,
        rst_sent: "-16".into(),
        rst_rcvd: "-09".into(),
        time_on_unix_ms: 1_792_860_405_000,
        time_off_unix_ms: 1_792_860_449_000,
        ..QsoRecord::default()
    }
}

struct Rig {
    bus: Bus,
    service: Arc<Service>,
    status: Arc<ReportStatus>,
    secrets: Arc<MemorySecrets>,
}

fn rig(endpoints: Endpoints, initial: Config) -> Rig {
    let bus = Bus::new(256);
    let status = Arc::new(ReportStatus::new());
    let secrets = Arc::new(MemorySecrets::default());
    let service = Service::spawn(&bus, Arc::clone(&status), Arc::clone(&secrets) as Arc<dyn SecretStore>, endpoints, Arc::new(|| NOW), initial);
    Rig { bus, service, status, secrets }
}

fn base() -> Config {
    Config { call: "K1ABC".into(), grid: "FN42".into(), ..Config::default() }
}

/// Un server HTTP che manda al canale la richiesta intera (intestazioni e corpo) e risponde con le risposte date, una per richiesta (l'ultima si ripete).
async fn server(replies: Vec<(&'static str, &'static str)>) -> (String, mpsc::UnboundedReceiver<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("server");
    let addr = listener.local_addr().expect("indirizzo");
    let (tx, rx) = mpsc::unbounded_channel();
    tokio::spawn(async move {
        let mut n = 0usize;
        loop {
            let Ok((mut stream, _)) = listener.accept().await else { break };
            let (status, body) = replies[n.min(replies.len() - 1)];
            n += 1;
            let tx = tx.clone();
            tokio::spawn(async move {
                let mut data = Vec::new();
                let mut buf = vec![0u8; 16_384];
                while let Ok(k) = stream.read(&mut buf).await {
                    if k == 0 {
                        break;
                    }
                    data.extend_from_slice(&buf[..k]);
                    let text = String::from_utf8_lossy(&data).to_string();
                    if let Some((head, rest)) = text.split_once("\r\n\r\n") {
                        let len: usize = head
                            .lines()
                            .find_map(|l| l.to_ascii_lowercase().strip_prefix("content-length:").map(|v| v.trim().parse().unwrap_or(0)))
                            .unwrap_or(0);
                        if rest.len() >= len {
                            break;
                        }
                    }
                }
                let _ = tx.send(String::from_utf8_lossy(&data).to_string());
                let response = format!("HTTP/1.1 {status}\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
                let _ = stream.write_all(response.as_bytes()).await;
                let _ = stream.shutdown().await;
            });
        }
    });
    (format!("http://{addr}"), rx)
}

fn decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 3 <= b.len() {
            out.push(u8::from_str_radix(&s[i + 1..i + 3], 16).unwrap_or(b'?'));
            i += 3;
        } else {
            out.push(if b[i] == b'+' { b' ' } else { b[i] });
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).to_string()
}

async fn wait_status(r: &Rig, kind: &str, ok: bool) -> reporting::status::SendState {
    for _ in 0..50 {
        let st = r.status.get(kind);
        if st.ok == Some(ok) {
            return st;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    r.status.get(kind)
}

#[tokio::test]
async fn qrz_gets_key_action_and_adif_and_the_key_never_reaches_the_state() {
    let (url, mut requests) = server(vec![("200 OK", "RESULT=OK&LOGID=42&COUNT=1")]).await;
    let r = rig(Endpoints { qrz_url: format!("{url}/api"), ..Endpoints::default() }, Config { qrz: true, ..base() });
    r.secrets.set(QRZ_SECRET, "ABCD-0A0B-1C1D-2E2F").unwrap();
    r.bus.events().send(Event::QsoLogged(qso("DL1ABC"))).unwrap();
    let request = timeout(Duration::from_secs(5), requests.recv()).await.expect("richiesta").unwrap();
    assert!(request.starts_with("POST /api "), "{request}");
    assert!(request.to_lowercase().contains("user-agent: aurora/"), "User-Agent riconoscibile: {request}");
    let body = decode(request.split("\r\n\r\n").nth(1).unwrap());
    assert!(body.contains("KEY=ABCD-0A0B-1C1D-2E2F") && body.contains("ACTION=INSERT"), "{body}");
    assert!(body.contains("ADIF=<BAND:3>20m <STATION_CALLSIGN:5>K1ABC") && body.contains("<CALL:6>DL1ABC") && body.trim_end().ends_with("<EOR>"), "{body}");
    let st = wait_status(&r, "qrz", true).await;
    assert_eq!(st.ok, Some(true), "{st:?}");
    assert!(st.message.contains("DL1ABC") && st.message.contains("LOGID 42"), "{}", st.message);
    assert!(!r.status.snapshot().to_string().contains("ABCD-0A0B"), "la chiave non e' mai nello stato");
    assert_eq!((st.pending, r.service.pending(Target::Qrz)), (0, 0));
}

#[tokio::test]
async fn wavelog_down_keeps_the_qso_in_the_queue_for_a_later_attempt() {
    let (url, mut requests) = server(vec![("503 Service Unavailable", "")]).await;
    let account = WavelogAccount { url: format!("{url}/index.php"), station_id: "3".into() };
    let r = rig(Endpoints::default(), Config { wavelog: Some(account), ..base() });
    r.secrets.set(WAVELOG_SECRET, "wl0123abcdef").unwrap();
    r.bus.events().send(Event::QsoLogged(qso("DL1ABC"))).unwrap();
    let request = timeout(Duration::from_secs(5), requests.recv()).await.expect("richiesta").unwrap();
    assert!(request.starts_with("POST /index.php/api/qso "), "{request}");
    let body: serde_json::Value = serde_json::from_str(request.split("\r\n\r\n").nth(1).unwrap()).unwrap();
    assert_eq!((body["key"].as_str(), body["station_profile_id"].as_str(), body["type"].as_str()), (Some("wl0123abcdef"), Some("3"), Some("adif")));
    assert!(body["string"].as_str().unwrap().contains("<CALL:6>DL1ABC"));
    let st = wait_status(&r, "wavelog", false).await;
    assert!(st.message.contains("503") && st.message.contains("nuovo tentativo"), "{}", st.message);
    assert_eq!((st.pending, r.service.pending(Target::Wavelog)), (1, 1), "il QSO resta in coda");
    // il tentativo successivo non parte subito
    assert!(timeout(Duration::from_millis(600), requests.recv()).await.is_err());
}

#[tokio::test]
async fn clublog_stops_on_a_credentials_error_and_restarts_when_they_change() {
    let (url, mut requests) = server(vec![("403 Forbidden", "Access denied"), ("200 OK", "QSO OK")]).await;
    let endpoints = Endpoints { clublog_url: format!("{url}/realtime.php"), ..Endpoints::default() };
    let r = rig(endpoints, Config { clublog: Some(ClublogAccount { email: "k1abc@example.com".into(), callsign: String::new() }), ..base() });
    r.secrets.set(CLUBLOG_SECRET, "app-password").unwrap();
    r.secrets.set(CLUBLOG_API_SECRET, "chiave-app").unwrap();
    r.bus.events().send(Event::QsoLogged(qso("DL1ABC"))).unwrap();
    let request = timeout(Duration::from_secs(5), requests.recv()).await.expect("richiesta").unwrap();
    let body = decode(request.split("\r\n\r\n").nth(1).unwrap());
    for part in ["email=k1abc@example.com", "password=app-password", "callsign=K1ABC", "api=chiave-app", "adif=<BAND:3>20m"] {
        assert!(body.contains(part), "{part}: {body}");
    }
    let st = wait_status(&r, "clublog", false).await;
    assert!(st.message.contains("fermo"), "{}", st.message);
    assert_eq!(st.code, "clublog_denied", "il codice stabile per tradurre l'errore");
    // fermo: un altro QSO resta in coda senza richieste
    r.bus.events().send(Event::QsoLogged(QsoRecord { time_off_unix_ms: 1_792_861_449_000, ..qso("F5ABC") })).unwrap();
    assert!(timeout(Duration::from_millis(800), requests.recv()).await.is_err(), "nessuna richiesta con credenziali sbagliate");
    assert_eq!(r.service.pending(Target::Clublog), 2);
    // credenziali nuove: riparte e manda tutto
    r.secrets.set(CLUBLOG_SECRET, "nuova").unwrap();
    r.service.credentials_changed();
    timeout(Duration::from_secs(5), requests.recv()).await.expect("ripartito").unwrap();
    timeout(Duration::from_secs(5), requests.recv()).await.expect("secondo QSO").unwrap();
    let st = wait_status(&r, "clublog", true).await;
    assert_eq!(st.ok, Some(true), "{st:?}");
    for _ in 0..20 {
        if r.service.pending(Target::Clublog) == 0 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert_eq!(r.service.pending(Target::Clublog), 0);
}

#[tokio::test]
async fn the_test_of_wavelog_reads_the_stations_without_writing_and_lotw_needs_tqsl() {
    let (url, mut requests) = server(vec![
        ("200 OK", "<auth><status>Valid</status><rights>rw</rights></auth>"),
        ("200 OK", r#"[{"station_id":"3","station_profile_name":"Casa","station_callsign":"K1ABC"}]"#),
    ])
    .await;
    let account = WavelogAccount { url: url.clone(), station_id: "3".into() };
    let endpoints = Endpoints { tqsl_candidates: vec![], ..Endpoints::default() };
    let r = rig(endpoints, Config { wavelog: Some(account), lotw: Some(reporting::LotwSetup { tqsl: String::new(), location: "Casa".into() }), ..base() });
    r.secrets.set(WAVELOG_SECRET, "wl0123abcdef").unwrap();
    let got = r.service.test_upload(Target::Wavelog).await.unwrap();
    assert!(got.contains("Casa") && got.contains("K1ABC"), "{got}");
    let request = requests.recv().await.unwrap();
    assert!(request.starts_with("GET /index.php/api/auth/wl0123abcdef "), "{request}");
    let request = requests.recv().await.unwrap();
    assert!(request.starts_with("GET /index.php/api/station_info/wl0123abcdef "), "{request}");
    let lotw = r.service.test_upload(Target::Lotw).await.unwrap_err();
    assert!(lotw.contains("TQSL non trovato"), "{lotw}");
    // spento: la prova lo dice
    assert!(r.service.test_upload(Target::Qrz).await.is_err());
}
