use std::{sync::Arc, time::Duration};

use anyhow::{anyhow, bail};
use async_openai::{config::OpenAIConfig, Client};
use chrono::Local;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tide::StatusCode;
use tokio::time::sleep;

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
    reply: String,
}

#[derive(Clone)]
struct OwnapiState {
    openai_client: Arc<Client<OpenAIConfig>>,
    llm_context: Arc<Option<String>>,
}

impl OwnapiState {
    fn new(llm_context: Option<String>) -> Self {
        let openai_config = OpenAIConfig::default();
        let openai_client = Client::with_config(openai_config);
        Self {
            openai_client: Arc::new(openai_client),
            llm_context: Arc::new(llm_context),
        }
    }
}

/// The task was to create an API that responds to the received questions.
///
/// * `config`: App configuration
/// * `token`: Task token
pub(super) async fn run(config: &Config, token: &str) -> anyhow::Result<()> {
    let api_listen_addr = config
        .api_listen_address
        .as_ref()
        .ok_or(anyhow!("API listen address not specified"))?
        .clone();
    let mut api_tunnel_url = config
        .api_tunnel_url
        .as_ref()
        .ok_or(anyhow!("Ngrok tunnel URL not found in configuration"))?
        .clone();
    api_tunnel_url.set_path("ownapi");
    log::info!("API tunneled endpoint: {api_tunnel_url}");

    let task_response = aidevs::get_task::<OwnapiTaskResponse>(config, token).await?;
    log::debug!("Task API response: {task_response:#?}");
    log::info!("Task message: {}", task_response.msg);

    if task_response.code != 0 {
        bail!("Code in response is not equal 0")
    }

    let today = Local::now();
    let llm_context = [
        "Answer concisely as possible",
        "If you do not know answer for the question say 'I do not know'",
        &format!("Today is: {today}"),
    ]
    .join("\n");

    let state = OwnapiState::new(Some(llm_context));
    let mut app = tide::with_state(state);
    app.at("/ownapi").post(answer);

    let api_future = tokio::spawn(app.listen(api_listen_addr));
    sleep(Duration::from_secs(1)).await;

    let payload = json!({ "answer" : api_tunnel_url});
    let answer_response = aidevs::post_answer(config, token, &payload).await?;
    if answer_response.code != 0 {
        bail!(
            "Post answer failed: [{}] {}",
            answer_response.code,
            answer_response.msg
        )
    }

    api_future.await??;

    Ok(())
}

async fn answer(mut request: tide::Request<OwnapiState>) -> tide::Result {
    let OwnapiRequest { question } = request.body_json().await?;
    log::debug!("Received question: {question}");

    let state = request.state();
    let OwnapiState {
        openai_client,
        llm_context,
    } = state;

    let reply = utils::ask_llm(openai_client, MODEL, &question, llm_context.as_deref()).await?;
    let response_body = tide::Body::from_json(&OwnapiResponse { reply })?;
    let mut response = tide::Response::new(StatusCode::Ok);
    response.set_body(response_body);

    Ok(response)
}
