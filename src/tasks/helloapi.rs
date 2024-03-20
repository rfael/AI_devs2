use anyhow::anyhow;
use reqwest::Response;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct HelloApiTaskResponse {
    code: i32,
    msg: String,
    cookie: Option<String>,
}

pub(super) async fn run(task_api_response: Response) -> anyhow::Result<String> {
    let task_response = task_api_response.json::<HelloApiTaskResponse>().await?;
    log::debug!("Task API response: {task_response:#?}");

    let cookie = task_response
        .cookie
        .ok_or(anyhow!("API task response do not contain 'cookie' field"))?;

    Ok(cookie)
}
