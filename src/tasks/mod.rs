mod helloapi;

use clap::Subcommand;
use std::string::ToString;
use strum_macros::Display;

use crate::{aidevs, config::Config};

#[derive(Debug, Subcommand, Display)]
pub enum Task {
    /// run 'helloapi' task
    #[strum(serialize = "helloapi")]
    Helloapi,
}

impl Task {
    pub async fn run(self, config: Config) -> anyhow::Result<()> {
        let task_name = self.to_string();
        log::info!("Start '{task_name}' task");

        let token = aidevs::get_task_token(&config, &task_name).await?;
        log::debug!("Received token: {token}");

        let task_api_response = aidevs::get_task(&config, &token).await?;

        let answer = match self {
            Self::Helloapi => helloapi::run(task_api_response),
        }
        .await?;

        aidevs::post_answer(&config, &token, &answer).await?;
        Ok(())
    }
}
