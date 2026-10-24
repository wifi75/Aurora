use aurora::{run, Config};
use aurora_core::version::{APP_AUTHOR, APP_NAME, APP_VERSION};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> std::process::ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    let config = match Config::from_env() {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("{e}");
            return std::process::ExitCode::from(2);
        }
    };
    let shutdown = async {
        let _ = tokio::signal::ctrl_c().await;
        tracing::info!("spegnimento richiesto");
    };
    match run(config, shutdown).await {
        Ok(server) => {
            tracing::info!("{APP_NAME} v{APP_VERSION} - Progettazione e sviluppo: {APP_AUTHOR}");
            tracing::info!("in ascolto su http://{}", server.addr);
            server.join().await;
            std::process::ExitCode::SUCCESS
        }
        Err(e) => {
            tracing::error!("avvio fallito: {e}");
            std::process::ExitCode::from(1)
        }
    }
}
