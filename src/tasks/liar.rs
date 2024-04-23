use anyhow::{anyhow, bail};
use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
        CreateChatCompletionRequestArgs,
    },
    Client,
};
use reqwest::multipart::Form;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::config::Config;

const MODEL: &str = "gpt-3.5-turbo";

#[derive(Debug, Deserialize)]
struct LiarTaskResponse {
    code: i32,
    msg: String,
    answer: Option<String>,
}

/// The task involved checking whether the test API responds to questions truthfully or not.
/// This is an example of the Guardrails method for verifying the responses of the LLM model.
///
/// * `config`: App configuration
/// * `token`: Task token
pub(super) async fn run(config: &Config, token: &str) -> anyhow::Result<Value> {
    let question = "What is SSL certificate?";
    let answer = get_task_api_answer(config, token, question).await?;
    log::info!("Task API answer: {answer}");

    let openai_config = OpenAIConfig::default();
    let client = Client::with_config(openai_config);
    let request = CreateChatCompletionRequestArgs::default()
        .model(MODEL)
        .messages([
            ChatCompletionRequestSystemMessageArgs::default()
                .content("You are a verifier of the truthfulness of answers. Respond briefly with YES or NO whether the given question and answer match.")
                .build()?
                .into(),
            ChatCompletionRequestUserMessageArgs::default()
                .content(format!("{question}\n\n{answer}"))
                .build()?
                .into()
        ]).build()?;

    let response = client.chat().create(request).await?;
    let mut verdicts = response
        .choices
        .into_iter()
        .filter_map(|c| c.message.content)
        .collect::<Vec<_>>();

    let firs_verdict = verdicts
        .pop()
        .ok_or(anyhow!("Model response do not contain any verdicts"))?;

    log::debug!("Versict: {firs_verdict}");
    if firs_verdict != "YES" && firs_verdict != "NO" {
        bail!("Model verdict is not equal YES or NO");
    }

    let payload = json!({ "answer" : firs_verdict});
    Ok(payload)
}

async fn get_task_api_answer(
    config: &Config,
    token: &str,
    question: &'static str,
) -> anyhow::Result<String> {
    let mut url = config.api_url.clone();
    url.set_path(&format!("task/{token}"));

    let form = Form::new().text("question", question);
    let api_client = reqwest::Client::new();
    let task_response = api_client
        .post(url)
        .multipart(form)
        .send()
        .await?
        .json::<LiarTaskResponse>()
        .await?;
    log::debug!("Task API response: {task_response:#?}");
    log::info!("Task message: {}", task_response.msg);

    if task_response.code != 0 {
        bail!("Code in response is not equal 0")
    }

    task_response
        .answer
        .ok_or(anyhow!("Liar task API response does not contain answer"))
}
