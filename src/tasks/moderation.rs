use anyhow::bail;
use async_openai::{
    config::OpenAIConfig,
    types::{CreateModerationRequestArgs, TextModerationModel},
    Client,
};
use reqwest::Response;
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Debug, Deserialize)]
struct ModerationTaskResponse {
    code: i32,
    msg: String,
    input: Vec<String>,
}

/// The task involved fetching a list of inputs from the API and assessing whether their content should be moderated.
///
/// * `task_api_response`:
pub(super) async fn run(task_api_response: Response) -> anyhow::Result<Value> {
    let task_response = task_api_response.json::<ModerationTaskResponse>().await?;
    log::debug!("Task API response: {task_response:#?}");
    log::info!("Task message: {}", task_response.msg);

    if task_response.code != 0 {
        bail!("Code in response is not equal 0")
    }

    let config = OpenAIConfig::default();
    let client = Client::with_config(config);
    let request = CreateModerationRequestArgs::default()
        .input(task_response.input)
        .model(TextModerationModel::Latest)
        .build()?;

    let model_response = client.moderations().create(request).await?;

    let flags = model_response
        .results
        .iter()
        .map(|r| r.flagged as u8)
        .collect::<Vec<_>>();

    log::debug!("Model response flags: {flags:?}");

    let payload = json!({ "answer" : flags});
    Ok(payload)
}
