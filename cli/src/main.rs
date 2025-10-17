mod cli;
mod logger;

use crate::cli::{Cli, Commands};
use crate::logger::LOGGER;
use clap::Parser;
use log::{info, LevelFilter};
use palladin_server::server::{Server, ServerConfig};
use palladin_shared::{canonicalize_with_strip, PalladinResult};
use std::sync::Arc;

#[tokio::main]
async fn main() -> PalladinResult {
    let cli = Cli::parse();

    // Map verbosity count (-v, -vv, -vvv) to log levels
    let log_level = match cli.verbose {
        0 => LevelFilter::Info,
        1 => LevelFilter::Debug,
        2 => LevelFilter::Trace,
        _ => LevelFilter::Trace,
    };

    log::set_logger(&LOGGER)
        .map(|()| log::set_max_level(log_level))
        .expect("Failed to set logger");

    match cli.command {
        Commands::Dev {
            host,
            port,
            root,
            entrypoint,
        } => {
            let mut config = ServerConfig::new()
                .with_host(host)
                .with_port(port)
                .with_root(root);

            if let Some(entry) = entrypoint {
                config = config.with_entrypoint(canonicalize_with_strip(entry)?);
            }

            info!(target: "server", "initializing...");

            let server = Server::new(config)?;

            let mut watcher = server.create_watcher()?;
            watcher.watch(server.context().root())?;

            info!(target: "server", "server running on http://{}", server.context().address());
            info!(target: "server", "watching for file changes...");

            let server = Arc::new(server);

            let watcher_server = server.clone();
            tokio::spawn(async move {
                watcher_server.watch_files(watcher).await;
            });

            server.serve().await
        }
    }
}
