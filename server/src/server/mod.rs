mod config;
mod context;
pub mod files;

use crate::file::File;
use crate::hmr::{create_hmr_channel, ws_handler, HmrBroadcaster, HmrMessage, Update};
use crate::rolldown::RolldownPipe;
pub use crate::server::config::ServerConfig;
use crate::server::files::{serve_chunk_handler, serve_file_handler, serve_index_handler};
use crate::watcher::FileWatcher;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
pub use context::*;
use log::{debug, info};
use palladin_shared::PalladinResult;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::net::TcpListener;
use tokio::time::sleep;

pub struct Server {
    pub ctx: Arc<Context>,
    pub files: RwLock<HashMap<PathBuf, File>>,
    pub rolldown_pipe: RolldownPipe,
    pub hmr_tx: HmrBroadcaster,
}

impl Server {
    pub fn new(config: ServerConfig) -> PalladinResult<Self> {
        let ctx = Arc::new(Context::new(config)?);

        Ok(Self {
            ctx: ctx.clone(),
            files: RwLock::new(HashMap::new()),
            rolldown_pipe: RolldownPipe::new(ctx),
            hmr_tx: create_hmr_channel(),
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

    pub async fn serve(self: Arc<Self>) -> PalladinResult {
        let tcp = TcpListener::bind(self.ctx.address()).await?;
        let app = Router::new()
            .route("/", get(serve_index_handler))
            .route("/__hmr", get(ws_handler))
            .route("/__chunks/{*chunk}", get(serve_chunk_handler))
            .route("/{*file}", get(serve_file_handler))
            .with_state(self);

        axum::serve(tcp, app).await.map_err(Into::into)
    }

    pub fn create_watcher(&self) -> PalladinResult<FileWatcher> {
        let mut watcher = FileWatcher::new()?;
        let build_dir = self.ctx.build_dir();
        debug!("Adding ignored path: {:?}", build_dir);
        watcher.add_ignored_path(build_dir)?;
        Ok(watcher)
    }

    pub async fn watch_files(self: &Arc<Self>, watcher: FileWatcher) {
        debug!(
            "File watcher started. Ignored paths: {:?}",
            watcher.ignored_paths()
        );

        loop {
            watcher.process_filtered_events(|event| {
                use notify::EventKind;

                match event.kind {
                    EventKind::Create(_) | EventKind::Modify(_) => {
                        for path in event.paths {
                            if self.ctx.is_within_root(&path) {
                                self.invalidate_file(&path);
                            }
                        }
                    }
                    EventKind::Remove(_) => {
                        for path in event.paths {
                            if self.ctx.is_within_root(&path) {
                                self.remove_file(&path);
                            }
                        }
                    }
                    _ => {}
                }
            });

            sleep(Duration::from_millis(50)).await;
        }
    }

    fn invalidate_file(&self, path: &PathBuf) {
        let relative_path = self
            .ctx
            .root()
            .parent()
            .and_then(|root| path.strip_prefix(root).ok())
            .unwrap_or(path);

        let mut files = self.files.write();
        if let Some(file) = files.get_mut(path) {
            file.dirty = true;
            debug!("File invalidated: {:?}", relative_path);
        } else {
            files.remove(path);
            debug!("File removed from cache: {:?}", relative_path);
        }
        drop(files);

        self.rolldown_pipe.clear_chunks();

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let _ = self.hmr_tx.send(HmrMessage::Update {
            updates: vec![Update {
                path: format!("/{}", relative_path.display()).replace('\\', "/"),
                timestamp,
            }],
        });

        info!("File changed: {}", relative_path.display());
    }

    fn remove_file(&self, path: &PathBuf) {
        let mut files = self.files.write();
        files.remove(path);
        drop(files);

        self.rolldown_pipe.clear_chunks();

        let _ = self.hmr_tx.send(HmrMessage::FullReload);

        info!("File removed: {:?}", path);
    }
}
