use anyhow::anyhow;
use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
        CreateChatCompletionRequestArgs,
    },
    Client,
};

pub(crate) async fn ask_llm(
    model: &str,
    question: &str,
    context: Option<&str>,
) -> anyhow::Result<String> {
    let context = context.unwrap_or("Answers concisely as possible");
    let system_message = ChatCompletionRequestSystemMessageArgs::default()
        .content(context)
        .build()?;

    let openai_config = OpenAIConfig::default();
    let client = Client::with_config(openai_config);

    let request = CreateChatCompletionRequestArgs::default()
        .model(model)
        .messages([
            system_message.into(),
            ChatCompletionRequestUserMessageArgs::default()
                .content(question)
                .build()?
                .into(),
        ])
        .build()?;

    log::info!("Question to {model}: {question}");

    let response = client.chat().create(request).await?;
    let answer = response
        .choices
        .into_iter()
        .find_map(|c| c.message.content)
        .ok_or(anyhow!("{model} response do not contain answer."))?;

    log::info!("{model} answer: {answer}");

    Ok(answer)
}
