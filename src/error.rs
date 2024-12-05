use axum::response::{IntoResponse, Response};
use hyper::StatusCode;
use tracing::error;

pub enum AppError {
    Client(anyhow::Error),
    Server(anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match &self {
            AppError::Client(e) => error!("Client error: {e:?}"),
            AppError::Server(e) => error!("Server error: {e:?}"),
        }

        match self {
            AppError::Client(e) => (StatusCode::BAD_REQUEST, format!("Client error: {e}")),
            AppError::Server(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Server error: {e}"),
            ),
        }
        .into_response()
    }
}

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self::Server(err.into())
    }
}
