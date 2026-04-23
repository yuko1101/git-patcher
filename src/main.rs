use std::path::PathBuf;

use anyhow::{Context, Ok};
use clap::{Parser, Subcommand};
use git2::Oid;

use crate::{config::config::Config, patcher::patcher::Patcher};

mod commands;
mod config;
mod patcher;
mod utils;

#[derive(Parser, Debug)]
struct Args {
    #[clap(short, long)]
    root: Option<PathBuf>,
    #[clap(short, long)]
    config: Option<PathBuf>,
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
    Source {
        #[clap(subcommand)]
        command: SourceSubCommand,
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

#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
enum SourceSubCommand {
    Sync,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let config = Config::load(args.config)?;

    match args.subcommand {
        SubCommand::Push | SubCommand::Pop | SubCommand::Source { .. } => {
            let root = args
                .root
                .or_else(|| utils::git_utils::find_root(&std::env::current_dir().ok()?)).context("Failed to find git repository root. Please specify the root directory with --root option.")?;

            let mut patcher = Patcher::new(root)?;
            match args.subcommand {
                SubCommand::Push => commands::push::push(&mut patcher)?,
                SubCommand::Pop => commands::pop::pop(&mut patcher)?,
                SubCommand::Source { command } => match command {
                    SourceSubCommand::Sync => commands::source::sync_source(&mut patcher, &config)?,
                },
                _ => unreachable!(),
            }
        }
        SubCommand::Patch { command } => match command {
            PatchSubCommand::Get { commit, parent } => commands::patch::get_patch(commit, parent)?,
        },
    }

    Ok(())
}
