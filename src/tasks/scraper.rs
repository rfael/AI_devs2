use anyhow::{anyhow, bail};
use reqwest::header;
use reqwest_middleware::ClientBuilder;
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
use serde::Deserialize;
use serde_json::{json, Value};
use url::Url;

use crate::{aidevs, config::Config, utils::ask_llm};

const MODEL: &str = "gpt-3.5-turbo";
const MAX_AMSWER_LENGTH: usize = 200;

#[derive(Debug, Deserialize)]
struct ScraperTaskResponse {
    code: i32,
    input: Url,
    msg: String,
    question: String,
}

/// The task involved retrieving the specified document and answering a given question based on it.
/// The main challenge was dealing with the article server errors, which was designed to be unstable.
///
/// * `config`: App configuration
/// * `token`: Task token
pub(super) async fn run(config: &Config, token: &str) -> anyhow::Result<Value> {
    let task_response = aidevs::get_task::<ScraperTaskResponse>(config, token).await?;
    log::debug!("Task API response: {task_response:#?}");
    if task_response.code != 0 {
        bail!("Code in response is not equal 0")
    }
    log::info!("Task message: {}", task_response.msg);
    log::info!("Task question: {}", task_response.question);

    let article = download_txt(task_response.input).await?;

    let context_header = [
        "Answer on my question only using data prowided after ### markers.",
        "Answers concisely as possible",
        "###",
    ]
    .join("\n");
    let context = format!("{context_header}\n{article}");
    log::debug!("Context for LLM: {context}");

    let answer = ask_llm(MODEL, &task_response.question, Some(&context)).await?;
    if answer.len() > MAX_AMSWER_LENGTH {
        bail!("{MODEL} answer too long.")
    }

    let payload = json!({ "answer" : answer});
    Ok(payload)
}

async fn download_txt(source: Url) -> anyhow::Result<String> {
    let retry_policy = ExponentialBackoff::builder().build_with_max_retries(5);
    let client = ClientBuilder::new(reqwest::Client::new())
        .with(RetryTransientMiddleware::new_with_policy(retry_policy))
        .build();

    log::info!("Downloading txt file from {source}");

    let user_agents = [
        "Chrome/123.0.0.0",
        "Safari/537.36",
        "Mozilla/5.0 (X11; Linux x86_64)",
        "AppleWebKit/537.36 (KHTML, like Gecko)",
    ];

    for user_agent in user_agents {
        log::debug!("Download try with user agent '{user_agent}'");
        let response = client
            .get(source.clone())
            .header(header::USER_AGENT, user_agent)
            .send()
            .await;

        let response = match response {
            Ok(r) => r,
            Err(err) => {
                log::error!("Requeset error: {err}");
                continue;
            }
        };

        let content = response.text().await?;
        if content.contains("bot detected") {
            log::debug!("Server deteced bot, trying next user agent.");
            continue;
        }

        return Ok(content);
    }

    Err(anyhow!("Text download from {source} failed"))
}
