//! La finestra «Salva con nome» di Windows per la pagina di esportazione del log: la pagina e' una pagina web dentro il programma, dove un
//! link di download non chiede dove salvare. Il core la apre qui, nel processo del programma, e scrive il file che l'operatore sceglie.

use std::{path::PathBuf, sync::Arc};

use tauri::{AppHandle, Manager};

/// La finestra, legata alla finestra principale di Aurora (resta sopra e la blocca finche' non si sceglie). Proponendo `name`, con il
/// filtro dei file ADIF; la cartella iniziale e' l'ultima usata (la ricorda Windows). `None` = annullato.
pub fn dialog(app: AppHandle) -> remote_api::SaveFileDialog {
    Arc::new(move |name: &str| -> Option<PathBuf> {
        // il filtro segue il nome proposto: Cabrillo (`.log`) o ADIF
        let mut picker = rfd::FileDialog::new().set_file_name(name);
        picker = if name.ends_with(".log") {
            picker.add_filter("Cabrillo (*.log)", &["log"]).add_filter("Cabrillo (*.cbr)", &["cbr"])
        } else {
            picker.add_filter("ADIF (*.adi)", &["adi"]).add_filter("ADIF (*.adif)", &["adif"])
        };
        if let Some(window) = app.get_webview_window("main") {
            picker = picker.set_parent(&window);
        }
        // la finestra principale resta ferma finche' l'operatore sceglie: non e' un blocco
        let _modal = crate::crash::modal("finestra Salva con nome");
        picker.save_file()
    })
}

/// La finestra «Apri file» per scegliere il database di Log4OM2 (filtri `*.SQLite`, `*.db`), legata alla finestra principale.
/// `None` = annullata.
pub fn open_dialog(app: AppHandle) -> remote_api::OpenFileDialog {
    Arc::new(move |title: &str| -> Option<PathBuf> {
        let mut picker = rfd::FileDialog::new()
            .set_title(title)
            .add_filter("Log4OM (*.SQLite, *.db)", &["SQLite", "sqlite", "db"])
            .add_filter("*.*", &["*"]);
        if let Some(window) = app.get_webview_window("main") {
            picker = picker.set_parent(&window);
        }
        let _modal = crate::crash::modal("finestra Apri file");
        picker.pick_file()
    })
}
