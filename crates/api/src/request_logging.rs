use std::time::{Duration, Instant};

use axum::{extract::Request, http::StatusCode, middleware::Next, response::Response};

const SLOW_REQUEST: Duration = Duration::from_secs(1);

fn should_warn(status: StatusCode, elapsed: Duration) -> bool {
    status.is_client_error() || status.is_server_error() || elapsed >= SLOW_REQUEST
}

pub async fn log_request(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let path = request.uri().path().to_owned();
    let start = Instant::now();
    let response = next.run(request).await;
    let elapsed = start.elapsed();
    let status = response.status();
    let elapsed_ms = elapsed.as_secs_f64() * 1000.0;
    if should_warn(status, elapsed) {
        quillai_log::warn!(%method, %path, status = status.as_u16(), elapsed_ms, "request completed");
    } else {
        quillai_log::debug!(%method, %path, status = status.as_u16(), elapsed_ms, "request completed");
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warnings_cover_errors_and_slow_requests() {
        assert!(!should_warn(
            StatusCode::OK,
            SLOW_REQUEST - Duration::from_nanos(1)
        ));
        assert!(!should_warn(StatusCode::FOUND, Duration::ZERO));
        assert!(should_warn(StatusCode::OK, SLOW_REQUEST));
        assert!(should_warn(StatusCode::INTERNAL_SERVER_ERROR, SLOW_REQUEST));
        assert!(should_warn(StatusCode::NOT_FOUND, Duration::ZERO));
        assert!(should_warn(
            StatusCode::INTERNAL_SERVER_ERROR,
            Duration::ZERO
        ));
    }
}
