pub use anyhow::*;
use thiserror::*;

#[derive(Error, Debug)]
pub enum PalladinError {
    #[error("I/O Error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("File not found: {0}")]
    FileNotFound(String),
}

pub type PalladinResult<T = ()> = Result<T, PalladinError>;

impl PalladinError {
    pub fn response(&self) -> axum::http::Response<String> {
        use axum::http::{Response, StatusCode};

        let (message, code) = match self {
            PalladinError::IoError(e) => (e.to_string(), StatusCode::INTERNAL_SERVER_ERROR),
            PalladinError::FileNotFound(file) => (
                format!("File not found: {}", file),
                StatusCode::NOT_FOUND,
            ),
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