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

/// Una regola del firewall di Windows, quanto serve per capire se Aurora puo' ricevere connessioni.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FirewallRule {
    pub name: String,
    /// Il programma della regola (vuoto: qualunque).
    pub program: String,
    pub inbound: bool,
    pub enabled: bool,
    /// `true` consente, `false` blocca.
    pub allow: bool,
}

/// Il nome della regola per la porta.
pub const PORT_RULE: &str = "Aurora (porta TCP)";

fn same_program(a: &str, b: &str) -> bool {
    !a.is_empty() && a.to_lowercase() == b.to_lowercase()
}

/// Il verdetto del firewall per il programma `exe`, come faceva lo script di prima: un blocco attivo in entrata sul programma vince
/// (`Some(false)`); un permesso sul programma o la regola attiva «Aurora (porta TCP)» consentono (`Some(true)`); altrimenti `Some(false)`.
pub fn firewall_verdict(rules: &[FirewallRule], exe: &str) -> Option<bool> {
    let on_program: Vec<&FirewallRule> = rules.iter().filter(|r| r.enabled && r.inbound && same_program(&r.program, exe)).collect();
    if on_program.iter().any(|r| !r.allow) {
        return Some(false);
    }
    let port_rule = rules.iter().any(|r| r.enabled && r.allow && r.name == PORT_RULE);
    Some(on_program.iter().any(|r| r.allow) || port_rule)
}

/// Le righe per `netsh -f` (da amministratore) che aprono il firewall di Windows ad Aurora **una volta per tutte**:
/// 1. tolgono le regole di BLOCCO sul programma: le crea Windows da solo se alla sua domanda «Consenti l'accesso» si preme Annulla o si
///    chiude la finestra, e in Windows un blocco vince sempre su un permesso (era il motivo per cui «Consenti Aurora» non bastava);
/// 2. tolgono le regole «Aurora*» rimaste, rimettono «Aurora» per il programma e aggiungono «Aurora (porta TCP)» per la porta, tutte e due
///    su ogni rete (anche «Pubblica»).
///
/// Le regole da togliere si leggono prima (senza diritti di amministratore) da `rules`. netsh va avanti anche se una riga non riesce.
/// Un nome con le virgolette non si puo' scrivere a netsh e si salta. `exe` e `port` arrivano dal programma, non dall'utente.
pub fn firewall_netsh_script(exe: &str, port: u16, rules: &[FirewallRule]) -> String {
    let quoted = |t: &str| !t.contains('"');
    let mut lines = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for r in rules {
        if !quoted(&r.name) {
            continue;
        }
        if r.name.starts_with("Aurora") {
            if seen.insert(("name", r.name.clone())) {
                lines.push(format!("advfirewall firewall delete rule name=\"{}\"", r.name));
            }
        } else if !r.allow && same_program(&r.program, exe) && quoted(exe) && seen.insert(("block", r.name.clone())) {
            lines.push(format!("advfirewall firewall delete rule name=\"{}\" dir={} program=\"{exe}\"", r.name, if r.inbound { "in" } else { "out" }));
        }
    }
    lines.push(format!("advfirewall firewall add rule name=\"Aurora\" dir=in action=allow program=\"{exe}\" profile=any enable=yes"));
    lines.push(format!("advfirewall firewall add rule name=\"{PORT_RULE}\" dir=in action=allow protocol=TCP localport={port} profile=any enable=yes"));
    let mut out = lines.join("\r\n");
    out.push_str("\r\n");
    out
}

/// Controlla rete e firewall (solo Windows; altrove restano «non si sa»). Si chiama dalla scheda di associazione. Tutto con le API di
/// Windows (WLAN, Network List Manager, firewall via COM): prima passava da PowerShell nascosto, che gli antivirus segnalano.
pub fn network_check() -> NetworkCheck {
    let named = lan_ipv4s_named();
    #[cfg_attr(not(windows), allow(unused_mut))]
    let mut check = NetworkCheck { addresses: named.iter().map(|(ip, n)| (ip.to_string(), n.clone())).collect(), ..NetworkCheck::default() };
    #[cfg(windows)]
    {
        check.ssid = win::wifi_ssid();
        let _com = win::Com::init();
        check.category = win::network_category();
        let exe = std::env::current_exe().ok().map(|p| p.display().to_string()).unwrap_or_default();
        check.firewall_allowed = win::firewall_rules().and_then(|rules| firewall_verdict(&rules, &exe));
    }
    check
}

