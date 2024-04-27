mod blogger;
mod embedding;
mod functions;
mod gnome;
mod helloapi;
mod inprompt;
mod knowledge;
mod liar;
mod meme;
mod moderation;
mod ownapi;
mod ownapipro;
mod people;
mod rodo;
mod scraper;
mod search;
mod tools;
mod whisper;
mod whoami;

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

    /// run 'scraper' task
    #[strum(serialize = "scraper")]
    Scraper,

    /// run 'whoami' task
    #[strum(serialize = "whoami")]
    Whoami,

    /// run 'search' task
    #[strum(serialize = "search")]
    Search,

    /// run 'people' task
    #[strum(serialize = "people")]
    People,

    /// run 'knowledge' task
    #[strum(serialize = "knowledge")]
    Knowledge,

    /// run 'tools' task
    #[strum(serialize = "tools")]
    Tools,

    /// run 'gnome' task
    #[strum(serialize = "gnome")]
    Gnome,

    /// run 'ownapi' task
    #[strum(serialize = "ownapi")]
    Ownapi,

    /// run 'ownapipro' task
    #[strum(serialize = "ownapipro")]
    Ownapipro,

    /// run 'meme' task
    #[strum(serialize = "meme")]
    Meme,
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
            Self::Embedding => embedding::run().await,
            Self::Whisper => whisper::run(&config, &token).await,
            Self::Functions => functions::run(&config, &token).await,
            Self::Rodo => rodo::run(&config, &token).await,
            Self::Scraper => scraper::run(&config, &token).await,
            Self::Whoami => whoami::run(&config, &token).await,
            Self::Search => search::run(&config, &token).await,
            Self::People => people::run(&config, &token).await,
            Self::Knowledge => knowledge::run(&config, &token).await,
            Self::Tools => tools::run(&config, &token).await,
            Self::Gnome => gnome::run(&config, &token).await,
            Self::Ownapi => {
                ownapi::run(&config, &token).await?;
                return Ok(());
            }
            Self::Ownapipro => {
                ownapipro::run(&config, &token).await?;
                return Ok(());
            }
            Self::Meme => meme::run(&config, &token).await,
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
