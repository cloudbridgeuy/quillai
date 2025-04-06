//! See <https://github.com/matklad/cargo-xtask/>
//!
//! This binary defines various auxiliary build commands, which are not
//! expressible with just `cargo`.
//!
//! The binary is integrated into the `cargo` command line by using an
//! alias in `.cargo/config`.
use clap::{Args, Parser, Subcommand};
use duct::cmd;
use std::error::Error;

#[derive(Debug, Parser)]
#[command(name = "xtasks")]
#[command(about = "Run project tasks using rust instead of scripts")]
pub struct App {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Run all the project services in development mode.
    Run(RunArgs),
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = App::parse();

    match cli.command {
        Some(command) => match command {
            Commands::Run(args) => run(args),
        },
        None => {
            println!("No command specified.");
            std::process::exit(1);
        }
    }
}

#[derive(Args, Debug)]
pub struct RunArgs {
    /// API port
    #[clap(long, default_value = "2908")]
    api_port: u32,
}

pub fn run(args: RunArgs) -> Result<(), Box<dyn Error>> {
    let port = format!("http::{}", args.api_port);
    let arguments = vec![
        "--no-pid",
        "-s",
        port.as_str(),
        "--",
        "cargo",
        "watch",
        "-x",
        "run --bin api",
        "-L",
        "info",
        "-C",
        "crates/api",
    ];

    bunt::println!("{$magenta}Running API on port {[bold]}...{/$}", port);
    cmd("systemfd", arguments).read()?;

    Ok(())
}
