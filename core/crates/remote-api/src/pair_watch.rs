//! Diagnosi dell'associazione dal telefono. Quando il telefono apre il QR e vede «pagina non trovata» non arriva nulla ad Aurora: il
//! rapporto lo puo' fare solo il PC. Qui si segue ogni codice mostrato: che indirizzo e' nel QR (solo il tipo, mai l'IP), se dal telefono
//! arriva almeno una richiesta e come finisce l'associazione. Se allo scadere del codice l'associazione non e' riuscita si scrive un
//! avviso (`remote_api::associazione`) che va nei rapporti anonimi con la causa piu' probabile: rete «Pubblica», firewall senza regola,
//! indirizzo di un adattatore virtuale o non privato, telefono mai arrivato (rete diversa, dati mobili, isolamento della Wi-Fi).

use std::{
    net::{IpAddr, Ipv4Addr},
    sync::Mutex,
    time::{Duration, Instant},
};

/// Il tipo dell'indirizzo messo nel QR: si dice il tipo, non l'indirizzo.
pub fn host_kind(url: &str) -> &'static str {
    let host = url.trim_start_matches("http://").trim_start_matches("https://").split(['/', ':']).next().unwrap_or("");
    if host.ends_with(".local") {
        return "nome mDNS (.local)";
    }
    match host.parse::<Ipv4Addr>() {
        Ok(ip) if ip.is_link_local() => "169.254 (nessun indirizzo dal router)",
        Ok(ip) if ip.octets()[0] == 192 && ip.octets()[1] == 168 => "192.168 (rete di casa)",
        Ok(ip) if ip.octets()[0] == 10 => "10.x (rete privata)",
        Ok(ip) if ip.octets()[0] == 172 && (16..=31).contains(&ip.octets()[1]) => "172.16-31 (rete privata o virtuale)",
        Ok(ip) if ip.is_loopback() => "localhost (non raggiungibile dal telefono)",
        Ok(_) => "pubblico (non della rete di casa)",
        Err(_) if host.is_empty() => "nessuno",
        Err(_) => "nome",
    }
}

/// Il tipo di adattatore (senza il suo nome, che puo' dire chi e' l'utente).
pub fn adapter_kind(name: &str) -> &'static str {
    let n = name.to_lowercase();
    if crate::lan::is_virtual_adapter(name) {
        "virtuale"
    } else if n.contains("wi-fi") || n.contains("wifi") || n.contains("wlan") || n.contains("wireless") {
        "Wi-Fi"
    } else {
        "cavo"
    }
}

/// Un indirizzo che viene dalla rete (non dallo stesso PC).
pub fn from_lan(ip: IpAddr) -> bool {
    !ip.is_loopback() && !ip.is_unspecified()
}

#[derive(Clone, Debug, PartialEq)]
struct Attempt {
    serial: u64,
    started: Instant,
    /// Tipo dell'indirizzo nel QR e dell'adattatore che lo porta.
    qr_host: &'static str,
    qr_adapter: &'static str,
    /// Quanti indirizzi della rete di casa ha il PC.
    addresses: usize,
    /// Almeno una richiesta arrivata da un altro dispositivo dopo il codice.
    reached: bool,
    /// L'ultimo esito di `pair/finish` (`None`: mai chiamato).
    finish: Option<&'static str>,
    done: bool,
}

/// Come e' andata un'associazione, per il rapporto.
#[derive(Clone, Debug, PartialEq)]
pub struct Outcome {
    pub qr_host: &'static str,
    pub qr_adapter: &'static str,
    pub addresses: usize,
    pub reached: bool,
    pub finish: Option<&'static str>,
    pub seconds: u64,
}

impl Outcome {
    /// Il messaggio del rapporto (in italiano come gli altri avvisi), senza indirizzi ne' nomi; `network` aggiunge rete e firewall.
    pub fn message(&self, category: Option<&str>, firewall: Option<bool>) -> String {
        let cause = if !self.reached {
            "il telefono non ha mai raggiunto Aurora (pagina non trovata sul telefono)"
        } else if self.finish.is_none() {
            "il telefono ha raggiunto Aurora ma non ha completato l'associazione"
        } else {
            "il telefono ha raggiunto Aurora ma l'associazione e' stata rifiutata"
        };
        let rete = match category {
            Some(c) if c.eq_ignore_ascii_case("public") => "Pubblica",
            Some(c) if c.eq_ignore_ascii_case("private") => "Privata",
            Some(c) if c.to_ascii_lowercase().contains("domain") => "di dominio",
            Some(_) => "altro",
            None => "non si sa",
        };
        let fw = match firewall {
            Some(true) => "regola presente",
            Some(false) => "nessuna regola o bloccato",
            None => "non si sa",
        };
        format!(
            "associazione dal telefono non riuscita: {cause}; QR su indirizzo {} ({}), indirizzi della rete di casa: {}, rete di Windows: {rete}, firewall: {fw}, esito: {}",
            self.qr_host,
            self.qr_adapter,
            self.addresses,
            self.finish.unwrap_or("nessuno"),
        )
    }
}

