use clap::Parser;

#[derive(Debug, Parser)]
#[command(name = "quillai-smtp")]
#[command(about = "Quillai SMTP")]
pub struct App {
    /// Socket to bind the STMP server.
    #[clap(long, default_value = "127.0.0.1:2525", env = "QUILLAI_SMTP_BIND")]
    pub bind: std::net::SocketAddr,

    /// Log level
    #[clap(
        long,
        value_enum,
        default_value = "warn",
        env = "QUILLAI_SMTP_LOG_LEVEL"
    )]
    pub log_level: quillai_log::LogLevel,

    /// Certificate file path.
    #[clap(long, env = "QUILLAI_SMTP_CERT")]
    pub cert: String,

    /// Certificate private key path.
    #[clap(long, env = "QUILLAI_SMTP_KEY")]
    pub key: String,
}
