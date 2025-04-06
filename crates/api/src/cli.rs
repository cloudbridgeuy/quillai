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
}
