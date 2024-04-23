mod blogger;
mod embedding;
mod functions;
mod helloapi;
mod inprompt;
mod liar;
mod moderation;
mod rodo;
mod whisper;

use clap::Subcommand;
use std::string::ToString;
use strum_macros::Display;

use crate::{aidevs, config::Config};

#[derive(Debug, Subcommand, Display)]
pub enum Task {
    /// run 'helloapi' task
    #[strum(serialize = "helloapi")]
    Helloapi,

    /// run 'moderation' task
    #[strum(serialize = "moderation")]
    Moderation,

    /// run 'blogger' task
    #[strum(serialize = "blogger")]
    Blogger,

    /// run 'liar' task
    #[strum(serialize = "liar")]
    Liar,

    /// run 'inprompt' task
    #[strum(serialize = "inprompt")]
    Inprompt,

    /// run 'embedding' task
    #[strum(serialize = "embedding")]
    Embedding,

    /// run 'whisper' task
    #[strum(serialize = "whisper")]
    Whisper,

    /// run 'functions' task
    #[strum(serialize = "functions")]
    Functions,

    /// run 'rodo' task
    #[strum(serialize = "rodo")]
    Rodo,
}

impl Task {
    pub async fn run(self, config: Config) -> anyhow::Result<()> {
        let task_name = self.to_string();
        log::info!("Start '{task_name}' task");

        let token = aidevs::get_task_token(&config, &task_name).await?;
        log::debug!("Received token: {token}");

        let answer = match self {
            Self::Helloapi => helloapi::run(&config, &token).await,
            Self::Moderation => moderation::run(&config, &token).await,
            Self::Blogger => blogger::run(&config, &token).await,
            Self::Liar => liar::run(&config, &token).await,
            Self::Inprompt => inprompt::run(&config, &token).await,
            Task::Embedding => embedding::run().await,
            Self::Whisper => whisper::run(&config, &token).await,
            Self::Functions => functions::run(&config, &token).await,
            Task::Rodo => rodo::run(&config, &token).await,
        }?;

        aidevs::post_answer(&config, &token, &answer).await?;
        Ok(())
    }

    pub async fn hint(self, config: Config) -> anyhow::Result<()> {
        let task_name = self.to_string();
        log::info!("Get '{task_name}' task hint");

        let response = aidevs::get_hint(&config, &task_name).await?;
        println!("{task_name} hint: {response}");

        Ok(())
    }
}
