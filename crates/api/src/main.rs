use clap::Parser;
use listenfd::ListenFd;
use sqlx::sqlite::SqliteConnectOptions;
use tokio::net::TcpListener;

mod cli;

use quillai_api::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let args = crate::cli::App::parse();

    quillai_log::init_simple_logger(args.log_level)?;

    let fallback_model = std::env::var("QUILLAI_AI_MODEL").ok();
    let catalog =
        quillai_api::ai::catalog::load(args.ai_models.as_deref(), fallback_model.as_deref())?;

    let options = args.db_url.parse::<SqliteConnectOptions>()?;
    let pool = quillai_api::database::connect(
        options,
        args.db_max_connections,
        std::time::Duration::from_secs(args.db_acquire_timeout),
    )
    .await?;
    quillai_api::database::migrate(&pool)
        .await
        .context("Failed to run database migrations")?;

    let app = quillai_api::app_with_cors(pool, args.cors_origin);
    let app = match catalog {
        Some(catalog) => app.layer(axum::Extension(std::sync::Arc::new(catalog))),
        None => app,
    };

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
