use anyhow::anyhow;
use reqwest::Response;

use crate::aidevs::HelloApiResponse;

pub async fn run(task_api_response: Response) -> anyhow::Result<String> {
    let task_response = task_api_response.json::<HelloApiResponse>().await?;
    log::debug!("Task API response: {task_response:#?}");

    let cookie = task_response
        .cookie
        .ok_or(anyhow!("API task response do not contain 'cookie' field"))?;

    Ok(cookie)
}
