use anyhow::{anyhow, bail};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{aidevs, config::Config, utils::ask_llm};

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

    let context = task_response
        .input
        .iter()
        .filter(|sentence| sentence.contains(name))
        .map(|c| c.as_str());

    let answer = ask_llm(MODEL, &task_response.question, Some(context)).await?;

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
