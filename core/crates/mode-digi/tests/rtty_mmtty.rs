//! Il ricevitore RTTY di MMTTY 1.70L come secondo arbitro: prima di tutto deve capire il nostro modulatore con ogni profilo.
//!
//! Solo con la feature (serve il sottomodulo `terze-parti/mmtty` per rigenerare l'estratto e un compilatore C++):
//! `cargo test -j 12 -p mode-digi --features mmtty-oracle --test rtty_mmtty -- --test-threads=2 --nocapture`
#![cfg(feature = "mmtty-oracle")]

use mode_digi::mmtty_oracle::{MmttyProfile, MmttyRtty};
use mode_digi::rtty::{self, RttyParams};

#[test]
fn mmtty_decodifica_il_nostro_segnale_pulito_con_ogni_profilo() {
    for profile in MmttyProfile::ALL {
        let shift = if profile == MmttyProfile::Pll23 { 23.0 } else { 170.0 };
        let p = RttyParams { centre_hz: 1500.0, shift_hz: shift, baud: 45.45, reverse: false };
        let mut audio = vec![0.0; 6000];
        audio.extend(rtty::modulate(b"RYRYRY CQ CQ DE IU3ABC IU3ABC 599 73 K", p, 0.5));
        audio.extend(vec![0.0; 6000]);
        let mut rx = MmttyRtty::new(profile, 45.45, f64::from(shift), 1500.0, false);
        let r = String::from_utf8_lossy(&rx.process(&audio)).into_owned();
        println!("{:<10} {r:?}", profile.name());
        assert!(r.contains("CQ CQ DE IU3ABC IU3ABC 599 73 K"), "MMTTY {} ha ricevuto: {r:?}", profile.name());
    }
}
