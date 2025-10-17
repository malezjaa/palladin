mod config;
mod context;
pub mod files;

use crate::file::File;
use crate::rolldown::{ChunkManager, ChunkProcessor, MainAsset, create_bundler};
pub use crate::server::config::ServerConfig;
use crate::server::files::{serve_chunk_handler, serve_file_handler, serve_index_handler};
use anyhow::anyhow;
use axum::Router;
use axum::routing::get;
pub use context::*;
use log::{error, warn};
use palladin_shared::PalladinResult;
use parking_lot::RwLock;
use rolldown::dev::{DevOptions, RebuildStrategy};
use rolldown::{BundleOutput, DevEngine};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;

pub struct Server {
    pub ctx: Arc<Context>,
    pub files: RwLock<HashMap<PathBuf, File>>,
    chunks: ChunkManager,
    entry_asset: RwLock<Option<MainAsset>>,
}

impl Server {
    pub fn new(config: ServerConfig) -> PalladinResult<Self> {
        let ctx = Arc::new(Context::new(config)?);

        Ok(Self {
            ctx: ctx.clone(),
            files: RwLock::new(HashMap::new()),
            chunks: ChunkManager::new(),
            entry_asset: RwLock::new(None),
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

    async fn spawn_engine(self: Arc<Self>) -> PalladinResult {
        let options = create_bundler(self.ctx.clone());
        let server_for_output = Arc::clone(&self);

        let dev_engine = DevEngine::new(
            options,
            DevOptions {
                rebuild_strategy: Some(RebuildStrategy::Always),
                on_output: Some(Arc::new(move |result| {
                    let server = Arc::clone(&server_for_output);
                    match result {
                        Ok(bundle_output) => {
                            for warning in &bundle_output.warnings {
                                warn!("rolldown warning: {warning:#?}");
                            }

                            if let Err(err) = server.handle_bundle_output(bundle_output) {
                                error!("failed to process rolldown output: {err:#}");
                            }
                        }
                        Err(err) => {
                            error!("rolldown build error: {err:#?}");
                        }
                    }
                })),
                on_hmr_updates: Some(Arc::new(|result| match result {
                    Ok((updates, changed_files)) => {
                        println!("HMR updates: {updates:#?} due to {changed_files:#?}");
                    }
                    Err(e) => {
                        eprintln!("HMR error: {e:#?}");
                    }
                })),
                ..Default::default()
            },
        )?;

        dev_engine.run().await?;

        dev_engine
            .wait_for_build_driver_service_close()
            .await
            .map_err(Into::into)
    }

    pub async fn serve(self: Arc<Self>) -> PalladinResult {
        let tcp = TcpListener::bind(self.ctx.address()).await?;
        let app = Router::new()
            .route("/", get(serve_index_handler))
            .route("/__chunks/{*chunk}", get(serve_chunk_handler))
            .route("/{*file}", get(serve_file_handler))
            .with_state(self.clone());

        tokio::spawn(self.spawn_engine());

        axum::serve(tcp, app).await.map_err(Into::into)
    }

    fn handle_bundle_output(self: &Arc<Self>, bundle_output: BundleOutput) -> PalladinResult {
        let entrypoint_path = self.ctx.entrypoint().clone();

        let (main_asset, chunks) =
            ChunkProcessor::process_assets(&bundle_output.assets, &entrypoint_path)
                .map_err(|err| anyhow!(err))?;

        self.chunks.clear();
        self.chunks.store_chunks(chunks);

        {
            let mut entry_asset = self.entry_asset.write();
            *entry_asset = Some(main_asset.clone());
        }

        self.apply_main_asset(&entrypoint_path, &main_asset)
    }

    fn apply_main_asset(
        self: &Arc<Self>,
        entrypoint_path: &PathBuf,
        asset: &MainAsset,
    ) -> PalladinResult {
        let _ = Self::get_or_load_file(self, entrypoint_path)?;

        let mut files = self.files.write();
        if let Some(entry) = files.get_mut(entrypoint_path) {
            entry.content.transformed = asset.content.clone();
        }

        Ok(())
    }

    pub(crate) fn entry_asset(&self) -> Option<MainAsset> {
        self.entry_asset.read().clone()
    }

    pub(crate) fn chunk_manager(&self) -> &ChunkManager {
        &self.chunks
    }
}
