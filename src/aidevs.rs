use anyhow::{anyhow, bail};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::json;

use crate::config::Config;

#[derive(Debug, Deserialize)]
pub(crate) struct TokenResponse {
    pub code: i32,
    pub msg: String,
    pub token: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub(crate) struct AnswerResponse {
    pub code: i32,
    pub msg: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct HintResponse {
    pub answer: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub(crate) struct HelloApiResponse {
    pub code: i32,
    pub msg: String,
    pub cookie: Option<String>,
}

pub async fn get_task_token(config: &Config, task_name: &str) -> anyhow::Result<String> {
    let mut url = config.api_url.clone();
    url.set_path(&format!("token/{task_name}"));

    let client = reqwest::Client::new();
    let payload = json!({"apikey": config.api_key});

    let response = client
        .post(url)
        .json(&payload)
        .send()
        .await?
        .json::<TokenResponse>()
        .await?;

    if response.code != 0 {
        bail!("API call error [{}]: {}", response.code, &response.msg);
    }

    response
        .token
        .ok_or(anyhow!("API response do not contain token"))
}

#[allow(dead_code)]
pub async fn get_hint(config: &Config, task_name: &str) -> anyhow::Result<()> {
    let mut url = config.api_url.clone();
    url.set_path(&format!("hint/{task_name}"));

    let response = reqwest::get(url).await?.json::<HintResponse>().await?;

    log::info!("Hint: {}", response.answer);

    Ok(())
}

pub async fn get_task<T: DeserializeOwned>(config: &Config, token: &str) -> anyhow::Result<T> {
    let mut url = config.api_url.clone();
    url.set_path(&format!("task/{token}"));

    let response = reqwest::get(url).await?.json::<T>().await?;

    Ok(response)
}

pub async fn post_answer(
    config: &Config,
    token: &str,
    answer: &str,
) -> anyhow::Result<AnswerResponse> {
    let mut url = config.api_url.clone();
    url.set_path(&format!("answer/{token}"));

    let client = reqwest::Client::new();
    let payload = json!({ "answer" : answer});

    let response = client
        .post(url)
        .json(&payload)
        .send()
        .await?
        .json::<AnswerResponse>()
        .await?;

    log::debug!("Answer response: {response:#?}");

    Ok(response)
}
