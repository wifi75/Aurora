mod common;

use std::sync::{Arc, Mutex};

use axum::http::{Method, StatusCode};
use common::*;
use remote_api::{
    audio_api::{AudioControl, AudioDevice, AudioDevices, AudioOutcome, AudioPart, PartStatus},
    engine_settings::{AudioValues, EngineSettingsStore, Feature},
    AppState,
};

/// Un gestore finto: l'ingresso «Rotto» non si apre e resta quello di prima; l'uscita «Occupata» e' rimandata alla fine della trasmissione.
struct FakeAudio {
    input: Mutex<String>,
    applied: Mutex<Vec<AudioValues>>,
}

impl AudioControl for FakeAudio {
    fn devices(&self) -> AudioDevices {
        AudioDevices {
            inputs: vec![AudioDevice { name: "Microfono USB".into(), channels: 2, is_default: true }, AudioDevice { name: "Line In".into(), channels: 1, is_default: false }],
            outputs: vec![AudioDevice { name: "Altoparlanti USB".into(), channels: 2, is_default: true }],
            input_in_use: Some(self.input.lock().unwrap().clone()),
            output_in_use: Some("Altoparlanti USB".into()),
            output_pending: None,
            default_save_dir: "D:/dati/save".into(),
        }
    }

    fn apply(&self, wanted: &AudioValues) -> AudioOutcome {
        self.applied.lock().unwrap().push(wanted.clone());
        let mut current = self.input.lock().unwrap();
        let input = if wanted.input == "Rotto" {
            AudioPart::failed(Some(current.clone()), true, "ingresso audio non aperto")
        } else {
            if !wanted.input.is_empty() {
                *current = wanted.input.clone();
            }
            AudioPart::new(PartStatus::Applied, Some(current.clone()))
        };
        let output = if wanted.output == "Occupata" { AudioPart::new(PartStatus::Deferred, Some("Altoparlanti USB".into())) } else { AudioPart::new(PartStatus::Applied, Some(wanted.output.clone())) };
        AudioOutcome { input, output, save: AudioPart::new(PartStatus::Unchanged, None) }
    }
}

