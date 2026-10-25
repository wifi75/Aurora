//! PTT sulla linea RTS o DTR di una porta seriale (il modo di JTDX «PTT = RTS» su una COM): si apre la porta, si alza o si abbassa la linea e basta.
//! La linea si abbassa sempre alla chiusura (Drop) e all'apertura, cosi' una radio non resta in trasmissione per un errore.

use std::sync::atomic::{AtomicBool, Ordering};

use crate::{error::TxError, ptt::Ptt};

/// Quale linea della seriale comanda il PTT.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Line {
    Rts,
    Dtr,
}

/// Il nome della porta nel formato che vuole Windows per le COM oltre la 9 (`\\.\COM12`); un nome come `COM9` va bene anche cosi'.
pub fn device_path(port: &str) -> Result<String, TxError> {
    let name = port.trim().trim_start_matches(r"\\.\");
    let digits = name.get(3..).unwrap_or("");
    let valid = name.len() >= 4 && name[..3].eq_ignore_ascii_case("COM") && digits.chars().all(|c| c.is_ascii_digit()) && digits.len() <= 3;
    if valid { Ok(format!(r"\\.\{}", name.to_ascii_uppercase())) } else { Err(TxError::Ptt(format!("porta seriale non valida: {port:?}"))) }
}

#[cfg(windows)]
mod sys {
    use windows::{
        Win32::{
            Devices::Communication::{CLRDTR, CLRRTS, EscapeCommFunction, SETDTR, SETRTS},
            Foundation::{CloseHandle, GENERIC_READ, GENERIC_WRITE, HANDLE},
            Storage::FileSystem::{CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_NONE, OPEN_EXISTING},
        },
        core::PCWSTR,
    };

    use super::Line;
    use crate::error::TxError;

    pub struct Port(HANDLE);

    // la maniglia della porta si usa da un thread alla volta (il pianificatore); si puo' spostare tra thread
    unsafe impl Send for Port {}
    unsafe impl Sync for Port {}

    impl Port {
        pub fn open(path: &str) -> Result<Port, TxError> {
            let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
            // SAFETY: `wide` termina con zero e vive per tutta la chiamata.
            let handle = unsafe { CreateFileW(PCWSTR(wide.as_ptr()), (GENERIC_READ | GENERIC_WRITE).0, FILE_SHARE_NONE, None, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, None) }
                .map_err(|e| TxError::Ptt(format!("porta {path} non apribile (occupata o assente): {e}")))?;
            Ok(Port(handle))
        }

        pub fn set(&self, line: Line, on: bool) -> Result<(), TxError> {
            let function = match (line, on) {
                (Line::Rts, true) => SETRTS,
                (Line::Rts, false) => CLRRTS,
                (Line::Dtr, true) => SETDTR,
                (Line::Dtr, false) => CLRDTR,
            };
            // SAFETY: la maniglia e' valida finche' `Port` esiste.
            unsafe { EscapeCommFunction(self.0, function) }.map_err(|e| TxError::Ptt(format!("linea seriale non comandabile: {e}")))
        }
    }

    impl Drop for Port {
        fn drop(&mut self) {
            // SAFETY: la maniglia e' stata aperta da `open` e si chiude una volta sola.
            let _ = unsafe { CloseHandle(self.0) };
        }
    }
}

/// PTT sulla linea di una porta seriale.
pub struct SerialLinePtt {
    #[cfg(windows)]
    port: sys::Port,
    line: Line,
    on: AtomicBool,
}

impl SerialLinePtt {
    /// Apre la porta e abbassa subito la linea.
    #[cfg(windows)]
    pub fn open(port: &str, line: Line) -> Result<SerialLinePtt, TxError> {
        let path = device_path(port)?;
        let port = sys::Port::open(&path)?;
        port.set(line, false)?;
        Ok(SerialLinePtt { port, line, on: AtomicBool::new(false) })
    }

    #[cfg(not(windows))]
    pub fn open(_port: &str, _line: Line) -> Result<SerialLinePtt, TxError> {
        Err(TxError::Ptt("il PTT sulla seriale e' disponibile solo su Windows".into()))
    }
}

impl Ptt for SerialLinePtt {
    #[cfg(windows)]
    fn set(&self, on: bool) -> Result<(), TxError> {
        self.port.set(self.line, on)?;
        self.on.store(on, Ordering::SeqCst);
        Ok(())
    }

    #[cfg(not(windows))]
    fn set(&self, _on: bool) -> Result<(), TxError> {
        Err(TxError::Ptt("il PTT sulla seriale e' disponibile solo su Windows".into()))
    }

    fn is_on(&self) -> bool {
        self.on.load(Ordering::SeqCst)
    }
}

impl Drop for SerialLinePtt {
    fn drop(&mut self) {
        // la linea si abbassa sempre alla chiusura; la porta si chiude subito dopo (Drop di `Port`)
        #[cfg(windows)]
        let _ = self.port.set(self.line, false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn com_names_become_windows_device_paths() {
        assert_eq!(device_path("COM9").as_deref().ok(), Some(r"\\.\COM9"));
        assert_eq!(device_path("com12").as_deref().ok(), Some(r"\\.\COM12"));
        assert_eq!(device_path(r"\\.\COM3").as_deref().ok(), Some(r"\\.\COM3"));
    }

    #[test]
    fn anything_else_is_refused() {
        for bad in ["", "COM", "COMX", "LPT1", "COM9;calc", r"C:\file", "COM1234", "COM 9"] {
            assert!(device_path(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn an_absent_port_is_an_error_not_a_hang() {
        assert!(SerialLinePtt::open("COM250", Line::Rts).is_err());
    }
}
