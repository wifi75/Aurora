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

/// Gli adattatori che non portano alla rete di casa: macchine virtuali, WSL, Docker, VPN, Bluetooth. Il loro indirizzo nel QR
/// mandava il telefono su una rete che non raggiunge (per esempio `172.18.x.x` di WSL, che in ordine veniva prima di `192.168.1.x`).
pub fn is_virtual_adapter(name: &str) -> bool {
    let n = name.to_lowercase();
    ["vethernet", "virtualbox", "vmware", "hyper-v", "wsl", "docker", "vpn", "tap-", "tap ", "tun", "wintun", "tailscale", "zerotier", "hamachi", "radmin", "bluetooth", "loopback", "npcap", "wireguard", "openvpn", "tunnel", "nordlynx", "virtual", "vbox", "utun", "br-", "veth", "virbr"]
        .iter()
        .any(|w| n.contains(w))
}

/// Un adattatore Wi-Fi (dal nome che gli da' il sistema).
pub fn is_wifi_adapter(name: &str) -> bool {
    let n = name.to_lowercase();
    ["wi-fi", "wifi", "wlan", "wireless", "wlp", "wlo", "airport"].iter().any(|w| n.contains(w))
}

/// Quanto un indirizzo e' probabilmente quello della rete di casa: prima 192.168.x, poi 10.x, poi 172.16-31.x, poi gli altri.
fn home_rank(ip: &Ipv4Addr) -> u8 {
    match ip.octets() {
        [192, 168, ..] => 0,
        [10, ..] => 1,
        [172, b, ..] if (16..=31).contains(&b) => 2,
        _ if ip.is_private() => 3,
        _ => 4,
    }
}

/// Gli IPv4 del PC sulla rete di casa, con il nome dell'adattatore: niente loopback, 169.254 e adattatori virtuali (se resta almeno
/// un adattatore vero), prima quelli che sembrano la rete di casa.
pub fn lan_ipv4s_named() -> Vec<(Ipv4Addr, String)> {
    let all: Vec<(Ipv4Addr, String)> = if_addrs::get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|iface| match iface.ip() {
            IpAddr::V4(v4) if !v4.is_loopback() && !v4.is_link_local() && !v4.is_unspecified() => Some((v4, iface.name.clone())),
            _ => None,
        })
        .collect();
    let real: Vec<(Ipv4Addr, String)> = all.iter().filter(|(_, n)| !is_virtual_adapter(n)).cloned().collect();
    let mut ips = if real.is_empty() { all } else { real };
    // la Wi-Fi per prima: il telefono e' quasi sempre in Wi-Fi, e con PC su cavo e Wi-Fi insieme l'indirizzo del cavo puo' essere di un'altra rete
    ips.sort_by_key(|(ip, name)| (!is_wifi_adapter(name), home_rank(ip), *ip));
    ips.dedup_by(|a, b| a.0 == b.0);
    ips
}

/// Gli IPv4 del PC sulla rete di casa (vedi [`lan_ipv4s_named`]).
pub fn lan_ipv4s() -> Vec<Ipv4Addr> {
    lan_ipv4s_named().into_iter().map(|(ip, _)| ip).collect()
}

/// Cosa sa Windows della rete del PC: la Wi-Fi (SSID), se la rete e' «Pubblica» e se il firewall lascia entrare Aurora.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct NetworkCheck {
    /// Il nome della rete Wi-Fi a cui e' collegato il PC (`None`: cavo, o non si sa).
    pub ssid: Option<String>,
    /// «Public», «Private», «DomainAuthenticated» (della rete con l'indirizzo scelto).
    pub category: Option<String>,
    /// Il firewall ha una regola che lascia entrare il programma (`None`: non si sa).
    pub firewall_allowed: Option<bool>,
    /// Gli indirizzi scelti, con il nome dell'adattatore.
    pub addresses: Vec<(String, String)>,
}

#[cfg(windows)]
fn powershell(script: &str) -> Option<String> {
    use std::os::windows::process::CommandExt;
    let out = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", script])
        .creation_flags(0x0800_0000)
        .output()
        .ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Controlla rete e firewall (solo Windows; altrove restano «non si sa»). Si chiama dalla scheda di associazione.
pub fn network_check() -> NetworkCheck {
    let named = lan_ipv4s_named();
    #[cfg_attr(not(windows), allow(unused_mut))]
    let mut check = NetworkCheck { addresses: named.iter().map(|(ip, n)| (ip.to_string(), n.clone())).collect(), ..NetworkCheck::default() };
    #[cfg(windows)]
    {
        let exe = std::env::current_exe().ok().map(|p| p.display().to_string()).unwrap_or_default().replace('\'', "''");
        let script = format!(
            "$ErrorActionPreference='SilentlyContinue'; \
             $s = (netsh wlan show interfaces | Select-String '^\\s*SSID\\s*:' | Select-Object -First 1); if ($s) {{ 'SSID=' + ($s.ToString() -replace '^\\s*SSID\\s*:\\s*','') }}; \
             $p = Get-NetConnectionProfile | Where-Object {{ $_.IPv4Connectivity -ne 'Disconnected' }} | Select-Object -First 1; if ($p) {{ 'CAT=' + $p.NetworkCategory }}; \
             $r = Get-NetFirewallApplicationFilter -Program '{exe}' | Get-NetFirewallRule | Where-Object {{ $_.Enabled -eq 'True' -and $_.Direction -eq 'Inbound' }}; \
             if ($r | Where-Object {{ $_.Action -eq 'Block' }}) {{ 'FW=block' }} elseif ($r | Where-Object {{ $_.Action -eq 'Allow' }}) {{ 'FW=allow' }} else {{ 'FW=none' }}"
        );
        if let Some(out) = powershell(&script) {
            for line in out.lines() {
                if let Some(v) = line.strip_prefix("SSID=") {
                    check.ssid = Some(v.trim().to_string()).filter(|s| !s.is_empty());
                } else if let Some(v) = line.strip_prefix("CAT=") {
                    check.category = Some(v.trim().to_string());
                } else if let Some(v) = line.strip_prefix("FW=") {
                    check.firewall_allowed = match v.trim() {
                        "allow" => Some(true),
                        "block" | "none" => Some(false),
                        _ => None,
                    };
                }
            }
        }
    }
    check
}

/// Aggiunge al firewall di Windows la regola che lascia entrare Aurora (Windows chiede la conferma da amministratore).
pub fn allow_in_firewall() -> Result<(), String> {
    #[cfg(windows)]
    {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?.display().to_string();
        let args = format!("advfirewall firewall add rule name=\"Aurora\" dir=in action=allow program=\"{exe}\" enable=yes profile=any");
        let script = format!("Start-Process netsh -Verb RunAs -Wait -WindowStyle Hidden -ArgumentList '{}'", args.replace('\'', "''"));
        return powershell(&script).map(|_| ()).ok_or_else(|| "regola non aggiunta (conferma annullata?)".to_string());
    }
    #[cfg(not(windows))]
    Err("solo su Windows".into())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_wifi_adapter_is_recognised_by_its_name() {
        for n in ["Wi-Fi", "WiFi 2", "WLAN", "Wireless Network Connection", "wlp3s0", "wlo1"] {
            assert!(is_wifi_adapter(n), "{n}");
        }
        for n in ["Ethernet", "eth0", "enp2s0", "vEthernet (WSL)"] {
            assert!(!is_wifi_adapter(n), "{n}");
        }
    }
}