/// Apre il firewall di Windows ad Aurora (vedi [`firewall_netsh_script`]): `netsh -f` avviato con «Esegui come amministratore»
/// (`ShellExecuteExW`, verbo `runas`), quindi Windows chiede la conferma. netsh e' un programma di sistema firmato; niente PowerShell.
pub fn allow_in_firewall(port: u16) -> Result<(), String> {
    #[cfg(windows)]
    {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?.display().to_string();
        let rules = {
            let _com = win::Com::init();
            win::firewall_rules().unwrap_or_default()
        };
        let script = firewall_netsh_script(&exe, port, &rules);
        let file = std::env::temp_dir().join(format!("aurora-firewall-{}.txt", std::process::id()));
        std::fs::write(&file, win::ansi(&script)?).map_err(|e| format!("file per netsh non scritto: {e}"))?;
        let ran = win::run_elevated(&win::netsh_exe(), &format!("-f \"{}\"", file.display()));
        let _ = std::fs::remove_file(&file);
        ran?;
        // netsh esce con errore anche se una sola riga «delete» non trova la regola: conta se la regola della porta adesso c'e'
        let _com = win::Com::init();
        let ok = win::firewall_rules().is_some_and(|rules| rules.iter().any(|r| r.enabled && r.allow && r.name == PORT_RULE));
        if ok { Ok(()) } else { Err("regola non aggiunta (conferma annullata?)".to_string()) }
    }
    #[cfg(not(windows))]
    {
        let _ = port;
        Err("solo su Windows".into())
    }
}

