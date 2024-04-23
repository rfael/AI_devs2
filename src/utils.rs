use anyhow::anyhow;
use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
        CreateChatCompletionRequestArgs, CreateEmbeddingRequestArgs, Embedding,
    },
    Client,
};

pub(crate) async fn ask_llm(
    client: &Client<OpenAIConfig>,
    model: &str,
    question: &str,
    context: Option<&str>,
) -> anyhow::Result<String> {
    let context = context.unwrap_or("Answer concisely as possible");
    let system_message = ChatCompletionRequestSystemMessageArgs::default()
        .content(context)
        .build()?;

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

pub(crate) async fn embed_text(
    client: &Client<OpenAIConfig>,
    model: &str,
    input: &str,
) -> anyhow::Result<Embedding> {
    let request = CreateEmbeddingRequestArgs::default()
        .model(model)
        .input(input)
        .build()?;

    let mut response = client.embeddings().create(request).await?;
    let embedding = response
        .data
        .pop()
        .ok_or(anyhow!("Model response do not containg embedding array"))?;
    Ok(embedding)
}
