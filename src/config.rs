use envconfig::Envconfig;
use url::Url;

#[derive(Debug, Envconfig)]
pub(crate) struct Config {
    #[envconfig(from = "AI_DEVS2_API_URL")]
    pub api_url: Url,
    #[envconfig(from = "AI_DEVS2_API_KEY")]
    pub api_key: String,
    #[envconfig(from = "QDRANT_URL")]
    pub qdrant_url: Option<Url>,
    #[envconfig(from = "API_LISTEN_ADDRESS")]
    pub api_listen_address: Option<String>,
    #[envconfig(from = "API_TUNNEL_URL")]
    pub api_tunnel_url: Option<Url>,
    #[envconfig(from = "RENDER_FORM_API_KEY")]
    pub render_form_api_key: Option<String>,
}
