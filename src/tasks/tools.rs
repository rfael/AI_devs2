use anyhow::{anyhow, bail};
use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
        CreateChatCompletionRequestArgs,
    },
    Client,
};
use chrono::{Local, NaiveDate};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{aidevs, config::Config};

const MODEL: &str = "gpt-3.5-turbo";

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ToolsTaskResponse {
    code: i32,
    #[serde(rename = "example for Calendar")]
    example_calendar: String,
    #[serde(rename = "example for ToDo")]
    example_todo: String,
    hint: String,
    msg: String,
    question: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "tool")]
enum Tool {
    Calendar {
        desc: String,
        #[serde(with = "date_format")]
        date: NaiveDate,
    },
    ToDo {
        desc: String,
    },
}

/// The task consisted of assigning the appropriate tool (Calendar or ToDo) to the received query.
///
/// * `config`: App configuration
/// * `token`: Task token
pub(super) async fn run(config: &Config, token: &str) -> anyhow::Result<Value> {
    let task_response = aidevs::get_task::<ToolsTaskResponse>(config, token).await?;
    log::debug!("Task API response: {task_response:#?}");
    log::info!("Task message: {}", task_response.msg);
    log::info!("Example for Calendar: {}", task_response.example_calendar);
    log::info!("Example for ToDo: {}", task_response.example_todo);

    if task_response.code != 0 {
        bail!("Code in response is not equal 0")
    }

    let openai_config = OpenAIConfig::default();
    let openai_client = Client::with_config(openai_config);

    let today = Local::now();

    let system_message = ChatCompletionRequestSystemMessageArgs::default()
        .content(format!(
            "{}\n{}\nToday date: {}\nExamples:\n{}\n{}",
            task_response.msg,
            task_response.hint,
            today.format(date_format::FORMAT),
            task_response.example_calendar,
            task_response.example_todo
        ))
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
    let response = openai_client.chat().create(request).await?;
    let answer = response
        .choices
        .into_iter()
        .find_map(|c| c.message.content)
        .ok_or(anyhow!("{MODEL} response do not contain answer."))?;
    log::info!("{MODEL} answer: {answer}");

    // Check answer format
    let tool: Tool = serde_json::from_str(&answer)?;
    log::debug!("Selected tool: {tool:?}");

    let payload = json!({ "answer" : tool});
    Ok(payload)
}

mod date_format {
    use chrono::NaiveDate;
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub const FORMAT: &str = "%Y-%m-%d";

    pub fn serialize<S>(date: &NaiveDate, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!("{}", date.format(FORMAT));
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<NaiveDate, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        NaiveDate::parse_from_str(&s, FORMAT).map_err(serde::de::Error::custom)
    }
}
