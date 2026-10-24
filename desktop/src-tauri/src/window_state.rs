//! Dimensione e posizione della finestra principale: si salvano chiudendo e si ripristinano all'avvio.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

const FILE: &str = "finestra.json";
pub const MIN_WIDTH: u32 = 360;
pub const MIN_HEIGHT: u32 = 560;

/// In pixel fisici, come li danno `outer_size` e `outer_position`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowState {
    pub width: u32,
    pub height: u32,
    pub x: i32,
    pub y: i32,
    pub maximized: bool,
}

fn path(dir: &Path) -> PathBuf {
    dir.join(FILE)
}

pub fn load(dir: &Path) -> Option<WindowState> {
    let text = std::fs::read_to_string(path(dir)).ok()?;
    serde_json::from_str(&text).ok()
}

pub fn save(dir: &Path, state: &WindowState) -> std::io::Result<()> {
    let text = serde_json::to_string(state).map_err(std::io::Error::other)?;
    std::fs::write(path(dir), text)
}

/// Un monitor come area (x, y, larghezza, altezza) in pixel fisici.
pub type Monitor = (i32, i32, u32, u32);

/// Lo stato e' ragionevole: dimensioni non minuscole e almeno una parte della barra del titolo su un monitor che c'e' ancora
/// (un monitor staccato non deve lasciare la finestra fuori dallo schermo).
pub fn is_usable(state: &WindowState, monitors: &[Monitor]) -> bool {
    if state.width < MIN_WIDTH || state.height < MIN_HEIGHT || state.width > 20_000 || state.height > 20_000 {
        return false;
    }
    let title_x = i64::from(state.x) + i64::from(state.width) / 2;
    let title_y = i64::from(state.y) + 16;
    monitors.iter().any(|&(mx, my, mw, mh)| {
        title_x >= i64::from(mx) && title_x < i64::from(mx) + i64::from(mw) && title_y >= i64::from(my) && title_y < i64::from(my) + i64::from(mh)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCREEN: [Monitor; 1] = [(0, 0, 1920, 1080)];

    fn state(x: i32, y: i32, w: u32, h: u32) -> WindowState {
        WindowState { width: w, height: h, x, y, maximized: false }
    }

    #[test]
    fn it_round_trips_through_the_file() {
        let dir = std::env::temp_dir().join(format!("aurora-finestra-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("cartella di prova");
        assert_eq!(load(&dir), None, "senza file non c'e' nulla");
        let s = WindowState { maximized: true, ..state(10, 20, 1300, 800) };
        save(&dir, &s).expect("salvataggio");
        assert_eq!(load(&dir), Some(s));
        std::fs::write(dir.join(FILE), "non e' json").expect("file rotto");
        assert_eq!(load(&dir), None, "un file rotto si ignora");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_window_on_a_screen_is_usable() {
        assert!(is_usable(&state(100, 50, 1360, 860), &SCREEN));
        assert!(is_usable(&state(-300, 0, 1360, 860), &SCREEN), "mezza fuori a sinistra ma il titolo e' sullo schermo");
    }

    #[test]
    fn a_window_left_on_a_missing_monitor_or_too_small_is_not() {
        assert!(!is_usable(&state(2500, 100, 1360, 860), &SCREEN), "il secondo monitor non c'e' piu'");
        assert!(!is_usable(&state(100, 2000, 1360, 860), &SCREEN));
        assert!(!is_usable(&state(100, 50, 100, 100), &SCREEN));
        assert!(!is_usable(&state(100, 50, 1360, 860), &[]), "nessun monitor noto");
    }

    #[test]
    fn two_monitors_both_count() {
        let two = [(0, 0, 1920, 1080), (1920, 0, 1920, 1080)];
        assert!(is_usable(&state(2000, 100, 1360, 860), &two));
    }
}
