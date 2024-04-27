use anyhow::{anyhow, bail};
use serde::Deserialize;
use serde_json::{json, Value};
use url::Url;

use crate::{
    aidevs,
    config::Config,
    render_form::{
        RenderFormClient, RenderFormRenderDataBuilder, RenderFormRenderDataField,
        RenderFormRenderRequest,
    },
};

const RENDER_FORM_TEMPLATE: &str = "lively-snakes-smash-gently-1459";

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct MemeTaskResponse {
    code: i32,
    hint: Url,
    image: Url,
    msg: String,
    service: Url,
    text: String,
}

/// The task was to generate a meme from the received image and text.
///
/// * `config`: App configuration
/// * `token`: Task token
pub(super) async fn run(config: &Config, token: &str) -> anyhow::Result<Value> {
    let render_form_api_key = config
        .render_form_api_key
        .as_ref()
        .ok_or(anyhow!("RenderForm API key not found in configuration"))?;

    let task_response = aidevs::get_task::<MemeTaskResponse>(config, token).await?;
    log::info!("Task message: {}", task_response.msg);

    if task_response.code != 0 {
        bail!("Code in response is not equal 0")
    }

    let render_client = RenderFormClient::new(render_form_api_key);
    let request_data = RenderFormRenderDataBuilder::new()
        .set(
            "image",
            RenderFormRenderDataField::Source(task_response.image),
        )
        .set("title", RenderFormRenderDataField::Text(task_response.text))
        .build();
    let request = RenderFormRenderRequest {
        template: RENDER_FORM_TEMPLATE.into(),
        data: Some(request_data),
        ..Default::default()
    };

    let response = render_client.render(request).await?;
    log::info!("Rendered image URL: {}", response.href);

    let payload = json!({ "answer" : response.href});
    Ok(payload)
}
