mod aidevs;
mod cli;
mod config;
mod tasks;

use std::env;

use clap::Parser;
use dotenv::dotenv;
use envconfig::Envconfig;

use crate::{
    cli::{Cli, Task},
    config::Config,
    tasks::helloapi,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    if env::var("RUST_LOG").is_ok() {
        env_logger::init();
    }

    let config = Config::init_from_env()?;
    let cli = Cli::parse();

    match cli.task {
        Task::Helloapi => helloapi::run(config),
    }
    .await?;

    Ok(())
}
