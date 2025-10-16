use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use palladin_shared::{canonicalize_with_strip, PalladinResult};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

/// A file watcher that monitors file system changes with filtering capabilities.
///
/// The watcher can filter events by:
/// - File extensions (only processes allowed extensions)
/// - Ignored paths (excludes specific directories using canonicalized paths)
/// - Temporary/backup files (automatically filtered)
///
/// # Example
///
/// ```no_run
/// use palladin_server::watcher::FileWatcher;
/// use std::path::PathBuf;
///
/// let mut watcher = FileWatcher::new().unwrap();
///
/// // Add the build directory to ignored paths (uses canonical path comparison)
/// watcher.add_ignored_path(PathBuf::from("./dist")).unwrap();
///
/// // Watch the current directory
/// watcher.watch(".").unwrap();
///
/// // Process only filtered events (ignoring dist directory)
/// watcher.process_filtered_events(|event| {
///     println!("File changed: {:?}", event);
/// });
/// ```
pub struct FileWatcher {
    watcher: RecommendedWatcher,
    rx: Receiver<Result<Event, notify::Error>>,
    watched_paths: HashSet<PathBuf>,
    allowed_extensions: HashSet<String>,
    ignored_paths: Vec<PathBuf>,
}

impl FileWatcher {
    pub fn new() -> PalladinResult<Self> {
        Self::with_poll_interval(Duration::from_millis(100))
    }

    /// Creates a new FileWatcher with a custom poll interval.
    pub fn with_poll_interval(poll_interval: Duration) -> PalladinResult<Self> {
        let (tx, rx) = channel();

        let watcher = RecommendedWatcher::new(
            move |res| {
                let _ = tx.send(res);
            },
            Config::default().with_poll_interval(poll_interval),
        )?;

        let allowed_extensions = [
            // JavaScript/TypeScript
            "js", "jsx", "ts", "tsx", "mjs", "cjs", // Styles
            "css", "scss", "sass", "less", "styl", "stylus", "pcss", "postcss",
            // Vue/Svelte
            "vue", "svelte", // HTML
            "html", "htm", // JSON
            "json", "json5", // Images
            "png", "jpg", "jpeg", "gif", "svg", "webp", "avif", "ico", // Fonts
            "woff", "woff2", "ttf", "otf", "eot", // Other assets
            "webm", "mp4", "mp3", "wav", "ogg", "pdf", "txt", "md",
        ]
        .iter()
        .map(|&s| s.to_string())
        .collect();

        Ok(Self {
            watcher,
            rx,
            watched_paths: HashSet::new(),
            allowed_extensions,
            ignored_paths: Vec::new(),
        })
    }

    pub fn watch<P: AsRef<Path>>(&mut self, path: P) -> PalladinResult {
        let path = path.as_ref().to_path_buf();
        self.watcher.watch(&path, RecursiveMode::Recursive)?;
        self.watched_paths.insert(path);
        Ok(())
    }

    pub fn unwatch<P: AsRef<Path>>(&mut self, path: P) -> PalladinResult {
        let path = path.as_ref();
        self.watcher.unwatch(path)?;
        self.watched_paths.remove(path);
        Ok(())
    }

    /// Gets the next file system event (non-blocking).
    ///
    /// Returns `None` if no events are available.
    pub fn try_recv_event(&self) -> Option<Result<Event, notify::Error>> {
        self.rx.try_recv().ok()
    }

    /// Gets the next file system event (blocking).
    ///
    /// Blocks until an event is available or the channel is disconnected.
    pub fn recv_event(&self) -> Result<Event, notify::Error> {
        self.rx
            .recv()
            .map_err(|_| notify::Error::generic("Channel disconnected"))?
    }

    pub fn watched_paths(&self) -> Vec<PathBuf> {
        self.watched_paths.iter().cloned().collect()
    }

    fn is_allowed_file(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| self.allowed_extensions.contains(&ext.to_lowercase()))
            .unwrap_or(false)
    }

    fn is_ignored_path(&self, path: &Path) -> bool {
        // Check for temporary/backup file patterns
        if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
            if self.is_temporary_file(file_name) {
                return true;
            }
        }

        // Check against ignored paths
        if let Ok(canonical) = canonicalize_with_strip(path) {
            self.ignored_paths
                .iter()
                .any(|ignored| canonical.starts_with(ignored))
        } else {
            false
        }
    }

    #[inline]
    fn is_temporary_file(&self, file_name: &str) -> bool {
        // Editor backup files
        if file_name.ends_with('~')
            || file_name.ends_with(".swp")
            || file_name.ends_with(".swo")
            || file_name.ends_with(".swx")
        {
            return true;
        }

        // Emacs auto-save files
        if file_name.starts_with('#') && file_name.ends_with('#') {
            return true;
        }

        // Temporary files
        if file_name.starts_with(".~")
            || file_name.ends_with(".tmp")
            || file_name.ends_with(".temp")
        {
            return true;
        }

        // Hidden temporary files
        if file_name.starts_with('.') && (file_name.contains(".swp") || file_name.contains(".tmp"))
        {
            return true;
        }

        // JetBrains IDE files
        if file_name.starts_with("___")
            || file_name.ends_with("___jb_tmp___")
            || file_name.ends_with("___jb_old___")
        {
            return true;
        }

        // Backup files
        file_name.ends_with(".bak") || file_name.ends_with(".backup")
    }

    /// Adds a path to the ignored paths list.
    ///
    /// The path will be canonicalized before being added. If the path doesn't exist,
    /// it will be silently skipped.
    pub fn add_ignored_path<P: AsRef<Path>>(&mut self, path: P) -> PalladinResult {
        let path_ref = path.as_ref();

        if path_ref.exists() {
            let canonical = canonicalize_with_strip(path_ref)?;
            if !self.ignored_paths.contains(&canonical) {
                self.ignored_paths.push(canonical);
            }
        }
        Ok(())
    }

    pub fn ignored_paths(&self) -> &[PathBuf] {
        &self.ignored_paths
    }

    pub fn remove_ignored_path<P: AsRef<Path>>(&mut self, path: P) -> PalladinResult {
        let path_ref = path.as_ref();
        let canonical = canonicalize_with_strip(path_ref)?;
        self.ignored_paths.retain(|p| p != &canonical);
        Ok(())
    }

    pub fn clear_ignored_paths(&mut self) {
        self.ignored_paths.clear();
    }

    pub fn process_filtered_events<F>(&self, mut callback: F)
    where
        F: FnMut(Event),
    {
        while let Some(res) = self.try_recv_event() {
            match res {
                Ok(event) => {
                    let should_process = event
                        .paths
                        .iter()
                        .any(|path| !self.is_ignored_path(path) && self.is_allowed_file(path));

                    if should_process {
                        callback(event);
                    }
                }
                Err(e) => eprintln!("Watch error: {:?}", e),
            }
        }
    }

    pub fn add_extension(&mut self, ext: &str) {
        let ext = ext.trim_start_matches('.').to_lowercase();
        self.allowed_extensions.insert(ext);
    }

    pub fn remove_extension(&mut self, ext: &str) {
        let ext = ext.trim_start_matches('.').to_lowercase();
        self.allowed_extensions.remove(&ext);
    }

    pub fn clear_extensions(&mut self) {
        self.allowed_extensions.clear();
    }

    pub fn allowed_extensions(&self) -> Vec<String> {
        self.allowed_extensions.iter().cloned().collect()
    }
}

impl Default for FileWatcher {
    fn default() -> Self {
        Self::new().expect("Failed to create FileWatcher")
    }
}
