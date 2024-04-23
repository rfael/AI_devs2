use std::borrow::Cow;

use anyhow::anyhow;
use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
        CreateChatCompletionRequestArgs,
    },
    Client,
};

pub(crate) async fn ask_llm<'a, I>(
    model: &str,
    question: &str,
    context: Option<I>,
) -> anyhow::Result<String>
where
    I: Iterator<Item = &'a str>,
{
    let system_message_content = match context {
        Some(ctx) => {
            let context_header = [
                "Answer on my question only using data prowided after ### markers.",
                "Answers concisely as possible",
                "###",
            ];

            let content = context_header
                .into_iter()
                .chain(ctx)
                .collect::<Vec<_>>()
                .join("\n");
            Cow::from(content)
        }
        None => Cow::from("Answers concisely as possible"),
    };

    log::debug!("System message content: {system_message_content}");

    let system_message = ChatCompletionRequestSystemMessageArgs::default()
        .content(system_message_content)
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
