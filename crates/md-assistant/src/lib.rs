//! AI provider protocol for the assistant panel. No dependency on Tauri or Moodle.
//! Keys are passed per request and never stored, logged or echoed in errors; provider
//! response bodies never reach an error message.
pub mod tools;
use serde_json::{json, Value};
use url::Url;

/// Longest assistant answer kept from one response.
pub const MAX_REPLY: usize = 32_000;
/// Tool calls accepted from one response.
pub const MAX_CALLS: usize = 6;
const MAX_ARGUMENTS: usize = 2_000;
const MAX_LINE: usize = 256 * 1024;
const MAX_MODELS: usize = 200;
const IDLE: std::time::Duration = std::time::Duration::from_secs(60);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Provider {
    Albert,
    Mistral,
}
impl Provider {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "albert" => Some(Self::Albert),
            "mistral" => Some(Self::Mistral),
            _ => None,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Albert => "albert",
            Self::Mistral => "mistral",
        }
    }
    /// Fixed HTTPS hosts; a user-supplied endpoint is never accepted.
    fn base(self) -> &'static str {
        match self {
            Self::Albert => "https://albert.api.etalab.gouv.fr/v1/",
            Self::Mistral => "https://api.mistral.ai/v1/",
        }
    }
    /// Used until the user picks one of the models the provider lists.
    pub fn default_model(self) -> &'static str {
        match self {
            Self::Albert => "openweight-medium",
            Self::Mistral => "mistral-small-latest",
        }
    }
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("The AI provider rejected this API key. Check the key in Settings.")]
    Key,
    #[error("This API key is not allowed to use the selected model or service.")]
    Access,
    #[error("The AI provider is limiting requests or your quota is used up. Try again later.")]
    Quota,
    #[error("The selected AI model is not available. Choose another model in Settings.")]
    Model,
    #[error("The AI provider refused this request. Try another model in Settings.")]
    Request,
    #[error("The AI provider could not be reached. Check your connection and try again.")]
    Network,
    #[error("The AI provider returned an unexpected response.")]
    Protocol,
}

