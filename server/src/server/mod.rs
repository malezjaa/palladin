mod config;
mod errors;
pub mod files;

pub use config::*;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use crate::file::File;
use crate::rolldown::RolldownPipe;
pub use crate::server::config::ServerConfig;
use crate::server::files::{serve_file_handler, serve_index_handler};
use axum::routing::get;
use axum::Router;
use palladin_shared::PalladinResult;
use tokio::net::TcpListener;

pub struct Server {
    config: ServerConfig,
    pub files: RwLock<HashMap<PathBuf, File>>,
    pub rolldown_pipe: RolldownPipe,
}

impl Server {
    pub fn new(config: ServerConfig) -> Self {
        let root = config.root.clone();

        Self {
            config,
            files: RwLock::new(HashMap::new()),
            rolldown_pipe: RolldownPipe::new(root),
        }
    }

    #[inline(always)]
    pub fn config(&self) -> &ServerConfig {
        &self.config
    }

    pub async fn serve(self) -> PalladinResult {
        let tcp = TcpListener::bind(self.config.address()).await?;
        let state = Arc::new(self);
        let app = Router::new()
            .route("/", get(serve_index_handler))
            .route("/{*file}", get(serve_file_handler))
            .with_state(state);

        axum::serve(tcp, app).await.map_err(Into::into)
    }
}
