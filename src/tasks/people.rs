use anyhow::{anyhow, bail};
use async_openai::{config::OpenAIConfig, Client};
use qdrant_client::{
    client::{Payload, QdrantClient},
    qdrant::{self, PointStruct},
};
use regex::Regex;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use url::Url;

use crate::{aidevs, config::Config, utils};

const QDRANT_COLLECTION: &str = "people";
const MODEL: &str = "gpt-3.5-turbo";

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct PeopleTaskResponse {
    code: i32,
    msg: String,
    data: Url,
    hint1: String,
    hint2: String,
    question: String,
}

#[derive(Debug, Deserialize)]
struct PersonInfo {
    #[serde(rename = "imie")]
    name: String,
    #[serde(rename = "nazwisko")]
    surname: String,
    #[serde(rename = "wiek")]
    age: u16,
    #[serde(rename = "o_mnie")]
    about: String,
    #[serde(rename = "ulubiona_postac_z_kapitana_bomby")]
    favourite_bomba_character: String,
    #[serde(rename = "ulubiony_serial")]
    favourite_series: String,
    #[serde(rename = "ulubiony_film")]
    favourite_movie: String,
    #[serde(rename = "ulubiony_kolor")]
    favourite_color: String,
}

impl From<PersonInfo> for Payload {
    fn from(value: PersonInfo) -> Self {
        let mut payload = Self::new();
        payload.insert("name", value.name);
        payload.insert("surname", value.surname);
        payload.insert("age", value.age as i64);
        payload.insert("about", value.about);
        payload.insert("favourite_bomba_character", value.favourite_bomba_character);
        payload.insert("favourite_series", value.favourite_series);
        payload.insert("favourite_movie", value.favourite_movie);
        payload.insert("favourite_color", value.favourite_color);
        payload
    }
}

/// The task involved retrieving a database about people set, saving it,
/// and then responding to a question asked by the AI Devs API.
///
/// * `config`: App configuration
/// * `token`: Task token
pub(super) async fn run(config: &Config, token: &str) -> anyhow::Result<Value> {
    let task_response = aidevs::get_task::<PeopleTaskResponse>(config, token).await?;
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
            utils::qdrant_create_collection(&qdrant_client, QDRANT_COLLECTION).await?;
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
        let people_data = get_people_data(&task_response.data).await?;
        qdrant_fill_collection(&qdrant_client, &openai_client, people_data).await?;
    }

    let fullname = find_fullname_in_question(&task_response.question)?;
    let response =
        utils::qdrand_search(&qdrant_client, &openai_client, QDRANT_COLLECTION, &fullname).await?;
    let result = response
        .result
        .first()
        .ok_or(anyhow!("Qdrant search response empty"))?;

    let context = build_context_from_payload(&result.payload);
    let answer = utils::ask_llm(
        &openai_client,
        MODEL,
        &task_response.question,
        Some(&context),
    )
    .await?;

    let payload = json!({ "answer" : answer});
    Ok(payload)
}

async fn get_people_data(url: &Url) -> anyhow::Result<Vec<PersonInfo>> {
    log::info!("Fetching UnkonwNews archive from {url}");
    let response = reqwest::get(url.clone())
        .await?
        .json::<Vec<PersonInfo>>()
        .await?;
    Ok(response)
}

async fn qdrant_fill_collection(
    qdrant_client: &QdrantClient,
    openai_client: &Client<OpenAIConfig>,
    peopple_data: Vec<PersonInfo>,
) -> anyhow::Result<()> {
    let mut points = Vec::with_capacity(peopple_data.len());
    for (index, person) in peopple_data.into_iter().enumerate() {
        let fullname = format!("{} {}", person.name, person.surname);
        let embedding = utils::embed_text(openai_client, utils::EMBEDDING_MODEL, &fullname)
            .await?
            .embedding;

        let point = PointStruct::new(index as u64, embedding, person.into());
        points.push(point);
    }
    qdrant_client
        .upsert_points_blocking(QDRANT_COLLECTION, None, points, None)
        .await?;

    Ok(())
}

fn find_uppercase_words(input: &str) -> Vec<&str> {
    let re = Regex::new(r"(?m)\b[A-Z]\w*\b").unwrap();
    re.find_iter(input).map(|m| m.as_str()).collect()
}

fn find_fullname_in_question(question: &str) -> anyhow::Result<String> {
    let uppercase_words = find_uppercase_words(question);

    let fullname = match uppercase_words.len() {
        len if len < 2 => bail!("Can not find person fullname in question '{question}'"),
        2 => uppercase_words.join(" "),
        len => uppercase_words[(len - 2)..].join(" "),
    };

    Ok(fullname)
}

fn build_context_from_payload(payload: &HashMap<String, qdrant::Value>) -> String {
    let context_lines = [
        match (payload.get("name"), payload.get("surnmae")) {
            (None, None) => None,
            (None, Some(s)) => Some(format!("Nazywam sie {s}\n")),
            (Some(n), None) => Some(format!("Nazywam sie {n}\n")),
            (Some(n), Some(s)) => Some(format!("Nazywam sie {s} {n}\n")),
        },
        payload.get("age").map(|a| format!("Mam {a} lat\n")),
        payload.get("about").map(|a| format!("O mnie: {a}\n")),
        payload
            .get("favourite_bomba_character")
            .map(|a| format!("Moja ulubiona postac z Kapitana Bomby: {a}\n")),
        payload
            .get("favourite_series")
            .map(|a| format!("Mój ulubiony serial: {a}\n")),
        payload
            .get("favourite_movie")
            .map(|a| format!("Mój ulubiony serial: {a}\n")),
        payload
            .get("favourite_color")
            .map(|a| format!("Mój ulubiony color: {a}\n")),
    ];

    let context = context_lines.into_iter().flatten().collect();

    log::debug!("Context:\n{context}");

    context
}
