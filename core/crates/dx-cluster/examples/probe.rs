//! Prova a mano contro un cluster vero: `cargo run -p dx-cluster --example probe -- host porta nominativo`.
use std::{sync::Arc, time::Duration};

use dx_cluster::{ClusterService, Config, ServerCfg};

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (host, port, call) = (args[1].clone(), args[2].parse::<u16>().unwrap_or(7300), args[3].clone());
    let dir = std::env::temp_dir().join("aurora-probe");
    let _ = std::fs::create_dir_all(&dir);
    let svc: Arc<ClusterService> = ClusterService::open(&dir.join("dx-cluster.json"));
    let config = Config { enabled: true, callsign: call, servers: vec![ServerCfg { name: "probe".into(), host, port, enabled: true }], ..Config::default() };
    svc.set_config(config).expect("config");
    for _ in 0..20 {
        tokio::time::sleep(Duration::from_secs(1)).await;
        println!("{:?} spot={}", svc.statuses().first().map(|s| (s.state, s.spots, s.detail.clone())), svc.spots_since(0, 1000).len());
    }
    for s in svc.spots_since(0, 5) {
        println!("{} {} {:?} {:?}", s.time_utc, s.dx_call, s.mode, s.band);
    }
}
