// niente finestra nera di console nel programma installato
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // `--agente`: solo l'icona nel tray con gli avvisi (aggiornamenti, messaggi), senza finestra e senza motore
    if std::env::args().any(|a| a == "--agente") {
        aurora_desktop_lib::agent::run();
    } else {
        aurora_desktop_lib::run();
    }
}
