use anyhow::bail;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use url::Url;

const API_V2_BASE_URL: &str = "https://get.renderform.io/api/v2";

pub enum RenderFormRenderDataField {
    Text(String),
    Source(Url),
}

pub struct RenderFormRenderDataBuilder(Map<String, Value>);

#[derive(Debug, Deserialize, Serialize)]
pub struct RenderFormRenderData(Value);

#[derive(Debug, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RenderFormRenderRequest {
    pub template: String,
    pub data: Option<RenderFormRenderData>,
    pub expires: Option<u32>,
    pub file_name: Option<String>,
    pub webhook_url: Option<Url>,
    pub metadata: Option<Value>,
    pub version: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderFormRenderResponse {
    request_id: String,
    pub href: Url,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct RenderFormApiError {
    msg: String,
    status: u32,
    #[serde(default)]
    errors: Vec<String>,
}

pub struct RenderFormClient {
    api_key: String,
    client: Client,
}

impl RenderFormRenderDataBuilder {
    pub fn new() -> Self {
        Self(Map::new())
    }

    pub fn set(mut self, component: &str, field: RenderFormRenderDataField) -> Self {
        let (key, value) = match field {
            RenderFormRenderDataField::Text(text) => (format!("{component}.text"), text),
            RenderFormRenderDataField::Source(src) => (format!("{component}.src"), src.into()),
        };
        self.0.insert(key, Value::String(value));
        self
    }

    pub fn build(self) -> RenderFormRenderData {
        RenderFormRenderData(self.0.into())
    }
}

impl RenderFormClient {
    pub fn new(api_key: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            client: Client::new(),
        }
    }

    pub async fn render(
        &self,
        request: RenderFormRenderRequest,
    ) -> anyhow::Result<RenderFormRenderResponse> {
        let response = self
            .client
            .post(format!("{API_V2_BASE_URL}/render"))
            .header("X-API-KEY", &self.api_key)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let api_error = response.json::<RenderFormApiError>().await?;
            bail!(
                "RenderForm API error: {} [{}] {:?}",
                api_error.msg,
                api_error.status,
                api_error.errors
            )
        }

        let response = response.json::<RenderFormRenderResponse>().await?;
        Ok(response)
    }
}
