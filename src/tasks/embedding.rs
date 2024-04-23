use anyhow::anyhow;
use async_openai::{config::OpenAIConfig, types::CreateEmbeddingRequestArgs, Client};
use serde_json::{json, Value};

const INPUT: &str = "Hawaiian pizza";
const MODEL: &str = "text-embedding-ada-002";

/// The task involved generating embedding array for 'Hawaiian pizza' input.
///
pub(super) async fn run() -> anyhow::Result<Value> {
    log::info!("Embbedding genetation for '{INPUT}' phrase using {MODEL} model.");

    let openai_config = OpenAIConfig::default();
    let client = Client::with_config(openai_config);
    let request = CreateEmbeddingRequestArgs::default()
        .model(MODEL)
        .input(INPUT)
        .build()?;

    let mut response = client.embeddings().create(request).await?;
    let embedding = response
        .data
        .pop()
        .ok_or(anyhow!("Model response do not containg embedding array"))?
        .embedding;

    log::info!("Received embedding array length: {}", embedding.len());

    let payload = json!({ "answer" : embedding});
    Ok(payload)
}
