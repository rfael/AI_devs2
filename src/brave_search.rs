use reqwest::{
    header::{self, HeaderMap, HeaderValue},
    Client, ClientBuilder,
};
use serde::Deserialize;
use serde_with::{serde_as, NoneAsEmptyString};
use url::Url;

enum BraveSearchHeader {
    SubscriptionToken,
    LocCountry,
}

#[serde_as]
#[derive(Default, Debug, Deserialize)]
pub struct BraveSearchResponseQuery {
    pub original: String,
    pub show_strict_warning: bool,
    pub is_navigational: bool,
    pub is_news_breaking: bool,
    pub spellcheck_off: bool,
    #[serde_as(as = "NoneAsEmptyString")]
    pub country: Option<String>,
    pub bad_results: bool,
    pub should_fallback: bool,
    #[serde_as(as = "NoneAsEmptyString")]
    pub postal_code: Option<String>,
    #[serde_as(as = "NoneAsEmptyString")]
    pub city: Option<String>,
    #[serde_as(as = "NoneAsEmptyString")]
    pub header_country: Option<String>,
    pub more_results_available: bool,
    #[serde_as(as = "NoneAsEmptyString")]
    pub state: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
pub struct Profile {
    pub name: String,
    pub url: String,
    pub long_name: String,
    pub img: String,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
pub struct MetaUrl {
    pub scheme: String,
    pub netloc: String,
    pub hostname: String,
    pub favicon: String,
    pub path: String,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
pub struct Thumbnail {
    pub src: String,
    pub original: String,
    pub logo: bool,
}

#[serde_as]
#[derive(Debug, Deserialize)]
pub struct SearchResultItem {
    pub title: String,
    pub url: Url,
    pub is_source_local: bool,
    pub is_source_both: bool,
    pub description: String,
    pub page_age: Option<String>,
    pub profile: Profile,
    pub language: String,
    pub family_friendly: bool,
    pub r#type: String,
    #[serde_as(as = "NoneAsEmptyString")]
    pub subtype: Option<String>,
    pub meta_url: Option<MetaUrl>,
    pub thumbnail: Option<Thumbnail>,
    pub age: Option<String>,
}

#[derive(Default, Debug, Deserialize)]
pub struct SearchResult {
    pub r#type: String,
    pub results: Vec<SearchResultItem>,
}

#[derive(Debug, Deserialize)]
pub struct BraveSearchResponse {
    pub query: BraveSearchResponseQuery,
    pub r#type: String,
    pub web: SearchResult,
}

pub struct BraveSearchClient {
    client: Client,
    headers: HeaderMap,
}

impl BraveSearchHeader {
    fn as_str(&self) -> &'static str {
        match self {
            Self::SubscriptionToken => "X-Subscription-Token",
            Self::LocCountry => "X-Loc-Country",
        }
    }
}

impl BraveSearchClient {
    const API_BASE_URL: &'static str = "https://api.search.brave.com/res/v1";

    pub fn new(api_key: &str) -> anyhow::Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(header::ACCEPT, HeaderValue::from_static("application/json"));
        headers.insert(header::ACCEPT_ENCODING, HeaderValue::from_static("gzip"));
        headers.insert(
            BraveSearchHeader::SubscriptionToken.as_str(),
            HeaderValue::from_str(api_key)?,
        );

        let client = ClientBuilder::new().gzip(true).build()?;

        Ok(Self { client, headers })
    }

    pub fn set_country(&mut self, country_code: &str) -> anyhow::Result<()> {
        self.headers.insert(
            BraveSearchHeader::LocCountry.as_str(),
            HeaderValue::from_str(country_code)?,
        );
        Ok(())
    }

    pub async fn search(&self, query: &str) -> anyhow::Result<BraveSearchResponse> {
        let url = Url::parse_with_params(
            &format!("{}/web/search", Self::API_BASE_URL),
            &[("q", query)],
        )?;

        let response = self
            .client
            .get(url)
            .headers(self.headers.clone())
            .send()
            .await?;

        let results: BraveSearchResponse = response.json().await?;
        Ok(results)
    }
}
