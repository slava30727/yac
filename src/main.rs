#![feature(decl_macro)]



pub mod dynamic;
pub mod new;
pub mod package;
pub mod build;
pub mod run;
pub mod clean;
pub mod update;
pub mod prettify;

use clap::*;
use std::error::Error;



#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let cli_args = CliArgs::parse();

    match cli_args.command {
        CliCommand::New { name } => new::new(&name).await?,
        CliCommand::Run { release } => run::run(release).await?,
        CliCommand::Build { release } => { build::build(release).await?; },
        CliCommand::Clean => clean::clean().await?,
    }

    Ok(())
}



#[derive(Parser, Debug)]
struct CliArgs {
    #[command(subcommand)]
    command: CliCommand,
}



#[derive(Parser, Debug)]
enum CliCommand {
    Build {
        #[arg(short, long)]
        release: bool,
    },
    Run {
        #[arg(short, long)]
        release: bool,
    },
    New {
        #[arg(short, long)]
        name: String,
    },
    Clean,
}