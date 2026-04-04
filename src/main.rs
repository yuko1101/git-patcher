use std::path::PathBuf;

use anyhow::Ok;
use clap::{Parser, Subcommand};

use crate::utils::patcher::Patcher;

mod commands;
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
    let root = args.root.unwrap_or_else(|| {
        utils::git_utils::find_root(&std::env::current_dir().unwrap())
            .expect("Failed to find git root")
    });

    let patcher = Patcher {
        root: root.clone(),
        upstream: root.join("upstream"),
        patches: root.join("patches"),
        internal: root.join(".git-patcher"),
    };

    match args.subcommand {
        SubCommand::Push => commands::push::push(&patcher)?,
        SubCommand::Pop => commands::pop::pop(&patcher)?,
    }

    Ok(())
}
