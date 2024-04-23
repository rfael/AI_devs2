use anyhow::bail;
use async_openai::types::ChatCompletionFunctionsArgs;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{aidevs, config::Config};

#[derive(Debug, Deserialize)]
struct FunctionsTaskResponse {
    code: i32,
    hint1: String,
    msg: String,
}

/// The task involved creating definition of function for LLM Function Call.
/// Function requirements: definition of function named addUser that require 3 params:
/// name (string), surname (string) and year of born in field named "year" (integer).
/// Set type of function to "object"
///
/// * `config`: App configuration
/// * `token`: Task token
pub(super) async fn run(config: &Config, token: &str) -> anyhow::Result<Value> {
    let task_response = aidevs::get_task::<FunctionsTaskResponse>(config, token).await?;
    log::debug!("Task API response: {task_response:#?}");
    if task_response.code != 0 {
        bail!("Code in response is not equal 0")
    }
    log::info!("Task hint: {}", task_response.hint1);
    log::info!("Task message: {}", task_response.msg);

    let add_user_function = ChatCompletionFunctionsArgs::default()
        .name("addUser")
        .description("Add user")
        .parameters(json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "User's name"
                },
                "surname": {
                    "type": "string",
                    "description": "User's surname"
                },
                "year": {
                    "type": "integer",
                    "description": "User's year of birth"
                },
            }
        }))
        .build()?;

    let payload = json!({ "answer" : add_user_function});

    log::info!("Answer: {payload:#?}");

    Ok(payload)
}
