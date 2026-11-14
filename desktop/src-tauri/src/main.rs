// niente finestra nera di console nel programma installato
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // `--applica-aggiornamento`: la copia di aiuto che installa l'aggiornamento mentre Aurora e' chiusa (nessuna finestra, nessun motore)
    if aurora_desktop_lib::apply_update_if_asked() {
        return;
    }
    // `--agente`: solo l'icona nel tray con gli avvisi (aggiornamenti, messaggi), senza finestra e senza motore
    if std::env::args().any(|a| a == "--agente") {
        aurora_desktop_lib::agent::run();
    } else {
        aurora_desktop_lib::run();
    }
}
