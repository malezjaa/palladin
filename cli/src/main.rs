mod cli;
mod logger;

use crate::cli::{Cli, Commands};
use crate::logger::LOGGER;
use clap::{Parser, Subcommand};
use log::info;
use palladin_server::server::{Server, ServerConfig};
use palladin_shared::PalladinResult;

#[tokio::main]
async fn main() -> PalladinResult {
    log::set_logger(&LOGGER)
        .map(|()| log::set_max_level(log::LevelFilter::Debug))
        .expect("Failed to set logger");

    let cli = Cli::parse();

    match cli.command {
        Commands::Dev { host, port, root } => {
            let config = ServerConfig::new()
                .with_host(host)
                .with_port(port)
                .with_root(root);
            info!(target: "server", "initializing...");

            let mut server = Server::new(config);
            info!(target: "server", "server running on http://{}", server.config().address());

            server.serve().await
        }
    }
}
