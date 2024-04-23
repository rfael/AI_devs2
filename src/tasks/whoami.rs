use anyhow::bail;
use async_openai::{config::OpenAIConfig, Client};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{aidevs, config::Config, utils::ask_llm};

const MODEL: &str = "gpt-4";

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct WhoAmITaskResponse {
    code: i32,
    hint: String,
    msg: String,
}

/// The task consisted of guessing the first and last name of a person based on hints received from the AI_Devs API.
/// Each query to this API returned a different hint.
///
/// * `config`: App configuration
/// * `token`: Task token
pub(super) async fn run(config: &Config, token: &str) -> anyhow::Result<Value> {
    let question = "Who is being talked about?";
    let context_header = [
        "Answer on my question using data prowided after ### markers and your base knowledge",
        "Answer concisely as possible",
        "If you do not know the persons name and surname reply only with 'Not enough data'",
        "",
        "###",
    ];

    let mut context = context_header.join("\n");

    let openai_config = OpenAIConfig::default();
    let client = Client::with_config(openai_config);

    loop {
        let hint = get_next_hint(config, token).await?;
        if context.contains(&hint) {
            continue;
        }

        context.push_str(&hint);

        let answer = ask_llm(&client, MODEL, question, Some(&context)).await?;
        if answer.contains("Not enough data") {
            log::info!("Not enough data, fetching next hint");
            continue;
        }

        let payload = json!({ "answer" : answer});
        return Ok(payload);
    }
}

async fn get_next_hint(config: &Config, token: &str) -> anyhow::Result<String> {
    let task_response = aidevs::get_task::<WhoAmITaskResponse>(config, token).await?;
    log::debug!("Task API response: {task_response:#?}");

    if task_response.code != 0 {
        bail!("Code in response is not equal 0")
    }

    Ok(task_response.hint)
}
