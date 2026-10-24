//! Il minimo di telnet che serve a un cluster: toglie le negoziazioni (IAC), rifiuta ogni opzione e
//! riconosce quando il server chiede il nominativo.

const IAC: u8 = 255;
const DONT: u8 = 254;
const DO: u8 = 253;
const WONT: u8 = 252;
const WILL: u8 = 251;
const SB: u8 = 250;
const SE: u8 = 240;

/// Toglie i comandi telnet dal flusso (anche spezzati tra due letture) e prepara le risposte che li rifiutano.
#[derive(Default)]
pub struct Decoder {
    pending: Vec<u8>,
}

impl Decoder {
    /// Restituisce il testo pulito e i byte da rimandare al server.
    pub fn feed(&mut self, data: &[u8]) -> (Vec<u8>, Vec<u8>) {
        let mut input = std::mem::take(&mut self.pending);
        input.extend_from_slice(data);
        let (mut text, mut replies) = (Vec::new(), Vec::new());
        let mut i = 0;
        while i < input.len() {
            let byte = input[i];
            if byte != IAC {
                text.push(byte);
                i += 1;
                continue;
            }
            let Some(&command) = input.get(i + 1) else {
                self.pending.extend_from_slice(&input[i..]);
                break;
            };
            match command {
                IAC => {
                    text.push(IAC);
                    i += 2;
                }
                DO | DONT | WILL | WONT => {
                    let Some(&option) = input.get(i + 2) else {
                        self.pending.extend_from_slice(&input[i..]);
                        break;
                    };
                    match command {
                        DO => replies.extend_from_slice(&[IAC, WONT, option]),
                        WILL => replies.extend_from_slice(&[IAC, DONT, option]),
                        _ => {}
                    }
                    i += 3;
                }
                SB => match (i + 2..input.len().saturating_sub(1)).find(|&j| input[j] == IAC && input[j + 1] == SE) {
                    Some(end) => i = end + 2,
                    None => {
                        self.pending.extend_from_slice(&input[i..]);
                        break;
                    }
                },
                _ => i += 2,
            }
        }
        (text, replies)
    }
}

/// Il server chiede il nominativo: `login:`, `Please enter your call:`, `callsign:`...
pub fn is_login_prompt(text: &str) -> bool {
    let lower = text.trim_end().to_lowercase();
    lower.ends_with(':') && (lower.contains("login") || lower.contains("call"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_passes_through() {
        let (text, replies) = Decoder::default().feed(b"login: ");
        assert_eq!(text, b"login: ");
        assert!(replies.is_empty());
    }

    #[test]
    fn refuses_every_option_the_server_proposes() {
        let (text, replies) = Decoder::default().feed(&[IAC, DO, 24, b'h', b'i', IAC, WILL, 1, IAC, WONT, 3]);
        assert_eq!(text, b"hi");
        assert_eq!(replies, vec![IAC, WONT, 24, IAC, DONT, 1]);
    }

    #[test]
    fn a_command_split_between_two_reads_is_still_understood() {
        let mut d = Decoder::default();
        let (t1, r1) = d.feed(&[b'a', IAC]);
        assert_eq!((t1, r1.len()), (b"a".to_vec(), 0));
        let (t2, r2) = d.feed(&[DO, 31, b'b']);
        assert_eq!(t2, b"b");
        assert_eq!(r2, vec![IAC, WONT, 31]);
    }

    #[test]
    fn subnegotiations_are_skipped_and_a_doubled_iac_is_a_real_byte() {
        let (text, replies) = Decoder::default().feed(&[b'x', IAC, SB, 24, 1, IAC, SE, b'y', IAC, IAC]);
        assert_eq!(text, vec![b'x', b'y', IAC]);
        assert!(replies.is_empty());
    }

    #[test]
    fn recognises_the_login_prompts_of_real_clusters() {
        for p in ["login: ", "Please enter your call: ", "Enter your callsign:", "Please enter your callsign: ", "callsign :"] {
            assert!(is_login_prompt(p), "{p:?}");
        }
        for p in ["Welcome to the cluster", "DX de W3LPL: 14074.0 K1ABC 1200Z", "Last login: yesterday", ""] {
            assert!(!is_login_prompt(p), "{p:?}");
        }
    }
}
