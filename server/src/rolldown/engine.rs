use std::{
    collections::VecDeque,
    ops::Deref,
    path::PathBuf,
    sync::{Arc, atomic::AtomicBool},
};

use anyhow::Context;
use arcstr::ArcStr;
use futures::{FutureExt, future::Shared};
use palladin_shared::{PalladinError, PalladinResult};
use rolldown::dev::{
    DevOptions, SharedClients,
    build_driver::{BuildDriver, SharedBuildDriver},
    build_driver_service::{BuildDriverService, BuildMessage},
    build_state_machine::BuildStateMachine,
    building_task::TaskInput,
    dev_context::{DevContext, PinBoxSendStaticFuture, SharedDevContext},
    dev_options::normalize_dev_options,
};
use rolldown::{Bundler, BundlerBuilder};
use rolldown_common::ClientHmrUpdate;
use rolldown_error::ResultExt;
use rolldown_utils::{dashmap::FxDashSet, indexmap::FxIndexSet};
use rolldown_watcher::{
    DebouncedPollWatcher, DebouncedRecommendedWatcher, DynWatcher, NoopWatcher, PollWatcher,
    RecommendedWatcher, RecursiveMode, Watcher, WatcherConfig, WatcherExt,
};
use sugar_path::SugarPath;
use tokio::sync::{Mutex, mpsc::unbounded_channel};

struct BuildDriverServiceState {
    service: Option<BuildDriverService>,
    handle: Option<Shared<PinBoxSendStaticFuture<()>>>,
}

pub struct DevEngine {
    build_driver: SharedBuildDriver,
    watcher: Mutex<DynWatcher>,
    watched_files: FxDashSet<ArcStr>,
    build_driver_service_state: Mutex<BuildDriverServiceState>,
    ctx: SharedDevContext,
    pub clients: SharedClients,
    is_closed: AtomicBool,
}

impl DevEngine {
    pub fn new(bundler_builder: BundlerBuilder, options: DevOptions) -> PalladinResult<Self> {
        let bundler = Arc::new(Mutex::new(bundler_builder.build()?));
        Self::with_bundler(bundler, options)
    }

    pub fn with_bundler(bundler: Arc<Mutex<Bundler>>, options: DevOptions) -> PalladinResult<Self> {
        let normalized_options = normalize_dev_options(options);
        let (build_channel_tx, build_channel_rx) = unbounded_channel();
        let clients = SharedClients::default();

        let ctx = Arc::new(DevContext {
            state: Mutex::new(BuildStateMachine {
                queued_tasks: VecDeque::from([TaskInput::new_initial_build_task()]),
                ..BuildStateMachine::new()
            }),
            options: normalized_options,
            build_channel_tx,
            clients: Arc::clone(&clients),
        });

        let build_driver = Arc::new(BuildDriver::new(bundler, Arc::clone(&ctx)));

        let build_driver_service = BuildDriverService::new(
            Arc::clone(&build_driver),
            Arc::clone(&ctx),
            build_channel_rx,
        );

        let watcher_config = WatcherConfig {
            poll_interval: ctx.options.poll_interval,
            debounce_delay: ctx.options.debounce_duration,
            compare_contents_for_polling: ctx.options.compare_contents_for_polling,
            debounce_tick_rate: ctx.options.debounce_tick_rate,
        };

        let watcher = Self::create_watcher(&ctx, &build_driver_service, watcher_config)?;

        Ok(Self {
            build_driver,
            watcher: Mutex::new(watcher),
            watched_files: FxDashSet::default(),
            build_driver_service_state: Mutex::new(BuildDriverServiceState {
                service: Some(build_driver_service),
                handle: None,
            }),
            ctx,
            clients,
            is_closed: AtomicBool::new(false),
        })
    }

