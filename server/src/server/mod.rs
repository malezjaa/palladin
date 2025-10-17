mod config;
mod context;
pub mod files;

use crate::file::File;
use crate::hmr::{create_hmr_channel, ws_handler, HmrBroadcaster, HmrMessage, Update};
use crate::rolldown::RolldownPipe;
pub use crate::server::config::ServerConfig;
use crate::server::files::{serve_chunk_handler, serve_file_handler, serve_index_handler};
use crate::watcher::FileWatcher;
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

        let server = Self {
            ctx: ctx.clone(),
            files: RwLock::new(HashMap::new()),
            rolldown_pipe: RolldownPipe::new(ctx),
            hmr_tx: create_hmr_channel(),
        };

        info!("Bundling entrypoint...");
        server.rolldown_pipe.bundle_entrypoint()?;
        info!("Entrypoint bundled successfully");

        Ok(server)
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

        let mut pending_changes = std::collections::HashSet::new();
        let mut last_change_time: Option<SystemTime> = None;
        let debounce_duration = Duration::from_millis(100);

        loop {
            watcher.process_filtered_events(|event| {
                use notify::EventKind;

                match event.kind {
                    EventKind::Create(_) | EventKind::Modify(_) => {
                        for path in event.paths {
                            if self.ctx.is_within_root(&path) {
                                pending_changes.insert(path);
                                last_change_time = Some(SystemTime::now());
                            }
                        }
                    }
                    _ => {}
                }
            });

            if let Some(last_time) = last_change_time {
                if !pending_changes.is_empty() {
                    let elapsed = SystemTime::now()
                        .duration_since(last_time)
                        .unwrap_or(Duration::ZERO);

                    if elapsed >= debounce_duration {
                        let changed_files: Vec<PathBuf> = pending_changes.drain().collect();
                        if !changed_files.is_empty() {
                            self.handle_file_changes(changed_files);
                        }
                        last_change_time = None;
                    }
                }
            }

            sleep(Duration::from_millis(10)).await;
        }
    }

    fn handle_file_changes(&self, paths: Vec<PathBuf>) {
        for path in &paths {
            let relative_path = self
                .ctx
                .root()
                .parent()
                .and_then(|root| path.strip_prefix(root).ok())
                .unwrap_or(path);
            info!("File changed: {}", relative_path.display());
        }

        info!("Rebuilding entrypoint...");
        if let Err(e) = self.rolldown_pipe.bundle_entrypoint() {
            log::error!("Failed to rebuild entrypoint: {:?}", e);
            return;
        }
        info!("Entrypoint rebuilt successfully");

        self.files.write().clear();

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let updates: Vec<Update> = paths
            .iter()
            .filter_map(|path| {
                self.ctx
                    .root()
                    .parent()
                    .and_then(|root| path.strip_prefix(root).ok())
                    .map(|p| Update {
                        path: format!("/{}", p.to_string_lossy().replace('\\', "/")),
                        timestamp,
                    })
            })
            .collect();

        if !updates.is_empty() {
            let _ = self.hmr_tx.send(HmrMessage::Update { updates });
        }
    }
}
