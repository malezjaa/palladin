use crate::handle_result;
use crate::server::errors::error_response;
use crate::server::Server;
use axum::extract::{Path, State};
use axum::http::{Response, StatusCode};
use log::debug;
use palladin_shared::{canonicalize_with_strip, PalladinError, PalladinResult};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct File {
    pub path: PathBuf,
    pub hash: String,
    pub dirty: bool,
    pub ty: FileType,

    pub content: FileContent,
}

#[derive(Debug, Clone)]
pub struct FileContent {
    pub original: String,
    pub transformed: String,
}

#[derive(Debug, Clone)]
pub enum FileType {
    CSS,

    JS,
    JSX,

    TS,
    TSX,

    HTML,
}

impl Server {
    pub async fn serve_file(
        State(server): State<Arc<Self>>,
        Path(file): Path<String>,
    ) -> Response<String> {
        match Self::serve_file_impl(server, file).await {
            Ok(content) => Response::new(content),
            Err(err) => err.response(),
        }
    }

    async fn serve_file_impl(server: Arc<Self>, file: String) -> PalladinResult<String> {
        let full_path = canonicalize_with_strip(server.config().root.join(file.clone()))
            .map_err(|_| (PalladinError::FileNotFound(file.clone())))?;
        debug!(
            "Serving file: {:?}",
            full_path
                .file_name()
                .map(|s| s.to_string_lossy())
                .unwrap_or("unknown".into())
        );

        let file_struct = Self::get_or_load_file(&server, &full_path)?;
        Ok(file_struct.content.original.clone())
    }

    fn get_or_load_file(server: &Arc<Self>, path: &PathBuf) -> PalladinResult<File> {
        let mut files = server.files.write();
        let metadata = fs::metadata(path)
            .map_err(|_| PalladinError::FileNotFound(path.display().to_string()))?;
        let content = fs_err::read_to_string(path)?;
        let hash = calculate_hash(&content);
        let ty = detect_file_type(path);

        let entry = files.entry(path.clone()).or_insert_with(|| File {
            path: path.clone(),
            hash: hash.clone(),
            dirty: false,
            ty: ty.clone(),
            content: FileContent {
                original: content.clone(),
                transformed: content.clone(),
            },
        });
        if entry.hash != hash {
            entry.hash = hash.clone();
            entry.dirty = true;
            entry.content.original = content.clone();
            entry.content.transformed = content.clone();
        } else {
            entry.dirty = false;
        }

        println!("{:?}", entry);
        Ok(entry.clone())
    }
}

fn calculate_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn detect_file_type(path: &PathBuf) -> FileType {
    match path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "css" => FileType::CSS,
        "js" => FileType::JS,
        "jsx" => FileType::JSX,
        "ts" => FileType::TS,
        "tsx" => FileType::TSX,
        "html" => FileType::HTML,
        _ => FileType::JS,
    }
}
