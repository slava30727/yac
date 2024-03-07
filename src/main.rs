#![allow(unused)]


pub mod dynamic;
pub mod new;
pub mod package;
pub mod build;
pub mod run;
pub mod clean;
pub mod update;

use clap::*;
use dynamic::Build;
use std::{error::Error, fs, path::{Path, PathBuf}, process::Command, str::FromStr};
use new::new;



#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let cli_args = CliArgs::parse();

    match cli_args.command {
        CliCommand::New { name } => new::new(name).await?,
        CliCommand::Run { .. } => run::run().await?,
        CliCommand::Build { release } => build::build().await?,
        CliCommand::Clean => clean::clean()?,
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