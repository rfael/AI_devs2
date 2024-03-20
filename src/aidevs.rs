use envconfig::Envconfig;
use serde::Deserialize;
use url::Url;

#[derive(Debug, Envconfig)]
pub struct Config {
    #[envconfig(from = "AI_DEVS2_API_URL")]
    pub api_url: Url,
    #[envconfig(from = "AI_DEVS2_API_KEY")]
    pub api_key: String,
}

#[derive(Debug, Deserialize)]
pub struct TokenResponse {
    pub code: i32,
    pub msg: String,
    pub token: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AnswerResponse {
    pub code: i32,
    pub msg: String,
}

#[derive(Debug, Deserialize)]
pub struct HintResponse {
    pub answer: String,
}

#[derive(Debug, Deserialize)]
pub struct HelloApiResponse {
    pub code: i32,
    pub msg: String,
    pub cookie: Option<String>,
}
