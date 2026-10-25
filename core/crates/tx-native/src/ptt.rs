//! PTT: il comando che mette la radio in trasmissione. Implementazioni su `rigctld` (Hamlib), su OmniRig e `NoPtt` per il VOX.
//!
//! Il PTT non va mai lasciato acceso: [`PttGuard`] lo spegne quando cade (anche per un panico) e il pianificatore ha in piu' un
//! controllo a tempo (`watchdog`).

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use rig_control::{hamlib::RigctldClient, session::SharedSession};

use crate::error::TxError;

/// Un comando di trasmissione. `set` puo' bloccare per qualche istante (porta seriale, PowerShell): chi lo usa lo chiama da un thread dedicato.
pub trait Ptt: Send + Sync {
    fn set(&self, on: bool) -> Result<(), TxError>;
    /// Lo stato che Aurora crede di aver impostato (acceso se l'ultimo `set(true)` e' riuscito e non c'e' stato un `set(false)` riuscito).
    fn is_on(&self) -> bool;
}

/// Nessun comando: la radio va in trasmissione col VOX, appena arriva l'audio.
#[derive(Debug, Default)]
pub struct NoPtt {
    on: AtomicBool,
}

impl NoPtt {
    pub fn new() -> NoPtt {
        NoPtt::default()
    }
}

impl Ptt for NoPtt {
    fn set(&self, on: bool) -> Result<(), TxError> {
        self.on.store(on, Ordering::SeqCst);
        Ok(())
    }

    fn is_on(&self) -> bool {
        self.on.load(Ordering::SeqCst)
    }
}

/// PTT con `T 1` / `T 0` su un `rigctld` (lo stesso client che comanda frequenza e modo).
pub struct HamlibPtt {
    client: Arc<RigctldClient>,
    on: AtomicBool,
}

impl HamlibPtt {
    pub fn new(client: Arc<RigctldClient>) -> HamlibPtt {
        HamlibPtt { client, on: AtomicBool::new(false) }
    }
}

impl Ptt for HamlibPtt {
    fn set(&self, on: bool) -> Result<(), TxError> {
        self.client.set_ptt(on).map_err(|e| TxError::Ptt(e.0))?;
        self.on.store(on, Ordering::SeqCst);
        Ok(())
    }

    fn is_on(&self) -> bool {
        self.on.load(Ordering::SeqCst)
    }
}

/// `PM_RX` e `PM_TX` di OmniRig (valori della sua libreria dei tipi).
const PM_RX: u32 = 0x0020_0000;
const PM_TX: u32 = 0x0040_0000;

/// Lo script che imposta `Rig<n>.Tx`. Contiene solo il numero della radio (1 o 2) e una costante: niente testo dell'utente.
/// L'oggetto COM si crea una volta sola nella sessione PowerShell; se qualcosa va storto si butta e si ricrea al comando dopo.
pub fn omnirig_ptt_script(rig: u8, on: bool) -> Result<String, TxError> {
    if !(1..=2).contains(&rig) {
        return Err(TxError::Ptt(format!("radio OmniRig non valida: {rig}")));
    }
    let value = if on { PM_TX } else { PM_RX };
    let check = if on { "if ($r.Status -ne 4) { throw 'radio non in linea' }; ".to_string() } else { String::new() };
    Ok(format!(
        "try {{ if (-not $global:AuroraOmni) {{ $global:AuroraOmni = New-Object -ComObject OmniRig.OmniRigX }}; \
         $r = $global:AuroraOmni.Rig{rig}; {check}$r.Tx = {value}; 'ok' }} catch {{ $global:AuroraOmni = $null; throw }}"
    ))
}

/// Sessione PowerShell dedicata al PTT (separata da quelle dei comandi a JTDX): aperta una volta, poi ogni comando costa pochi decimi.
static PTT_SESSION: SharedSession = SharedSession::new();

/// Tempo massimo di un comando PTT via OmniRig.
const OMNIRIG_TIMEOUT: Duration = Duration::from_secs(5);

/// PTT via OmniRig (`Rig1.Tx` / `Rig2.Tx`), con PowerShell nascosto. Solo Windows.
pub struct OmniRigPtt {
    rig: u8,
    on: AtomicBool,
}

