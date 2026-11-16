//! La finestrina «Aggiornamento in corso» della copia di aiuto ([`crate::installer::apply_update`]): mentre Aurora e' chiusa e
//! l'installatore lavora in silenzio, dice all'utente cosa succede (chiusura, installazione, riapertura) con una barra che si muove.
//! Sta sempre davanti alle altre finestre e si chiude da sola alla fine. Finestra Win32 disegnata a mano: la copia di aiuto non apre
//! WebView2 ne' Tauri.

use std::sync::Mutex;
use std::sync::atomic::{AtomicIsize, Ordering};

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    BeginPaint, CreateFontW, CreateSolidBrush, DT_CENTER, DT_END_ELLIPSIS, DT_SINGLELINE, DT_VCENTER, DT_WORDBREAK, DeleteObject, DrawTextW, EndPaint,
    FillRect, HFONT, InvalidateRect, PAINTSTRUCT, SelectObject, SetBkMode, SetTextColor, TRANSPARENT,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetClientRect, GetMessageW, GetSystemMetrics, IDC_ARROW, LoadCursorW, MSG,
    PostMessageW, PostQuitMessage, RegisterClassW, SM_CXSCREEN, SM_CYSCREEN, SetTimer, TranslateMessage, WM_APP, WM_CLOSE, WM_DESTROY, WM_PAINT,
    WM_TIMER, WNDCLASSW, WS_BORDER, WS_EX_APPWINDOW, WS_EX_TOPMOST, WS_POPUP, WS_VISIBLE,
};

const WIDTH: i32 = 460;
const HEIGHT: i32 = 170;
/// Messaggio della finestra: il testo del passo e' cambiato.
const WM_STEP: u32 = WM_APP + 1;

/// Titolo, passo in corso e nota in fondo; la posizione della barra che si muove.
struct State {
    title: String,
    step: String,
    note: String,
    tick: u32,
}

static STATE: Mutex<State> = Mutex::new(State { title: String::new(), step: String::new(), note: String::new(), tick: 0 });
static HWND_NOW: AtomicIsize = AtomicIsize::new(0);

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

const fn rgb(r: u8, g: u8, b: u8) -> u32 {
    r as u32 | (g as u32) << 8 | (b as u32) << 16
}

/// La finestra aperta: si chiude quando il valore si butta.
pub struct UpdateWindow;

impl UpdateWindow {
    /// Apre la finestra (in un thread suo, con il suo ciclo dei messaggi) con titolo, primo passo e nota.
    pub fn show(title: &str, step: &str, note: &str) -> UpdateWindow {
        if let Ok(mut s) = STATE.lock() {
            s.title = title.to_string();
            s.step = step.to_string();
            s.note = note.to_string();
        }
        let spawned = std::thread::Builder::new().name("aurora-aggiorna-finestra".into()).spawn(run);
        if let Err(e) = spawned {
            tracing::warn!("finestra dell'aggiornamento non aperta: {e}");
        }
        UpdateWindow
    }

    /// Cambia il passo mostrato («Installo la versione nuova…»).
    pub fn step(&self, text: &str) {
        if let Ok(mut s) = STATE.lock() {
            s.step = text.to_string();
        }
        let hwnd = HWND_NOW.load(Ordering::SeqCst);
        if hwnd != 0 {
            // SAFETY: la finestra e' del thread della finestra; PostMessageW e' sicuro da qualunque thread
            unsafe { PostMessageW(hwnd as HWND, WM_STEP, 0, 0) };
        }
    }
}

