mod config;
mod context;
pub mod files;

pub use config::*;
pub use context::*;
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
    pub ctx: Arc<Context>,
    pub files: RwLock<HashMap<PathBuf, File>>,
    pub rolldown_pipe: RolldownPipe,
}

impl Server {
    pub fn new(config: ServerConfig) -> PalladinResult<Self> {
        let ctx = Arc::new(Context::new(config)?);

        Ok(Self {
            ctx: ctx.clone(),
            files: RwLock::new(HashMap::new()),
            rolldown_pipe: RolldownPipe::new(ctx),
        })
    }

    #[inline(always)]
    pub fn context(&self) -> &Arc<Context> {
        &self.ctx
    }

    #[inline(always)]
    pub fn config(&self) -> &ServerConfig {
        self.ctx.config()
    }

    pub async fn serve(self) -> PalladinResult {
        let tcp = TcpListener::bind(self.ctx.address()).await?;
        let state = Arc::new(self);
        let app = Router::new()
            .route("/", get(serve_index_handler))
            .route("/{*file}", get(serve_file_handler))
            .with_state(state);

        axum::serve(tcp, app).await.map_err(Into::into)
    }
}
