// SPDX-License-Identifier: MPL-2.0

use kanche::{app_with_store, config::Config, identity::Store};
use std::{process::ExitCode, time::Duration};

#[tokio::main]
async fn main() -> ExitCode {
    let config = match Config::from_env() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("Configuration error: {error}");
            return ExitCode::FAILURE;
        }
    };
    tracing_subscriber::fmt()
        .json()
        .with_max_level(config.log_level)
        .with_target(false)
        .init();

    match run(config).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            if error.kind() == std::io::ErrorKind::InvalidInput {
                tracing::error!(
                    "Database configuration invalid; use KANCHE_DATABASE_URL with an explicit loopback PostgreSQL IP; remote TLS profile is not delivered"
                );
            } else {
                tracing::error!(kind = ?error.kind(), "Server failed; check bind address, port availability, and OS permissions");
            }
            ExitCode::FAILURE
        }
    }
}

async fn run(config: Config) -> std::io::Result<()> {
    // Install signal handlers before accepting traffic; installation errors fail startup.
    #[cfg(unix)]
    let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    #[cfg(unix)]
    let mut interrupt = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?;
    let listener = tokio::net::TcpListener::bind(config.bind_addr).await?;
    tracing::info!(address = %listener.local_addr()?, "Kanche listening");
    let shutdown = async move {
        #[cfg(unix)]
        tokio::select! {
            _ = terminate.recv() => {},
            _ = interrupt.recv() => {},
        }
        #[cfg(not(unix))]
        if tokio::signal::ctrl_c().await.is_err() {
            tracing::error!("Unable to receive shutdown signal");
        }
        tracing::info!("Shutdown requested; draining requests for up to 10 seconds");
        // Bound draining even if a future handler never completes.
        tokio::spawn(async {
            tokio::time::sleep(Duration::from_secs(10)).await;
            tracing::error!("Shutdown deadline exceeded");
            std::process::exit(1);
        });
    };
    let store = match std::env::var("KANCHE_DATABASE_URL") {
        Ok(url) => Some(Store::new(&url).map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid database configuration; use an explicit loopback PostgreSQL address",
            )
        })?),
        Err(std::env::VarError::NotPresent) => None,
        Err(_) => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid database configuration",
            ));
        }
    };
    axum::serve(listener, app_with_store(store))
        .with_graceful_shutdown(shutdown)
        .await
}
