mod config;
mod errors;
mod files;

pub use config::*;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

pub use crate::server::config::ServerConfig;
use axum::routing::get;
use axum::Router;
use palladin_shared::PalladinResult;
use tokio::net::TcpListener;

pub struct Server {
    config: ServerConfig,
    pub files: RwLock<HashMap<PathBuf, crate::server::files::File>>,
}

impl Server {
    pub fn new(config: ServerConfig) -> Self {
        Self {
            config,
            files: RwLock::new(HashMap::new()),
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
            .route("/{*file}", get(Self::serve_file))
            .with_state(state);

        axum::serve(tcp, app).await.map_err(Into::into)
    }
}
