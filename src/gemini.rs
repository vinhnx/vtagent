use anyhow::{Context, Result};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::time;

#[derive(Clone)]
pub struct Client {
    api_key: String,
    model: String,
    http: reqwest::Client,
}

impl Client {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            api_key,
            model,
            http: reqwest::Client::new(),
        }
    }

    fn endpoint(&self) -> Result<Url> {
        let model = if self.model.starts_with("models/") {
            self.model.clone()
        } else {
            format!("models/{}", self.model)
        };
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/{}:generateContent",
            model
        );
        Url::parse(&url).context("invalid Gemini endpoint URL")
    }

    fn stream_endpoint(&self) -> Result<Url> {
        let model = if self.model.starts_with("models/") {
            self.model.clone()
        } else {
            format!("models/{}", self.model)
        };
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/{}:streamGenerateContent",
            model
        );
        Url::parse(&url).context("invalid Gemini stream endpoint URL")
    }

    pub async fn generate_content(
        &self,
        req: &GenerateContentRequest,
    ) -> Result<GenerateContentResponse> {
        let url = self.endpoint()?;
        let resp = self
            .http
            .post(url)
            .query(&[("key", self.api_key.as_str())])
            .json(req)
            .send()
            .await
            .context("request to Gemini API failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            let msg = format!("Gemini API error: {} - {}", status, text);
            return Err(anyhow::anyhow!(msg));
        }
        let data = resp
            .json::<GenerateContentResponse>()
            .await
            .context("invalid response JSON from Gemini API")?;
        Ok(data)
    }

    /// Stream generate content with real-time output
    pub async fn generate_content_stream<F>(
        &self,
        req: &GenerateContentRequest,
        on_chunk: F,
    ) -> Result<GenerateContentResponse>
    where
        F: Fn(&str) -> Result<()>,
    {
        let url = self.stream_endpoint()?;
        let resp = self
            .http
            .post(url)
            .query(&[("key", self.api_key.as_str())])
            .json(req)
            .send()
            .await
            .context("request to Gemini streaming API failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            let msg = format!("Gemini streaming API error: {} - {}", status, text);
            return Err(anyhow::anyhow!(msg));
        }

        // Read the entire response and parse as JSON array
        let body_text = resp.text().await.context("error reading response body")?;
        let mut accumulated_response = String::new();

        // Parse the JSON array response
        if let Ok(response_array) = serde_json::from_str::<Vec<GenerateContentResponse>>(&body_text) {
            // Process each response in the array sequentially
            for response in response_array {
                if let Some(candidate) = response.candidates.into_iter().next() {
                    for part in candidate.content.parts {
                        if let Part::Text { text } = part {
                            // Display each text chunk with streaming effect
                            on_chunk(&text)?;
                            accumulated_response.push_str(&text);

                            // Add a small delay to simulate streaming effect
                            time::sleep(time::Duration::from_millis(50)).await;
                        }
                    }
                }
            }
        } else {
            // If parsing fails, try to extract text directly from the response
            // Fallback: try to extract text content manually
            if body_text.contains("\"text\":") {
                // Simple regex-like extraction of text content
                let mut current_pos = 0;
                while let Some(text_start) = body_text[current_pos..].find("\"text\":\"") {
                    let absolute_start = current_pos + text_start + 8; // +8 for "\"text\":\""
                    if let Some(text_end) = body_text[absolute_start..].find("\"") {
                        let text_content = &body_text[absolute_start..absolute_start + text_end];
                        if !text_content.is_empty() {
                            on_chunk(text_content)?;
                            accumulated_response.push_str(text_content);
                            time::sleep(time::Duration::from_millis(30)).await;
                        }
                        current_pos = absolute_start + text_end + 1;
                    } else {
                        break;
                    }
                }
            }
        }

        // If no response was accumulated, fall back to regular API
        if accumulated_response.trim().is_empty() {
            return self.generate_content(req).await;
        }

        // Return the final accumulated response as a complete response
        Ok(GenerateContentResponse {
            candidates: vec![Candidate {
                content: Content {
                    role: "model".to_string(),
                    parts: vec![Part::Text {
                        text: accumulated_response,
                    }],
                },
                finish_reason: None,
            }],
            prompt_feedback: None,
            usage_metadata: None,
        })
    }
}

// Request/Response types

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateContentRequest {
    pub contents: Vec<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "toolConfig")]
    pub tool_config: Option<ToolConfig>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "systemInstruction")]
    pub system_instruction: Option<Content>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "generationConfig")]
    pub generation_config: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateContentResponse {
    pub candidates: Vec<Candidate>,
    #[serde(default, rename = "promptFeedback")]
    pub prompt_feedback: Option<Value>,
    #[serde(default, rename = "usageMetadata")]
    pub usage_metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingResponse {
    pub candidates: Vec<StreamingCandidate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingCandidate {
    pub content: Content,
    #[serde(default, rename = "finishReason")]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candidate {
    pub content: Content,
    #[serde(default, rename = "finishReason")]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Content {
    pub role: String,
    pub parts: Vec<Part>,
}

impl Content {
    pub fn user_text(text: impl Into<String>) -> Self {
        Content {
            role: "user".into(),
            parts: vec![Part::Text { text: text.into() }],
        }
    }
    pub fn system_text(text: impl Into<String>) -> Self {
        Content {
            role: "system".into(),
            parts: vec![Part::Text { text: text.into() }],
        }
    }
    pub fn user_parts(parts: Vec<Part>) -> Self {
        Content {
            role: "user".into(),
            parts,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Part {
    Text {
        text: String,
    },
    #[serde(rename_all = "camelCase")]
    FunctionCall {
        function_call: FunctionCall,
    },
    #[serde(rename_all = "camelCase")]
    FunctionResponse {
        function_response: FunctionResponse,
    },
}

impl Part {
    /// Get the text content if this is a Text part
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Part::Text { text } => Some(text),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub args: Value,
    #[serde(default)]
    pub id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionResponse {
    pub name: String,
    pub response: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    #[serde(rename = "functionDeclarations")]
    pub function_declarations: Vec<FunctionDeclaration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDeclaration {
    pub name: String,
    pub description: String,
    pub parameters: Value, // OpenAPI-ish JSON schema
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolConfig {
    #[serde(rename = "functionCallingConfig")]
    pub function_calling_config: FunctionCallingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCallingConfig {
    pub mode: String, // "AUTO" | "ANY" | "NONE" (as of docs)
}

impl ToolConfig {
    pub fn auto() -> Self {
        Self {
            function_calling_config: FunctionCallingConfig {
                mode: "AUTO".into(),
            },
        }
    }
}
