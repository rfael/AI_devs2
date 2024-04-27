use std::{sync::Arc, time::Duration};

use anyhow::{anyhow, bail};
use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionMessageToolCall, ChatCompletionRequestSystemMessageArgs,
        ChatCompletionRequestUserMessageArgs, ChatCompletionTool, ChatCompletionToolArgs,
        ChatCompletionToolType, CreateChatCompletionRequestArgs, FunctionCall, FunctionObjectArgs,
    },
    Client,
};
use chrono::Local;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tide::StatusCode;
use tokio::{sync::Mutex, time::sleep};

use crate::{aidevs, config::Config, utils};

const MODEL: &str = "gpt-3.5-turbo";

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct OwnapiProTaskResponse {
    code: i32,
    hint1: String,
    hint2: String,
    hint3: String,
    msg: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct OwnapiProRequest {
    question: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct OwnapiProResponse {
    reply: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct OwnapiProRemeberFuncArgs {
    data: String,
    category: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct OwnapiProAnswerFuncArgs {
    question: String,
}

struct OwnapiProContext {
    openai_client: Client<OpenAIConfig>,
    llm_context: String,
    chat_tools: Vec<ChatCompletionTool>,
}

impl OwnapiProContext {
    fn new() -> anyhow::Result<Self> {
        let openai_config = OpenAIConfig::default();
        let openai_client = Client::with_config(openai_config);

        let today = Local::now();
        let llm_context = [
            "Answer concisely as possible",
            "If you do not know answer for the question say 'I do not know'",
            &format!("Today is: {today}"),
        ]
        .join("\n");

        let chat_tools = Self::chat_tools()?;

        Ok(Self {
            openai_client,
            llm_context,
            chat_tools,
        })
    }

    async fn tool_fn_answer(
        &self,
        args: OwnapiProAnswerFuncArgs,
    ) -> anyhow::Result<OwnapiProResponse> {
        let reply = utils::ask_llm(
            &self.openai_client,
            MODEL,
            &args.question,
            Some(&self.llm_context),
        )
        .await?;

        Ok(OwnapiProResponse { reply })
    }

    fn tool_fn_remember(
        &mut self,
        args: OwnapiProRemeberFuncArgs,
    ) -> anyhow::Result<OwnapiProResponse> {
        log::debug!("Data to remember: {args:?}");
        self.llm_context.push_str(&format!(
            "\n Fact about me: {} {}",
            args.category, args.data
        ));

        Ok(OwnapiProResponse { reply: "Ok".into() })
    }

    fn chat_tools() -> anyhow::Result<Vec<ChatCompletionTool>> {
        let remember_function = FunctionObjectArgs::default()
            .name("remember")
            .description("Remember provided data")
            .parameters(json!({
                "type": "object",
                "properties": {
                    "data": {
                        "type": "string",
                        "description": "Data to remember and"
                    },
                    "category": {
                        "type": "string",
                        "description": "Data category"
                    },
                }
            }))
            .build()?;

        let answer_function = FunctionObjectArgs::default()
            .name("answer")
            .description("Answer on the question")
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
                .function(remember_function)
                .build()?,
            ChatCompletionToolArgs::default()
                .r#type(ChatCompletionToolType::Function)
                .function(answer_function)
                .build()?,
        ];

        Ok(tools)
    }

    async fn handle(
        &mut self,
        tool_call: ChatCompletionMessageToolCall,
    ) -> anyhow::Result<OwnapiProResponse> {
        match tool_call.r#type {
            ChatCompletionToolType::Function => self.handle_function_call(tool_call.function).await,
        }
    }

    async fn handle_function_call(
        &mut self,
        function_call: FunctionCall,
    ) -> anyhow::Result<OwnapiProResponse> {
        log::debug!("Calling '{}' function", function_call.name);

        match function_call.name.as_str() {
            "answer" => {
                let args: OwnapiProAnswerFuncArgs = serde_json::from_str(&function_call.arguments)?;
                self.tool_fn_answer(args).await
            }
            "remember" => {
                let args: OwnapiProRemeberFuncArgs =
                    serde_json::from_str(&function_call.arguments)?;
                self.tool_fn_remember(args)
            }

            other => bail!("Unexptected function name: {other}"),
        }
    }
}

/// This task is an expanded version of the 'ownapi' task.
/// An additional requirement was to recognize whether the received request contains data to remember or a question.
///
/// * `config`: App configuration
/// * `token`: Task token
pub(super) async fn run(config: &Config, token: &str) -> anyhow::Result<()> {
    let api_listen_addr = config
        .api_listen_address
        .as_ref()
        .ok_or(anyhow!("API listen address not specified"))?
        .clone();
    let mut api_tunnel_url = config
        .api_tunnel_url
        .as_ref()
        .ok_or(anyhow!("Ngrok tunnel URL not found in configuration"))?
        .clone();
    api_tunnel_url.set_path("ownapipro");
    log::info!("API tunneled endpoint: {api_tunnel_url}");

    let task_response = aidevs::get_task::<OwnapiProTaskResponse>(config, token).await?;
    log::debug!("Task API response: {task_response:#?}");
    log::info!("Task message: {}", task_response.msg);

    if task_response.code != 0 {
        bail!("Code in response is not equal 0")
    }

    let api_state = OwnapiProContext::new()?;
    let api_state = Arc::new(Mutex::new(api_state));
    let mut app = tide::with_state(api_state);
    app.at("/ownapipro").post(ownapipro_request_handler);

    let api_future = tokio::spawn(app.listen(api_listen_addr));
    sleep(Duration::from_secs(1)).await;

    let payload = json!({ "answer" : api_tunnel_url});
    let answer_response = aidevs::post_answer(config, token, &payload).await?;
    if answer_response.code != 0 {
        bail!(
            "Post answer failed: [{}] {}",
            answer_response.code,
            answer_response.msg
        )
    }

    api_future.await??;

    Ok(())
}

async fn ownapipro_request_handler(
    mut request: tide::Request<Arc<Mutex<OwnapiProContext>>>,
) -> tide::Result {
    let OwnapiProRequest { question } = request.body_json().await?;
    log::debug!("Received question: {question}");

    let mut context = request.state().lock().await;

    let request = CreateChatCompletionRequestArgs::default()
        .model(MODEL)
        .messages([
            ChatCompletionRequestSystemMessageArgs::default()
                .content("Decide if provided iput is data to remember or a question")
                .build()?
                .into(),
            ChatCompletionRequestUserMessageArgs::default()
                .content(question)
                .build()?
                .into(),
        ])
        .tools(context.chat_tools.clone())
        .build()?;

    let tool_call = context
        .openai_client
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

    let reply = context.handle(tool_call).await?;
    log::debug!("Reply: {reply:?}");
    let response_body = tide::Body::from_json(&reply)?;
    let mut response = tide::Response::new(StatusCode::Ok);
    response.set_body(response_body);

    Ok(response)
}
