//! ╭─────────────────────────────────────────────────────────────────────────────╮
//! │ Development Mode                                                            │
//! ╰─────────────────────────────────────────────────────────────────────────────╯
//!
//! To start the API in Development mode run it with.
//!
//! ```not_rust
//! cargo run -p auto-load
//! ```
use axum::{response::Html, routing::get, Router};
use listenfd::ListenFd;
use tokio::net::TcpListener;

mod error;
mod prelude;

use crate::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    let app = Router::new().route("/", get(handler));

    let mut listenfd = ListenFd::from_env();
    let listener = match listenfd.take_tcp_listener(0)? {
        // If we are given a TCP listener on listen `fd` 0,
        Some(listener) => {
            listener
                .set_nonblocking(true)
                .context("Failed to set non-blocking")?;
            TcpListener::from_std(listener).context("Failed to convert to TcpListener")?
        }
        // Otherwise fall back to local listening.
        None => TcpListener::bind("127.0.0.1:3000").await?,
    };

    let local_addr = listener.local_addr()?;
    log::info!("Listening on {}", local_addr);

    axum::serve(listener, app).await?;

    Ok(())
}

async fn handler() -> Html<&'static str> {
    Html("<h1>Hello, World!</h1>")
}
