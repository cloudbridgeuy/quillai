use axum::extract::rejection::JsonRejection;
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
    #[error("chapter revision conflict")]
    Conflict,
    #[error("validation error: {0}")]
    Validation(String),
    #[error("invalid json: {0}")]
    InvalidJson(String),
    #[error("AI provider configuration error: {0}")]
    AiProviderConfig(String),
    #[error("internal server error: {0}")]
    Internal(String),
}

impl Error {
    pub fn status_code(&self) -> StatusCode {
        match self {
            Error::Sqlx(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::InvalidLogLevel(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::NotFound => StatusCode::NOT_FOUND,
            Error::Conflict => StatusCode::CONFLICT,
            Error::Validation(_) => StatusCode::UNPROCESSABLE_ENTITY,
            Error::InvalidJson(_) => StatusCode::BAD_REQUEST,
            Error::AiProviderConfig(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn error_code(&self) -> &'static str {
        match self {
            Error::Sqlx(_) => "SQLX",
            Error::InvalidLogLevel(_) => "INVALID_LOG_LEVEL",
            Error::NotFound => "NOT_FOUND",
            Error::Conflict => "CONFLICT",
            Error::Validation(_) => "VALIDATION_ERROR",
            Error::InvalidJson(_) => "INVALID_JSON",
            Error::AiProviderConfig(_) => "AI_PROVIDER_CONFIG_ERROR",
            Error::Internal(_) => "INTERNAL_SERVER_ERROR",
        }
    }

    pub fn from_json_rejection(rejection: JsonRejection) -> Self {
        Self::InvalidJson(rejection.body_text())
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

impl From<crate::chapter_revisions::RevisionError> for Error {
    fn from(error: crate::chapter_revisions::RevisionError) -> Self {
        use crate::chapter_revisions::RevisionError;

        match error {
            RevisionError::MissingChapter => Self::NotFound,
            RevisionError::StaleHead | RevisionError::IncompleteHistory => Self::Conflict,
            RevisionError::Delta(error) => Self::Validation(error.to_string()),
            RevisionError::Database(error) => Self::Sqlx(error),
            error => Self::Internal(error.to_string()),
        }
    }
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

    #[test]
    fn invalid_json_error_code_and_status() {
        let err = Error::InvalidJson("bad payload".into());
        assert_eq!(err.status_code(), StatusCode::BAD_REQUEST);
        assert_eq!(err.error_code(), "INVALID_JSON");
    }

    #[test]
    fn invalid_json_error_message() {
        let err = Error::InvalidJson("bad payload".into());
        assert_eq!(err.to_string(), "invalid json: bad payload");
    }

    #[test]
    fn internal_error_code_and_status() {
        let err = Error::Internal("failed to build response".into());
        assert_eq!(err.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(err.error_code(), "INTERNAL_SERVER_ERROR");
    }

    #[test]
    fn ai_provider_config_error_code_and_status() {
        let err = Error::AiProviderConfig("missing api key".into());
        assert_eq!(err.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(err.error_code(), "AI_PROVIDER_CONFIG_ERROR");
    }

    #[test]
    fn ai_provider_config_error_message() {
        let err = Error::AiProviderConfig("missing api key".into());
        assert_eq!(
            err.to_string(),
            "AI provider configuration error: missing api key"
        );
    }

    #[test]
    fn internal_error_message() {
        let err = Error::Internal("failed to build response".into());
        assert_eq!(
            err.to_string(),
            "internal server error: failed to build response"
        );
    }
}
