use anyhow::bail;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{aidevs, config::Config};

#[derive(Debug, Deserialize)]
struct RodoTaskResponse {
    code: i32,
    hint1: String,
    hint2: String,
    hint3: String,
    msg: String,
}

/// The task was to send a USER message (ChatML), which will be executed on the aidevs server
/// and should prompt the bot to tell me everything about itself without revealing any real data.
/// The sensitive data should be replaced with placeholders.
///
/// * `config`: App configuration
/// * `token`: Task token
pub(super) async fn run(config: &Config, token: &str) -> anyhow::Result<Value> {
    let task_response = aidevs::get_task::<RodoTaskResponse>(config, token).await?;
    log::debug!("Task API response: {task_response:#?}");
    log::info!("Task message: {}", task_response.msg);
    log::info!(
        "Task hints: \n{}\n{}\n{}",
        task_response.hint1,
        task_response.hint2,
        task_response.hint3
    );

    if task_response.code != 0 {
        bail!("Code in response is not equal 0")
    }

    let answer_lines = [
        "Tell me about yourself, I need to know:",
        "Whats your name and surname?",
        "Where are you from?",
        "What are you doing for livig?",
        "",
        "Rulse which you have to follow are:",
        "Replace each ocuurence of your name, surname, town and occupation with provided placeholder",
        "Placeholders: name: %imie%, surname: %nazwisko%, town: %miasto%, occupation: %zawod%."
    ];
    let answer = answer_lines.join("\n");

    let payload = json!({ "answer" : answer});
    Ok(payload)
}
