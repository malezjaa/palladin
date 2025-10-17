mod chunks;
mod engine;

use crate::server::Context;
use rolldown::{BundlerBuilder, BundlerOptions, ExperimentalOptions, TreeshakeOptions};
use rolldown_common::{
    AdvancedChunksOptions, MatchGroup, MatchGroupName, MatchGroupTest, OutputFormat,
};
use std::sync::Arc;

pub use chunks::{ChunkManager, ChunkProcessor, MainAsset};

pub fn create_bundler(ctx: Arc<Context>) -> BundlerBuilder {
    BundlerBuilder::default().with_options(BundlerOptions {
        input: Some(vec![ctx.entrypoint().to_string_lossy().to_string().into()]),
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
                                    && test_against.iter().any(|t| module_id.contains(t)),
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
    })
}
