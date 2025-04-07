use axum::http::StatusCode;

#[derive(thiserror::Error, axum_thiserror::ErrorStatus)]
pub enum Error {
    #[error("sqlx error")]
    #[status(StatusCode::INTERNAL_SERVER_ERROR)]
    Sqlx(#[from] sqlx::Error),
    #[error("invalid log level")]
    #[status(StatusCode::INTERNAL_SERVER_ERROR)]
    InvalidLogLevel,
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
