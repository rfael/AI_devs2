use anyhow::{anyhow, bail};
use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
        CreateChatCompletionRequestArgs,
    },
    Client,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{aidevs, config::Config};

const MODEL: &str = "gpt-3.5-turbo";

#[derive(Debug, Deserialize)]
struct InPromptTaskResponse {
    code: i32,
    msg: String,
    input: Vec<String>,
    question: String,
}

/// The task involved initially filtering the data to reduce the length of the context
/// and then responding to the received question based on it.
///
/// * `config`: App configuration
/// * `token`: Task token
pub(super) async fn run(config: &Config, token: &str) -> anyhow::Result<Value> {
    let task_response = aidevs::get_task::<InPromptTaskResponse>(config, token).await?;
    log::info!("Task message: {}", task_response.msg);

    if task_response.code != 0 {
        bail!("Code in response is not equal 0")
    }
    log::info!("Question: {}", task_response.question);

    let name = find_capitalized_word(&task_response.question)
        .ok_or(anyhow!("Name in question not found."))?;
    let context_header = vec![
        "Answer on my question only using data prowided after ### markers.",
        "Answers concisely as possible",
        "###",
    ];
    let context = context_header
        .into_iter()
        .chain(
            task_response
                .input
                .iter()
                .filter(|sentence| sentence.contains(name))
                .map(|s| s.as_str()),
        )
        .collect::<Vec<_>>()
        .join("\n");

    log::debug!("System message content: {context}");

    let openai_config = OpenAIConfig::default();
    let client = Client::with_config(openai_config);
    let system_message = ChatCompletionRequestSystemMessageArgs::default()
        .content(context)
        .build()?;

    let request = CreateChatCompletionRequestArgs::default()
        .model(MODEL)
        .messages([
            system_message.into(),
            ChatCompletionRequestUserMessageArgs::default()
                .content(task_response.question)
                .build()?
                .into(),
        ])
        .build()?;

    let response = client.chat().create(request).await?;
    let answer = response
        .choices
        .into_iter()
        .find_map(|c| c.message.content)
        .ok_or(anyhow!("GPT response do not contain answer."))?;

    log::info!("GPT answer: {answer}");

    let payload = json!({ "answer" : answer});
    Ok(payload)
}

fn find_capitalized_word(line: &str) -> Option<&str> {
    line.split_whitespace()
        .find(|word| word.chars().next().map_or(false, |c| c.is_uppercase()))
        .map(|s| s.trim_end_matches(|c: char| !c.is_alphabetic()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_capitalized_word() {
        assert_eq!(find_capitalized_word("siema Michał!"), Some("Michał"));
        assert_eq!(find_capitalized_word("no body is here!"), None);
        assert_eq!(find_capitalized_word("my name is James."), Some("James"));
    }
}
