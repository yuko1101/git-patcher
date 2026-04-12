use std::path::PathBuf;

use anyhow::{Context, Ok};
use clap::{Parser, Subcommand};
use git2::Oid;

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
    Patch {
        #[clap(subcommand)]
        command: PatchSubCommand,
    },
}

#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
enum PatchSubCommand {
    Get {
        commit: Oid,
        #[clap(short, long)]
        parent: Option<Oid>,
    },
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    match args.subcommand {
        SubCommand::Push | SubCommand::Pop => {
            let root = args
                .root
                .or_else(|| utils::git_utils::find_root(&std::env::current_dir().ok()?)).context("Failed to find git repository root. Please specify the root directory with --root option.")?;

            let mut patcher = Patcher::new(root)?;
            match args.subcommand {
                SubCommand::Push => commands::push::push(&mut patcher)?,
                SubCommand::Pop => commands::pop::pop(&mut patcher)?,
                _ => unreachable!(),
            }
        }
        SubCommand::Patch { command } => match command {
            PatchSubCommand::Get { commit, parent } => commands::patch::get_patch(commit, parent)?,
        },
    }

    Ok(())
}
