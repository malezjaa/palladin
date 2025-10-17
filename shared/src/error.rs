pub use anyhow::*;
use rolldown_error::BatchedBuildDiagnostic;
use thiserror::*;

#[derive(Error, Debug)]
pub enum PalladinError {
    #[error("I/O Error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("UTF-8 Error: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),

    #[error("Notify Error: {0}")]
    NotifyError(#[from] notify::Error),

    #[error("Rolldown Error: {0}")]
    RolldownError(#[from] BatchedBuildDiagnostic),

    #[error("Build error: {0}")]
    Build(#[from] anyhow::Error),

    #[error("Watcher error: {0}")]
    Watcher(String),

    #[error("Engine is closed")]
    EngineClosed,

    #[error("Service communication error: {0}")]
    ServiceCommunication(String),
}

pub type PalladinResult<T = ()> = Result<T, PalladinError>;

impl PalladinError {
    pub fn response(&self) -> axum::http::Response<String> {
        use axum::http::{Response, StatusCode};

        let (message, code) = match self {
            PalladinError::IoError(e) => (e.to_string(), StatusCode::INTERNAL_SERVER_ERROR),
            PalladinError::FileNotFound(file) => {
                (format!("File not found: {}", file), StatusCode::NOT_FOUND)
            }
            _ => (self.to_string(), StatusCode::INTERNAL_SERVER_ERROR),
        };

        Response::builder()
            .status(code)
            .body(message)
            .unwrap_or_else(|_| {
                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body("Internal Server Error".to_string())
                    .unwrap()
            })
    }
}
