use anyhow::{anyhow, bail};
use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestSystemMessage, ChatCompletionRequestSystemMessageArgs,
        ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs,
    },
    Client,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{aidevs, config::Config};

const MODEL: &str = "gpt-3.5-turbo";

#[derive(Debug, Deserialize)]
struct BloggerTaskResponse {
    code: i32,
    msg: String,
    blog: Vec<String>,
}

/// The task involves fetching paragraph topics from the API of a culinary blog about Margherita pizza.
/// Each topic should be expanded using an LLM model and return the resulting paragraphs to the API.
///
/// * `config`: App configuration
/// * `token`: Task token
pub(super) async fn run(config: &Config, token: &str) -> anyhow::Result<Value> {
    let task_response = aidevs::get_task::<BloggerTaskResponse>(config, token).await?;
    log::debug!("Task API response: {task_response:#?}");
    log::info!("Task message: {}", task_response.msg);

    if task_response.code != 0 {
        bail!("Code in response is not equal 0")
    }

    let openai_config = OpenAIConfig::default();
    let client = Client::with_config(openai_config);
    let system_message = ChatCompletionRequestSystemMessageArgs::default()
        .content("You're a culinary blogger, you write a blog about Margherita pizza. Expand on the topic provided in Polish.")
        .build()?;

    let mut chapters = Vec::with_capacity(task_response.blog.len());
    for chapter_topic in task_response.blog {
        log::info!("Request for chapter about: {chapter_topic}");

        let chapter =
            generate_chapter_content(&client, system_message.clone(), &chapter_topic).await?;

        log::info!("Chapter content: {chapter}");
        chapters.push(chapter);
    }

    let payload = json!({ "answer" : chapters});
    Ok(payload)
}

async fn generate_chapter_content(
    client: &Client<OpenAIConfig>,
    system_message: ChatCompletionRequestSystemMessage,
    topic: &str,
) -> anyhow::Result<String> {
    log::info!("Request for chapter about: {topic}");

    let request = CreateChatCompletionRequestArgs::default()
        .model(MODEL)
        .messages([
            system_message.into(),
            ChatCompletionRequestUserMessageArgs::default()
                .content(topic)
                .build()?
                .into(),
        ])
        .build()?;

    let response = client.chat().create(request).await?;

    let mut chapter_variants = response
        .choices
        .into_iter()
        .filter_map(|c| c.message.content)
        .collect::<Vec<_>>();

    chapter_variants.pop().ok_or(anyhow!(
        "Choices for chapter topic '{topic}' do not contain content."
    ))
}
