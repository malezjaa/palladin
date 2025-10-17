use crate::file::{calculate_content_hash, detect_file_type, File, FileContent, FileType};
use crate::hmr::inject_hmr_script;
use crate::server::Server;
use axum::extract::{Path, State};
use axum::http::Response;
use axum::response::IntoResponse;
use log::debug;
use palladin_shared::{PalladinError, PalladinResult};
use std::path::PathBuf;
use std::sync::Arc;

pub async fn serve_file_handler(
    State(server): State<Arc<Server>>,
    Path(file): Path<String>,
) -> impl IntoResponse {
    Server::serve_file_impl(server, file)
        .await
        .unwrap_or_else(|err| err.response())
}

pub async fn serve_index_handler(State(server): State<Arc<Server>>) -> impl IntoResponse {
    Server::serve_index_impl(server)
        .await
        .unwrap_or_else(|err| err.response())
}

pub async fn serve_chunk_handler(
    State(server): State<Arc<Server>>,
    Path(chunk_name): Path<String>,
) -> impl IntoResponse {
    Server::serve_chunk_impl(server, chunk_name).unwrap_or_else(|err| err.response())
}

impl Server {
    async fn serve_file_impl(server: Arc<Self>, file: String) -> PalladinResult<Response<String>> {
        if file.ends_with(".js") && file.contains('-') {
            let filename = file.split('/').last().unwrap_or(&file);
            if server.rolldown_pipe.has_chunk(filename) {
                debug!("Serving chunk from cache: {}", filename);
                return Self::serve_chunk_impl(server, filename.to_string());
            }
        }

        let full_path = server
            .ctx
            .resolve_path(&file)
            .map_err(|_| PalladinError::FileNotFound(file.clone()));

        let full_path = match full_path {
            Ok(path) if path.is_file() && server.ctx.is_within_root(&path) => path,
            _ if file.contains('.') => {
                // treat as file request that failed
                return Err(PalladinError::FileNotFound(file.clone()));
            }
            _ => {
                // No extension - assume that it's a route
                return Self::serve_index_impl(server).await;
            }
        };

        debug!(
            "Serving file: {:?}",
            full_path
                .file_name()
                .map(|s| s.to_string_lossy())
                .unwrap_or("unknown".into())
        );

        let file_struct = Self::get_or_load_file(&server, &full_path)?;
        Self::build_file_response(&file_struct)
    }

    async fn serve_index_impl(server: Arc<Self>) -> PalladinResult<Response<String>> {
        let index_path = server
            .ctx
            .resolve_path("index.html")
            .map_err(|_| PalladinError::FileNotFound("index.html".to_string()))?;

        debug!("Serving index.html");

        let file_struct = Self::get_or_load_file(&server, &index_path)?;
        Self::build_file_response(&file_struct)
    }

    fn serve_chunk_impl(server: Arc<Self>, chunk_name: String) -> PalladinResult<Response<String>> {
        if let Some(content) = server.rolldown_pipe.get_chunk(&chunk_name) {
            Ok(Response::builder()
                .header("content-type", "application/javascript")
                .header("cache-control", "public, max-age=31536000, immutable")
                .body(content)
                .unwrap())
        } else {
            Err(PalladinError::FileNotFound(format!(
                "Chunk not found: {}",
                chunk_name
            )))
        }
    }

    fn build_file_response(file: &File) -> PalladinResult<Response<String>> {
        let mut content = file.content.transformed.clone();

        if file.ty == FileType::HTML {
            content = inject_hmr_script(&content);
        }

        Ok(Response::builder()
            .header("content-type", file.content_type())
            .body(content)
            .unwrap())
    }

    fn get_or_load_file(server: &Arc<Self>, path: &PathBuf) -> PalladinResult<File> {
        let mut files = server.files.write();
        let content = fs_err::read_to_string(path)
            .map_err(|_| PalladinError::FileNotFound(path.display().to_string()))?;
        let hash = calculate_content_hash(&content);
        let ty = detect_file_type(path);

        let is_new = !files.contains_key(path);

        let entry = files.entry(path.clone()).or_insert_with(|| File {
            path: path.clone(),
            hash: hash.clone(),
            ty: ty.clone(),
            content: FileContent {
                original: content.clone(),
                transformed: content.clone(),
            },
        });

        if is_new || entry.hash != hash {
            entry.hash = hash.clone();
            entry.content.original = content.clone();
            entry.content.transformed = content.clone();
            server.rolldown_pipe.transform(entry)?;
        }

        Ok(entry.clone())
    }
}
