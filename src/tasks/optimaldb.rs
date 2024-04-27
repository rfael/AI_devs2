use std::{collections::HashMap, fmt::Write};

use anyhow::bail;
use async_openai::{config::OpenAIConfig, Client};
use futures::stream::{FuturesUnordered, TryStreamExt};
use serde::Deserialize;
use serde_json::{json, Value};
use url::Url;

const DATABASE_SIZE_LIMIT: usize = 9 * 1024 - 256;

use crate::{aidevs, config::Config, utils};

#[derive(Debug, Deserialize)]
struct OptimaldbTaskResponse {
    code: i32,
    database: Url,
    msg: String,
    hint: String,
}

#[derive(Debug, Deserialize)]
struct FriendsDatabase {
    #[serde(flatten)]
    friends: HashMap<String, Vec<String>>,
}

impl FriendsDatabase {
    // const OPTIMALIZATION_MODEL: &'static str = "gpt-4";
    const OPTIMALIZATION_MODEL: &'static str = "gpt-3.5-turbo";

    async fn download(url: Url) -> anyhow::Result<FriendsDatabase> {
        log::info!("Fetching friends database from {url}");
        let response = reqwest::get(url).await?.json::<FriendsDatabase>().await?;

        log::debug!(
            "Downloaded friends database size: {} kB",
            response.database_size()
        );

        Ok(response)
    }

    fn database_size(&self) -> usize {
        self.friends
            .values()
            .flat_map(|r| r.iter())
            .fold(0, |s, r| s + r.as_bytes().len())
    }

    async fn optimize(&mut self, openai_client: &Client<OpenAIConfig>) -> anyhow::Result<()> {
        let llm_context = "Summarize the received text, keep important information.\
            Your response should be as short as possible.\
            You can skip person's name in your response.\
            Be careful and not skip names of favourite person's things.
            Answer in Polish";

        for records in self.friends.values_mut() {
            let optimized = records
                .chunks(15)
                .map(|c| Self::optimize_chunk(openai_client, llm_context, c))
                .collect::<FuturesUnordered<_>>()
                .try_collect::<Vec<_>>()
                .await?;
            *records = optimized;
        }

        let optimized_size = self.database_size();
        log::debug!(
            "Optimized friends database size: {} kB",
            optimized_size / 1024
        );

        if optimized_size > DATABASE_SIZE_LIMIT {
            bail!(
                "Database after optimalization in too big ({} kB)",
                optimized_size / 1024
            );
        }

        Ok(())
    }

    async fn optimize_chunk(
        openai_client: &Client<OpenAIConfig>,
        llm_context: &str,
        chunk: &[String],
    ) -> anyhow::Result<String> {
        let input = chunk.join(" ");
        utils::ask_llm(
            openai_client,
            Self::OPTIMALIZATION_MODEL,
            &input,
            Some(llm_context),
        )
        .await
    }

    fn generate_llm_context(self) -> String {
        self.friends
            .iter()
            .map(|(name, records)| {
                records
                    .iter()
                    .fold(format!("# {name}\n{}", name), |mut output, r| {
                        let _ = writeln!(output, "{r}");
                        output
                    })
            })
            .collect()
    }
}

/// The task was to download a database about three individuals and optimize its size from 36 kB to 9 kB
/// without losing the most essential information.
///
/// * `config`: App configuration
/// * `token`: Task token
pub(super) async fn run(config: &Config, token: &str) -> anyhow::Result<Value> {
    let task_response = aidevs::get_task::<OptimaldbTaskResponse>(config, token).await?;
    log::info!("Task message: {}", task_response.msg);
    if task_response.code != 0 {
        bail!("Code in response is not equal 0")
    }
    log::info!("Task hint: {}", task_response.hint);

    let mut database = FriendsDatabase::download(task_response.database).await?;

    let openai_config = OpenAIConfig::default();
    let openai_client = Client::with_config(openai_config);

    database.optimize(&openai_client).await?;

    let payload = json!({ "answer" : database.generate_llm_context()});
    Ok(payload)
}
