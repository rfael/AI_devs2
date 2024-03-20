use envconfig::Envconfig;
use url::Url;

#[derive(Debug, Envconfig)]
pub(crate) struct Config {
    #[envconfig(from = "AI_DEVS2_API_URL")]
    pub api_url: Url,
    #[envconfig(from = "AI_DEVS2_API_KEY")]
    pub api_key: String,
}
