//! Etichette sui decode: paese e novita' (nuovo DXCC, DXCC su nuova banda, zona, griglia), con dati di fantasia.

use std::{path::PathBuf, sync::Arc, time::Duration};

use aurora_core::{Bus, DecodeTag, Engine, EngineIo, Event};
use dxcc::DxccDb;
use engine_jtdx::{worked::{HighlightFlags, WorkedLog}, JtdxConfig, JtdxEngine, SystemClock};
use tokio::{net::UdpSocket, sync::broadcast, time::timeout};
use udp_bridge::{wire::Writer, MAGIC};
use udp_hub::{Hub, HubConfig};

const CTY: &str = "\
Atlantis:                 14:  27:  EU:   43.00:    -7.00:    -1.0:  AT:
    AT,AU,AT5(15)[30];
Lemuria:                  05:  08:  NA:   10.00:    60.00:     5.0:  LM:
    LM,LMA,LMB;
Utopia:                   14:  27:  EU:   44.00:    -8.00:    -1.0:  UT:
    UT;
";

struct Rig {
    hub: Arc<Hub>,
    jtdx: UdpSocket,
    events: broadcast::Receiver<Event>,
    last_continent: std::sync::Mutex<Option<String>>,
}

async fn rig(adif: &str) -> Rig {
    rig_flags(adif, HighlightFlags::all_on()).await
}

async fn rig_flags(adif: &str, flags: HighlightFlags) -> Rig {
    let db = Arc::new(DxccDb::parse(CTY).unwrap());
    let worked = WorkedLog::from_adif(adif, &db);
    let hub = Arc::new(Hub::start(HubConfig { listen: "127.0.0.1:0".parse().unwrap(), forward: vec![] }).await.unwrap());
    let bus = Bus::new(1024);
    let events = bus.subscribe_events();
    let io = EngineIo { events: bus.events(), spectrum: bus.spectrum(), commands: bus.take_command_rx().unwrap() };
    let engine = JtdxEngine::new(Arc::clone(&hub), JtdxConfig::default(), Arc::new(SystemClock)).with_enrichment(db, worked).with_highlight_flags(Arc::new(std::sync::RwLock::new(flags)));
    tokio::spawn(engine.run(io));
    let jtdx = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    // il Rig tiene vivo il bus finche' vive
    std::mem::forget(bus);
    Rig { hub, jtdx, events, last_continent: std::sync::Mutex::new(None) }
}

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../udp-bridge/tests/fixtures").join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn header(kind: u32) -> Writer {
    let mut w = Writer::new();
    w.u32(MAGIC);
    w.u32(2);
    w.u32(kind);
    w.utf8(Some("JTDX"));
    w
}

fn decode_packet(text: &str, time_ms: u32) -> Vec<u8> {
    let mut w = header(2);
    w.bool(true);
    w.u32(time_ms);
    w.i32(-10);
    w.f64(0.2);
    w.u32(1500);
    w.utf8(Some("~"));
    w.utf8(Some(text));
    w.bool(false);
    w.bool(false);
    w.into_bytes()
}

fn qso_packet(call: &str, grid: &str, freq: u64) -> Vec<u8> {
    let mut w = header(5);
    w.i64(2_461_000);
    w.u32(45_015_000);
    w.u8(1);
    w.utf8(Some(call));
    w.utf8(Some(grid));
    w.u64(freq);
    w.utf8(Some("FT8"));
    w.utf8(Some("-08"));
    w.utf8(Some("-12"));
    w.into_bytes()
}

impl Rig {
    async fn send(&self, bytes: &[u8]) {
        self.jtdx.send_to(bytes, self.hub.local_addr()).await.unwrap();
    }

    /// Manda lo Status reale (21,074 MHz = 15m) e aspetta di averlo visto.
    async fn on_15m(&mut self) {
        self.send(&fixture("status-decoding.bin")).await;
        loop {
            match timeout(Duration::from_secs(1), self.events.recv()).await {
                Ok(Ok(Event::Status(_))) => return,
                Ok(Ok(_)) => {}
                _ => panic!("Status non ricevuto"),
            }
        }
    }

    async fn decode(&mut self, text: &str, time_ms: u32) -> (Option<DecodeTag>, Option<String>) {
        self.send(&decode_packet(text, time_ms)).await;
        loop {
            match timeout(Duration::from_secs(1), self.events.recv()).await {
                Ok(Ok(Event::Decode(d))) => {
                    *self.last_continent.lock().unwrap() = d.continent.clone();
                    return (d.tag, d.country);
                }
                Ok(Ok(_)) => {}
                _ => panic!("Decode non ricevuto"),
            }
        }
    }
}

/// Atlantis collegata su 15m (FN42, zona CQ 14); nient'altro.
const ADIF_ATLANTIS_15M: &str = "<CALL:5>AT3AB<BAND:3>15m<GRIDSQUARE:4>FN42<EOR>\n<CALL:5>AT5EF<BAND:3>15m<GRIDSQUARE:4>FN43<EOR>\n";

#[tokio::test]
async fn cq_from_a_never_worked_country_is_new_dxcc_with_country() {
    let mut r = rig(ADIF_ATLANTIS_15M).await;
    r.on_15m().await;
    // Utopia ha le stesse zone CQ e ITU di Atlantis: resta nuovo il paese (la zona CQ verrebbe prima, come in JTDX)
    let (tag, country) = r.decode("CQ UT1XY KK00", 100).await;
    assert_eq!(tag, Some(DecodeTag::NewDxcc));
    assert_eq!(country.as_deref(), Some("Utopia"));
}

