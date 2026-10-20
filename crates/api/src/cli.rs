use clap::Parser;

#[derive(Debug, Parser)]
#[command(name = "quillai-api")]
#[command(about = "Quillai API")]
pub struct App {
    #[clap(long, default_value = "127.0.0.1")]
    pub host: String,

    #[clap(long, default_value = "2908")]
    pub port: u16,

    #[clap(long)]
    pub cors_origin: Vec<axum::http::HeaderValue>,

    #[clap(long, default_value = "sqlite://", env = "QUILLAI_API_DB_URL")]
    pub db_url: String,

    #[clap(long, default_value = "1", env = "QUILLAI_API_DB_MAX_CONNECTIONS")]
    pub db_max_connections: u32,

    #[clap(long, default_value = "3", env = "QUILLAI_API_DB_ACQUIRE_TIMEOUT")]
    pub db_acquire_timeout: u64,

    #[clap(
        long,
        value_enum,
        default_value = "warn",
        env = "QUILLAI_API_LOG_LEVEL"
    )]
    pub log_level: quillai_log::LogLevel,
}