impl Drop for UpdateWindow {
    fn drop(&mut self) {
        // la finestra potrebbe non essere ancora nata: si aspetta un attimo
        for _ in 0..20 {
            let hwnd = HWND_NOW.load(Ordering::SeqCst);
            if hwnd != 0 {
                // SAFETY: come sopra
                unsafe { PostMessageW(hwnd as HWND, WM_CLOSE, 0, 0) };
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }
}

/// Il thread della finestra: la crea al centro dello schermo, sempre davanti, e tiene il ciclo dei messaggi fino alla chiusura.
fn run() {
    let class = wide("AuroraAggiornamento");
    let title = STATE.lock().map(|s| s.title.clone()).unwrap_or_default();
    let title_w = wide(&title);
    // SAFETY: chiamate Win32 con strutture azzerate e stringhe che vivono per tutta la funzione
    unsafe {
        let instance = GetModuleHandleW(std::ptr::null());
        let wc = WNDCLASSW {
            lpfnWndProc: Some(proc_),
            hInstance: instance,
            lpszClassName: class.as_ptr(),
            hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
            ..std::mem::zeroed()
        };
        RegisterClassW(&wc);
        let x = (GetSystemMetrics(SM_CXSCREEN) - WIDTH) / 2;
        let y = (GetSystemMetrics(SM_CYSCREEN) - HEIGHT) / 2;
        let hwnd = CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_APPWINDOW,
            class.as_ptr(),
            title_w.as_ptr(),
            WS_POPUP | WS_VISIBLE | WS_BORDER,
            x,
            y,
            WIDTH,
            HEIGHT,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            instance,
            std::ptr::null(),
        );
        if hwnd.is_null() {
            return;
        }
        HWND_NOW.store(hwnd as isize, Ordering::SeqCst);
        SetTimer(hwnd, 1, 40, None);
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        HWND_NOW.store(0, Ordering::SeqCst);
    }
}

fn font(height: i32, weight: i32) -> HFONT {
    let face = wide("Segoe UI");
    // SAFETY: il nome del carattere vive per tutta la chiamata
    unsafe { CreateFontW(height, 0, 0, 0, weight, 0, 0, 0, 1, 0, 0, 5, 0, face.as_ptr()) }
}

unsafe extern "system" fn proc_(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // SAFETY: tutte le chiamate usano la finestra ricevuta da Windows e strutture locali
    unsafe {
        match msg {
            WM_TIMER => {
                if let Ok(mut s) = STATE.lock() {
                    s.tick = s.tick.wrapping_add(1);
                }
                // si ridisegna solo la barra: niente sfarfallio del testo
                let mut rc: RECT = std::mem::zeroed();
                GetClientRect(hwnd, &mut rc);
                let bar = RECT { left: 0, top: 88, right: rc.right, bottom: 102 };
                InvalidateRect(hwnd, &bar, 0);
                0
            }
            WM_STEP => {
                InvalidateRect(hwnd, std::ptr::null(), 0);
                0
            }
            WM_PAINT => {
                paint(hwnd);
                0
            }
            WM_CLOSE => {
                DestroyWindow(hwnd);
                0
            }
            WM_DESTROY => {
                PostQuitMessage(0);
                0
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

/// Sfondo scuro, «Aurora» in alto, il passo al centro, la barra che scorre e la nota in fondo.
unsafe fn paint(hwnd: HWND) {
    let (title, step, note, tick) = match STATE.lock() {
        Ok(s) => (s.title.clone(), s.step.clone(), s.note.clone(), s.tick),
        Err(_) => return,
    };
    // SAFETY: BeginPaint/EndPaint in coppia; ogni oggetto GDI creato si cancella
    unsafe {
        let mut ps: PAINTSTRUCT = std::mem::zeroed();
        let hdc = BeginPaint(hwnd, &mut ps);
        let mut rc: RECT = std::mem::zeroed();
        GetClientRect(hwnd, &mut rc);
        let bg = CreateSolidBrush(rgb(0x14, 0x18, 0x24));
        FillRect(hdc, &rc, bg);
        DeleteObject(bg);
        SetBkMode(hdc, TRANSPARENT as i32);

        let big = font(-22, 600);
        let mid = font(-16, 400);
        let small = font(-13, 400);

        let old = SelectObject(hdc, big);
        SetTextColor(hdc, rgb(0x8e, 0x7b, 0xff));
        let mut r = RECT { left: 20, top: 16, right: rc.right - 20, bottom: 46 };
        let t = wide(&title);
        DrawTextW(hdc, t.as_ptr(), -1, &mut r, DT_CENTER | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS);

        SelectObject(hdc, mid);
        SetTextColor(hdc, rgb(0xe8, 0xea, 0xf0));
        let mut r = RECT { left: 20, top: 54, right: rc.right - 20, bottom: 80 };
        let t = wide(&step);
        DrawTextW(hdc, t.as_ptr(), -1, &mut r, DT_CENTER | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS);

        // barra: binario chiaro e un tratto che scorre da sinistra a destra
        let track = RECT { left: 40, top: 92, right: rc.right - 40, bottom: 98 };
        let tb = CreateSolidBrush(rgb(0x2a, 0x30, 0x40));
        FillRect(hdc, &track, tb);
        DeleteObject(tb);
        let span = track.right - track.left;
        let seg = span / 4;
        let pos = ((tick as i32 * 6) % (span + seg)) - seg;
        let fill = RECT { left: (track.left + pos).max(track.left), top: track.top, right: (track.left + pos + seg).min(track.right), bottom: track.bottom };
        if fill.right > fill.left {
            let fb = CreateSolidBrush(rgb(0x8e, 0x7b, 0xff));
            FillRect(hdc, &fill, fb);
            DeleteObject(fb);
        }

        SelectObject(hdc, small);
        SetTextColor(hdc, rgb(0x9a, 0xa0, 0xb0));
        let mut r = RECT { left: 24, top: 110, right: rc.right - 24, bottom: rc.bottom - 10 };
        let t = wide(&note);
        DrawTextW(hdc, t.as_ptr(), -1, &mut r, DT_CENTER | DT_WORDBREAK);

        SelectObject(hdc, old);
        DeleteObject(big);
        DeleteObject(mid);
        DeleteObject(small);
        EndPaint(hwnd, &ps);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Prova a vista: `cargo test --lib update_window -- --ignored` apre la finestrina per 8 secondi.
    #[test]
    #[ignore]
    fn la_finestrina_si_vede() {
        let w = UpdateWindow::show("Aggiornamento di Aurora", "Chiudo Aurora…", "Non spegnere il PC: Aurora si riapre da sola fra poco.");
        std::thread::sleep(std::time::Duration::from_secs(3));
        w.step("Installo la versione nuova…");
        std::thread::sleep(std::time::Duration::from_secs(5));
        drop(w);
        std::thread::sleep(std::time::Duration::from_millis(300));
        assert_eq!(HWND_NOW.load(Ordering::SeqCst), 0, "la finestra si e' chiusa");
    }
}
