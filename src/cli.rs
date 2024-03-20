use clap::Parser;

use crate::tasks::Task;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
pub(super) struct Cli {
    #[command(subcommand)]
    pub task: Task,
}
