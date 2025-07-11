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
    /// Run the project API.
    Api(ApiArgs),
    /// Run the blog.
    Blog(BlogArgs),

    /// Run editor examples.
    #[command(subcommand)]
    Editor(EditorCommands),
}

#[derive(Debug, Subcommand)]
pub enum EditorCommands {
    /// Run the basic editor web example.
    Web,
    /// Run the basic editor desktop example.
    Desktop,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = App::parse();

    match cli.command {
        Some(command) => match command {
            Commands::Api(args) => api(args),
            Commands::Blog(args) => blog(args),
            Commands::Editor(editor_cmd) => match editor_cmd {
                EditorCommands::Web => editor_web(),
                EditorCommands::Desktop => editor_desktop(),
            },
        },
        None => {
            println!("No command specified.");
            std::process::exit(1);
        }
    }
}

#[derive(Args, Debug)]
pub struct BlogArgs {
    /// Blog port
    #[clap(long, default_value = "5150")]
    blog_port: String,

    /// Specify the environment
    #[clap(long, default_value = "development")]
    environment: String,
}

pub fn blog(args: BlogArgs) -> Result<(), Box<dyn Error>> {
    let arguments = vec![
        "loco",
        "start",
        "--port",
        &args.blog_port,
        "--environment",
        &args.environment,
    ];

    bunt::println!("{$magenta}Running Blog...{/$}");
    cmd("cargo", arguments)
        .dir("./crates/blog")
        .stdout_to_stderr()
        .stderr_capture()
        .run()?;

    Ok(())
}

#[derive(Args, Debug)]
pub struct ApiArgs {
    /// API port
    #[clap(long, default_value = "2908")]
    api_port: u32,

    /// Log level
    #[clap(long, default_value = "info")]
    log_level: String,
}

pub fn api(args: ApiArgs) -> Result<(), Box<dyn Error>> {
    let port = format!("http::{}", args.api_port);
    let command = format!("run --bin api -- --log-level {}", args.log_level);

    let arguments = vec![
        "--no-pid",
        "-s",
        port.as_str(),
        "--",
        "cargo",
        "watch",
        "-x",
        &command,
        "-C",
        "crates/api",
    ];

    bunt::println!("{$magenta}Running API on port {[bold]}...{/$}", port);
    cmd("systemfd", arguments).read()?;

    Ok(())
}

pub fn editor_web() -> Result<(), Box<dyn Error>> {
    bunt::println!("{$magenta}Running Basic Editor Web Example...{/$}");

    cmd("dx", vec!["serve"])
        .dir("./crates/editor/examples/basic_editor_web")
        .run()?;

    Ok(())
}

pub fn editor_desktop() -> Result<(), Box<dyn Error>> {
    bunt::println!("{$magenta}Running Basic Editor Desktop Example...{/$}");

    cmd("cargo", vec!["run"])
        .dir("./crates/editor/examples/basic_editor_desktop")
        .run()?;

    Ok(())
}
