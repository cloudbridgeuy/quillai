use clap::Parser;

#[derive(Debug, Parser)]
#[command(name = "quillai-smtp")]
#[command(about = "Quillai SMTP")]
pub struct App {
    /// Log level
    #[clap(
        long,
        value_enum,
        default_value = "warn",
        env = "QUILLAI_API_LOG_LEVEL"
    )]
    pub log_level: quillai_log::LogLevel,
}
