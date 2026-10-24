//! Prova con JTDX aperto: quanto costa un comando con la sessione fissa. Non preme nulla: riporta un interruttore al valore che ha gia'.
//! `cargo test -p rig-control --test live_session -- --ignored --nocapture`

use std::time::Instant;

use rig_control::{Rig, WindowsRig};

#[test]
#[ignore = "richiede JTDX aperto"]
fn a_command_in_the_open_session_is_much_faster_than_the_first() {
    let rig = WindowsRig;
    let started = Instant::now();
    let reading = rig.controls().expect("lettura degli interruttori");
    println!("prima lettura (apre la sessione): {} ms", started.elapsed().as_millis());
    let (id, on) = reading.iter().find(|(id, _)| id == "monitor").cloned().expect("interruttore Monitora");

    let mut times = Vec::new();
    for _ in 0..5 {
        let t = Instant::now();
        rig.set_control(&id, Some(on)).expect("comando");
        times.push(t.elapsed().as_millis());
    }
    println!("comandi che non cambiano nulla, in ms: {times:?}");
    let t = Instant::now();
    rig.controls().expect("lettura");
    println!("lettura a sessione aperta: {} ms", t.elapsed().as_millis());
}
