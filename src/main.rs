mod aidevs;
mod brave_search;
mod cli;
mod config;
mod render_form;
mod tasks;
mod utils;

use std::env;

use clap::Parser;
use dotenv::dotenv;
use envconfig::Envconfig;

use crate::{cli::Cli, config::Config};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    if env::var("RUST_LOG").is_ok() {
        env_logger::init();
    }

    let config = Config::init_from_env()?;
    let cli = Cli::parse();

    if cli.hint {
        cli.task.hint(config).await?;
        return Ok(());
    }

    cli.task.run(config).await
}
