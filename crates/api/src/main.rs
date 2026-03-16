//! ╭─────────────────────────────────────────────────────────────────────────────╮
//! │ Development Mode                                                            │
//! ╰─────────────────────────────────────────────────────────────────────────────╯
//!
//! To start the API in Development mode run it with.
//!
//! ```not_rust
//! cargo run -p auto-load
//! ```
use clap::Parser;
use listenfd::ListenFd;
use sqlx::sqlite::SqlitePoolOptions;
use tokio::net::TcpListener;

mod cli;

use quillai_api::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    // ╭─────────────────────────────────────────────────────────────────────────────╮
    // │ App                                                                         │
    // ╰─────────────────────────────────────────────────────────────────────────────╯
    let args = crate::cli::App::parse();

    // ╭─────────────────────────────────────────────────────────────────────────────╮
    // │ Logger                                                                      │
    // ╰─────────────────────────────────────────────────────────────────────────────╯
    quillai_log::init_simple_logger(args.log_level)?;

    // ╭─────────────────────────────────────────────────────────────────────────────╮
    // │ DB                                                                          │
    // ╰─────────────────────────────────────────────────────────────────────────────╯
    let pool = SqlitePoolOptions::new()
        .max_connections(args.db_max_connections)
        .acquire_timeout(std::time::Duration::from_secs(args.db_acquire_timeout))
        .connect(&args.db_url)
        .await?;

    // ╭─────────────────────────────────────────────────────────────────────────────╮
    // │ Router                                                                      │
    // ╰─────────────────────────────────────────────────────────────────────────────╯
    let app = quillai_api::app(pool);

    // ╭─────────────────────────────────────────────────────────────────────────────╮
    // │ Dev mode                                                                    │
    // ╰─────────────────────────────────────────────────────────────────────────────╯
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
        None => {
            let host_port = format!("{}:{}", args.host, args.port);
            TcpListener::bind(host_port).await?
        }
    };

    // ╭─────────────────────────────────────────────────────────────────────────────╮
    // │ Run                                                                         │
    // ╰─────────────────────────────────────────────────────────────────────────────╯
    let local_addr = listener.local_addr()?;
    quillai_log::info!("Listening on {}", local_addr);

    axum::serve(listener, app).await?;

    Ok(())
}
