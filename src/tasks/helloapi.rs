use anyhow::{anyhow, bail};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{aidevs, config::Config};

#[derive(Debug, Deserialize)]
struct HelloApiTaskResponse {
    code: i32,
    msg: String,
    cookie: Option<String>,
}

/// Test task for learning how AI_Devs 2 task API works.
/// The task involved retrieving messages from the API and returning the contents of the 'cookie' field in the response.
///
/// * `config`: App configuration
/// * `token`: Task token
pub(super) async fn run(config: &Config, token: &str) -> anyhow::Result<Value> {
    let task_response = aidevs::get_task::<HelloApiTaskResponse>(config, token).await?;
    log::debug!("Task API response: {task_response:#?}");
    log::info!("Task message: {}", task_response.msg);

    if task_response.code != 0 {
        bail!("Code in response is not equal 0")
    }

    let cookie = task_response
        .cookie
        .ok_or(anyhow!("API task response do not contain 'cookie' field"))?;

    let payload = json!({ "answer" : cookie});
    Ok(payload)
}
