use sha2::{Digest, Sha256};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct File {
    pub path: PathBuf,
    pub hash: String,
    pub ty: FileType,

    pub content: FileContent,
}

impl File {
    /// Return MIME content type
    pub fn content_type(&self) -> &str {
        match self.ty {
            FileType::CSS => "text/css",
            FileType::JS | FileType::JSX | FileType::TS | FileType::TSX => "application/javascript",
            FileType::HTML => "text/html",
        }
    }
}

#[derive(Debug, Clone)]
pub struct FileContent {
    pub original: String,
    pub transformed: String,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum FileType {
    CSS,

    JS,
    JSX,

    TS,
    TSX,

    HTML,
}

pub fn calculate_content_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn detect_file_type(path: &PathBuf) -> FileType {
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
