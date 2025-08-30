use anyhow::Result;
use serde_json::json;
use std::collections::HashMap;

/// Factory for instantiating a concrete backend based on the model ID.
pub enum BackendKind {
    Gemini,
    OpenAi,
    Anthropic,
}

impl BackendKind {
    pub fn from_model(model: &str) -> Self {
        let m = model.to_lowercase();
        if m.starts_with("gpt-") || m.starts_with("o3") || m.starts_with("o1") {
            BackendKind::OpenAi
        } else if m.starts_with("claude-") {
            BackendKind::Anthropic
        } else {
            // Default to Gemini
            BackendKind::Gemini
        }
    }
}

/// A single enum type to avoid dyn/async issues while remaining backend-agnostic.
pub enum AnyClient {
    Gemini(GeminiBackend),
    OpenAi(OpenAiBackend),
    Anthropic(AnthropicBackend),
}

impl AnyClient {
    pub fn model_id(&self) -> &str {
        match self {
            AnyClient::Gemini(c) => &c.model,
            AnyClient::OpenAi(c) => &c.model,
            AnyClient::Anthropic(c) => &c.model,
        }
    }

    pub async fn generate_content(&mut self, req: &crate::gemini::GenerateContentRequest) -> Result<crate::gemini::GenerateContentResponse> {
        match self {
            AnyClient::Gemini(c) => c.generate_content(req).await,
            AnyClient::OpenAi(c) => c.generate_content(req).await,
            AnyClient::Anthropic(c) => c.generate_content(req).await,
        }
    }

    pub async fn generate_content_stream<F>(&mut self, req: &crate::gemini::GenerateContentRequest, on_chunk: F) -> Result<crate::gemini::GenerateContentResponse>
    where
        F: FnMut(&str) -> Result<()>,
    {
        match self {
            AnyClient::Gemini(c) => c.generate_content_stream(req, on_chunk).await,
            AnyClient::OpenAi(c) => c.generate_content_stream(req, on_chunk).await,
            AnyClient::Anthropic(c) => c.generate_content_stream(req, on_chunk).await,
        }
    }
}

/// Create a new client for the given API key and model.
pub fn make_client(api_key: String, model: String) -> AnyClient {
    match BackendKind::from_model(&model) {
        BackendKind::Gemini => AnyClient::Gemini(GeminiBackend::new(api_key, model)),
        BackendKind::OpenAi => AnyClient::OpenAi(OpenAiBackend::new(api_key, model)),
        BackendKind::Anthropic => AnyClient::Anthropic(AnthropicBackend::new(api_key, model)),
    }
}

/// Gemini adapter implementing the model-agnostic trait.
pub struct GeminiBackend {
    inner: crate::gemini::Client,
    model: String,
}

impl GeminiBackend {
    pub fn new(api_key: String, model: String) -> Self {
        let inner = crate::gemini::Client::new(api_key, model.clone());
        Self { inner, model }
    }
}

impl GeminiBackend {
    async fn generate_content(
        &mut self,
        req: &crate::gemini::GenerateContentRequest,
    ) -> Result<crate::gemini::GenerateContentResponse> {
        self.inner.generate_content(req).await
    }

    async fn generate_content_stream<F>(
        &mut self,
        req: &crate::gemini::GenerateContentRequest,
        on_chunk: F,
    ) -> Result<crate::gemini::GenerateContentResponse>
    where
        F: FnMut(&str) -> Result<()>,
    {
        self.inner.generate_content_stream(req, on_chunk).await
    }
}

/// OpenAI adapter (stub). Implement translation to/from request/response types later.
pub struct OpenAiBackend {
    api_key: String,
    model: String,
    http: reqwest::Client,
    last_tool_call_id_by_name: HashMap<String, String>,
}

