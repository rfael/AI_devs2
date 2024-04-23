use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
pub(super) struct Cli {
    #[command(subcommand)]
    pub task: Task,
}

#[derive(Debug, Subcommand)]
pub(super) enum Task {
    /// run 'helloapi' task
    Helloapi,
}
