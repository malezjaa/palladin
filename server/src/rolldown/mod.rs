mod chunks;

use crate::file::{File, FileType};
use crate::rolldown::chunks::{ChunkManager, ChunkProcessor};
use crate::server::Context;
use log::debug;
use palladin_shared::{PalladinError, PalladinResult};
use rolldown::{Bundler, BundlerOptions, ExperimentalOptions, TreeshakeOptions};
use rolldown_common::{
    AdvancedChunksOptions, MatchGroup, MatchGroupName, MatchGroupTest, OutputFormat,
};
use std::collections::HashMap;
use std::io::BufRead;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tokio::runtime::Handle;
use tokio::task::block_in_place;

pub struct RolldownPipe {
    ctx: Arc<Context>,
    chunk_manager: ChunkManager,
}

impl RolldownPipe {
    pub fn new(ctx: Arc<Context>) -> Self {
        Self {
            ctx,
            chunk_manager: ChunkManager::new(),
        }
    }

    pub fn transform(&self, file: &mut File) -> PalladinResult<()> {
        match file.ty {
            FileType::CSS | FileType::HTML => {
                file.content.transformed = file.content.original.clone();
                return Ok(());
            }
            _ => {}
        }

        let (transformed, chunks) = self.transform_with_rolldown(&file.path)?;
        file.content.transformed = transformed;
        self.chunk_manager.store_chunks(chunks);

        Ok(())
    }

    pub fn get_chunk(&self, name: &str) -> Option<String> {
        self.chunk_manager.get_chunk(name)
    }

    pub fn list_chunks(&self) -> Vec<String> {
        self.chunk_manager.list_chunks()
    }

    pub fn has_chunk(&self, name: &str) -> bool {
        self.chunk_manager.has_chunk(name)
    }

    pub fn clear_chunks(&self) {
        self.chunk_manager.clear();
    }

    fn transform_with_rolldown(
        &self,
        file_path: &PathBuf,
    ) -> PalladinResult<(String, HashMap<String, String>)> {
        let file_path = file_path.clone();
        let ctx = self.ctx.clone();

        block_in_place(|| {
            Handle::current().block_on(async move {
                let duration = Instant::now();

                let bundler_options = BundlerOptions {
                    input: Some(vec![file_path.to_string_lossy().to_string().into()]),
                    cwd: Some(ctx.root().clone()),
                    tsconfig: ctx.tsconfig_path().map(|p| p.to_string_lossy().to_string()),

                    entry_filenames: Some("[name].js".to_string().into()),
                    chunk_filenames: Some("[name]-[hash].js".to_string().into()),

                    dir: Some(ctx.build_dir().to_string_lossy().to_string()),

                    format: Some(OutputFormat::Esm),

                    treeshake: TreeshakeOptions::Boolean(true),

                    experimental: Some(ExperimentalOptions {
                        strict_execution_order: Some(true),
                        incremental_build: Some(true),
                        ..Default::default()
                    }),

                    advanced_chunks: Some(AdvancedChunksOptions {
                        groups: Some(vec![
                            MatchGroup {
                                name: MatchGroupName::Static("vendor".into()),
                                test: Some(MatchGroupTest::Regex(r#"node_modules[\\/]"#.into())),
                                min_size: Some((100 * 1024) as f64),
                                priority: Some(0),
                                ..Default::default()
                            },
                            MatchGroup {
                                name: MatchGroupName::Static("react-vendor".to_string()),
                                test: Some(MatchGroupTest::Function(Arc::new(|module_id| {
                                    let module_id = module_id.to_string();
                                    Box::pin(async move {
                                        Ok(Some(
                                            module_id.contains("node_modules")
                                                && (module_id.contains("react")
                                                    || module_id.contains("react-dom")
                                                    || module_id.contains("scheduler")),
                                        ))
                                    })
                                }))),
                                priority: Some(20),
                                min_size: None,
                                max_size: None,
                                min_share_count: None,
                                min_module_size: None,
                                max_module_size: None,
                            },
                            MatchGroup {
                                name: MatchGroupName::Static("ui-vendor".to_string()),
                                test: Some(MatchGroupTest::Function(Arc::new(|module_id| {
                                    let module_id = module_id.to_string();
                                    let test_against = ["@mui", "antd", "chakra-ui", "icons"];

                                    Box::pin(async move {
                                        Ok(Some(
                                            module_id.contains("node_modules")
                                                && test_against
                                                    .iter()
                                                    .any(|t| module_id.contains(t)),
                                        ))
                                    })
                                }))),
                                priority: Some(15),
                                min_size: None,
                                max_size: None,
                                min_share_count: None,
                                min_module_size: None,
                                max_module_size: None,
                            },
                        ]),
                        ..Default::default()
                    }),

                    ..Default::default()
                };

                let mut bundler =
                    Bundler::new(bundler_options).map_err(PalladinError::RolldownError)?;

                let result = bundler
                    .write()
                    .await
                    .map_err(PalladinError::RolldownError)?;

                let end = Instant::now();
                debug!(
                    "Rolldown bundled {} asset(s) in {:.2}ms",
                    result.assets.len(),
                    end.duration_since(duration).as_millis()
                );

                ChunkProcessor::process_assets(&result.assets, &file_path)
                    .map_err(|e| PalladinError::FileNotFound(e))
            })
        })
    }
}
