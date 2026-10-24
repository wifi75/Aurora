//! Probe usa-e-getta: ascolta su una porta UDP e salva ogni datagramma su disco.
//! Uso: udp_capture <porta> <cartella>

use std::{fs, net::UdpSocket, path::PathBuf, process::ExitCode};

/// Tipo di messaggio WSJT-X/JTDX: u32 big-endian alle posizioni 8..12
/// (dopo il magic e il numero di schema).
fn message_kind(datagram: &[u8]) -> Option<u32> {
    let bytes: [u8; 4] = datagram.get(8..12)?.try_into().ok()?;
    Some(u32::from_be_bytes(bytes))
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("uso: udp_capture <porta> <cartella>");
        return ExitCode::from(2);
    }
    let Ok(port) = args[1].parse::<u16>() else {
        eprintln!("porta non valida: {}", args[1]);
        return ExitCode::from(2);
    };
    let dir = PathBuf::from(&args[2]);
    if let Err(e) = fs::create_dir_all(&dir) {
        eprintln!("impossibile creare {}: {e}", dir.display());
        return ExitCode::from(1);
    }
    let socket = match UdpSocket::bind(("0.0.0.0", port)) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("porta {port} non disponibile (gia' in uso?): {e}");
            return ExitCode::from(1);
        }
    };
    println!("in ascolto su 0.0.0.0:{port}, salvo in {}", dir.display());
    let mut buf = vec![0u8; 65535];
    let mut index = 0u32;
    loop {
        let (len, from) = match socket.recv_from(&mut buf) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("errore di ricezione: {e}");
                return ExitCode::from(1);
            }
        };
        index += 1;
        let datagram = &buf[..len];
        let kind = message_kind(datagram).map_or("x".to_string(), |k| k.to_string());
        let file = dir.join(format!("{index:04}-{kind}.bin"));
        if let Err(e) = fs::write(&file, datagram) {
            eprintln!("scrittura di {} fallita: {e}", file.display());
        }
        println!("#{index} da {from} lunghezza {len} tipo {kind}");
    }
}

#[cfg(test)]
mod tests {
    use super::message_kind;

    fn packet(kind: u32) -> Vec<u8> {
        let mut p = Vec::new();
        p.extend_from_slice(&0xadbccbdau32.to_be_bytes());
        p.extend_from_slice(&2u32.to_be_bytes());
        p.extend_from_slice(&kind.to_be_bytes());
        p
    }

    #[test]
    fn kind_is_read_from_bytes_8_to_12() {
        assert_eq!(message_kind(&packet(2)), Some(2));
    }

    #[test]
    fn short_datagram_has_no_kind() {
        assert_eq!(message_kind(&[1, 2, 3]), None);
    }
}