impl OpenAiBackend {
    pub fn new(api_key: String, model: String) -> Self {
        let http = reqwest::Client::builder()
            .user_agent("vtagent/llm-openai")
            .build()
            .expect("http client");
        Self { api_key, model, http, last_tool_call_id_by_name: HashMap::new() }
    }
    // Extract parts from a non-streaming OpenAI choice message
    fn extract_parts_from_choice(choice: &serde_json::Value, id_map: &mut std::collections::HashMap<String, String>) -> Vec<crate::gemini::Part> {
        use crate::gemini::{Part, FunctionCall};
        let mut parts = Vec::new();
        if let Some(msg) = choice.get("message") {
            if let Some(text) = msg.get("content").and_then(|x| x.as_str()) {
                if !text.is_empty() { parts.push(Part::Text { text: text.to_string() }); }
            }
            if let Some(tool_calls) = msg.get("tool_calls").and_then(|x| x.as_array()) {
                for tc in tool_calls {
                    let id = tc.get("id").and_then(|x| x.as_str()).map(|s| s.to_string());
                    let func = tc.get("function").cloned().unwrap_or(serde_json::json!({}));
                    let name = func.get("name").and_then(|x| x.as_str()).unwrap_or("").to_string();
                    let args_str = func.get("arguments").and_then(|x| x.as_str()).unwrap_or("{}");
                    let args_val = serde_json::from_str(args_str).unwrap_or(serde_json::json!({}));
                    if let Some(idv) = &id { id_map.insert(name.clone(), idv.clone()); }
                    parts.push(Part::FunctionCall { function_call: FunctionCall { name, args: args_val, id } });
                }
            }
        }
        parts
    }
}