#[tokio::test]
async fn same_country_on_another_band_is_new_band() {
    // solo il paese (con «per banda»): con le zone accese la zona CQ verrebbe prima, come in JTDX
    let dxcc = HighlightFlags { dxcc: engine_jtdx::worked::Criterion::new(true, true, false), ..HighlightFlags::none() };
    let mut r = rig_flags("<CALL:5>AT3AB<BAND:3>20m<GRIDSQUARE:4>FN42<EOR>\n", dxcc).await;
    r.on_15m().await;
    let (tag, country) = r.decode("CQ AT9ZZ FN42", 100).await;
    assert_eq!(tag, Some(DecodeTag::NewDxccBand));
    assert_eq!(country.as_deref(), Some("Atlantis"));
}

#[tokio::test]
async fn an_already_worked_station_is_tagged_worked() {
    let mut r = rig(ADIF_ATLANTIS_15M).await;
    r.on_15m().await;
    assert_eq!(r.decode("CQ AT3AB FN42", 100).await.0, Some(DecodeTag::WorkedCall));
}

#[tokio::test]
async fn new_zone_then_new_grid_for_cq_calls() {
    let mut r = rig("<CALL:5>AT3AB<BAND:3>15m<GRIDSQUARE:4>FN42<EOR>\n").await;
    r.on_15m().await;
    // AT5 ha la zona CQ 15, mai collegata
    assert_eq!(r.decode("CQ AT5XX FN42", 100).await.0, Some(DecodeTag::NewZone));
    // stessa zona (14), griglia nuova
    assert_eq!(r.decode("CQ AT9ZZ KK00", 101).await.0, Some(DecodeTag::NewGrid));
}

#[tokio::test]
async fn a_reply_between_other_stations_is_tagged_only_for_dxcc_level_news() {
    let mut r = rig(ADIF_ATLANTIS_15M).await;
    r.on_15m().await;
    assert_eq!(r.decode("BB2BB LM1XY -10", 100).await.0, Some(DecodeTag::NewZone), "zona CQ nuova: viene prima del paese, come in JTDX");
    // nuova griglia, ma e' un messaggio tra altre stazioni: nessuna etichetta
    assert_eq!(r.decode("BB2BB AT9ZZ KK00", 101).await.0, None);
}

#[tokio::test]
async fn a_logged_qso_updates_the_novelty_immediately() {
    let mut r = rig(ADIF_ATLANTIS_15M).await;
    r.on_15m().await;
    assert_eq!(r.decode("CQ LM1XY KK00", 100).await.0, Some(DecodeTag::NewZone), "zona CQ nuova: viene prima del paese");
    r.send(&qso_packet("LM1XY", "KK00", 21_074_000)).await;
    // aspetta l'evento del QSO
    loop {
        match timeout(Duration::from_secs(1), r.events.recv()).await {
            Ok(Ok(Event::QsoLogged(_))) => break,
            Ok(Ok(_)) => {}
            _ => panic!("QSO non ricevuto"),
        }
    }
    assert_eq!(r.decode("CQ LM2ZZ KK00", 101).await.0, Some(DecodeTag::NewPx), "il paese non e' piu' nuovo, il prefisso LM2 si");
}

#[tokio::test]
async fn new_prefix_and_new_call_are_reported_for_cq() {
    let mut r = rig("<CALL:5>AT3AB<BAND:3>15m<GRIDSQUARE:4>FN42<EOR>
").await;
    r.on_15m().await;
    assert_eq!(r.decode("CQ AT9ZZ FN42", 100).await.0, Some(DecodeTag::NewPx));
    assert_eq!(r.decode("CQ AT3CD FN42", 101).await.0, Some(DecodeTag::NewCall));
}

#[tokio::test]
async fn an_empty_worked_log_means_no_novelty_tags_only_cq() {
    let mut r = rig("").await;
    r.on_15m().await;
    let (tag, country) = r.decode("CQ LM1XY KK00", 100).await;
    assert_eq!(tag, Some(DecodeTag::Cq), "senza lavorati tutto sarebbe una novita': non si etichetta");
    assert_eq!(country.as_deref(), Some("Lemuria"), "il paese si mostra comunque");
}

#[tokio::test]
async fn decodes_carry_the_continent_of_the_sender() {
    let mut r = rig(ADIF_ATLANTIS_15M).await;
    r.on_15m().await;
    r.decode("CQ LM1XY KK00", 100).await;
    assert_eq!(r.last_continent.lock().unwrap().as_deref(), Some("NA"));
    r.decode("CQ AT9ZZ FN42", 101).await;
    assert_eq!(r.last_continent.lock().unwrap().as_deref(), Some("EU"));
    r.decode("TNX 73 GL", 102).await;
    assert_eq!(*r.last_continent.lock().unwrap(), None, "senza nominativo non c'e' continente");
}

#[tokio::test]
async fn before_the_first_status_only_the_cq_tag_is_given() {
    let mut r = rig(ADIF_ATLANTIS_15M).await;
    r.send(&decode_packet("CQ LM1XY KK00", 100)).await;
    match timeout(Duration::from_secs(1), r.events.recv()).await {
        Ok(Ok(Event::Decode(d))) => assert_eq!(d.tag, Some(DecodeTag::Cq)),
        other => panic!("{other:?}"),
    }
}
