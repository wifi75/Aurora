//! Probe usa-e-getta: invia a JTDX un comando UDP (HaltTx, FreeText, Reply).
//! Uso:
//!   udp_send <host:porta> <status.bin> halt
//!   udp_send <host:porta> <status.bin> free "<testo>"
//!   udp_send <host:porta> <status.bin> reply <decode.bin>
//! `<host:porta>` e' l'indirizzo di JTDX che `udp_capture` stampa nella colonna "da".
//! `<status.bin>` e' un Status catturato: da li' si leggono identificativo e schema.

use std::{fs, net::UdpSocket, process::ExitCode};

use udp_bridge::{decode, encode_free_text, encode_halt_tx, encode_reply, Message};

fn run(args: &[String]) -> Result<(), String> {
    let target = args.first().ok_or("manca <host:porta>")?;
    let status_path = args.get(1).ok_or("manca <status.bin>")?;
    let command = args.get(2).ok_or("manca il comando")?;

    let status_bytes = fs::read(status_path).map_err(|e| format!("{status_path}: {e}"))?;
    let status = decode(&status_bytes).map_err(|e| format!("{status_path}: {e}"))?;
    let (schema, id) = match &status.message {
        Message::Status(s) => (status.schema, s.id.clone().unwrap_or_default()),
        _ => return Err(format!("{status_path} non e' un messaggio di Status")),
    };

    let packet = match command.as_str() {
        "halt" => encode_halt_tx(&id, schema, false),
        "free" => {
            let text = args.get(3).ok_or("manca il testo")?;
            encode_free_text(&id, schema, text, false)
        }
        "reply" => {
            let path = args.get(3).ok_or("manca <decode.bin>")?;
            let bytes = fs::read(path).map_err(|e| format!("{path}: {e}"))?;
            match decode(&bytes).map_err(|e| format!("{path}: {e}"))?.message {
                Message::Decode(d) => encode_reply(&id, schema, &d, 0),
                _ => return Err(format!("{path} non e' un messaggio di Decode")),
            }
        }
        other => return Err(format!("comando sconosciuto: {other}")),
    };

    let socket = UdpSocket::bind("0.0.0.0:0").map_err(|e| e.to_string())?;
    let sent = socket.send_to(&packet, target).map_err(|e| format!("invio a {target}: {e}"))?;
    println!("inviati {sent} byte a {target} (comando {command}, id {id:?}, schema {schema})");
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("errore: {e}");
            eprintln!("uso: udp_send <host:porta> <status.bin> halt | free \"<testo>\" | reply <decode.bin>");
            ExitCode::from(2)
        }
    }
}