impl OmniRigPtt {
    /// `rig` e' 1 o 2 (le due radio di OmniRig).
    pub fn new(rig: u8) -> Result<OmniRigPtt, TxError> {
        omnirig_ptt_script(rig, false)?;
        Ok(OmniRigPtt { rig, on: AtomicBool::new(false) })
    }

    /// Apre la sessione PowerShell e l'oggetto OmniRig in anticipo, cosi' il primo PTT non paga l'avvio.
    pub fn warm_up(&self) -> Result<(), TxError> {
        PTT_SESSION.run_with_timeout("'ok'", OMNIRIG_TIMEOUT).map(|_| ()).map_err(|e| TxError::Ptt(e.0))
    }
}

impl Ptt for OmniRigPtt {
    fn set(&self, on: bool) -> Result<(), TxError> {
        let script = omnirig_ptt_script(self.rig, on)?;
        PTT_SESSION.run_with_timeout(&script, OMNIRIG_TIMEOUT).map_err(|e| TxError::Ptt(e.0))?;
        self.on.store(on, Ordering::SeqCst);
        Ok(())
    }

    fn is_on(&self) -> bool {
        self.on.load(Ordering::SeqCst)
    }
}

/// Spegne il PTT con qualche tentativo (la porta seriale o OmniRig possono essere occupate per un istante).
pub fn release_ptt(ptt: &dyn Ptt) -> Result<(), TxError> {
    let mut last = TxError::Ptt("nessun tentativo".into());
    for attempt in 0..3 {
        match ptt.set(false) {
            Ok(()) => return Ok(()),
            Err(e) => {
                tracing::warn!("PTT: spegnimento fallito (tentativo {}): {e}", attempt + 1);
                last = e;
                std::thread::sleep(Duration::from_millis(20));
            }
        }
    }
    Err(last)
}

/// Tiene acceso il PTT finche' vive: cadendo (anche per un panico) lo spegne.
pub struct PttGuard {
    ptt: Arc<dyn Ptt>,
    armed: bool,
}

impl PttGuard {
    /// Accende il PTT. Se il comando fallisce si prova comunque a spegnere (la risposta potrebbe essersi persa a radio gia' in trasmissione).
    pub fn engage(ptt: Arc<dyn Ptt>) -> Result<PttGuard, TxError> {
        let mut guard = PttGuard { ptt, armed: true };
        match guard.ptt.set(true) {
            Ok(()) => Ok(guard),
            Err(e) => {
                let _ = guard.release();
                Err(e)
            }
        }
    }

    /// Spegne il PTT. Se non ci riesce resta armato: lo riprovera' `Drop`.
    pub fn release(&mut self) -> Result<(), TxError> {
        release_ptt(&*self.ptt)?;
        self.armed = false;
        Ok(())
    }
}

impl Drop for PttGuard {
    fn drop(&mut self) {
        if self.armed
            && let Err(e) = release_ptt(&*self.ptt)
        {
            tracing::error!("PTT non spento: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{BufRead, Write},
        net::{SocketAddr, TcpListener},
        sync::mpsc,
    };

    /// Un finto rigctld: ricorda i comandi e risponde `RPRT 0`, salvo per quelli per cui `reply` dice altro.
    fn fake_server(reply: fn(&str) -> &'static str) -> (SocketAddr, mpsc::Receiver<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("porta");
        let addr = listener.local_addr().expect("indirizzo");
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let Ok((stream, _)) = listener.accept() else { return };
            let mut reader = std::io::BufReader::new(stream.try_clone().expect("clone"));
            let mut writer = stream;
            let mut line = String::new();
            while reader.read_line(&mut line).unwrap_or(0) > 0 {
                let cmd = line.trim().to_string();
                line.clear();
                let _ = tx.send(cmd.clone());
                if cmd == "q" {
                    return;
                }
                let _ = writer.write_all(reply(&cmd).as_bytes());
            }
        });
        (addr, rx)
    }

    fn client(addr: SocketAddr) -> Arc<RigctldClient> {
        Arc::new(RigctldClient::connect(addr, Duration::from_secs(2)).expect("connessione"))
    }

    #[test]
    fn hamlib_ptt_sends_t1_and_t0_and_tracks_the_state() {
        let (addr, seen) = fake_server(|_| "RPRT 0\n");
        let ptt = HamlibPtt::new(client(addr));
        assert!(!ptt.is_on());
        ptt.set(true).expect("T 1");
        assert!(ptt.is_on());
        ptt.set(false).expect("T 0");
        assert!(!ptt.is_on());
        assert_eq!(seen.try_iter().collect::<Vec<_>>(), ["T 1", "T 0"]);
    }

