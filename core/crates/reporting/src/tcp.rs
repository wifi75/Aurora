//! Invio del record ADIF di ogni QSO a un programma in ascolto su TCP (`TCPServer`, `TCPServerPort`, `EnableTCPConnection`).
//!
//! Come `LogQSO::accept` di JTDX (sorgente e `jtdx.exe` 2.2.159): per ogni QSO registrato una connessione nuova, attesa di 1 s per la connessione, un solo
//! messaggio `<command:3>Log <parameters:N> <record ADIF> ` (N = lunghezza del record con lo spazio finale), attesa di 1 s per una risposta, chiusura.
//! Una risposta che comincia per `NAK` vuol dire che il programma ha rifiutato il QSO.

use std::{net::SocketAddr, time::Duration};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    time::timeout,
};

/// Attesa della connessione e della risposta (`waitForConnected(1000)`, `waitForReadyRead(1000)`).
pub const WAIT: Duration = Duration::from_millis(1000);

/// Il messaggio per un record (`myadif` di JTDX): il comando, la lunghezza del record con il suo spazio finale e il record seguito da uno spazio.
pub fn payload(record: &str) -> String {
    let with_space = format!("{record} ");
    // `QString::length()` conta unita' UTF-16
    let len = with_space.encode_utf16().count();
    format!("<command:3>Log <parameters:{len}> {with_space}")
}

/// Manda il messaggio. `Ok` con la risposta (anche vuota se il programma non risponde entro 1 s: JTDX lo ammette); `Err` se non si connette o se risponde `NAK`.
pub async fn send(addr: SocketAddr, record: &str) -> Result<String, String> {
    let mut stream = match timeout(WAIT, TcpStream::connect(addr)).await {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => return Err(format!("{addr}: {e}")),
        Err(_) => return Err(format!("{addr}: nessuna connessione entro 1 s")),
    };
    timeout(WAIT, stream.write_all(payload(record).as_bytes())).await.map_err(|_| format!("{addr}: scrittura troppo lenta"))?.map_err(|e| format!("{addr}: {e}"))?;
    let mut buf = vec![0u8; 1024];
    let reply = match timeout(WAIT, stream.read(&mut buf)).await {
        Ok(Ok(n)) => String::from_utf8_lossy(&buf[..n]).trim().to_string(),
        Ok(Err(_)) | Err(_) => String::new(),
    };
    let _ = stream.shutdown().await;
    if reply.starts_with("NAK") {
        return Err(format!("QSO rifiutato dal programma ({})", reply.chars().take(60).collect::<String>()));
    }
    Ok(reply)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_message_is_the_log_command_with_the_length_of_the_record_and_its_trailing_space() {
        let m = payload("<BAND:3>20m <CALL:5>K1ABC <EOR>");
        assert_eq!(m, "<command:3>Log <parameters:32> <BAND:3>20m <CALL:5>K1ABC <EOR> ");
        assert_eq!("<BAND:3>20m <CALL:5>K1ABC <EOR> ".len(), 32);
    }

    #[test]
    fn the_length_counts_utf16_units_like_qstring() {
        let m = payload("<NAME:3>é <EOR>");
        // "<NAME:3>é <EOR> " = 16 unita' (la `é` e' una sola unita' UTF-16 ma due byte)
        assert!(m.starts_with("<command:3>Log <parameters:16> "), "{m}");
    }
}