    fn create_watcher(
        ctx: &SharedDevContext,
        build_driver_service: &BuildDriverService,
        config: WatcherConfig,
    ) -> PalladinResult<DynWatcher> {
        let event_handler = build_driver_service.create_watcher_event_handler();

        if ctx.options.disable_watcher {
            return NoopWatcher::with_config(event_handler, config)
                .map(|w| w.into_dyn_watcher())
                .map_err(|e| PalladinError::Watcher(e.to_string()));
        }

        let watcher = match (ctx.options.use_polling, ctx.options.use_debounce) {
            (true, false) => PollWatcher::with_config(event_handler, config)
                .map_err(|e| PalladinError::Watcher(e.to_string()))?
                .into_dyn_watcher(),
            (true, true) => DebouncedPollWatcher::with_config(event_handler, config)
                .map_err(|e| PalladinError::Watcher(e.to_string()))?
                .into_dyn_watcher(),
            (false, false) => RecommendedWatcher::with_config(event_handler, config)
                .map_err(|e| PalladinError::Watcher(e.to_string()))?
                .into_dyn_watcher(),
            (false, true) => DebouncedRecommendedWatcher::with_config(event_handler, config)
                .map_err(|e| PalladinError::Watcher(e.to_string()))?
                .into_dyn_watcher(),
        };

        Ok(watcher)
    }

    pub async fn run(&self) -> PalladinResult<()> {
        let mut service_state = self.build_driver_service_state.lock().await;

        if service_state.service.is_none() {
            return Ok(());
        }

        self.build_driver
            .ensure_latest_build_output()
            .await
            .context("Failed to ensure latest build output")?;

        if let Some(service) = service_state.service.take() {
            let handle = tokio::spawn(service.run());
            let future = Box::pin(async move {
                handle.await.unwrap();
            }) as PinBoxSendStaticFuture;
            service_state.handle = Some(future.shared());
        }
        drop(service_state);

        self.watch_bundler_files().await?;
        Ok(())
    }

    async fn watch_bundler_files(&self) -> PalladinResult<()> {
        let bundler = self.build_driver.bundler.lock().await;
        let watch_files = bundler.get_watch_files();

        let mut watcher = self.watcher.lock().await;
        let mut paths = watcher.paths_mut();

        for file in watch_files.iter().map(|s| s.deref().clone()) {
            tracing::trace!("Watching file: {:?}", file);

            if self.watched_files.insert(file.to_string().into()) {
                paths
                    .add(file.as_path(), RecursiveMode::NonRecursive)
                    .map_err(|e| {
                        PalladinError::Watcher(format!("Failed to watch {}: {}", file, e))
                    })?;
            }
        }

        paths.commit().map_err(|e| {
            PalladinError::Watcher(format!("Failed to commit watched paths: {}", e))
        })?;

        Ok(())
    }

    pub async fn wait_for_service_close(&self) -> PalladinResult<()> {
        self.ensure_not_closed()?;

        let service_state = self.build_driver_service_state.lock().await;
        if let Some(handle) = service_state.handle.clone() {
            handle.await;
        }

        Ok(())
    }

    pub async fn ensure_build_finished(&self) -> PalladinResult<()> {
        self.ensure_not_closed()?;
        self.ctx.ensure_current_build_finish().await;
        Ok(())
    }

    pub async fn invalidate(
        &self,
        caller: String,
        first_invalidated_by: Option<String>,
    ) -> PalladinResult<Vec<ClientHmrUpdate>> {
        self.ensure_not_closed()?;
        Ok(self
            .build_driver
            .invalidate(caller, first_invalidated_by)
            .await?)
    }

    pub async fn close(&self) -> PalladinResult<()> {
        if self
            .is_closed
            .swap(true, std::sync::atomic::Ordering::SeqCst)
        {
            return Ok(());
        }

        self.ctx
            .build_channel_tx
            .send(BuildMessage::Close)
            .map_err_to_unhandleable()
            .map_err(|e| {
                PalladinError::ServiceCommunication(format!("Failed to close service: {}", e))
            })?;

        let watcher = std::mem::replace(
            &mut *self.watcher.lock().await,
            NoopWatcher.into_dyn_watcher(),
        );
        drop(watcher);

        self.build_driver.bundler.lock().await.close().await?;

        let service_state = self.build_driver_service_state.lock().await;
        if let Some(handle) = service_state.handle.clone() {
            handle.await;
        }

        Ok(())
    }

    pub fn is_closed(&self) -> bool {
        self.is_closed.load(std::sync::atomic::Ordering::SeqCst)
    }

    fn ensure_not_closed(&self) -> PalladinResult<()> {
        if self.is_closed() {
            Err(PalladinError::EngineClosed)
        } else {
            Ok(())
        }
    }
}

impl Deref for DevEngine {
    type Target = BuildDriver;

    fn deref(&self) -> &Self::Target {
        &self.build_driver
    }
}