    #[test]
    fn the_guard_turns_the_ptt_off_when_dropped() {
        let (addr, seen) = fake_server(|_| "RPRT 0\n");
        let ptt: Arc<dyn Ptt> = Arc::new(HamlibPtt::new(client(addr)));
        {
            let _guard = PttGuard::engage(Arc::clone(&ptt)).expect("acceso");
            assert!(ptt.is_on());
        }
        assert!(!ptt.is_on());
        assert_eq!(seen.try_iter().collect::<Vec<_>>(), ["T 1", "T 0"]);
    }

    #[test]
    fn the_guard_turns_the_ptt_off_on_a_panic() {
        let (addr, seen) = fake_server(|_| "RPRT 0\n");
        let ptt: Arc<dyn Ptt> = Arc::new(HamlibPtt::new(client(addr)));
        let inner = Arc::clone(&ptt);
        let result = std::thread::spawn(move || {
            let _guard = PttGuard::engage(inner).expect("acceso");
            panic!("prova");
        })
        .join();
        assert!(result.is_err());
        assert!(!ptt.is_on());
        assert_eq!(seen.try_iter().collect::<Vec<_>>(), ["T 1", "T 0"]);
    }

    #[test]
    fn a_refused_t1_is_an_error_and_an_off_is_still_sent() {
        let (addr, seen) = fake_server(|cmd| if cmd == "T 1" { "RPRT -9\n" } else { "RPRT 0\n" });
        let ptt: Arc<dyn Ptt> = Arc::new(HamlibPtt::new(client(addr)));
        let err = PttGuard::engage(Arc::clone(&ptt)).err().expect("doveva fallire");
        assert!(matches!(err, TxError::Ptt(_)), "{err}");
        assert!(!ptt.is_on());
        assert_eq!(seen.try_iter().collect::<Vec<_>>(), ["T 1", "T 0"]);
    }

    #[test]
    fn a_failing_off_is_retried_and_leaves_the_state_on() {
        let (addr, seen) = fake_server(|cmd| if cmd == "T 0" { "RPRT -5\n" } else { "RPRT 0\n" });
        let ptt = HamlibPtt::new(client(addr));
        ptt.set(true).expect("T 1");
        let mut guard = PttGuard { ptt: Arc::new(ptt), armed: true };
        assert!(guard.release().is_err());
        assert!(guard.ptt.is_on(), "lo stato deve restare acceso: lo spegnimento e' fallito");
        assert!(guard.armed);
        let sent: Vec<String> = seen.try_iter().collect();
        assert_eq!(sent.iter().filter(|c| *c == "T 0").count(), 3, "{sent:?}");
        guard.armed = false; // niente tentativi in piu' alla chiusura della prova
    }

    #[test]
    fn a_dead_daemon_is_an_error_not_a_hang() {
        let (addr, _seen) = fake_server(|_| "RPRT 0\n");
        let c = client(addr);
        let ptt = HamlibPtt::new(Arc::clone(&c));
        // il finto server si ferma alla chiusura della connessione: qui la si rompe chiedendo `q`
        c.quit();
        std::thread::sleep(Duration::from_millis(100));
        assert!(ptt.set(true).is_err());
        assert!(!ptt.is_on());
    }

    #[test]
    fn no_ptt_tracks_the_state_for_vox() {
        let ptt = NoPtt::new();
        ptt.set(true).expect("ok");
        assert!(ptt.is_on());
        ptt.set(false).expect("ok");
        assert!(!ptt.is_on());
    }

    #[test]
    fn the_omnirig_script_sets_tx_and_rx_on_the_chosen_rig() {
        let on = omnirig_ptt_script(1, true).expect("script");
        assert!(on.contains("Rig1") && on.contains("$r.Tx = 4194304") && on.contains("Status -ne 4"), "{on}");
        let off = omnirig_ptt_script(2, false).expect("script");
        assert!(off.contains("Rig2") && off.contains("$r.Tx = 2097152") && !off.contains("Status"), "{off}");
        assert!(omnirig_ptt_script(0, true).is_err());
        assert!(omnirig_ptt_script(3, false).is_err());
        assert!(OmniRigPtt::new(3).is_err());
        assert!(OmniRigPtt::new(1).is_ok());
    }
}
