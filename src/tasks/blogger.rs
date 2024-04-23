use anyhow::{anyhow, bail};
use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
        CreateChatCompletionRequestArgs,
    },
    Client,
};
use reqwest::Response;
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Debug, Deserialize)]
struct BloggerTaskResponse {
    code: i32,
    msg: String,
    blog: Vec<String>,
}

/// The task involves fetching paragraph topics from the API of a culinary blog about Margherita pizza.
/// Each topic should be expanded using an LLM model and return the resulting paragraphs to the API.
///
/// * `task_api_response`:
pub(super) async fn run(task_api_response: Response) -> anyhow::Result<Value> {
    let task_response = task_api_response.json::<BloggerTaskResponse>().await?;
    log::debug!("Task API response: {task_response:#?}");
    log::info!("Task message: {}", task_response.msg);

    if task_response.code != 0 {
        bail!("Code in response is not equal 0")
    }

    let config = OpenAIConfig::default();
    let client = Client::with_config(config);
    let system_message = ChatCompletionRequestSystemMessageArgs::default()
        .content("You're a culinary blogger, you write a blog about Margherita pizza. Expand on the topic provided in Polish.")
        .build()?;

    let mut chapters = Vec::with_capacity(task_response.blog.len());
    for chapter_topic in task_response.blog {
        log::info!("Request for chapter about: {chapter_topic}");
        let request = CreateChatCompletionRequestArgs::default()
            .model("gpt-3.5-turbo")
            .messages([
                system_message.clone().into(),
                ChatCompletionRequestUserMessageArgs::default()
                    .content(chapter_topic.as_str())
                    .build()?
                    .into(),
            ])
            .build()?;

        let mut response = client.chat().create(request).await?;
        let choice = response.choices.pop().ok_or(anyhow!(
            "Response for chapter topic '{chapter_topic}' does not contain choises."
        ))?;
        let chapter = choice.message.content.ok_or(anyhow!(
            "First choice for chapter topic '{chapter_topic}' does not contain content."
        ))?;

        log::info!("Chapter content: {chapter}");
        chapters.push(chapter);
    }

    let payload = json!({ "answer" : chapters});
    Ok(payload)
}