fn with_audio(name: &str) -> (AppState, Arc<EngineSettingsStore>, Arc<FakeAudio>, std::path::PathBuf) {
    let dir = std::env::temp_dir().join(format!("aurora-route-audio-{name}-{}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    let path = dir.join("settings-motore.json");
    let store = Arc::new(EngineSettingsStore::open(&path, remote_api::engine_settings::defaults));
    store.set_feature(Feature::AudioControl, true);
    let fake = Arc::new(FakeAudio { input: Mutex::new("Microfono USB".into()), applied: Mutex::new(Vec::new()) });
    (state().with_engine_settings(Arc::clone(&store)).with_audio(fake.clone()), store, fake, path)
}

fn put(cookie: &str, body: &str) -> Req {
    req(Method::PUT, "/api/v1/audio/settings").cookie(cookie).json(body)
}

#[tokio::test]
async fn needs_a_paired_device_and_the_audio_manager() {
    let s = state();
    status_is(&call(&router(&s), req(Method::GET, "/api/v1/audio/devices")).await, StatusCode::UNAUTHORIZED);
    let cookie = cookie_for(&s);
    let resp = call(&router(&s), req(Method::GET, "/api/v1/audio/devices").cookie(&cookie)).await;
    status_is(&resp, StatusCode::NOT_FOUND);
    assert_eq!(body_json(resp).await["error"], "audio_unavailable");
    let resp = call(&router(&s), put(&cookie, r#"{"input":"x"}"#)).await;
    status_is(&resp, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn lists_the_devices_with_channels_default_and_the_chosen_values() {
    let (s, _, _, _) = with_audio("list");
    let cookie = cookie_for(&s);
    let resp = call(&router(&s), req(Method::GET, "/api/v1/audio/devices").cookie(&cookie)).await;
    status_is(&resp, StatusCode::OK);
    let doc = body_json(resp).await;
    assert_eq!(doc["inputs"][0], serde_json::json!({ "name": "Microfono USB", "channels": 2, "is_default": true }));
    assert_eq!(doc["inputs"][1]["channels"], 1);
    assert_eq!(doc["input_in_use"], "Microfono USB");
    assert_eq!(doc["output_in_use"], "Altoparlanti USB");
    assert_eq!(doc["default_save_dir"], "D:/dati/save");
    assert_eq!(doc["system_default"], "Default");
    assert_eq!(doc["values"]["input_channel"], "Mono");
    assert_eq!(doc["values"]["save_mode"], "0");
}

#[tokio::test]
async fn a_change_is_applied_then_saved_and_survives_a_restart() {
    let (s, store, fake, path) = with_audio("put");
    let cookie = cookie_for(&s);
    let body = r#"{"input":"Line In","output":"Altoparlanti USB","input_channel":"Left","output_channel":"Both","save_dir":"D:/audio/ft8","save_mode":1}"#;
    let resp = call(&router(&s), put(&cookie, body).via_loopback()).await;
    status_is(&resp, StatusCode::OK);
    let doc = body_json(resp).await;
    assert_eq!(doc["ok"], true);
    assert_eq!(doc["input"]["status"], "applied");
    assert_eq!(doc["values"]["input"], "Line In");
    let seen = fake.applied.lock().unwrap().last().cloned().unwrap();
    assert_eq!((seen.input.as_str(), seen.input_channel.as_str(), seen.output_channel.as_str(), seen.save_mode.as_str()), ("Line In", "Left", "Both", "1"));
    let again = EngineSettingsStore::open(&path, remote_api::engine_settings::defaults);
    assert_eq!(again.audio(), store.audio());
    assert_eq!(again.audio().save_dir, "D:/audio/ft8");
    assert_eq!(again.audio().output_channel, "Both");
}

#[tokio::test]
async fn a_device_that_does_not_open_is_not_saved_and_the_rest_is() {
    let (s, store, _, _) = with_audio("restore");
    let cookie = cookie_for(&s);
    let resp = call(&router(&s), put(&cookie, r#"{"input":"Rotto","input_channel":"Right","output":"Occupata","output_channel":"Left"}"#)).await;
    status_is(&resp, StatusCode::OK);
    let doc = body_json(resp).await;
    assert_eq!(doc["ok"], false);
    assert_eq!(doc["input"]["status"], "failed");
    assert_eq!(doc["input"]["restored"], true);
    assert_eq!(doc["input"]["in_use"], "Microfono USB");
    assert_eq!(doc["output"]["status"], "deferred");
    let audio = store.audio();
    assert_eq!((audio.input.as_str(), audio.input_channel.as_str()), ("", "Mono"), "l'ingresso fallito non cambia il file");
    assert_eq!((audio.output.as_str(), audio.output_channel.as_str()), ("Occupata", "Left"), "l'uscita riuscita si salva");
}

#[tokio::test]
async fn refuses_bad_values_empty_bodies_and_remote_peers_without_touching_the_devices() {
    let (s, store, fake, _) = with_audio("refuse");
    let cookie = cookie_for(&s);
    let before = store.audio();
    for body in [r#"{}"#, r#"{"input_channel":"Both"}"#, r#"{"output_channel":"Centro"}"#, r#"{"save_mode":3}"#, r#"{"save_dir":"D:/a|b"}"#, r#"{"input":"a\nb"}"#] {
        let resp = call(&router(&s), put(&cookie, body)).await;
        status_is(&resp, StatusCode::BAD_REQUEST);
        assert_eq!(body_json(resp).await["error"], "bad_audio_setting", "{body}");
    }
    let mut outside = put(&cookie, r#"{"input":"Line In"}"#);
    outside.peer = axum::extract::ConnectInfo("8.8.8.8:50000".parse().unwrap());
    let resp = call(&router(&s), outside).await;
    status_is(&resp, StatusCode::FORBIDDEN);
    assert_eq!(body_json(resp).await["error"], "local_only");
    assert_eq!(store.audio(), before);
    assert!(fake.applied.lock().unwrap().is_empty(), "i rifiuti non aprono niente");
}
