use anyhow::anyhow;
use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
        CreateChatCompletionRequestArgs, CreateEmbeddingRequestArgs, Embedding,
    },
    Client,
};
use qdrant_client::{
    client::QdrantClient,
    qdrant::{
        vectors_config, CreateCollection, Distance, SearchPoints, SearchResponse, VectorParams,
        VectorsConfig,
    },
};

pub const EMBEDDING_MODEL: &str = "text-embedding-ada-002";

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

pub(crate) async fn qdrant_create_collection(
    client: &QdrantClient,
    collection: &str,
) -> anyhow::Result<()> {
    let vector_params = VectorParams {
        size: 1536,
        distance: Distance::Cosine.into(),
        ..Default::default()
    };
    let vectors_config = VectorsConfig {
        config: Some(vectors_config::Config::Params(vector_params)),
    };
    let collection_details = CreateCollection {
        collection_name: collection.into(),
        vectors_config: Some(vectors_config),
        ..Default::default()
    };
    client.create_collection(&collection_details).await?;

    Ok(())
}

pub(crate) async fn qdrand_search(
    qdrant_client: &QdrantClient,
    openai_client: &Client<OpenAIConfig>,
    collection: &str,
    query: &str,
) -> anyhow::Result<SearchResponse> {
    let query_embedding = embed_text(openai_client, EMBEDDING_MODEL, query)
        .await?
        .embedding;

    let request = SearchPoints {
        collection_name: collection.into(),
        vector: query_embedding,
        limit: 1,
        with_payload: Some(true.into()),
        ..Default::default()
    };

    let search_result = qdrant_client.search_points(&request).await?;
    Ok(search_result)
}