impl OpenAiBackend {
    async fn generate_content(
        &mut self,
        req: &crate::gemini::GenerateContentRequest,
    ) -> Result<crate::gemini::GenerateContentResponse> {
        use crate::gemini::{Content, Part};
        // Map Gemini-like request to OpenAI Chat Completions
        let url = "https://api.openai.com/v1/chat/completions";

        // Extract messages with tool support
        let mut messages: Vec<serde_json::Value> = Vec::new();
        if let Some(sys) = &req.system_instruction {
            let text = sys.parts.iter().filter_map(|p| p.as_text()).collect::<Vec<_>>().join("");
            if !text.is_empty() { messages.push(json!({"role":"system","content": text})); }
        }
        // Helper to push assistant tool_calls or text
        let mut gen_id_counter: usize = 0;
        for c in &req.contents {
            for part in &c.parts {
                match part {
                    crate::gemini::Part::Text { text } => {
                        let role = match c.role.as_str() { "user" => "user", "system" => "system", _ => "assistant" };
                        messages.push(json!({"role": role, "content": text}));
                    }
                    crate::gemini::Part::FunctionCall { function_call } => {
                        // OpenAI: assistant message with tool_calls
                        let id = function_call.id.clone().unwrap_or_else(|| { gen_id_counter += 1; format!("call_{}", gen_id_counter) });
                        self.last_tool_call_id_by_name.insert(function_call.name.clone(), id.clone());
                        messages.push(json!({
                            "role": "assistant",
                            "content": "",
                            "tool_calls": [
                                {"id": id, "type": "function", "function": {"name": function_call.name, "arguments": function_call.args.to_string()}}
                            ]
                        }));
                    }
                    crate::gemini::Part::FunctionResponse { function_response } => {
                        // OpenAI: tool role message with tool_call_id
                        let tool_call_id = self.last_tool_call_id_by_name.get(&function_response.name).cloned().unwrap_or_else(|| {
                            gen_id_counter += 1; format!("call_{}", gen_id_counter)
                        });
                        messages.push(json!({
                            "role": "tool",
                            "tool_call_id": tool_call_id,
                            "content": function_response.response.to_string()
                        }));
                    }
                }
            }
        }

        // Tools mapping
        let mut tools_json: Option<Vec<serde_json::Value>> = None;
        if let Some(tools) = &req.tools {
            let mut arr = Vec::new();
            for t in tools {
                for f in &t.function_declarations {
                    arr.push(json!({
                        "type": "function",
                        "function": {"name": f.name, "description": f.description, "parameters": f.parameters}
                    }));
                }
            }
            if !arr.is_empty() { tools_json = Some(arr); }
        }

        // Generation params (best-effort mapping)
        let mut temperature: Option<f32> = None;
        let mut max_tokens: Option<u32> = None;
        if let Some(cfg) = &req.generation_config {
            if let Some(t) = cfg.get("temperature").and_then(|v| v.as_f64()) { temperature = Some(t as f32); }
            if let Some(m) = cfg.get("maxOutputTokens").and_then(|v| v.as_u64()) { max_tokens = Some(m as u32); }
        }

        let mut body = json!({
            "model": self.model,
            "messages": messages,
        });
        if let Some(ts) = tools_json { body["tools"] = json!(ts); }
        if let Some(t) = temperature { body["temperature"] = serde_json::json!(t); }
        if let Some(m) = max_tokens { body["max_tokens"] = serde_json::json!(m); }

        let resp = self.http
            .post(url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("OpenAI request failed: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("OpenAI error ({}): {}", status, text));
        }

        let v: serde_json::Value = resp.json().await.map_err(|e| anyhow::anyhow!("OpenAI parse failed: {}", e))?;
        let choice = v.get("choices").and_then(|c| c.as_array()).and_then(|a| a.get(0)).cloned().unwrap_or(json!({}));
        let parts: Vec<Part> = Self::extract_parts_from_choice(&choice, &mut self.last_tool_call_id_by_name);
        Ok(crate::gemini::GenerateContentResponse { candidates: vec![crate::gemini::Candidate { content: Content { role: "model".to_string(), parts }, finish_reason: None }], prompt_feedback: None, usage_metadata: None })
    }

    async fn generate_content_stream<F>(
        &mut self,
        req: &crate::gemini::GenerateContentRequest,
        mut on_chunk: F,
    ) -> Result<crate::gemini::GenerateContentResponse>
    where
        F: FnMut(&str) -> Result<()>,
    {
        use tokio_stream::StreamExt;
        use crate::gemini::{Content, Part};

        // Build request like in generate_content, with stream=true
        let mut messages: Vec<serde_json::Value> = Vec::new();
        if let Some(sys) = &req.system_instruction {
            let text = sys.parts.iter().filter_map(|p| p.as_text()).collect::<Vec<_>>().join("");
            if !text.is_empty() { messages.push(serde_json::json!({"role":"system","content": text})); }
        }
        for c in &req.contents {
            let role = match c.role.as_str() { "user" => "user", "system" => "system", _ => "assistant" };
            let text = c.parts.iter().filter_map(|p| p.as_text()).collect::<Vec<_>>().join("");
            if !text.is_empty() { messages.push(serde_json::json!({"role": role, "content": text})); }
        }
        let mut body = serde_json::json!({"model": self.model, "messages": messages, "stream": true});
        if let Some(cfg) = &req.generation_config {
            if let Some(t) = cfg.get("temperature").and_then(|v| v.as_f64()) { body["temperature"] = serde_json::json!(t as f32); }
            if let Some(m) = cfg.get("maxOutputTokens").and_then(|v| v.as_u64()) { body["max_tokens"] = serde_json::json!(m as u32); }
        }
        if let Some(tools) = &req.tools {
            let mut arr = Vec::new();
            for t in tools { for f in &t.function_declarations { arr.push(serde_json::json!({"type":"function","function":{"name":f.name,"description":f.description,"parameters":f.parameters}})); } }
            if !arr.is_empty() { body["tools"] = serde_json::json!(arr); }
        }

        let resp = self.http.post("https://api.openai.com/v1/chat/completions")
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("OpenAI request failed: {}", e))?;

        let mut full_text = String::new();
        let mut stream = resp.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let bytes = match chunk { Ok(b) => b, Err(_) => break };
            let s = String::from_utf8_lossy(&bytes);
            for line in s.lines() {
                let line = line.trim_start();
                if !line.starts_with("data:") { continue; }
                let payload = line.trim_start_matches("data:").trim();
                if payload == "[DONE]" { continue; }
                if payload.is_empty() { continue; }
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(payload) {
                    if let Some(delta) = v.get("choices").and_then(|c| c.as_array()).and_then(|a| a.get(0)).and_then(|c| c.get("delta")) {
                        if let Some(t) = delta.get("content").and_then(|x| x.as_str()) {
                            if !t.is_empty() { let _ = on_chunk(t); full_text.push_str(t); }
                        }
                    }
                }
            }
        }

        Ok(crate::gemini::GenerateContentResponse {
            candidates: vec![crate::gemini::Candidate { content: Content { role: "model".into(), parts: vec![Part::Text{ text: full_text }] }, finish_reason: None }],
            prompt_feedback: None,
            usage_metadata: None,
        })
    }
}

/// Anthropic adapter (stub). Implement translation to/from request/response types later.
pub struct AnthropicBackend {
    api_key: String,
    model: String,
    http: reqwest::Client,
    last_tool_call_id_by_name: HashMap<String, String>,
}

