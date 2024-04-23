use std::{collections::HashMap, pin::Pin, str::FromStr};

use anyhow::{anyhow, bail};
use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionMessageToolCall, ChatCompletionRequestUserMessageArgs, ChatCompletionTool,
        ChatCompletionToolArgs, ChatCompletionToolType, CreateChatCompletionRequestArgs,
        FunctionCall, FunctionObjectArgs,
    },
    Client,
};
use chrono::NaiveDate;
use futures::Future;
use rust_decimal::Decimal;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{aidevs, config::Config, utils};

const MODEL: &str = "gpt-3.5-turbo";

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct KnowledgeTaskResponse {
    code: i32,
    msg: String,
    #[serde(rename = "database #1")]
    database1: String,
    #[serde(rename = "database #2")]
    database2: String,
    question: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct CurrencyRate {
    #[serde(rename = "effectiveDate")]
    date: NaiveDate,
    mid: Decimal,
    no: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct CurrencyApiResponse {
    code: String,
    currency: String,
    rates: Vec<CurrencyRate>,
    table: String,
}

type KnowledgeFunction = Box<dyn Fn(Value) -> Pin<Box<dyn Future<Output = anyhow::Result<Value>>>>>;
struct KnowledgeChatTools {
    functions: HashMap<String, KnowledgeFunction>,
}

/// The task involved providing an answer to the received question.
/// In the case of questions about a country's population or currency exchange rates,
/// it was necessary to use the appropriate databases, and in all other cases, the base knowledge of the LLM model.
///
/// * `config`: App configuration
/// * `token`: Task token
pub(super) async fn run(config: &Config, token: &str) -> anyhow::Result<Value> {
    let task_response = aidevs::get_task::<KnowledgeTaskResponse>(config, token).await?;
    log::debug!("Task API response: {task_response:#?}");
    log::info!("Task message: {}", task_response.msg);

    if task_response.code != 0 {
        bail!("Code in response is not equal 0")
    }
    log::info!("Task question: {}", task_response.question);

    let openai_config = OpenAIConfig::default();
    let openai_client = Client::with_config(openai_config);
    let knowledge_chat_tools = KnowledgeChatTools::new();

    let request = CreateChatCompletionRequestArgs::default()
        .model(MODEL)
        .messages([ChatCompletionRequestUserMessageArgs::default()
            .content(task_response.question.as_str())
            .build()?
            .into()])
        .tools(KnowledgeChatTools::chat_tools()?)
        .build()?;

    let tool_call = openai_client
        .chat()
        .create(request)
        .await?
        .choices
        .into_iter()
        .find_map(|c| c.message.tool_calls)
        .ok_or(anyhow!("{MODEL} response do not contain tool calls."))?
        .into_iter()
        .next()
        .ok_or(anyhow!("Tool calls empty"))?;

    let answer = knowledge_chat_tools.handle(tool_call).await?;
    log::debug!("Answer: {answer:?}");
    Ok(answer)
}

impl KnowledgeChatTools {
    fn new() -> Self {
        let mut functions: HashMap<String, KnowledgeFunction> = HashMap::new();
        functions.insert(
            "get_population_api_call".to_string(),
            Box::new(|s| Box::pin(Self::get_population_api_call(s))),
        );
        functions.insert(
            "get_currency_rate_api_call".to_string(),
            Box::new(|s| Box::pin(Self::get_currency_rate_api_call(s))),
        );
        functions.insert(
            "ask_llm".to_string(),
            Box::new(|s| Box::pin(Self::ask_llm(s))),
        );

        Self { functions }
    }

    async fn get_population_api_call(args: Value) -> anyhow::Result<Value> {
        let country = args
            .get("country")
            .and_then(|c| c.as_str())
            .ok_or(anyhow!("No field 'country' in args"))?;
        let url = format!("https://restcountries.com/v3.1/name/{country}");

        let population = reqwest::get(url)
            .await?
            .json::<Vec<Value>>()
            .await?
            .first()
            .and_then(|r| r.get("population"))
            .and_then(|p| p.as_u64())
            .ok_or(anyhow!("Can not get {country} population from API"))?;

        Ok(json!({"answer": population}))
    }

    async fn get_currency_rate_api_call(args: Value) -> anyhow::Result<Value> {
        let currency_code = args
            .get("currency_code")
            .and_then(|c| c.as_str())
            .ok_or(anyhow!("No field 'currency_code' in args"))?;
        let url = format!("https://api.nbp.pl/api/exchangerates/rates/A/{currency_code}");
        let response = reqwest::get(url)
            .await?
            .json::<CurrencyApiResponse>()
            .await?;
        let rate = response.rates.first().ok_or(anyhow!(
            "Currency rate to PLN for {currency_code} not found."
        ))?;

        Ok(json!({"answer": rate.mid}))
    }

    async fn ask_llm(args: Value) -> anyhow::Result<Value> {
        let question = args
            .get("question")
            .and_then(|c| c.as_str())
            .ok_or(anyhow!("No field 'question' in args"))?;
        let openai_config = OpenAIConfig::default();
        let openai_client = Client::with_config(openai_config);
        let answer = utils::ask_llm(&openai_client, MODEL, question, None).await?;
        Ok(json!({"answer": answer}))
    }

    fn chat_tools() -> anyhow::Result<Vec<ChatCompletionTool>> {
        let get_population_function = FunctionObjectArgs::default()
            .name("get_population_api_call")
            .description("Get country population")
            .parameters(json!({
                "type": "object",
                "properties": {
                    "country": {
                        "type": "string",
                        "description": "Country name (in english)"
                    },
                }
            }))
            .build()?;

        let get_currency_rate_function = FunctionObjectArgs::default()
            .name("get_currency_rate_api_call")
            .description("Get currency rate to Polish Złoty (PLN)")
            .parameters(json!({
                "type": "object",
                "properties": {
                    "currency_code": {
                        "type": "string",
                        "description": "Currency code"
                    },
                }
            }))
            .build()?;

        let ask_llm_function = FunctionObjectArgs::default()
            .name("ask_llm")
            .description("Ask LLM base knowledge")
            .parameters(json!({
                "type": "object",
                "properties": {
                    "question": {
                        "type": "string",
                        "description": "Question"
                    },
                }
            }))
            .build()?;

        let tools = vec![
            ChatCompletionToolArgs::default()
                .r#type(ChatCompletionToolType::Function)
                .function(get_population_function)
                .build()?,
            ChatCompletionToolArgs::default()
                .r#type(ChatCompletionToolType::Function)
                .function(get_currency_rate_function)
                .build()?,
            ChatCompletionToolArgs::default()
                .r#type(ChatCompletionToolType::Function)
                .function(ask_llm_function)
                .build()?,
        ];

        Ok(tools)
    }

    async fn handle(&self, tool_call: ChatCompletionMessageToolCall) -> anyhow::Result<Value> {
        match tool_call.r#type {
            ChatCompletionToolType::Function => self.handle_function_call(tool_call.function).await,
        }
    }

    async fn handle_function_call(&self, function_call: FunctionCall) -> anyhow::Result<Value> {
        let args = Value::from_str(&function_call.arguments)?;
        let function = self
            .functions
            .get(&function_call.name)
            .ok_or(anyhow!("Function '{}' does not exists", function_call.name))?;

        log::debug!("Calling '{}' with args: {args:?}", function_call.name);

        function(args).await
    }
}
