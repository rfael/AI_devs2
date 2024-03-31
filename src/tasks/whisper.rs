use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail};
use async_openai::{
    config::OpenAIConfig,
    types::{AudioResponseFormat, CreateTranscriptionRequestArgs},
    Client,
};
use futures::stream::StreamExt;
use regex::Regex;
use serde::Deserialize;
use serde_json::{json, Value};
use tempfile::tempdir;
use tokio::{fs::File, io::AsyncWriteExt};
use url::Url;

use crate::{aidevs, config::Config};

const MODEL: &str = "whisper-1";

#[derive(Debug, Deserialize)]
struct WhisperTaskResponse {
    code: i32,
    hint: String,
    msg: String,
}

/// The task involved downloading an audio file from the received link and converting it to text using the Whisper model.
///
/// * `config`: App configuration
/// * `token`: Task token
pub(super) async fn run(config: &Config, token: &str) -> anyhow::Result<Value> {
    let task_response = aidevs::get_task::<WhisperTaskResponse>(config, token).await?;
    log::debug!("Task API response: {task_response:#?}");
    if task_response.code != 0 {
        bail!("Code in response is not equal 0")
    }
    log::info!("Task hint: {}", task_response.hint);
    log::info!("Task message: {}", task_response.msg);

    let url_re = Regex::new(r"http://[^\s]+|https://[^\s]+")?;

    let audio_source_url = url_re
        .captures_iter(&task_response.msg)
        .next()
        .and_then(|c| c.get(0))
        .map(|c| Url::parse(c.as_str()))
        .ok_or(anyhow!(
            "Audio source URL not found in task API response message."
        ))??;
    log::debug!("Audio source URL: {audio_source_url}");

    let tmp_dir = tempdir()?;
    let audio_file_path = download_as_tmp_file(audio_source_url, tmp_dir.path()).await?;

    let openai_config = OpenAIConfig::default();
    let client = Client::with_config(openai_config);
    let request = CreateTranscriptionRequestArgs::default()
        .file(audio_file_path)
        .model(MODEL)
        .response_format(AudioResponseFormat::Json)
        .build()?;

    let response = client.audio().transcribe(request).await?;
    log::info!("{MODEL} response: {}", response.text);

    let payload = json!({ "answer" : response.text});
    Ok(payload)
}

async fn download_as_tmp_file(url: Url, dest_dir: impl AsRef<Path>) -> anyhow::Result<PathBuf> {
    let out_file_name = url
        .path_segments()
        .and_then(|s| s.last())
        .ok_or(anyhow!("Can not extract last path segnemt from {url}"))?;
    let out_file_path = dest_dir.as_ref().join(out_file_name);
    log::debug!(
        "Downloading {url} to {path}",
        path = out_file_path.display()
    );

    let mut audio_stream = reqwest::get(url).await?.bytes_stream();
    let mut out_file = File::create(&out_file_path).await?;
    while let Some(item) = audio_stream.next().await {
        let audio_bytes = item?;
        out_file.write_all(&audio_bytes).await?;
    }

    Ok(out_file_path)
}
