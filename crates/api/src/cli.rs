use clap::Parser;

#[derive(Debug, Parser)]
#[command(name = "quillai-api")]
#[command(about = "Quillai API")]
pub struct App {
    /// Host to attach the service to.
    #[clap(long, default_value = "127.0.0.1")]
    pub host: String,

    /// Port the API should listen.
    #[clap(long, default_value = "2908")]
    pub port: u32,

    /// Database host url
    #[clap(long, default_value = "sqlite://", env = "QUILLAI_API_DB_URL")]
    pub db_url: String,

    /// Database maximum connections
    #[clap(long, default_value = "1", env = "QUILLAI_API_DB_MAX_CONNECTIONS")]
    pub db_max_connections: u32,

    /// Database acquire timeout
    #[clap(long, default_value = "3", env = "QUILLAI_API_DB_ACQUIRE_TIMEOUT")]
    pub db_acquire_timeout: u64,

    /// Log level
    #[clap(
        long,
        value_enum,
        default_value = "warn",
        env = "QUILLAI_API_LOG_LEVEL"
    )]
    pub log_level: quillai_log::LogLevel,
}
