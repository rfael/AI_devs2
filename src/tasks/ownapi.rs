use std::sync::Arc;

use anyhow::{anyhow, bail};
use async_openai::{config::OpenAIConfig, Client};
use serde::{Deserialize, Serialize};
use tide::StatusCode;
use tokio::sync::Mutex;

use crate::{aidevs, config::Config, utils};

const MODEL: &str = "gpt-3.5-turbo";

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct OwnapiTaskResponse {
    code: i32,
    hint1: String,
    hint2: String,
    hint3: String,
    msg: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct OwnapiRequest {
    question: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct OwnapiResponse {
    answer: String,
}

#[derive(Clone)]
struct OwnapiState {
    openai_client: Arc<Mutex<Client<OpenAIConfig>>>,
}

impl OwnapiState {
    fn new() -> Self {
        let openai_config = OpenAIConfig::default();
        let openai_client = Client::with_config(openai_config);
        Self {
            openai_client: Arc::new(Mutex::new(openai_client)),
        }
    }
}

/// Test task for learning how AI_Devs 2 task API works.
/// The task involved retrieving messages from the API and returning the contents of the 'cookie' field in the response.
///
/// * `config`: App configuration
/// * `token`: Task token
pub(super) async fn run(config: &Config, token: &str) -> anyhow::Result<()> {
    let api_listen_addr = config
        .api_listen_address
        .as_ref()
        .ok_or(anyhow!("API listen address not specified"))?;
    let _ngrok_tunnel_url = config
        .ngrok_tunnel_url
        .as_ref()
        .ok_or(anyhow!("Ngrok tunnel URL not found in configuration"))?;

    let task_response = aidevs::get_task::<OwnapiTaskResponse>(config, token).await?;
    log::debug!("Task API response: {task_response:#?}");
    log::info!("Task message: {}", task_response.msg);

    if task_response.code != 0 {
        bail!("Code in response is not equal 0")
    }

    let state = OwnapiState::new();
    let mut app = tide::with_state(state);
    app.at("/ownapi").post(answer);
    app.listen(api_listen_addr).await?;

    // let payload = json!({ "answer" : cookie});
    // Ok(payload)
    Ok(())
}

async fn answer(mut request: tide::Request<OwnapiState>) -> tide::Result {
    let OwnapiRequest { question } = request.body_json().await?;
    log::debug!("Received question: {question}");

    let openai_client = request.state().openai_client.lock().await;

    let answer = utils::ask_llm(&openai_client, MODEL, &question, None).await?;
    let response_body = tide::Body::from_json(&OwnapiResponse { answer })?;
    let mut response = tide::Response::new(StatusCode::Ok);
    response.set_body(response_body);

    Ok(response)
}
