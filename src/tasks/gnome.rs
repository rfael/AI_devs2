use anyhow::{anyhow, bail};
use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestMessageContentPartImageArgs,
        ChatCompletionRequestMessageContentPartTextArgs, ChatCompletionRequestUserMessageArgs,
        CreateChatCompletionRequestArgs, ImageUrlArgs, ImageUrlDetail,
    },
    Client,
};
use serde::Deserialize;
use serde_json::{json, Value};
use url::Url;

use crate::{aidevs, config::Config};

const MODEL: &str = "gpt-4-vision-preview";

#[derive(Debug, Deserialize)]
struct GnomeTaskResponse {
    code: i32,
    hint: String,
    msg: String,
    url: Url,
}

/// The task was to determine the color of the gnome's hat in the picture.
///
/// * `config`: App configuration
/// * `token`: Task token
pub(super) async fn run(config: &Config, token: &str) -> anyhow::Result<Value> {
    let task_response = aidevs::get_task::<GnomeTaskResponse>(config, token).await?;
    log::debug!("Task API response: {task_response:#?}");
    log::info!("Task hint: {}", task_response.hint);
    log::info!("Task message: {}", task_response.msg);

    if task_response.code != 0 {
        bail!("Code in response is not equal 0")
    }

    let openai_config = OpenAIConfig::default();
    let openai_client = Client::with_config(openai_config);

    let request_message_text = ChatCompletionRequestMessageContentPartTextArgs::default()
        .text(
            [
                &task_response.msg,
                "hint: it won't always be a drawing of a gnome, return ERROR in this case",
                "Answer concisely as possible",
            ]
            .join("\n"),
        )
        .build()?;
    let image_url = ImageUrlArgs::default()
        .url(task_response.url)
        .detail(ImageUrlDetail::High)
        .build()?;
    let request_message_image = ChatCompletionRequestMessageContentPartImageArgs::default()
        .image_url(image_url)
        .build()?;
    let request_message = ChatCompletionRequestUserMessageArgs::default()
        .content(vec![
            request_message_text.into(),
            request_message_image.into(),
        ])
        .build()?;
    let request = CreateChatCompletionRequestArgs::default()
        .model(MODEL)
        .messages([request_message.into()])
        .build()?;

    let response = openai_client.chat().create(request).await?;
    let answer = response
        .choices
        .into_iter()
        .find_map(|c| c.message.content)
        .ok_or(anyhow!("{MODEL} response do not contain answer."))?;

    log::info!("{MODEL} answer: {answer}");

    let payload = json!({ "answer" : answer});
    Ok(payload)
}
