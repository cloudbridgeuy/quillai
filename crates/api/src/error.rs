use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

#[derive(thiserror::Error)]
pub enum Error {
    #[error("sqlx error")]
    Sqlx(#[from] sqlx::Error),
    #[error("invalid log level")]
    InvalidLogLevel(#[from] quillai_log::Error),
    #[error("not found")]
    NotFound,
    #[error("validation error: {0}")]
    Validation(String),
}

impl Error {
    pub fn status_code(&self) -> StatusCode {
        match self {
            Error::Sqlx(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::InvalidLogLevel(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::NotFound => StatusCode::NOT_FOUND,
            Error::Validation(_) => StatusCode::UNPROCESSABLE_ENTITY,
        }
    }

    pub fn error_code(&self) -> &'static str {
        match self {
            Error::Sqlx(_) => "SQLX",
            Error::InvalidLogLevel(_) => "INVALID_LOG_LEVEL",
            Error::NotFound => "NOT_FOUND",
            Error::Validation(_) => "VALIDATION_ERROR",
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        (
            self.status_code(),
            Json(json!({
                "error": self.to_string(),
                "code": self.error_code(),
            })),
        )
            .into_response()
    }
}

/// Format error messages for display
pub(crate) fn format_error(
    e: &impl std::error::Error,
    f: &mut std::fmt::Formatter,
) -> std::fmt::Result {
    write!(f, "{e}")?;

    let mut source = e.source();

    if e.source().is_some() {
        writeln!(f, "\ncaused by:")?;
        let mut i: usize = 0;
        while let Some(inner) = source {
            writeln!(f, "{i: >5}: {inner}")?;
            source = inner.source();
            i += 1;
        }
    }

    Ok(())
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        format_error(self, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sqlx_error_code_and_status() {
        let err = Error::Sqlx(sqlx::Error::RowNotFound);
        assert_eq!(err.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(err.error_code(), "SQLX");
    }

    #[test]
    fn invalid_log_level_error_code_and_status() {
        let err = Error::InvalidLogLevel(quillai_log::Error::InvalidLogLevel("bad".into()));
        assert_eq!(err.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(err.error_code(), "INVALID_LOG_LEVEL");
    }

    #[test]
    fn not_found_error_code_and_status() {
        let err = Error::NotFound;
        assert_eq!(err.status_code(), StatusCode::NOT_FOUND);
        assert_eq!(err.error_code(), "NOT_FOUND");
    }

    #[test]
    fn validation_error_code_and_status() {
        let err = Error::Validation("email is required".into());
        assert_eq!(err.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(err.error_code(), "VALIDATION_ERROR");
    }

    #[test]
    fn validation_error_message() {
        let err = Error::Validation("email is required".into());
        assert_eq!(err.to_string(), "validation error: email is required");
    }
}
