use anyhow::{anyhow, bail};
use reqwest::Response;
use serde::{Deserialize, Serialize};
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

pub async fn get_hint(config: &Config, task_name: &str) -> anyhow::Result<String> {
    let mut url = config.api_url.clone();
    url.set_path(&format!("hint/{task_name}"));

    let response = reqwest::get(url).await?.json::<HintResponse>().await?;

    log::debug!("Hint response: {response:?}");

    Ok(response.answer)
}

pub async fn get_task(config: &Config, token: &str) -> anyhow::Result<Response> {
    let mut url = config.api_url.clone();
    url.set_path(&format!("task/{token}"));

    let response = reqwest::get(url).await?;
    Ok(response)
}

pub async fn post_answer<T: Serialize>(
    config: &Config,
    token: &str,
    payload: &T,
) -> anyhow::Result<AnswerResponse> {
    let mut url = config.api_url.clone();
    url.set_path(&format!("answer/{token}"));

    let client = reqwest::Client::new();

    let response = client
        .post(url)
        .json(payload)
        .send()
        .await?
        .json::<AnswerResponse>()
        .await?;

    log::debug!("Answer response: {response:?}");

    Ok(response)
}