/// I tentativi di associazione: uno alla volta (un codice nuovo chiude quello di prima).
#[derive(Default)]
pub struct PairWatch {
    current: Mutex<Option<Attempt>>,
    next: Mutex<u64>,
}

impl PairWatch {
    pub fn new() -> PairWatch {
        PairWatch::default()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Option<Attempt>> {
        self.current.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// Un codice appena mostrato: restituisce il suo numero (per chiuderlo allo scadere) e, se c'era un tentativo non riuscito, il suo esito.
    pub fn start(&self, qr_url: Option<&str>, adapter: &str, addresses: usize) -> (u64, Option<Outcome>) {
        let serial = {
            let mut next = self.next.lock().unwrap_or_else(|p| p.into_inner());
            *next += 1;
            *next
        };
        let mut current = self.lock();
        let previous = current.take().and_then(|a| outcome(&a));
        *current = Some(Attempt {
            serial,
            started: Instant::now(),
            qr_host: qr_url.map_or("nessuno", host_kind),
            qr_adapter: adapter_kind(adapter),
            addresses,
            reached: false,
            finish: None,
            done: false,
        });
        (serial, previous)
    }

    /// Una richiesta arrivata da un altro dispositivo.
    pub fn seen_from_lan(&self) {
        if let Some(a) = self.lock().as_mut() {
            a.reached = true;
        }
    }

    /// L'esito di `pair/finish`: `ok` chiude il tentativo come riuscito.
    pub fn finished(&self, result: &'static str) {
        if let Some(a) = self.lock().as_mut() {
            a.reached = true;
            a.finish = Some(result);
            if result == "ok" {
                a.done = true;
            }
        }
    }

    /// Allo scadere del codice `serial`: l'esito se non e' riuscito (e il tentativo e' ancora quello).
    pub fn expire(&self, serial: u64) -> Option<Outcome> {
        let mut current = self.lock();
        if current.as_ref().is_some_and(|a| a.serial == serial) { current.take().and_then(|a| outcome(&a)) } else { None }
    }
}

fn outcome(a: &Attempt) -> Option<Outcome> {
    (!a.done).then(|| Outcome {
        qr_host: a.qr_host,
        qr_adapter: a.qr_adapter,
        addresses: a.addresses,
        reached: a.reached,
        finish: a.finish,
        seconds: a.started.elapsed().as_secs(),
    })
}

/// Quanto si aspetta oltre la durata del codice prima di dire che non e' riuscita.
pub const GRACE: Duration = Duration::from_secs(30);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_qr_address_is_described_by_its_kind_never_by_itself() {
        assert_eq!(host_kind("http://192.168.1.225:8787"), "192.168 (rete di casa)");
        assert_eq!(host_kind("http://172.18.0.1:8787"), "172.16-31 (rete privata o virtuale)");
        assert_eq!(host_kind("http://169.254.3.4:8787"), "169.254 (nessun indirizzo dal router)");
        assert_eq!(host_kind("http://aurora.local:8787"), "nome mDNS (.local)");
        assert_eq!(host_kind("http://8.8.8.8:8787"), "pubblico (non della rete di casa)");
        assert_eq!(adapter_kind("vEthernet (WSL)"), "virtuale");
        assert_eq!(adapter_kind("Wi-Fi"), "Wi-Fi");
        assert_eq!(adapter_kind("Ethernet 2"), "cavo");
    }

    #[test]
    fn a_phone_that_never_arrives_is_reported_and_a_successful_one_is_not() {
        let w = PairWatch::new();
        let (first, none) = w.start(Some("http://192.168.1.225:8787/#/associa"), "Wi-Fi", 1);
        assert!(none.is_none());
        let o = w.expire(first).expect("non riuscita");
        assert!(!o.reached);
        let text = o.message(Some("Public"), Some(false));
        assert!(text.contains("non ha mai raggiunto") && text.contains("Pubblica") && text.contains("nessuna regola"), "{text}");
        assert!(!text.contains("192.168.1.225"), "mai l'indirizzo");
        let (second, _) = w.start(Some("http://192.168.1.225:8787"), "Wi-Fi", 1);
        w.seen_from_lan();
        w.finished("ok");
        assert!(w.expire(second).is_none(), "riuscita: niente rapporto");
    }

    #[test]
    fn a_new_code_closes_the_previous_attempt_and_a_refusal_is_told_apart() {
        let w = PairWatch::new();
        let (first, _) = w.start(Some("http://10.0.0.5:8787"), "cavo", 2);
        w.finished("expired");
        let (_, previous) = w.start(Some("http://10.0.0.5:8787"), "cavo", 2);
        let previous = previous.expect("il tentativo di prima");
        assert!(previous.reached && previous.finish == Some("expired"));
        assert!(previous.message(None, None).contains("rifiutata"));
        assert!(w.expire(first).is_none(), "il tentativo vecchio non c'e' piu'");
    }
}