impl AnthropicBackend {
    pub fn new(api_key: String, model: String) -> Self {
        let http = reqwest::Client::builder()
            .user_agent("vtagent/llm-anthropic")
            .build()
            .expect("http client");
        Self { api_key, model, http, last_tool_call_id_by_name: HashMap::new() }
    }
}

impl AnthropicBackend {
    async fn generate_content(
        &mut self,
        req: &crate::gemini::GenerateContentRequest,
    ) -> Result<crate::gemini::GenerateContentResponse> {
        use crate::gemini::{Content, Part};
        // Map to Anthropic Messages API
        let url = "https://api.anthropic.com/v1/messages";

        // Build messages with tool support
        let mut messages: Vec<serde_json::Value> = Vec::new();
        let mut gen_id_counter = 0usize;
        for c in &req.contents {
            let role = match c.role.as_str() { "user" => "user", _ => "assistant" };
            let mut content_arr: Vec<serde_json::Value> = Vec::new();
            for part in &c.parts {
                match part {
                    crate::gemini::Part::Text { text } => {
                        if !text.is_empty() { content_arr.push(json!({"type":"text","text": text})); }
                    }
                    crate::gemini::Part::FunctionCall { function_call } => {
                        // assistant tool_use piece
                        let id = function_call.id.clone().unwrap_or_else(|| { gen_id_counter += 1; format!("call_{}", gen_id_counter) });
                        self.last_tool_call_id_by_name.insert(function_call.name.clone(), id.clone());
                        content_arr.push(json!({"type":"tool_use","id": id, "name": function_call.name, "input": function_call.args}));
                    }
                    crate::gemini::Part::FunctionResponse { function_response } => {
                        // user tool_result piece
                        let tool_use_id = self.last_tool_call_id_by_name.get(&function_response.name).cloned().unwrap_or_else(|| { gen_id_counter += 1; format!("call_{}", gen_id_counter) });
                        content_arr.push(json!({"type":"tool_result","tool_use_id": tool_use_id, "content": function_response.response}));
                    }
                }
            }
            if !content_arr.is_empty() { messages.push(json!({"role": role, "content": content_arr})); }
        }
        let system_text = req.system_instruction.as_ref().map(|s| s.parts.iter().filter_map(|p| p.as_text()).collect::<Vec<_>>().join(""));

        // Tools mapping
        let mut tools_json: Option<Vec<serde_json::Value>> = None;
        if let Some(tools) = &req.tools {
            let mut arr = Vec::new();
            for t in tools {
                for f in &t.function_declarations {
                    arr.push(json!({
                        "name": f.name,
                        "description": f.description,
                        "input_schema": f.parameters
                    }));
                }
            }
            if !arr.is_empty() { tools_json = Some(arr); }
        }

        let mut body = json!({
            "model": self.model,
            "messages": messages,
        });
        if let Some(sys) = system_text { if !sys.is_empty() { body["system"] = json!(sys); } }
        if let Some(ts) = tools_json { body["tools"] = json!(ts); }
        if let Some(cfg) = &req.generation_config {
            if let Some(t) = cfg.get("temperature").and_then(|v| v.as_f64()) {
                body["temperature"] = json!(t);
            }
            if let Some(m) = cfg.get("maxOutputTokens").and_then(|v| v.as_u64()) {
                body["max_tokens"] = json!(m as u32);
            }
        }

        let resp = self.http
            .post(url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Anthropic request failed: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Anthropic error ({}): {}", status, text));
        }

        let v: serde_json::Value = resp.json().await.map_err(|e| anyhow::anyhow!("Anthropic parse failed: {}", e))?;
        let pieces = v.get("content").and_then(|c| c.as_array()).cloned().unwrap_or_default();
        let mut parts: Vec<Part> = Vec::new();
        for seg in pieces {
            if let Some(t) = seg.get("type").and_then(|x| x.as_str()) {
                match t {
                    "text" => {
                        if let Some(txt) = seg.get("text").and_then(|x| x.as_str()) { parts.push(Part::Text { text: txt.to_string() }); }
                    }
                    "tool_use" => {
                        let id = seg.get("id").and_then(|x| x.as_str()).map(|s| s.to_string());
                        let name = seg.get("name").and_then(|x| x.as_str()).unwrap_or("").to_string();
                        let input = seg.get("input").cloned().unwrap_or(json!({}));
                        let idc = id.clone();
                        parts.push(Part::FunctionCall { function_call: crate::gemini::FunctionCall { name: name.clone(), args: input, id } });
                        if let Some(idv) = idc { self.last_tool_call_id_by_name.insert(name, idv); }
                    }
                    _ => {}
                }
            }
        }
        Ok(crate::gemini::GenerateContentResponse { candidates: vec![crate::gemini::Candidate { content: Content { role: "model".to_string(), parts }, finish_reason: None }], prompt_feedback: None, usage_metadata: None })
    }

