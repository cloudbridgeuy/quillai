use clap::Parser;
use listenfd::ListenFd;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use tokio::net::TcpListener;

mod cli;

use quillai_api::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let args = crate::cli::App::parse();

    quillai_log::init_simple_logger(args.log_level)?;

    let options = args
        .db_url
        .parse::<SqliteConnectOptions>()?
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(args.db_max_connections)
        .acquire_timeout(std::time::Duration::from_secs(args.db_acquire_timeout))
        .connect_with(options)
        .await?;
    quillai_api::database::migrate(&pool)
        .await
        .context("Failed to run database migrations")?;

    let app = quillai_api::app(pool);

    let mut listenfd = ListenFd::from_env();
    let listener = match listenfd.take_tcp_listener(0)? {
        Some(listener) => {
            listener
                .set_nonblocking(true)
                .context("Failed to set non-blocking")?;
            TcpListener::from_std(listener).context("Failed to convert to TcpListener")?
        }
        None => {
            let host_port = format!("{}:{}", args.host, args.port);
            TcpListener::bind(host_port).await?
        }
    };

    let local_addr = listener.local_addr()?;
    quillai_log::info!("Listening on {}", local_addr);

    axum::serve(listener, app).await?;

    Ok(())
}
