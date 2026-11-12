//! Il menu del tasto destro di WebView2 nelle caselle di testo: restano solo Annulla, Ripeti, Taglia, Copia, Incolla, Elimina e
//! Seleziona tutto. Le altre voci (Emoji, Incolla come testo normale, Direzione di scrittura, Lettura vocale, Altri strumenti...)
//! a una stazione radio non servono. Le voci tenute sono quelle native di WebView2: copia e incolla funzionano sempre.
//! Fuori dalle caselle di testo il menu lo blocca gia' la pagina (`app/src/main.tsx`, ascoltatore `contextmenu`).

/// I nomi (non tradotti) delle voci che restano: per WebView2 sono l'etichetta inglese in «lowerCamelCase».
const KEPT: [&str; 7] = ["undo", "redo", "cut", "copy", "paste", "delete", "selectAll"];

/// Una voce del menu proposto da WebView2: il suo nome e se e' una linea di separazione.
#[cfg_attr(not(windows), allow(dead_code))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Entry {
    pub name: String,
    pub separator: bool,
}

/// Le posizioni delle voci da tenere, in ordine: le voci ammesse e le linee che le separano, senza linee in testa, in coda o doppie.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn kept_indices(entries: &[Entry]) -> Vec<usize> {
    let mut kept: Vec<usize> = Vec::new();
    let mut pending_separator: Option<usize> = None;
    for (i, entry) in entries.iter().enumerate() {
        if entry.separator {
            // una linea serve solo fra due voci tenute
            if !kept.is_empty() && pending_separator.is_none() {
                pending_separator = Some(i);
            }
        } else if KEPT.contains(&entry.name.as_str()) {
            kept.extend(pending_separator.take());
            kept.push(i);
        }
    }
    kept
}

/// Collega il filtro del menu alla webview della finestra (la principale e ognuna di quelle a parte).
#[cfg(windows)]
pub(crate) fn install(window: &tauri::WebviewWindow) {
    let result = window.with_webview(|webview| {
        if let Err(e) = attach(&webview.controller()) {
            tracing::warn!("menu del tasto destro non filtrato: {e}");
        }
    });
    if let Err(e) = result {
        tracing::warn!("menu del tasto destro non filtrato: {e}");
    }
}

/// Su Linux e macOS il menu e' quello del sistema: niente da fare.
#[cfg(not(windows))]
pub(crate) fn install(_window: &tauri::WebviewWindow) {}

#[cfg(windows)]
fn attach(controller: &webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2Controller) -> windows_core::Result<()> {
    use webview2_com::{ContextMenuRequestedEventHandler, Microsoft::Web::WebView2::Win32::ICoreWebView2_11};
    use windows_core::Interface as _;
    // SAFETY: chiamate COM sulla webview appena creata, dal thread della finestra (`with_webview` ci porta li')
    unsafe {
        let webview = controller.CoreWebView2()?.cast::<ICoreWebView2_11>()?;
        let mut token: i64 = 0;
        webview.add_ContextMenuRequested(
            &ContextMenuRequestedEventHandler::create(Box::new(|_, args| {
                let Some(args) = args else { return Ok(()) };
                filter(&args)
            })),
            &mut token,
        )
    }
}

#[cfg(windows)]
fn filter(args: &webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2ContextMenuRequestedEventArgs) -> windows_core::Result<()> {
    use webview2_com::Microsoft::Web::WebView2::Win32::{COREWEBVIEW2_CONTEXT_MENU_ITEM_KIND, COREWEBVIEW2_CONTEXT_MENU_ITEM_KIND_SEPARATOR};
    // SAFETY: l'elenco delle voci appartiene a questo evento e si tocca solo dentro il suo gestore, sul thread della finestra
    unsafe {
        let items = args.MenuItems()?;
        let mut count = 0u32;
        items.Count(&mut count)?;
        let mut entries = Vec::with_capacity(count as usize);
        for i in 0..count {
            let item = items.GetValueAtIndex(i)?;
            let mut name = windows_core::PWSTR::null();
            item.Name(&mut name)?;
            let mut kind = COREWEBVIEW2_CONTEXT_MENU_ITEM_KIND::default();
            item.Kind(&mut kind)?;
            entries.push(Entry { name: webview2_com::take_pwstr(name), separator: kind == COREWEBVIEW2_CONTEXT_MENU_ITEM_KIND_SEPARATOR });
        }
        let kept = kept_indices(&entries);
        if kept.is_empty() {
            // nessuna voce utile: niente menu
            return args.SetHandled(true);
        }
        // dal fondo, cosi' le posizioni delle voci ancora da guardare non cambiano
        for i in (0..count).rev() {
            if !kept.contains(&(i as usize)) {
                items.RemoveValueAtIndex(i)?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(name: &str) -> Entry {
        Entry { name: name.to_string(), separator: false }
    }

    fn line() -> Entry {
        Entry { name: "separator".to_string(), separator: true }
    }

    #[test]
    fn tiene_solo_le_voci_di_modifica() {
        // il menu di una casella di testo come lo propone WebView2
        let entries = vec![
            item("emoji"),
            line(),
            item("undo"),
            item("redo"),
            line(),
            item("cut"),
            item("copy"),
            item("paste"),
            item("pasteAndMatchStyle"),
            item("delete"),
            line(),
            item("selectAll"),
            line(),
            item("writingDirection"),
            item("readAloud"),
            line(),
            item("moreTools"),
            item("inspectElement"),
        ];
        let names: Vec<&str> = kept_indices(&entries).into_iter().map(|i| entries[i].name.as_str()).collect();
        assert_eq!(names, ["undo", "redo", "separator", "cut", "copy", "paste", "delete", "separator", "selectAll"]);
    }

    #[test]
    fn niente_linee_in_testa_in_coda_o_doppie() {
        let entries = vec![line(), item("copy"), line(), item("emoji"), line(), item("paste"), line(), item("other")];
        assert_eq!(kept_indices(&entries), [1, 2, 5]);
    }

    #[test]
    fn nessuna_voce_utile() {
        let entries = vec![item("back"), line(), item("reload"), item("inspectElement")];
        assert!(kept_indices(&entries).is_empty());
    }
}
