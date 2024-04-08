use anyhow::{anyhow, bail};
use async_openai::{config::OpenAIConfig, Client};
use chrono::NaiveDate;
use qdrant_client::{
    client::{Payload, QdrantClient},
    qdrant::{
        vectors_config, CreateCollection, Distance, PointStruct, SearchPoints, SearchResponse,
        VectorParams, VectorsConfig,
    },
};
use serde::Deserialize;
use serde_json::{json, Value};
use url::Url;

use crate::{aidevs, config::Config, utils::embed_text};

const QDRANT_COLLECTION: &str = "unknowNews";
const UNKNOW_NEWS_ARCHIVE_URL: &str = "https://unknow.news/archiwum_aidevs.json";
const EMBEDDING_MODEL: &str = "text-embedding-ada-002";

#[derive(Debug, Deserialize)]
struct SearchTaskResponse {
    code: i32,
    msg: String,
    question: String,
}

#[derive(Debug, Deserialize)]
struct UnknowNewsItem {
    title: String,
    url: Url,
    info: String,
    date: NaiveDate,
}

#[derive(Debug, Deserialize)]
#[serde(transparent)]
struct UnknowNews {
    news: Vec<UnknowNewsItem>,
}

impl From<UnknowNewsItem> for Payload {
    fn from(value: UnknowNewsItem) -> Self {
        let mut payload = Self::new();
        payload.insert("title", value.title);
        payload.insert("url", value.url.to_string());
        payload.insert("info", value.info);
        payload.insert("date", value.date.to_string());
        payload
    }
}

/// The task involved creating a vector database collection containing an archive of links from the UnknowNews newsletter
/// and then finding a link in it that matches the received query.
///
/// * `config`: App configuration
/// * `token`: Task token
pub(super) async fn run(config: &Config, token: &str) -> anyhow::Result<Value> {
    let task_response = aidevs::get_task::<SearchTaskResponse>(config, token).await?;
    log::debug!("Task API response: {task_response:#?}");
    log::info!("Task message: {}", task_response.msg);

    if task_response.code != 0 {
        bail!("Code in response is not equal 0")
    }
    log::info!("Task question: {}", task_response.question);

    let qdrant_url = config
        .qdrant_url
        .as_ref()
        .ok_or(anyhow!("Qdrant URL not found in configuration"))?;
    let qdrant_client = QdrantClient::from_url(qdrant_url.as_str()).build()?;

    let collection_info_result = qdrant_client
        .collection_info(QDRANT_COLLECTION)
        .await
        .ok()
        .and_then(|i| i.result);
    let collection_info = match collection_info_result {
        Some(i) => i,
        None => {
            log::info!("Qdrant collection '{QDRANT_COLLECTION}' does not exists, creating it");
            qdrant_create_collection(&qdrant_client).await?;
            qdrant_client
                .collection_info(QDRANT_COLLECTION)
                .await?
                .result
                .ok_or(anyhow!(
                    "Qdrant collection '{QDRANT_COLLECTION}' created but still can not get it info"
                ))?
        }
    };

    let openai_config = OpenAIConfig::default();
    let openai_client = Client::with_config(openai_config);

    if collection_info.vectors_count() == 0 {
        log::info!("Qdrant collection '{QDRANT_COLLECTION}' empty, filling it");
        qdrant_fill_collection(&qdrant_client, &openai_client).await?;
    }

    let response = qdrand_search(&qdrant_client, &openai_client, &task_response.question).await?;
    let result = response
        .result
        .first()
        .ok_or(anyhow!("Qdrant search response empty"))?;
    let answer = result
        .payload
        .get("url")
        .ok_or(anyhow!("Qdrand result payload od not contain 'url' field"))?;

    log::info!("Answer: {answer}");

    let payload = json!({ "answer" : answer});
    Ok(payload)
}

async fn get_unknow_news_archive() -> anyhow::Result<UnknowNews> {
    log::info!("Fetching UnkonwNews archive from {UNKNOW_NEWS_ARCHIVE_URL}");
    let response = reqwest::get(UNKNOW_NEWS_ARCHIVE_URL)
        .await?
        .json::<UnknowNews>()
        .await?;
    Ok(response)
}

async fn qdrant_create_collection(client: &QdrantClient) -> anyhow::Result<()> {
    let vector_params = VectorParams {
        size: 1536,
        distance: Distance::Cosine.into(),
        ..Default::default()
    };
    let vectors_config = VectorsConfig {
        config: Some(vectors_config::Config::Params(vector_params)),
    };
    let collection_details = CreateCollection {
        collection_name: QDRANT_COLLECTION.into(),
        vectors_config: Some(vectors_config),
        ..Default::default()
    };
    client.create_collection(&collection_details).await?;

    Ok(())
}

async fn qdrant_fill_collection(
    qdrant_client: &QdrantClient,
    openai_client: &Client<OpenAIConfig>,
) -> anyhow::Result<()> {
    let news = get_unknow_news_archive().await?.news;

    let mut points = Vec::with_capacity(news.len());
    for (index, item) in news.into_iter().enumerate() {
        let embedding = embed_text(openai_client, EMBEDDING_MODEL, &item.info)
            .await?
            .embedding;

        let point = PointStruct::new(index as u64, embedding, item.into());
        points.push(point);
    }
    qdrant_client
        .upsert_points_blocking(QDRANT_COLLECTION, None, points, None)
        .await?;

    Ok(())
}

async fn qdrand_search(
    qdrant_client: &QdrantClient,
    openai_client: &Client<OpenAIConfig>,
    query: &str,
) -> anyhow::Result<SearchResponse> {
    let query_embedding = embed_text(openai_client, EMBEDDING_MODEL, query)
        .await?
        .embedding;

    let request = SearchPoints {
        collection_name: QDRANT_COLLECTION.into(),
        vector: query_embedding,
        limit: 1,
        with_payload: Some(true.into()),
        ..Default::default()
    };

    let search_result = qdrant_client.search_points(&request).await?;
    Ok(search_result)
}
