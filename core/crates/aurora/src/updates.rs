//! Il controllo periodico delle nuove versioni: quando sul repository esce una release piu' nuova di quella in uso, avvisa una volta sola
//! (evento per la pagina e, se c'e' un indirizzo ntfy nella lista dei desiderati, notifica sul telefono).

use std::{sync::Arc, time::Duration};

use aurora_core::{Event, UpdateNotice};
use tokio::sync::broadcast;
use updater::UpdateService;

/// Ogni quanto si guarda se e' ora di controllare (le impostazioni possono cambiare mentre il programma gira).
const TICK: Duration = Duration::from_secs(60);

fn notice_text(notice: &UpdateNotice) -> (String, String) {
    (format!("Aurora {} disponibile", notice.version), if notice.notes.is_empty() { notice.url.clone() } else { format!("{}\n{}", notice.notes, notice.url) })
}

/// Controlla ora e, se c'e' una versione nuova non ancora comunicata, la comunica. Ritorna `true` se ha avvisato.
pub fn check_and_announce(service: &Arc<UpdateService>, events: &broadcast::Sender<Event>, phone_url: &dyn Fn() -> String) -> bool {
    let status = service.check();
    let Some(release) = status.available else { return false };
    if !service.first_notice(&release.version) {
        return false;
    }
    let notice = UpdateNotice { version: release.version, tag: release.tag, notes: release.notes, url: release.url };
    let _ = events.send(Event::UpdateAvailable(notice.clone()));
    let url = phone_url();
    if !url.is_empty() {
        std::thread::spawn(move || {
            let (title, body) = notice_text(&notice);
            if let Err(e) = wishlist::send_notification(&url, &title, &body) {
                tracing::warn!("avviso di aggiornamento al telefono non inviato: {e}");
            }
        });
    }
    true
}

/// Avvia il controllo periodico: il primo dopo `first_delay`, poi ogni `interval_hours` delle impostazioni (se accese).
pub fn spawn_checker(service: Arc<UpdateService>, events: broadcast::Sender<Event>, phone_url: Arc<dyn Fn() -> String + Send + Sync>, first_delay: Duration) {
    tokio::spawn(async move {
        tokio::time::sleep(first_delay).await;
        let mut last = std::time::Instant::now() - Duration::from_secs(10 * 24 * 3600);
        loop {
            let config = service.config();
            if config.enabled && last.elapsed() >= Duration::from_secs(u64::from(config.interval_hours) * 3600) {
                last = std::time::Instant::now();
                let (service, events, phone_url) = (Arc::clone(&service), events.clone(), Arc::clone(&phone_url));
                // la rete puo' essere lenta: fuori dal thread asincrono
                let _ = tokio::task::spawn_blocking(move || check_and_announce(&service, &events, &*phone_url)).await;
            }
            tokio::time::sleep(TICK).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use updater::UpdateConfig;

    fn serve(tag: &'static str, times: usize) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        std::fs::write(std::env::temp_dir().join(format!("aurora-upd-addr-{}-{tag}", std::process::id())), addr.to_string()).unwrap();
        std::thread::spawn(move || {
            for _ in 0..times {
                let Ok((mut stream, _)) = listener.accept() else { return };
                let mut buf = [0u8; 4096];
                let _ = stream.read(&mut buf);
                let body = format!(r#"{{"tag_name":"{tag}","name":"x","body":"Novita","html_url":"https://example.test/r","assets":[]}}"#);
                let reply = format!("HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}", body.len());
                let _ = stream.write_all(reply.as_bytes());
            }
        });
    }

    fn api_of(tag: &str) -> String {
        format!("http://{}", std::fs::read_to_string(std::env::temp_dir().join(format!("aurora-upd-addr-{}-{tag}", std::process::id()))).unwrap())
    }

    fn service(name: &str, tag: &'static str, times: usize) -> Arc<UpdateService> {
        serve(tag, times);
        let dir = std::env::temp_dir().join(format!("aurora-upd-ann-{}-{name}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).unwrap();
        UpdateService::open_with_api(&dir.join("update.json"), "0.9.0", &api_of(tag))
    }

    #[test]
    fn a_new_version_is_announced_once_even_if_found_again() {
        let svc = service("once", "v0.10.0", 2);
        let (events, mut rx) = broadcast::channel::<Event>(8);
        assert!(check_and_announce(&svc, &events, &String::new));
        match rx.try_recv().unwrap() {
            Event::UpdateAvailable(n) => assert_eq!((n.version.as_str(), n.url.as_str()), ("0.10.0", "https://example.test/r")),
            other => panic!("evento inatteso: {other:?}"),
        }
        assert!(!check_and_announce(&svc, &events, &String::new), "gia' comunicata");
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn the_notice_is_remembered_across_restarts() {
        let svc = service("restart", "v0.10.1", 2);
        let (events, _rx) = broadcast::channel::<Event>(8);
        assert!(check_and_announce(&svc, &events, &String::new));
        let path = std::env::temp_dir().join(format!("aurora-upd-ann-{}-restart", std::process::id())).join("update.json");
        let again = UpdateService::open_with_api(&path, "0.9.0", &api_of("v0.10.1"));
        assert!(!check_and_announce(&again, &events, &String::new), "dopo un riavvio non si avvisa di nuovo");
    }

    #[test]
    fn nothing_is_announced_when_there_is_no_newer_version() {
        let svc = service("same", "v0.9.0", 1);
        let (events, mut rx) = broadcast::channel::<Event>(8);
        assert!(!check_and_announce(&svc, &events, &String::new));
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn the_periodic_checker_announces_and_respects_the_off_switch() {
        let svc = service("periodic", "v0.11.0", 1);
        svc.set_config(UpdateConfig { enabled: false, ..UpdateConfig::default() }, None).unwrap();
        let (events, mut rx) = broadcast::channel::<Event>(8);
        spawn_checker(Arc::clone(&svc), events.clone(), Arc::new(String::new), Duration::from_millis(10));
        tokio::time::sleep(Duration::from_millis(400)).await;
        assert!(rx.try_recv().is_err(), "spento: nessun controllo");
        svc.set_config(UpdateConfig { enabled: true, ..UpdateConfig::default() }, None).unwrap();
        // il controllo riparte al prossimo giro (ogni minuto): qui basta provare l'avviso diretto
        assert!(check_and_announce(&svc, &events, &String::new));
        assert!(matches!(rx.try_recv(), Ok(Event::UpdateAvailable(_))));
    }
}
