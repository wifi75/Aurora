// niente finestra nera di console nel programma installato
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    aurora_desktop_lib::run();
}
