use clap::{ArgAction, Parser};

use crate::tasks::Task;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
pub(super) struct Cli {
    /// Display task hint instead calling solution routine
    #[arg(short = 'H', long, action = ArgAction::SetTrue)]
    pub hint: bool,

    #[command(subcommand)]
    pub task: Task,
}
