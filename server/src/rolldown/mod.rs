use crate::file::{File, FileType};
use crate::server::Context;
use log::debug;
use palladin_shared::{PalladinError, PalladinResult};
use rolldown::{Bundler, BundlerOptions};
use std::env::current_dir;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tokio::runtime::Handle;
use tokio::task::block_in_place;

pub struct RolldownPipe {
    ctx: Arc<Context>,
}

impl RolldownPipe {
    pub fn new(ctx: Arc<Context>) -> Self {
        Self { ctx }
    }

    pub fn transform(&self, file: &mut File) -> PalladinResult<()> {
        match file.ty {
            FileType::CSS | FileType::HTML => {
                file.content.transformed = file.content.original.clone();
                return Ok(());
            }
            _ => {}
        }

        let transformed = self.transform_with_rolldown(&file.path)?;
        file.content.transformed = transformed;

        Ok(())
    }

    fn transform_with_rolldown(&self, file_path: &PathBuf) -> PalladinResult<String> {
        let file_path = file_path.clone();

        block_in_place(|| {
            Handle::current().block_on(async move {
                let duration = Instant::now();
                let bundler_options = BundlerOptions {
                    input: Some(vec![file_path.to_string_lossy().to_string().into()]),
                    cwd: Some(self.ctx.root().clone()),
                    tsconfig: self
                        .ctx
                        .tsconfig_path()
                        .map(|p| p.to_string_lossy().to_string()),
                    ..Default::default()
                };

                let mut bundler =
                    Bundler::new(bundler_options).map_err(|e| PalladinError::RolldownError(e))?;

                let result = bundler
                    .write()
                    .await
                    .map_err(|e| PalladinError::RolldownError(e))?;

                for asset in &result.assets {
                    let end = Instant::now();
                    debug!(
                        "Rolldown took {:.2}ms",
                        end.duration_since(duration).as_millis()
                    );

                    return Ok(String::from_utf8(asset.content_as_bytes().to_vec())?);
                }

                panic!("No output chunk found");
            })
        })
    }
}
