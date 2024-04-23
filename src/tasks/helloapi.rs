use anyhow::anyhow;

use crate::{
    aidevs::{self, HelloApiResponse},
    config::Config,
};

const TASK_NAME: &str = "helloapi";

pub async fn run(config: Config) -> anyhow::Result<()> {
    log::info!("Start 'helloapi' task");

    let token = aidevs::get_task_token(&config, TASK_NAME).await?;
    log::debug!("Received token: {token}");

    let task_response = aidevs::get_task::<HelloApiResponse>(&config, &token).await?;
    log::debug!("Task response: {task_response:#?}");

    let cookie = task_response
        .cookie
        .ok_or(anyhow!("API task response do not contain 'cookie' field"))?;

    aidevs::post_answer(&config, &token, &cookie).await?;

    Ok(())
}
