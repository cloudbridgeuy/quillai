mod cli;
mod parser;
mod prelude;
mod server;

use crate::prelude::*;
use crate::server::Server;
use clap::Parser;

#[tokio::main]
async fn main() -> Result<()> {
    // ╭─────────────────────────────────────────────────────────────────────────────╮
    // │ App                                                                         │
    // ╰─────────────────────────────────────────────────────────────────────────────╯
    let args = crate::cli::App::parse();

    // ╭─────────────────────────────────────────────────────────────────────────────╮
    // │ Logger                                                                      │
    // ╰─────────────────────────────────────────────────────────────────────────────╯
    env_logger::builder()
        .filter_level(log::LevelFilter::from(args.log_level))
        .init();

    // ╭─────────────────────────────────────────────────────────────────────────────╮
    // │ Start listening                                                             │
    // ╰─────────────────────────────────────────────────────────────────────────────╯
    Server::new(args.cert, args.key)?.listen(args.bind).await
}
