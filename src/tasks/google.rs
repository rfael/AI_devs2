use std::{sync::Arc, time::Duration};

use anyhow::{anyhow, bail};
use async_openai::{config::OpenAIConfig, Client};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tide::StatusCode;
use tokio::time::sleep;
use url::Url;

use crate::{aidevs, brave_search::BraveSearchClient, config::Config, utils};

const MODEL: &str = "gpt-3.5-turbo";

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GoogleTaskResponse {
    code: i32,
    hint1: String,
    hint2: String,
    hint3: String,
    hint4: String,
    msg: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct GoogleRequest {
    question: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct GoogleResponse {
    reply: Url,
}

struct GoogleApiState {
    openai_client: Client<OpenAIConfig>,
    search_client: BraveSearchClient,
}

/// The task was to create an API that searches the internet and returns the URL associated with the given query.
///
/// * `config`: App configuration
/// * `token`: Task token
pub(super) async fn run(config: &Config, token: &str) -> anyhow::Result<()> {
    let brave_search_api_key = config
        .brave_search_api_key
        .as_ref()
        .ok_or(anyhow!("Brave Search API key not found in configuration"))?;
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
    api_tunnel_url.set_path("search");
    log::info!("API tunneled endpoint: {api_tunnel_url}");

    let task_response = aidevs::get_task::<GoogleTaskResponse>(config, token).await?;
    log::debug!("Task API response: {task_response:#?}");
    log::info!("Task message: {}", task_response.msg);

    if task_response.code != 0 {
        bail!("Code in response is not equal 0")
    }

    let mut search_client = BraveSearchClient::new(brave_search_api_key)?;
    search_client.set_country("PL")?;

    let openai_config = OpenAIConfig::default();
    let openai_client = Client::with_config(openai_config);

    let api_state = GoogleApiState {
        openai_client,
        search_client,
    };
    let api_state = Arc::new(api_state);
    let mut app = tide::with_state(api_state);
    app.at("/search").post(search_request_handler);

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

async fn search_request_handler(mut request: tide::Request<Arc<GoogleApiState>>) -> tide::Result {
    let GoogleRequest { question } = request.body_json().await?;
    log::debug!("Received question: {question}");

    let state = request.state();

    let context = "Rephrase provided query to format which can be used as input for search enginge like Google";
    let response = utils::ask_llm(&state.openai_client, MODEL, &question, Some(context)).await?;

    let result = state.search_client.search(&response).await?;
    let reply = result
        .web
        .results
        .first()
        .ok_or(anyhow!("No search results"))?
        .url
        .clone();

    let response_body = tide::Body::from_json(&GoogleResponse { reply })?;
    let mut response = tide::Response::new(StatusCode::Ok);
    response.set_body(response_body);

    Ok(response)
}
