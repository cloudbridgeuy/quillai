use std::path::PathBuf;

use anyhow::Context;
use clap::{Parser, Subcommand};

use quillai_api::eval::{
    cases::parse_cases, export::export, fixture::Fixture, fixture::FIXTURE_FILE,
};

#[derive(Debug, Parser)]
#[command(name = "quillai-eval", about = "QuillAI eval set tools")]
struct App {
    #[command(subcommand)]
    command: EvalCommand,
}

#[derive(Debug, Subcommand)]
enum EvalCommand {
    Export {
        #[arg(long, default_value = "http://127.0.0.1:2908")]
        api: String,
        #[arg(long)]
        series: i64,
        #[arg(long)]
        book: i64,
        #[arg(long)]
        out: PathBuf,
    },
    Check {
        #[arg(long)]
        fixture: PathBuf,
        #[arg(long)]
        cases: PathBuf,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    match App::parse().command {
        EvalCommand::Export {
            api,
            series,
            book,
            out,
        } => {
            let path = export(&api, series, book, &out).await?;
            println!("wrote {}", path.display());
        }
        EvalCommand::Check { fixture, cases } => {
            let fixture_path = fixture.join(FIXTURE_FILE);
            let fixture: Fixture = serde_json::from_str(
                &std::fs::read_to_string(&fixture_path)
                    .with_context(|| format!("failed to read {}", fixture_path.display()))?,
            )
            .with_context(|| format!("failed to parse {}", fixture_path.display()))?;
            let cases = parse_cases(
                &std::fs::read_to_string(&cases)
                    .with_context(|| format!("failed to read {}", cases.display()))?,
                &fixture,
            )?;
            println!("{} valid cases", cases.len());
        }
    }
    Ok(())
}