/// Model identifiers are sent in JSON only, but keep them to a plain token anyway.
pub fn valid_model(model: &str) -> bool {
    !model.is_empty()
        && model.len() <= 100
        && model
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'/' | b':'))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    /// JSON text as produced by the model: untrusted until `tools::parse` accepts it.
    pub arguments: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Message {
    System(String),
    User(String),
    Assistant {
        content: String,
        calls: Vec<ToolCall>,
    },
    Tool {
        id: String,
        name: String,
        content: String,
    },
}
impl Message {
    fn json(&self) -> Value {
        match self {
            Self::System(content) => json!({"role": "system", "content": content}),
            Self::User(content) => json!({"role": "user", "content": content}),
            Self::Assistant { content, calls } if calls.is_empty() => {
                json!({"role": "assistant", "content": content})
            }
            Self::Assistant { content, calls } => json!({
                "role": "assistant",
                "content": content,
                "tool_calls": calls.iter().map(|call| json!({
                    "id": call.id,
                    "type": "function",
                    "function": {"name": call.name, "arguments": call.arguments},
                })).collect::<Vec<_>>(),
            }),
            Self::Tool { id, name, content } => {
                json!({"role": "tool", "tool_call_id": id, "name": name, "content": content})
            }
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Reply {
    pub content: String,
    pub calls: Vec<ToolCall>,
}

pub struct Client {
    base: Url,
    http: reqwest::Client,
}
impl Client {
    pub fn new(provider: Provider) -> Result<Self, Error> {
        let base = Url::parse(provider.base()).map_err(|_| Error::Network)?;
        let http = reqwest::Client::builder()
            .https_only(true)
            .connect_timeout(std::time::Duration::from_secs(15))
            // A key must never follow a redirect to another origin.
            .redirect(reqwest::redirect::Policy::none())
            .user_agent(concat!("MoodBuro/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|_| Error::Network)?;
        Ok(Self { base, http })
    }
    fn url(&self, path: &str) -> Result<Url, Error> {
        self.base.join(path).map_err(|_| Error::Network)
    }
    /// Validates the key and lists chat-capable model identifiers, sorted.
    pub async fn models(&self, key: &str) -> Result<Vec<String>, Error> {
        let request = self.http.get(self.url("models")?).bearer_auth(key).send();
        let response = tokio::time::timeout(IDLE, request)
            .await
            .map_err(|_| Error::Network)?
            .map_err(|_| Error::Network)?;
        check(response.status())?;
        let body: Value = tokio::time::timeout(IDLE, response.json())
            .await
            .map_err(|_| Error::Network)?
            .map_err(|_| Error::Protocol)?;
        Ok(model_ids(&body))
    }
    /// One streamed completion. `on_text` receives each text fragment as it arrives.
    pub async fn chat(
        &self,
        key: &str,
        model: &str,
        messages: &[Message],
        tools: Option<&Value>,
        mut on_text: impl FnMut(&str),
    ) -> Result<Reply, Error> {
        if !valid_model(model) {
            return Err(Error::Model);
        }
        // Only fields both providers document: unknown fields can be rejected.
        let mut body = json!({
            "model": model,
            "stream": true,
            "messages": messages.iter().map(Message::json).collect::<Vec<_>>(),
        });
        if let Some(tools) = tools {
            body["tools"] = tools.clone();
            body["tool_choice"] = json!("auto");
        }
        let request = self
            .http
            .post(self.url("chat/completions")?)
            .bearer_auth(key)
            .json(&body)
            .send();
        let mut response = tokio::time::timeout(IDLE, request)
            .await
            .map_err(|_| Error::Network)?
            .map_err(|_| Error::Network)?;
        check(response.status())?;
        let mut stream = Stream::default();
        let mut buffer: Vec<u8> = Vec::new();
        loop {
            let chunk = tokio::time::timeout(IDLE, response.chunk())
                .await
                .map_err(|_| Error::Network)?
                .map_err(|_| Error::Network)?;
            let Some(chunk) = chunk else { break };
            buffer.extend_from_slice(&chunk);
            while let Some(end) = buffer.iter().position(|b| *b == b'\n') {
                let line: Vec<u8> = buffer.drain(..=end).collect();
                // Lines are split on bytes, so a character cut by a chunk is whole again here.
                let line = String::from_utf8_lossy(&line);
                if stream.line(line.trim(), &mut on_text)? {
                    return stream.finish();
                }
            }
            if buffer.len() > MAX_LINE {
                return Err(Error::Protocol);
            }
        }
        let rest = String::from_utf8_lossy(&buffer).into_owned();
        stream.line(rest.trim(), &mut on_text)?;
        stream.finish()
    }
}

fn check(status: reqwest::StatusCode) -> Result<(), Error> {
    match status.as_u16() {
        200..=299 => Ok(()),
        401 => Err(Error::Key),
        403 => Err(Error::Access),
        404 => Err(Error::Model),
        429 => Err(Error::Quota),
        400 | 413 | 422 => Err(Error::Request),
        300..=399 => Err(Error::Protocol),
        _ => Err(Error::Network),
    }
}

/// Chat-capable identifiers from either provider's `/models` shape. When no entry
/// carries a recognised capability marker, every identifier is offered.
fn model_ids(body: &Value) -> Vec<String> {
    let items = body
        .get("data")
        .and_then(Value::as_array)
        .or_else(|| body.as_array())
        .map(Vec::as_slice)
        .unwrap_or_default();
    let id = |item: &Value| {
        item.get("id")
            .and_then(Value::as_str)
            .filter(|id| valid_model(id))
            .map(str::to_owned)
    };
    let chat = |item: &Value| {
        // Mistral uses type=base/fine-tuned and a separate capability object.
        if let Some(chat) = item
            .get("capabilities")
            .and_then(|c| c.get("completion_chat"))
            .and_then(Value::as_bool)
        {
            return Some(chat);
        }
        match item.get("type").and_then(Value::as_str) {
            Some("base" | "fine-tuned") | None => None,
            Some(kind) => Some(matches!(kind, "text-generation" | "image-text-to-text")),
        }
    };
    let mut marked: Vec<String> = items
        .iter()
        .filter(|item| chat(item) == Some(true))
        .filter_map(id)
        .collect();
    if !items.iter().any(|item| chat(item).is_some()) {
        marked = items.iter().filter_map(id).collect();
    }
    marked.sort();
    marked.dedup();
    marked.truncate(MAX_MODELS);
    marked
}

#[derive(Default)]
struct Stream {
    content: String,
    /// Keyed by the provider's index; arguments may arrive whole or in fragments.
    calls: Vec<(u64, ToolCall)>,
}
impl Stream {
    /// Returns true when the provider marked the end of the stream.
    fn line(&mut self, line: &str, on_text: &mut impl FnMut(&str)) -> Result<bool, Error> {
        let Some(data) = line.strip_prefix("data:") else {
            return Ok(false);
        };
        let data = data.trim();
        if data == "[DONE]" {
            return Ok(true);
        }
        let event: Value = serde_json::from_str(data).map_err(|_| Error::Protocol)?;
        if event.get("error").is_some_and(|e| !e.is_null()) {
            return Err(Error::Protocol);
        }
        let Some(delta) = event
            .get("choices")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("delta"))
        else {
            // Usage-only and keep-alive events carry no choice.
            return Ok(false);
        };
        let mut text = String::new();
        match delta.get("content") {
            Some(Value::String(part)) => text.push_str(part),
            // Some models send typed parts; only plain text is shown.
            Some(Value::Array(parts)) => {
                for part in parts {
                    if part.get("type").and_then(Value::as_str) == Some("text") {
                        text.push_str(part.get("text").and_then(Value::as_str).unwrap_or(""));
                    }
                }
            }
            _ => {}
        }
        if !text.is_empty() {
            if self.content.len() + text.len() > MAX_REPLY {
                return Err(Error::Protocol);
            }
            self.content.push_str(&text);
            on_text(&text);
        }
        for (position, call) in delta
            .get("tool_calls")
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or_default()
            .iter()
            .enumerate()
        {
            let index = call
                .get("index")
                .and_then(Value::as_u64)
                .unwrap_or(position as u64);
            let slot = match self.calls.iter().position(|(i, _)| *i == index) {
                Some(slot) => slot,
                None => {
                    if self.calls.len() >= MAX_CALLS {
                        return Err(Error::Protocol);
                    }
                    self.calls.push((
                        index,
                        ToolCall {
                            id: String::new(),
                            name: String::new(),
                            arguments: String::new(),
                        },
                    ));
                    self.calls.len() - 1
                }
            };
            let entry = &mut self.calls[slot].1;
            if let Some(id) = call.get("id").and_then(Value::as_str) {
                if entry.id.is_empty() {
                    entry.id = id.chars().take(64).collect();
                }
            }
            let function = call.get("function");
            if let Some(name) = function.and_then(|f| f.get("name")).and_then(Value::as_str) {
                if entry.name.is_empty() {
                    entry.name = name.chars().take(64).collect();
                }
            }
            match function.and_then(|f| f.get("arguments")) {
                Some(Value::String(part)) => entry.arguments.push_str(part),
                Some(Value::Object(map)) => {
                    entry.arguments = Value::Object(map.clone()).to_string();
                }
                _ => {}
            }
            if entry.arguments.len() > MAX_ARGUMENTS {
                return Err(Error::Protocol);
            }
        }
        Ok(false)
    }
    fn finish(mut self) -> Result<Reply, Error> {
        self.calls.sort_by_key(|(index, _)| *index);
        let calls: Vec<ToolCall> = self.calls.into_iter().map(|(_, call)| call).collect();
        if calls.iter().any(|c| c.id.is_empty() || c.name.is_empty()) {
            return Err(Error::Protocol);
        }
        if self.content.trim().is_empty() && calls.is_empty() {
            return Err(Error::Protocol);
        }
        Ok(Reply {
            content: self.content,
            calls,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{
        matchers::{header, method, path},
        Mock, MockServer, ResponseTemplate,
    };
    const KEY: &str = "sk-test-secret-key";
    async fn client() -> (MockServer, Client) {
        let server = MockServer::start().await;
        // HTTP is confined to this test-only constructor; production always uses HTTPS.
        let client = Client {
            base: Url::parse(&format!("{}/v1/", server.uri())).unwrap(),
            http: reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .unwrap(),
        };
        (server, client)
    }
    fn sse(events: &[&str]) -> ResponseTemplate {
        let body: String = events.iter().map(|e| format!("data: {e}\n\n")).collect();
        ResponseTemplate::new(200).set_body_raw(body, "text/event-stream")
    }
    async fn mount_chat(server: &MockServer, response: ResponseTemplate) {
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .and(header("authorization", format!("Bearer {KEY}").as_str()))
            .respond_with(response)
            .mount(server)
            .await;
    }

    #[test]
    fn providers_use_fixed_https_hosts() {
        for provider in [Provider::Albert, Provider::Mistral] {
            let client = Client::new(provider).unwrap();
            assert_eq!(client.base.scheme(), "https");
            assert_eq!(Provider::parse(provider.as_str()), Some(provider));
            assert!(valid_model(provider.default_model()));
            assert!(client
                .url("chat/completions")
                .unwrap()
                .path()
                .ends_with("/v1/chat/completions"));
        }
        assert_eq!(Provider::parse("https://evil.example"), None);
        assert!(!valid_model("a b"));
        assert!(!valid_model(""));
        assert!(!valid_model(&"m".repeat(101)));
    }

    #[tokio::test]
    async fn streams_text_and_sends_only_the_common_request_fields() {
        let (server, client) = client().await;
        mount_chat(
            &server,
            sse(&[
                r#"{"choices":[{"delta":{"role":"assistant","content":"Bon"}}]}"#,
                r#"{"choices":[{"delta":{"content":"jour é"}}]}"#,
                r#"{"choices":[{"delta":{},"finish_reason":"stop"}],"usage":{"total_tokens":3}}"#,
                "[DONE]",
            ]),
        )
        .await;
        let mut seen = Vec::new();
        let reply = client
            .chat(
                KEY,
                "test-model",
                &[Message::System("s".into()), Message::User("hi".into())],
                None,
                |part| seen.push(part.to_owned()),
            )
            .await
            .unwrap();
        assert_eq!(reply.content, "Bonjour é");
        assert!(reply.calls.is_empty());
        assert_eq!(seen, ["Bon", "jour é"]);
        let request = &server.received_requests().await.unwrap()[0];
        let body: Value = serde_json::from_slice(&request.body).unwrap();
        let mut keys: Vec<_> = body.as_object().unwrap().keys().cloned().collect();
        keys.sort();
        assert_eq!(keys, ["messages", "model", "stream"]);
        assert!(!String::from_utf8_lossy(&request.body).contains(KEY));
    }

    #[tokio::test]
    async fn accumulates_fragmented_and_whole_tool_calls() {
        let (server, client) = client().await;
        mount_chat(
            &server,
            sse(&[
                r#"{"choices":[{"delta":{"tool_calls":[{"index":0,"id":"a1","function":{"name":"find_activities","arguments":"{\"que"}}]}}]}"#,
                r#"{"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"ry\":\"quiz\"}"}}]}}]}"#,
                r#"{"choices":[{"delta":{"tool_calls":[{"index":1,"id":"b2","function":{"name":"list_courses","arguments":"{}"}}]}}]}"#,
            ]),
        )
        .await;
        let tools = tools::definitions();
        let reply = client
            .chat(
                KEY,
                "test-model",
                &[Message::User("x".into())],
                Some(&tools),
                |_| {},
            )
            .await
            .unwrap();
        assert_eq!(reply.calls.len(), 2);
        assert_eq!(reply.calls[0].arguments, r#"{"query":"quiz"}"#);
        assert_eq!(reply.calls[1].name, "list_courses");
        let request = &server.received_requests().await.unwrap()[0];
        let body: Value = serde_json::from_slice(&request.body).unwrap();
        assert_eq!(body["tool_choice"], "auto");
        assert_eq!(body["tools"].as_array().unwrap().len(), 6);
    }

    #[tokio::test]
    async fn tool_results_are_replayed_in_provider_format() {
        let (server, client) = client().await;
        mount_chat(
            &server,
            sse(&[r#"{"choices":[{"delta":{"content":"ok"}}]}"#, "[DONE]"]),
        )
        .await;
        let call = ToolCall {
            id: "a1".into(),
            name: "list_courses".into(),
            arguments: "{}".into(),
        };
        client
            .chat(
                KEY,
                "test-model",
                &[
                    Message::User("x".into()),
                    Message::Assistant {
                        content: String::new(),
                        calls: vec![call],
                    },
                    Message::Tool {
                        id: "a1".into(),
                        name: "list_courses".into(),
                        content: "{}".into(),
                    },
                ],
                None,
                |_| {},
            )
            .await
            .unwrap();
        let request = &server.received_requests().await.unwrap()[0];
        let body: Value = serde_json::from_slice(&request.body).unwrap();
        assert_eq!(body["messages"][1]["tool_calls"][0]["type"], "function");
        assert_eq!(body["messages"][2]["tool_call_id"], "a1");
        assert_eq!(body["messages"][2]["role"], "tool");
    }

    #[tokio::test]
    async fn provider_failures_map_to_fixed_messages_without_bodies_or_keys() {
        for (status, expected) in [
            (401, Error::Key),
            (403, Error::Access),
            (404, Error::Model),
            (422, Error::Request),
            (429, Error::Quota),
            (503, Error::Network),
            (302, Error::Protocol),
        ] {
            let (server, client) = client().await;
            mount_chat(
                &server,
                ResponseTemplate::new(status)
                    .set_body_string(format!("leaked {KEY} detail"))
                    .insert_header("location", "https://elsewhere.example/"),
            )
            .await;
            let error = client
                .chat(
                    KEY,
                    "test-model",
                    &[Message::User("x".into())],
                    None,
                    |_| {},
                )
                .await
                .unwrap_err();
            assert_eq!(error, expected);
            assert!(!error.to_string().contains(KEY));
            assert!(!error.to_string().contains("leaked"));
            assert!(!format!("{error:?}").contains(KEY));
        }
    }

    #[tokio::test]
    async fn malformed_empty_or_oversized_streams_are_rejected() {
        let long = format!(
            r#"{{"choices":[{{"delta":{{"content":"{}"}}}}]}}"#,
            "a".repeat(MAX_REPLY + 1)
        );
        let many: String = (0..=MAX_CALLS)
            .map(|i| format!(r#"{{"index":{i},"id":"c{i}","function":{{"name":"list_courses","arguments":"{{}}"}}}}"#))
            .collect::<Vec<_>>()
            .join(",");
        let many = format!(r#"{{"choices":[{{"delta":{{"tool_calls":[{many}]}}}}]}}"#);
        for events in [
            vec!["not json"],
            vec!["[DONE]"],
            vec![r#"{"error":{"message":"boom"}}"#],
            vec![
                r#"{"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"{}"}}]}}]}"#,
            ],
            vec![long.as_str()],
            vec![many.as_str()],
        ] {
            let (server, client) = client().await;
            mount_chat(&server, sse(&events)).await;
            let error = client
                .chat(
                    KEY,
                    "test-model",
                    &[Message::User("x".into())],
                    None,
                    |_| {},
                )
                .await
                .unwrap_err();
            assert_eq!(error, Error::Protocol);
        }
    }

    #[tokio::test]
    async fn an_invalid_model_is_refused_before_any_request() {
        let (server, client) = client().await;
        let error = client
            .chat(KEY, "bad model", &[Message::User("x".into())], None, |_| {})
            .await
            .unwrap_err();
        assert_eq!(error, Error::Model);
        assert!(server.received_requests().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn lists_chat_models_from_both_provider_shapes() {
        let (server, client) = client().await;
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .and(header("authorization", format!("Bearer {KEY}").as_str()))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": [
                {"id": "openweight-medium", "type": "text-generation"},
                {"id": "embeddings-small", "type": "text-embeddings-inference"},
                {"id": "mistral-small-latest", "type": "base", "capabilities": {"completion_chat": true}},
                {"id": "mistral-embed", "type": "base", "capabilities": {"completion_chat": false}},
                {"id": "bad id"},
            ]})))
            .mount(&server)
            .await;
        assert_eq!(
            client.models(KEY).await.unwrap(),
            ["mistral-small-latest", "openweight-medium"]
        );
        // No capability markers at all: offer every identifier rather than none.
        assert_eq!(
            model_ids(&json!({"data": [{"id": "b"}, {"id": "a"}]})),
            ["a", "b"]
        );
        assert!(model_ids(&json!({"unexpected": true})).is_empty());
        assert!(model_ids(&json!({"data": [
            {"id": "embed-only", "type": "base", "capabilities": {"completion_chat": false}}
        ]}))
        .is_empty());
    }

    #[tokio::test]
    async fn a_rejected_key_is_reported_when_listing_models() {
        let (server, client) = client().await;
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&server)
            .await;
        assert_eq!(client.models("wrong").await.unwrap_err(), Error::Key);
    }
}
