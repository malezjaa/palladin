use std::path::PathBuf;

/// Configuration for the server, including host and port settings.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// The IP address or hostname where the server will bind.
    pub host: String,
    /// The TCP port on which the server will listen to.
    pub port: u16,
    /// The root directory from which files will be served.
    pub root: PathBuf,
}

impl ServerConfig {
    /// Creates a new `ServerConfig` with default values:
    /// host: `127.0.0.1`, port: `8080`.
    pub fn new() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            root: PathBuf::from("."),
        }
    }

    /// Returns a reference to the current host.
    #[inline(always)]
    pub fn host(&self) -> &str {
        &self.host
    }

    /// Returns the configured port.
    #[inline(always)]
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Returns a reference to the current root directory.
    #[inline(always)]
    pub fn root(&self) -> &PathBuf {
        &self.root
    }

    /// Returns a new `ServerConfig` with the specified port.
    #[must_use]
    #[inline(always)]
    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Returns a new `ServerConfig` with the specified host.
    #[must_use]
    #[inline(always)]
    pub fn with_host(mut self, host: String) -> Self {
        self.host = host;
        self
    }

    /// Returns a new `ServerConfig` with the specified root directory.
    #[must_use]
    #[inline(always)]
    pub fn with_root(mut self, root: PathBuf) -> Self {
        self.root = root;
        self
    }

    /// Returns the full address in the format `host:port`.
    pub fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
