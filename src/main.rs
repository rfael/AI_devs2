mod aidevs;

use std::env;

use aidevs::{AnswerResponse, HelloApiResponse};
use anyhow::{anyhow, bail};
use dotenv::dotenv;
use envconfig::Envconfig;
use serde_json::json;

use crate::aidevs::{Config, HintResponse, TokenResponse};

const TASK_NAME: &str = "helloapi";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    if env::var("RUST_LOG").is_ok() {
        env_logger::init();
    }

    let config = Config::init_from_env()?;

    let token = get_task_token(&config, TASK_NAME).await?;
    log::debug!("Received token: {token}");

    let task_response = get_task(&config, &token).await?;
    let cookie = task_response
        .cookie
        .ok_or(anyhow!("API task response do not contain 'cookie' field"))?;

    post_answer(&config, &token, &cookie).await?;

    Ok(())
}

async fn get_task_token(config: &Config, task_name: &str) -> anyhow::Result<String> {
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
async fn get_hint(config: &Config, task_name: &str) -> anyhow::Result<()> {
    let mut url = config.api_url.clone();
    url.set_path(&format!("hint/{task_name}"));

    let response = reqwest::get(url).await?.json::<HintResponse>().await?;

    log::info!("Hint: {}", response.answer);

    Ok(())
}

async fn get_task(config: &Config, token: &str) -> anyhow::Result<HelloApiResponse> {
    let mut url = config.api_url.clone();
    url.set_path(&format!("task/{token}"));

    let response = reqwest::get(url).await?.json::<HelloApiResponse>().await?;

    log::debug!("Task response: {response:#?}");

    Ok(response)
}

async fn post_answer(config: &Config, token: &str, answer: &str) -> anyhow::Result<AnswerResponse> {
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
