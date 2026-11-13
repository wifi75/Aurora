//! Peso degli eventi verso l'interfaccia (vedi `docs/prestazioni-motore.md`): byte e microsecondi di serializzazione JSON degli eventi
//! piu' frequenti, come li manda il WebSocket (`remote-api/src/ws.rs`, un `serde_json::to_string` per evento e per client).
//! `cargo test -p aurora-core --release --test prestazioni -- --ignored --nocapture`

use std::time::Instant;

use aurora_core::{AudioLevel, DecodedMessage, DigiEvent, DigiSignal, DigiSignalCall, Event, LevelState, SstvEvent, StationStatus};

fn misura(nome: &str, per_secondo: f64, ev: &Event) {
    const N: u32 = 20_000;
    let mut bytes = 0usize;
    let t0 = Instant::now();
    for _ in 0..N {
        bytes = serde_json::to_string(ev).map_or(0, |s| s.len());
    }
    let us = t0.elapsed().as_secs_f64() * 1e6 / f64::from(N);
    println!("{nome:<34} {bytes:>7} byte  {us:>8.2} us  ~{per_secondo:>5.1}/s  -> {:>8.1} kB/s, {:.3} ms CPU/s per client", bytes as f64 * per_secondo / 1000.0, us * per_secondo / 1000.0);
}

#[test]
#[ignore]
fn peso_degli_eventi() {
    let decode = Event::Decode(DecodedMessage {
        id: 1_000_000_123,
        time_ms: 43_215_000,
        snr: -12,
        delta_time: 0.3,
        freq_hz: 1234,
        text: "IU3ABC K1ABC FN42".into(),
        country: Some("United States".into()),
        continent: Some("NA".into()),
        lotw_last_upload: Some("2026-10-01".into()),
        ..DecodedMessage::default()
    });
    // una banda FT8 affollata: 40 messaggi in un periodo di 15 s
    misura("Decode (FT8, 40 per periodo)", 40.0 / 15.0, &decode);
    misura("AudioLevel (4 al secondo)", 4.0, &Event::AudioLevel(AudioLevel { peak: 0.31, rms: 0.08, clipped: 0.0, state: LevelState::Good }));
    misura("Status (vuoto)", 1.0, &Event::Status(StationStatus::default()));
    let signal = |i: u32| DigiSignal {
        id: i,
        freq_hz: 300.0 + 80.0 * i as f32,
        wpm: Some(22.0),
        snr_db: 6.5,
        text: "CQ CQ DE IU3ABC IU3ABC K   CQ CQ DE IU3ABC IU3ABC K   CQ DE IU3".into(),
        unsure: vec![3, 17],
        calls: vec![DigiSignalCall { call: "IU3ABC".into(), role: "cq".into(), verified: true, ..DigiSignalCall::default() }],
        cq: true,
        report: None,
    };
    for n in [10u32, 30] {
        let band = Event::Digi(DigiEvent { kind: "band".into(), mode: Some("cw".into()), signals: Some((0..n).map(signal).collect()), ..DigiEvent::default() });
        misura(&format!("Digi band ({n} segnali, 2 al s)"), 2.0, &band);
    }
    misura("Digi rx_text", 5.0, &Event::Digi(DigiEvent { kind: "rx_text".into(), text: Some("cq de iu3abc".into()), mode: Some("psk31".into()), ..DigiEvent::default() }));
    // Robot36: 320 x 240, un gruppo di 2 righe ogni 0,3 s circa, in base64
    let rgb = "A".repeat((320 * 3 * 2_usize).div_ceil(3) * 4);
    misura(
        "Sstv rows (Robot36, 2 righe)",
        3.3,
        &Event::Sstv(SstvEvent { kind: "rows".into(), first_row: Some(10), rows: Some(2), width: Some(320), height: Some(240), rgb: Some(rgb), ..SstvEvent::default() }),
    );
    println!("riga di spettro (binaria, 15 al secondo): {} byte -> {:.1} kB/s per client", 17 + 6827, (17.0 + 6827.0) * 15.0 / 1000.0);
}