#[cfg(windows)]
mod win {
    use windows::Win32::Foundation::{CloseHandle, ERROR_CANCELLED, HANDLE, VARIANT_BOOL, WAIT_OBJECT_0};
    use windows::Win32::Globalization::{CP_ACP, WC_NO_BEST_FIT_CHARS, WideCharToMultiByte};
    use windows::Win32::NetworkManagement::WiFi::{
        WLAN_CONNECTION_ATTRIBUTES, WLAN_INTERFACE_INFO_LIST, WlanCloseHandle, WlanEnumInterfaces, WlanFreeMemory, WlanOpenHandle, WlanQueryInterface, wlan_interface_state_connected,
        wlan_intf_opcode_current_connection,
    };
    use windows::Win32::NetworkManagement::WindowsFirewall::{INetFwPolicy2, INetFwRule, NET_FW_ACTION_ALLOW, NET_FW_RULE_DIR_IN, NetFwPolicy2};
    use windows::Win32::Networking::NetworkListManager::{
        INetwork, INetworkListManager, NLM_ENUM_NETWORK_CONNECTED, NLM_NETWORK_CATEGORY_DOMAIN_AUTHENTICATED, NLM_NETWORK_CATEGORY_PRIVATE, NetworkListManager,
    };
    use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoUninitialize, IDispatch};
    use windows::Win32::System::Ole::IEnumVARIANT;
    use windows::Win32::System::Threading::{GetExitCodeProcess, WaitForSingleObject};
    use windows::Win32::System::Variant::{VARIANT, VT_DISPATCH, VariantClear};
    use windows::Win32::UI::Shell::{SEE_MASK_NOASYNC, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW, ShellExecuteExW};
    use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;
    use windows::core::{HSTRING, Interface, PCWSTR, w};

    use super::FirewallRule;

    /// COM acceso per il thread finche' vive (se era gia' acceso in un altro modo si usa quello).
    pub struct Com(bool);
    impl Com {
        pub fn init() -> Com {
            // SAFETY: nessun puntatore; bilanciata da CoUninitialize nel Drop solo se e' riuscita
            Com(unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }.is_ok())
        }
    }
    impl Drop for Com {
        fn drop(&mut self) {
            if self.0 {
                // SAFETY: bilancia la CoInitializeEx riuscita dello stesso thread
                unsafe { CoUninitialize() };
            }
        }
    }

    /// Il nome della Wi-Fi collegata (API WLAN), `None` se il PC non ne ha una collegata.
    pub fn wifi_ssid() -> Option<String> {
        let mut version = 0u32;
        let mut client = HANDLE::default();
        // SAFETY: puntatori a variabili locali; l'handle si chiude in fondo
        if unsafe { WlanOpenHandle(2, None, &mut version, &mut client) } != 0 {
            return None;
        }
        let mut found = None;
        let mut list: *mut WLAN_INTERFACE_INFO_LIST = std::ptr::null_mut();
        // SAFETY: la lista la alloca WLAN e si libera con WlanFreeMemory; si leggono solo dwNumberOfItems voci
        unsafe {
            if WlanEnumInterfaces(client, None, &mut list) == 0 && !list.is_null() {
                let count = (*list).dwNumberOfItems as usize;
                let first = std::ptr::addr_of!((*list).InterfaceInfo).cast::<windows::Win32::NetworkManagement::WiFi::WLAN_INTERFACE_INFO>();
                for i in 0..count {
                    let info = &*first.add(i);
                    if info.isState != wlan_interface_state_connected {
                        continue;
                    }
                    let mut size = 0u32;
                    let mut data: *mut core::ffi::c_void = std::ptr::null_mut();
                    if WlanQueryInterface(client, &info.InterfaceGuid, wlan_intf_opcode_current_connection, None, &mut size, &mut data, None) == 0 && !data.is_null() {
                        if size as usize >= size_of::<WLAN_CONNECTION_ATTRIBUTES>() {
                            let attrs = &*data.cast::<WLAN_CONNECTION_ATTRIBUTES>();
                            let ssid = attrs.wlanAssociationAttributes.dot11Ssid;
                            let len = (ssid.uSSIDLength as usize).min(ssid.ucSSID.len());
                            let name = String::from_utf8_lossy(&ssid.ucSSID[..len]).trim().to_string();
                            if !name.is_empty() {
                                found = Some(name);
                            }
                        }
                        WlanFreeMemory(data);
                    }
                    if found.is_some() {
                        break;
                    }
                }
                WlanFreeMemory(list.cast());
            }
            WlanCloseHandle(client, None);
        }
        found
    }

    /// La categoria della prima rete collegata (Network List Manager): «Public», «Private», «DomainAuthenticated». Serve COM acceso.
    pub fn network_category() -> Option<String> {
        // SAFETY: oggetti COM con conteggio dei riferimenti gestito da windows-rs
        unsafe {
            let manager: INetworkListManager = CoCreateInstance(&NetworkListManager, None, CLSCTX_INPROC_SERVER).ok()?;
            let networks = manager.GetNetworks(NLM_ENUM_NETWORK_CONNECTED).ok()?;
            let mut one: [Option<INetwork>; 1] = [None];
            let mut fetched = 0u32;
            networks.Next(&mut one, Some(&mut fetched)).ok()?;
            let network = one[0].take().filter(|_| fetched == 1)?;
            let category = network.GetCategory().ok()?;
            Some(
                if category == NLM_NETWORK_CATEGORY_PRIVATE {
                    "Private"
                } else if category == NLM_NETWORK_CATEGORY_DOMAIN_AUTHENTICATED {
                    "DomainAuthenticated"
                } else {
                    "Public"
                }
                .to_string(),
            )
        }
    }

    /// Tutte le regole del firewall di Windows (COM `HNetCfg.FwPolicy2`, sola lettura). Serve COM acceso.
    pub fn firewall_rules() -> Option<Vec<FirewallRule>> {
        // SAFETY: oggetti COM gestiti da windows-rs; ogni VARIANT letto si libera con VariantClear
        unsafe {
            let policy: INetFwPolicy2 = CoCreateInstance(&NetFwPolicy2, None, CLSCTX_INPROC_SERVER).ok()?;
            let rules = policy.Rules().ok()?;
            let items: IEnumVARIANT = rules._NewEnum().ok()?.cast().ok()?;
            let mut out = Vec::new();
            loop {
                let mut slot = [VARIANT::default()];
                let mut fetched = 0u32;
                let hr = items.Next(&mut slot, &mut fetched);
                if hr.is_err() || fetched == 0 {
                    break;
                }
                let inner = &slot[0].Anonymous.Anonymous;
                if inner.vt == VT_DISPATCH {
                    let dispatch: Option<&IDispatch> = inner.Anonymous.pdispVal.as_ref();
                    if let Some(rule) = dispatch.and_then(|d| d.cast::<INetFwRule>().ok()) {
                        out.push(FirewallRule {
                            name: rule.Name().map(|b| b.to_string()).unwrap_or_default(),
                            program: rule.ApplicationName().map(|b| b.to_string()).unwrap_or_default(),
                            inbound: rule.Direction().is_ok_and(|d| d == NET_FW_RULE_DIR_IN),
                            enabled: rule.Enabled().is_ok_and(|e| e != VARIANT_BOOL(0)),
                            allow: rule.Action().is_ok_and(|a| a == NET_FW_ACTION_ALLOW),
                        });
                    }
                }
                let _ = VariantClear(&mut slot[0]);
            }
            Some(out)
        }
    }

    /// `%SystemRoot%\System32\netsh.exe`, col percorso intero.
    pub fn netsh_exe() -> String {
        let root = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".to_string());
        format!(r"{root}\System32\netsh.exe")
    }

    /// Il testo nella code page di Windows (netsh non legge i file UTF-16). Errore se un carattere non si puo' scrivere.
    pub fn ansi(text: &str) -> Result<Vec<u8>, String> {
        let wide: Vec<u16> = text.encode_utf16().collect();
        if text.is_ascii() {
            return Ok(text.as_bytes().to_vec());
        }
        let mut lossy = windows::core::BOOL(0);
        // SAFETY: prima chiamata per la misura, seconda con un buffer della misura data
        unsafe {
            let size = WideCharToMultiByte(CP_ACP, WC_NO_BEST_FIT_CHARS, &wide, None, windows::core::PCSTR::null(), Some(&mut lossy));
            let mut out = vec![0u8; size.max(0) as usize];
            let written = WideCharToMultiByte(CP_ACP, WC_NO_BEST_FIT_CHARS, &wide, Some(&mut out), windows::core::PCSTR::null(), Some(&mut lossy));
            if written <= 0 || lossy.as_bool() {
                return Err("il percorso di Aurora ha caratteri che netsh non legge: aggiungi la regola a mano".to_string());
            }
            out.truncate(written as usize);
            Ok(out)
        }
    }

    /// Avvia `program` con «Esegui come amministratore», nascosto, e ne aspetta la fine (al massimo due minuti).
    pub fn run_elevated(program: &str, args: &str) -> Result<(), String> {
        let file = HSTRING::from(program);
        let params = HSTRING::from(args);
        let mut info = SHELLEXECUTEINFOW {
            cbSize: size_of::<SHELLEXECUTEINFOW>() as u32,
            fMask: SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC,
            lpVerb: w!("runas"),
            lpFile: PCWSTR(file.as_ptr()),
            lpParameters: PCWSTR(params.as_ptr()),
            nShow: SW_HIDE.0,
            ..Default::default()
        };
        // SAFETY: le stringhe vivono fino alla fine della funzione; hProcess si chiude dopo l'attesa
        unsafe {
            if let Err(e) = ShellExecuteExW(&mut info) {
                return Err(if e.code() == ERROR_CANCELLED.to_hresult() { "conferma da amministratore annullata".to_string() } else { format!("netsh non avviato: {e}") });
            }
            if info.hProcess.is_invalid() {
                return Ok(());
            }
            let waited = WaitForSingleObject(info.hProcess, 120_000);
            let mut code = 0u32;
            let _ = GetExitCodeProcess(info.hProcess, &mut code);
            let _ = CloseHandle(info.hProcess);
            if waited != WAIT_OBJECT_0 {
                return Err("netsh non ha finito in tempo".to_string());
            }
        }
        Ok(())
    }
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

    fn rule(name: &str, program: &str, allow: bool) -> FirewallRule {
        FirewallRule { name: name.into(), program: program.into(), inbound: true, enabled: true, allow }
    }

    #[test]
    fn the_netsh_lines_remove_the_blocks_then_allow_program_and_port_everywhere() {
        let exe = r"C:\Users\o'neil\Aurora\aurora-desktop.exe";
        let rules = [rule("aurora-desktop.exe", &exe.to_uppercase(), false), rule("Aurora", exe, true), rule("Aurora (porta TCP)", "", true), rule("Altro", r"C:\x.exe", false), rule("Strano \"nome\"", exe, false)];
        let s = firewall_netsh_script(exe, 8787, &rules);
        let lines: Vec<&str> = s.lines().collect();
        assert_eq!(
            lines,
            [
                r#"advfirewall firewall delete rule name="aurora-desktop.exe" dir=in program="C:\Users\o'neil\Aurora\aurora-desktop.exe""#,
                r#"advfirewall firewall delete rule name="Aurora""#,
                r#"advfirewall firewall delete rule name="Aurora (porta TCP)""#,
                r#"advfirewall firewall add rule name="Aurora" dir=in action=allow program="C:\Users\o'neil\Aurora\aurora-desktop.exe" profile=any enable=yes"#,
                r#"advfirewall firewall add rule name="Aurora (porta TCP)" dir=in action=allow protocol=TCP localport=8787 profile=any enable=yes"#,
            ]
        );
        assert!(s.ends_with("\r\n"));
    }

    #[cfg(windows)]
    #[test]
    fn the_real_network_and_firewall_can_be_read() {
        let check = network_check();
        eprintln!("{check:?}");
        let _com = win::Com::init();
        assert!(!win::firewall_rules().expect("regole del firewall").is_empty());
    }

    #[test]
    fn the_firewall_verdict_is_like_the_old_script() {
        let exe = r"C:\Aurora\aurora.exe";
        assert_eq!(firewall_verdict(&[], exe), Some(false));
        assert_eq!(firewall_verdict(&[rule("Aurora", r"c:\aurora\AURORA.exe", true)], exe), Some(true));
        assert_eq!(firewall_verdict(&[rule(PORT_RULE, "", true)], exe), Some(true));
        // un blocco sul programma vince su tutto
        assert_eq!(firewall_verdict(&[rule(PORT_RULE, "", true), rule("aurora.exe", exe, false)], exe), Some(false));
        // regole spente o in uscita non contano
        let off = FirewallRule { enabled: false, ..rule("aurora.exe", exe, false) };
        let out = FirewallRule { inbound: false, ..rule("aurora.exe", exe, false) };
        assert_eq!(firewall_verdict(&[off, out, rule("Aurora", exe, true)], exe), Some(true));
    }

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
