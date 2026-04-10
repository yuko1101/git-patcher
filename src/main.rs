use std::path::PathBuf;

use anyhow::{Context, Ok};
use clap::{Parser, Subcommand};

use crate::patcher::patcher::Patcher;

mod commands;
mod patcher;
mod utils;

#[derive(Parser, Debug)]
struct Args {
    #[clap(short, long)]
    root: Option<PathBuf>,
    #[clap(subcommand)]
    subcommand: SubCommand,
}

#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
enum SubCommand {
    Push,
    Pop,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let root = args
        .root
        .or_else(|| utils::git_utils::find_root(&std::env::current_dir().ok()?)).context("Failed to find git repository root. Please specify the root directory with --root option.")?;

    let mut patcher = Patcher::new(root)?;

    match args.subcommand {
        SubCommand::Push => commands::push::push(&mut patcher)?,
        SubCommand::Pop => commands::pop::pop(&mut patcher)?,
    }

    Ok(())
}
