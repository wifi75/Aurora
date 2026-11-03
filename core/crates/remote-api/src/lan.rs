//! Rete di casa: gli indirizzi del PC e l'annuncio mDNS (`aurora.local`), perche' il telefono ritrovi Aurora
//! anche quando il router da' al PC un IP diverso.

use std::{
    net::{IpAddr, Ipv4Addr},
    sync::RwLock,
    time::Duration,
};

use mdns_sd::{HostnameResolutionEvent, ServiceDaemon, ServiceInfo};

/// Tipo del servizio annunciato: una pagina web.
const SERVICE_TYPE: &str = "_http._tcp.local.";
/// Il nome che si prova per primo; se e' gia' preso si aggiunge il nome del PC.
const PREFERRED_HOST: &str = "aurora";
/// Quanto si aspetta una risposta per capire se `aurora.local` e' gia' di un altro.
const PROBE_MS: u64 = 1500;

/// Dove si trova Aurora sulla rete di casa: porta in ascolto e nome mDNS (se l'annuncio e' partito).
#[derive(Default)]
pub struct LanInfo {
    inner: RwLock<(u16, Option<String>)>,
}

impl LanInfo {
    pub fn set_port(&self, port: u16) {
        if let Ok(mut g) = self.inner.write() {
            g.0 = port;
        }
    }

    pub fn set_host(&self, host: Option<String>) {
        if let Ok(mut g) = self.inner.write() {
            g.1 = host;
        }
    }

    /// Porta in ascolto (0 finche' il server non e' partito).
    pub fn port(&self) -> u16 {
        self.inner.read().map(|g| g.0).unwrap_or(0)
    }

    /// Nome mDNS annunciato, per esempio `aurora.local`.
    pub fn host(&self) -> Option<String> {
        self.inner.read().ok().and_then(|g| g.1.clone())
    }
}

/// Gli IPv4 del PC sulla rete di casa (niente loopback ne' indirizzi automatici 169.254), prima quelli privati.
pub fn lan_ipv4s() -> Vec<Ipv4Addr> {
    let mut ips: Vec<Ipv4Addr> = if_addrs::get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|iface| match iface.ip() {
            IpAddr::V4(v4) if !v4.is_loopback() && !v4.is_link_local() && !v4.is_unspecified() => Some(v4),
            _ => None,
        })
        .collect();
    ips.sort_by_key(|ip| (!ip.is_private(), *ip));
    ips.dedup();
    ips
}

/// Il nome del PC ridotto a lettere, cifre e trattini, come vuole un nome host.
fn pc_label() -> String {
    let raw = std::env::var("COMPUTERNAME").or_else(|_| std::env::var("HOSTNAME")).unwrap_or_default();
    let label: String = raw
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let label = label.trim_matches('-').chars().take(40).collect::<String>();
    if label.is_empty() { "pc".to_string() } else { label }
}

/// Vero se qualcun altro (non questo PC) risponde gia' al nome `<host>.local`.
fn taken_by_other(daemon: &ServiceDaemon, host: &str) -> bool {
    let ours = lan_ipv4s();
    let Ok(rx) = daemon.resolve_hostname(&format!("{host}.local."), Some(PROBE_MS)) else {
        return false;
    };
    let deadline = std::time::Instant::now() + Duration::from_millis(PROBE_MS + 300);
    while let Some(left) = deadline.checked_duration_since(std::time::Instant::now()) {
        match rx.recv_timeout(left) {
            Ok(HostnameResolutionEvent::AddressesFound(_, found)) => {
                let other = found.iter().any(|ip| match ip.to_ip_addr() {
                    IpAddr::V4(v4) => !ours.contains(&v4),
                    IpAddr::V6(_) => false,
                });
                if other {
                    return true;
                }
            }
            Ok(HostnameResolutionEvent::SearchTimeout(_) | HostnameResolutionEvent::SearchStopped(_)) | Err(_) => break,
            Ok(_) => {}
        }
    }
    false
}

/// L'annuncio mDNS di Aurora: vive quanto il server e si spegne con lui.
pub struct Announcer {
    daemon: ServiceDaemon,
}

impl Announcer {
    /// Annuncia il servizio HTTP su tutte le interfacce della rete di casa. Gli indirizzi si aggiornano da soli se cambia
    /// l'IP del PC. Se qualcosa va storto resta solo un avviso nel log: Aurora funziona lo stesso con l'IP.
    pub fn start(port: u16, info: &LanInfo) -> Option<Announcer> {
        let daemon = match ServiceDaemon::new() {
            Ok(d) => d,
            Err(e) => {
                tracing::warn!("annuncio mDNS non attivo: {e}");
                return None;
            }
        };
        let host = if taken_by_other(&daemon, PREFERRED_HOST) {
            format!("{PREFERRED_HOST}-{}", pc_label())
        } else {
            PREFERRED_HOST.to_string()
        };
        let service = ServiceInfo::new(SERVICE_TYPE, "Aurora", &format!("{host}.local."), (), port, &[("path", "/")][..]);
        let registered = service.map(ServiceInfo::enable_addr_auto).and_then(|s| daemon.register(s));
        if let Err(e) = registered {
            tracing::warn!("annuncio mDNS non attivo: {e}");
            let _ = daemon.shutdown();
            return None;
        }
        tracing::info!("Aurora annunciata sulla rete di casa come http://{host}.local:{port}");
        info.set_host(Some(format!("{host}.local")));
        Some(Announcer { daemon })
    }
}

impl Drop for Announcer {
    fn drop(&mut self) {
        let _ = self.daemon.shutdown();
    }
}
