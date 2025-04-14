use loco_rs::cli;
use migration::Migrator;
use quillai_blog::app::App;

#[tokio::main]
async fn main() -> loco_rs::Result<()> {
    cli::main::<App, Migrator>().await
}
