use async_openai::{config::OpenAIConfig, Client};
use serde_json::{json, Value};

use crate::utils::embed_text;

const INPUT: &str = "Hawaiian pizza";
const MODEL: &str = "text-embedding-ada-002";

/// The task involved generating embedding array for 'Hawaiian pizza' input.
///
pub(super) async fn run() -> anyhow::Result<Value> {
    log::info!("Embbedding genetation for '{INPUT}' phrase using {MODEL} model.");

    let openai_config = OpenAIConfig::default();
    let client = Client::with_config(openai_config);
    let embedding = embed_text(&client, MODEL, INPUT).await?.embedding;

    log::info!("Received embedding array length: {}", embedding.len());

    let payload = json!({ "answer" : embedding});
    Ok(payload)
}