    async fn generate_content_stream<F>(
        &mut self,
        req: &crate::gemini::GenerateContentRequest,
        mut on_chunk: F,
    ) -> Result<crate::gemini::GenerateContentResponse>
    where
        F: FnMut(&str) -> Result<()>,
    {
        use tokio_stream::StreamExt;
        use crate::gemini::{Content, Part};
        // Assemble body
        let mut messages: Vec<serde_json::Value> = Vec::new();
        let mut gen_id_counter = 0usize;
        for c in &req.contents {
            let role = match c.role.as_str() { "user" => "user", _ => "assistant" };
            let mut content_arr: Vec<serde_json::Value> = Vec::new();
            for part in &c.parts {
                match part {
                    crate::gemini::Part::Text { text } => { if !text.is_empty() { content_arr.push(serde_json::json!({"type":"text","text": text})); } }
                    crate::gemini::Part::FunctionCall { function_call } => {
                        let id = function_call.id.clone().unwrap_or_else(|| { gen_id_counter += 1; format!("call_{}", gen_id_counter) });
                        self.last_tool_call_id_by_name.insert(function_call.name.clone(), id.clone());
                        content_arr.push(serde_json::json!({"type":"tool_use","id": id, "name": function_call.name, "input": function_call.args}));
                    }
                    crate::gemini::Part::FunctionResponse { function_response } => {
                        let tool_use_id = self.last_tool_call_id_by_name.get(&function_response.name).cloned().unwrap_or_else(|| { gen_id_counter += 1; format!("call_{}", gen_id_counter) });
                        content_arr.push(serde_json::json!({"type":"tool_result","tool_use_id": tool_use_id, "content": function_response.response}));
                    }
                }
            }
            if !content_arr.is_empty() { messages.push(serde_json::json!({"role": role, "content": content_arr})); }
        }
        let mut body = serde_json::json!({"model": self.model, "messages": messages, "stream": true});
        if let Some(cfg) = &req.generation_config {
            if let Some(t) = cfg.get("temperature").and_then(|v| v.as_f64()) { body["temperature"] = serde_json::json!(t); }
            if let Some(m) = cfg.get("maxOutputTokens").and_then(|v| v.as_u64()) { body["max_tokens"] = serde_json::json!(m as u32); }
        }
        if let Some(tools) = &req.tools {
            let mut arr = Vec::new();
            for t in tools { for f in &t.function_declarations { arr.push(serde_json::json!({"name":f.name,"description":f.description,"input_schema":f.parameters})); } }
            if !arr.is_empty() { body["tools"] = serde_json::json!(arr); }
        }

        let resp = self.http
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Anthropic request failed: {}", e))?;

        let mut full_text = String::new();
        let mut stream = resp.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let bytes = match chunk { Ok(b) => b, Err(_) => break };
            let s = String::from_utf8_lossy(&bytes);
            for line in s.lines() {
                let line = line.trim_start();
                if !line.starts_with("data:") { continue; }
                let payload = line.trim_start_matches("data:").trim();
                if payload.is_empty() { continue; }
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(payload) {
                    if v.get("type").and_then(|x| x.as_str()) == Some("content_block_delta") {
                        if let Some(delta) = v.get("delta") {
                            if delta.get("type").and_then(|x| x.as_str()) == Some("text_delta") {
                                if let Some(t) = delta.get("text").and_then(|x| x.as_str()) {
                                    if !t.is_empty() { let _ = on_chunk(t); full_text.push_str(t); }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(crate::gemini::GenerateContentResponse {
            candidates: vec![crate::gemini::Candidate { content: Content { role: "model".into(), parts: vec![Part::Text{ text: full_text }] }, finish_reason: None }],
            prompt_feedback: None,
            usage_metadata: None,
        })
    }
}
