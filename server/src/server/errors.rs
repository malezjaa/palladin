use axum::http::{Response, StatusCode};

pub fn error_response(message: impl Into<String>, code: StatusCode) -> Response<String> {
    Response::builder()
        .status(code)
        .body(message.into())
        .unwrap()
}

#[macro_export]
macro_rules! handle_result {
    ($result:expr) => {
        if let Err(e) = $result {
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(e.to_string());
        }
    };
}
