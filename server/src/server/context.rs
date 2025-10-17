use std::path::{Path, PathBuf};

use super::ServerConfig;
use fs_err::create_dir_all;
use palladin_shared::PalladinError::FileNotFound;
use palladin_shared::{canonicalize_with_strip, PalladinResult};

/// Context holds all the application-wide data including configuration,
/// canonicalized paths, and runtime state.
#[derive(Debug, Clone)]
pub struct Context {
    /// The server configuration (host, port, etc.)
    config: ServerConfig,
    /// The canonicalized root directory path
    root: PathBuf,
    /// The canonicalized build directory path (absolute path)
    build_dir: PathBuf,
    /// The path to the tsconfig.json file, if it exists
    tsconfig_path: Option<PathBuf>,
}

impl Context {
    /// Creates a new Context from the given ServerConfig.
    /// This will canonicalize the root path and locate the tsconfig.json if it exists.
    ///
    /// # Errors
    ///
    /// Returns an error if the root path cannot be canonicalized.
    pub fn new(mut config: ServerConfig) -> PalladinResult<Self> {
        let root = canonicalize_with_strip(&config.root)
            .map_err(|_| FileNotFound(config.root.to_string_lossy().to_string()))?;

        let build_dir_path = root.join(&config.build_dir);
        if !build_dir_path.exists() {
            create_dir_all(&build_dir_path)
                .map_err(|e| FileNotFound(format!("Failed to create build dir: {}", e)))?;
        }
        let build_dir = canonicalize_with_strip(&build_dir_path)
            .map_err(|_| FileNotFound(build_dir_path.to_string_lossy().to_string()))?;

        let tsconfig_path = {
            let path = root.join("tsconfig.json");
            if path.exists() { Some(path) } else { None }
        };

        let entrypoint_path = canonicalize_with_strip(&config.entrypoint)?;
        config.entrypoint = entrypoint_path;

        Ok(Self {
            config,
            root,
            build_dir,
            tsconfig_path,
        })
    }

    /// Returns a reference to the server configuration.
    #[inline(always)]
    pub fn config(&self) -> &ServerConfig {
        &self.config
    }

    /// Returns a reference to the canonicalized root directory.
    #[inline(always)]
    pub fn root(&self) -> &PathBuf {
        &self.root
    }

    /// Returns a reference to the canonicalized build directory.
    #[inline(always)]
    pub fn build_dir(&self) -> &PathBuf {
        &self.build_dir
    }

    /// Returns the host address.
    #[inline(always)]
    pub fn host(&self) -> &str {
        self.config.host()
    }

    /// Returns the port number.
    #[inline(always)]
    pub fn port(&self) -> u16 {
        self.config.port()
    }

    /// Returns the entrypoint path.
    #[inline(always)]
    pub fn entrypoint(&self) -> &PathBuf {
        &self.config.entrypoint
    }

    /// Returns the full address in the format `host:port`.
    #[inline(always)]
    pub fn address(&self) -> String {
        self.config.address()
    }

    /// Returns a reference to the tsconfig.json path if it exists.
    #[inline(always)]
    pub fn tsconfig_path(&self) -> Option<&PathBuf> {
        self.tsconfig_path.as_ref()
    }

    /// Resolves a path relative to the root directory and canonicalizes it.
    ///
    /// # Errors
    ///
    /// Returns an error if the path cannot be canonicalized.
    pub fn resolve_path<P: AsRef<Path>>(&self, path: P) -> PalladinResult<PathBuf> {
        let full_path = self.root.join(path);
        canonicalize_with_strip(&full_path)
            .map_err(|_| FileNotFound(full_path.to_string_lossy().to_string()))
    }

    /// Checks if a path is within the root directory (prevents directory traversal).
    pub fn is_within_root(&self, path: &Path) -> bool {
        path.starts_with(&self.root)
    }
}
