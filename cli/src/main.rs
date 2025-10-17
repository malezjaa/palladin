mod cli;
mod logger;

use crate::cli::{Cli, Commands};
use crate::logger::LOGGER;
use clap::Parser;
use log::{info, LevelFilter};
use palladin_server::server::{Server, ServerConfig};
use palladin_shared::{canonicalize_with_strip, PalladinResult};
use std::env::set_current_dir;
use std::path::PathBuf;
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
            let root = canonicalize_with_strip(&root)?;
            set_current_dir(&root)?;

            let mut config = ServerConfig::new()
                .with_host(host)
                .with_port(port)
                .with_root(root)
                .with_entrypoint(entrypoint);

            let server = Arc::new(Server::new(config)?);
            info!(target: "server", "server running on http://{}", server.context().address());

            server.serve().await
        }
    }
}
