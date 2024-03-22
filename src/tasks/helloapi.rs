use anyhow::{anyhow, bail};
use reqwest::Response;
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Debug, Deserialize)]
struct HelloApiTaskResponse {
    code: i32,
    msg: String,
    cookie: Option<String>,
}

/// Test task for learning how AI_Devs 2 task API works.
/// The task involved retrieving messages from the API and returning the contents of the 'cookie' field in the response.
///
/// * `task_api_response`:
pub(super) async fn run(task_api_response: Response) -> anyhow::Result<Value> {
    let task_response = task_api_response.json::<HelloApiTaskResponse>().await?;
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
