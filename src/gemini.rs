use anyhow::{Context, Result};
use reqwest::{Client as ReqwestClient, StatusCode, Url};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::{Duration, Instant};
use tokio::time;

/// Configuration for HTTP client optimization
#[derive(Clone)]
pub struct ClientConfig {
    /// Maximum number of idle connections per host
    pub pool_max_idle_per_host: usize,
    /// How long to keep idle connections alive
    pub pool_idle_timeout: Duration,
    /// TCP keepalive duration
    pub tcp_keepalive: Duration,
    /// Request timeout
    pub request_timeout: Duration,
    /// Connection timeout
    pub connect_timeout: Duration,
    /// User agent string
    pub user_agent: String,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            pool_max_idle_per_host: 10,
            pool_idle_timeout: Duration::from_secs(90),
            tcp_keepalive: Duration::from_secs(60),
            request_timeout: Duration::from_secs(60),
            connect_timeout: Duration::from_secs(10),
            user_agent: "vtagent/1.0.0".to_string(),
        }
    }
}

impl ClientConfig {
    /// Configuration optimized for high-throughput scenarios
    pub fn high_throughput() -> Self {
        Self {
            pool_max_idle_per_host: 20,
            pool_idle_timeout: Duration::from_secs(120),
            tcp_keepalive: Duration::from_secs(60),
            request_timeout: Duration::from_secs(120),
            connect_timeout: Duration::from_secs(15),
            user_agent: "vtagent/1.0.0-high-throughput".to_string(),
        }
    }

    /// Configuration optimized for low-latency scenarios
    pub fn low_latency() -> Self {
        Self {
            pool_max_idle_per_host: 5,
            pool_idle_timeout: Duration::from_secs(30),
            tcp_keepalive: Duration::from_secs(30),
            request_timeout: Duration::from_secs(30),
            connect_timeout: Duration::from_secs(5),
            user_agent: "vtagent/1.0.0-low-latency".to_string(),
        }
    }
}

/// Streaming error types for better error classification and handling
#[derive(Debug, Clone)]
pub enum StreamingError {
    /// Network-related errors (connection, timeout, DNS, etc.)
    NetworkError {
        message: String,
        is_retryable: bool,
    },
    /// API-related errors (rate limits, authentication, etc.)
    ApiError {
        status_code: u16,
        message: String,
        is_retryable: bool,
    },
    /// Response parsing errors
    ParseError {
        message: String,
        raw_response: String,
    },
    /// Timeout errors
    TimeoutError {
        operation: String,
        duration: Duration,
    },
    /// Content validation errors
    ContentError {
        message: String,
    },
    /// Streaming-specific errors
    StreamingError {
        message: String,
        partial_content: Option<String>,
    },
}

impl std::fmt::Display for StreamingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StreamingError::NetworkError { message, .. } => {
                write!(f, "Network error: {}", message)
            }
            StreamingError::ApiError { status_code, message, .. } => {
                write!(f, "API error ({}): {}", status_code, message)
            }
            StreamingError::ParseError { message, .. } => {
                write!(f, "Parse error: {}", message)
            }
            StreamingError::TimeoutError { operation, duration } => {
                write!(f, "Timeout during {} after {:?}", operation, duration)
            }
            StreamingError::ContentError { message } => {
                write!(f, "Content error: {}", message)
            }
            StreamingError::StreamingError { message, .. } => {
                write!(f, "Streaming error: {}", message)
            }
        }
    }
}

impl std::error::Error for StreamingError {}

/// Retry configuration for streaming operations
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub backoff_multiplier: f64,
    pub retryable_errors: Vec<String>,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            backoff_multiplier: 2.0,
            retryable_errors: vec![
                "timeout".to_string(),
                "connection".to_string(),
                "rate_limit".to_string(),
                "server_error".to_string(),
            ],
        }
    }
}

/// Streaming metrics for monitoring and debugging
#[derive(Debug, Clone, Default)]
pub struct StreamingMetrics {
    pub request_start_time: Option<Instant>,
    pub first_chunk_time: Option<Instant>,
    pub total_chunks: usize,
    pub total_bytes: usize,
    pub error_count: usize,
    pub retry_count: usize,
}

#[derive(Clone)]
pub struct Client {
    api_key: String,
    model: String,
    http: ReqwestClient,
    config: ClientConfig,
    retry_config: RetryConfig,
    metrics: StreamingMetrics,
}

impl Client {
    pub fn new(api_key: String, model: String) -> Self {
        Self::with_config(api_key, model, ClientConfig::default())
    }

    /// Create a client with custom configuration
    pub fn with_config(api_key: String, model: String, config: ClientConfig) -> Self {
        let http_client = ReqwestClient::builder()
            .pool_max_idle_per_host(config.pool_max_idle_per_host)
            .pool_idle_timeout(config.pool_idle_timeout)
            .tcp_keepalive(config.tcp_keepalive)
            .timeout(config.request_timeout)
            .connect_timeout(config.connect_timeout)
            .user_agent(&config.user_agent)
            .build()
            .expect("Failed to build HTTP client");

        Self {
            api_key,
            model,
            http: http_client,
            config,
            retry_config: RetryConfig::default(),
            metrics: StreamingMetrics::default(),
        }
    }

    /// Get current client configuration
    pub fn config(&self) -> &ClientConfig {
        &self.config
    }

    /// Set retry configuration
    pub fn with_retry_config(mut self, retry_config: RetryConfig) -> Self {
        self.retry_config = retry_config;
        self
    }

    /// Get current retry configuration
    pub fn retry_config(&self) -> &RetryConfig {
        &self.retry_config
    }

    /// Get streaming metrics
    pub fn metrics(&self) -> &StreamingMetrics {
        &self.metrics
    }

    /// Reset streaming metrics
    pub fn reset_metrics(&mut self) {
        self.metrics = StreamingMetrics::default();
    }

    /// Classify error to determine if it's retryable
    fn classify_error(&self, error: &anyhow::Error) -> StreamingError {
        let error_str = error.to_string().to_lowercase();

        // Network errors
        if error_str.contains("timeout") || error_str.contains("connection") ||
           error_str.contains("network") || error_str.contains("dns") {
            return StreamingError::NetworkError {
                message: error.to_string(),
                is_retryable: true,
            };
        }

        // API errors
        if error_str.contains("rate limit") || error_str.contains("429") {
            return StreamingError::ApiError {
                status_code: 429,
                message: "Rate limit exceeded".to_string(),
                is_retryable: true,
            };
        }

        if error_str.contains("unauthorized") || error_str.contains("401") {
            return StreamingError::ApiError {
                status_code: 401,
                message: "Authentication failed".to_string(),
                is_retryable: false,
            };
        }

        if error_str.contains("server error") || error_str.contains("5") {
            return StreamingError::ApiError {
                status_code: 500,
                message: "Server error".to_string(),
                is_retryable: true,
            };
        }

        // Parse errors
        if error_str.contains("parse") || error_str.contains("json") {
            return StreamingError::ParseError {
                message: error.to_string(),
                raw_response: String::new(),
            };
        }

        // Content errors
        if error_str.contains("content") || error_str.contains("empty") {
            return StreamingError::ContentError {
                message: error.to_string(),
            };
        }

        // Default to streaming error
        StreamingError::StreamingError {
            message: error.to_string(),
            partial_content: None,
        }
    }

    /// Calculate retry delay with exponential backoff
    fn calculate_retry_delay(&self, attempt: u32) -> Duration {
        let base_delay = self.retry_config.initial_delay.as_millis() as f64;
        let multiplier = self.retry_config.backoff_multiplier.powi(attempt as i32);
        let delay_ms = (base_delay * multiplier) as u64;

        // Cap at max delay
        let max_delay_ms = self.retry_config.max_delay.as_millis() as u64;
        Duration::from_millis(delay_ms.min(max_delay_ms))
    }

    /// Check if an error should be retried
    fn should_retry(&self, error: &StreamingError) -> bool {
        match error {
            StreamingError::NetworkError { is_retryable, .. } => *is_retryable,
            StreamingError::ApiError { is_retryable, .. } => *is_retryable,
            StreamingError::ParseError { .. } => false, // Parse errors usually aren't retryable
            StreamingError::TimeoutError { .. } => true,
            StreamingError::ContentError { .. } => false,
            StreamingError::StreamingError { .. } => true, // Retry streaming errors
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

    /// Stream generate content with real-time output and comprehensive error handling
    pub async fn generate_content_stream<F>(
        &mut self,
        req: &GenerateContentRequest,
        mut on_chunk: F,
    ) -> Result<GenerateContentResponse>
    where
        F: FnMut(&str) -> Result<()>,
    {
        let mut attempt = 0;
        let mut last_error: Option<StreamingError> = None;

        // Initialize metrics
        self.metrics.request_start_time = Some(Instant::now());
        self.metrics.error_count = 0;
        self.metrics.retry_count = 0;

        loop {
            match self.attempt_streaming_request(req, &mut on_chunk).await {
                Ok(response) => {
                    // Update metrics on success
                    if let Some(start_time) = self.metrics.request_start_time {
                        let total_duration = start_time.elapsed();
                        eprintln!("Streaming completed successfully in {:?}", total_duration);
                    }
                    return Ok(response);
                }
                Err(error) => {
                    self.metrics.error_count += 1;
                    let streaming_error = self.classify_error(&error);

                    eprintln!("Streaming attempt {} failed: {}", attempt + 1, streaming_error);

                    // Check if we should retry
                    if attempt < self.retry_config.max_attempts && self.should_retry(&streaming_error) {
                        attempt += 1;
                        self.metrics.retry_count += 1;
                        last_error = Some(streaming_error);

                        let delay = self.calculate_retry_delay(attempt);
                        eprintln!("Retrying in {:?}... (attempt {}/{})",
                                delay, attempt + 1, self.retry_config.max_attempts + 1);

                        time::sleep(delay).await;
                        continue;
                    } else {
                        // No more retries or error is not retryable
                        return Err(match streaming_error {
                            StreamingError::NetworkError { message, .. } => {
                                anyhow::anyhow!("Network error after {} attempts: {}", attempt + 1, message)
                            }
                            StreamingError::ApiError { status_code, message, .. } => {
                                anyhow::anyhow!("API error ({}): {} (attempts: {})", status_code, message, attempt + 1)
                            }
                            StreamingError::ParseError { message, raw_response } => {
                                if raw_response.is_empty() {
                                    anyhow::anyhow!("Parse error: {} (attempts: {})", message, attempt + 1)
                                } else {
                                    anyhow::anyhow!("Parse error: {} (attempts: {}). Raw response: {}",
                                                  message, attempt + 1, &raw_response.chars().take(500).collect::<String>())
                                }
                            }
                            StreamingError::TimeoutError { operation, duration } => {
                                anyhow::anyhow!("Timeout during {} after {:?} (attempts: {})",
                                              operation, duration, attempt + 1)
                            }
                            StreamingError::ContentError { message } => {
                                anyhow::anyhow!("Content error: {} (attempts: {})", message, attempt + 1)
                            }
                            StreamingError::StreamingError { message, partial_content } => {
                                if let Some(partial) = partial_content {
                                    anyhow::anyhow!("Streaming error: {} (attempts: {}). Partial content: {}",
                                                  message, attempt + 1, partial)
                                } else {
                                    anyhow::anyhow!("Streaming error: {} (attempts: {})", message, attempt + 1)
                                }
                            }
                        });
                    }
                }
            }
        }
    }

    /// Execute a single streaming request attempt
    async fn attempt_streaming_request<F>(
        &mut self,
        req: &GenerateContentRequest,
        on_chunk: &mut F,
    ) -> Result<GenerateContentResponse>
    where
        F: FnMut(&str) -> Result<()>,
    {
        let url = self.stream_endpoint()?;

        // Create request with timeout
        let request_future = self.http
            .post(url)
            .query(&[("key", self.api_key.as_str())])
            .json(req)
            .send();

        // Add timeout wrapper
        let resp = match time::timeout(self.config.request_timeout, request_future).await {
            Ok(result) => result.context("HTTP request failed")?,
            Err(_) => {
                return Err(anyhow::anyhow!("Timeout during HTTP request after {:?}", self.config.request_timeout));
            }
        };

        if !resp.status().is_success() {
            let status = resp.status();
            let text = match resp.text().await {
                Ok(text) => text,
                Err(e) => format!("Failed to read error response: {}", e),
            };

            match status {
                StatusCode::TOO_MANY_REQUESTS => {
                    return Err(anyhow::anyhow!("API error (429): Rate limit exceeded"));
                }
                StatusCode::UNAUTHORIZED => {
                    return Err(anyhow::anyhow!("API error (401): Authentication failed"));
                }
                StatusCode::INTERNAL_SERVER_ERROR | StatusCode::BAD_GATEWAY | StatusCode::SERVICE_UNAVAILABLE => {
                    return Err(anyhow::anyhow!("API error ({}): Server error: {}", status.as_u16(), text));
                }
                _ => {
                    return Err(anyhow::anyhow!("API error ({}): {}", status.as_u16(), text));
                }
            }
        }

        // Read the entire response body with timeout
        let body_text = match time::timeout(
            Duration::from_secs(30),
            resp.text()
        ).await {
            Ok(result) => result.context("Failed to read response body")?,
            Err(_) => {
                return Err(anyhow::anyhow!("Timeout during response reading after 30 seconds"));
            }
        };

        // Early exit if response is empty
        if body_text.trim().is_empty() {
            return Err(anyhow::anyhow!("Empty response from streaming API"));
        }

        let mut accumulated_response = String::new();
        let mut has_valid_content = false;
        let mut partial_content = String::new();

        // Process the response as streaming by splitting and simulating real-time output
        match serde_json::from_str::<Vec<GenerateContentResponse>>(&body_text) {
            Ok(response_array) => {
                // Process each response in the array sequentially
                for (i, response) in response_array.iter().enumerate() {
                    if let Some(candidate) = response.candidates.first() {
                        for part in &candidate.content.parts {
                            match part {
                                Part::Text { text } => {
                                    if !text.trim().is_empty() {
                                        partial_content.push_str(text);
                                        self.metrics.total_bytes += text.len();

                                        // Simulate streaming by outputting in chunks
                                        for chunk in text.chars().collect::<Vec<char>>().chunks(10) {
                                            let chunk_str: String = chunk.iter().collect();
                                            if let Err(e) = on_chunk(&chunk_str) {
                                                return Err(anyhow::anyhow!("Failed to output chunk: {}", e));
                                            }
                                            accumulated_response.push_str(&chunk_str);
                                            has_valid_content = true;
                                            self.metrics.total_chunks += 1;

                                            // Add a small delay to simulate streaming effect
                                            time::sleep(time::Duration::from_millis(20)).await;
                                        }
                                    }
                                }
                                Part::FunctionCall { function_call } => {
                                    // Handle function calls by serializing them as JSON
                                    match serde_json::to_string(function_call) {
                                        Ok(function_call_json) => {
                                            let function_call_text = format!("[FUNCTION_CALL:{}]", function_call_json);
                                            partial_content.push_str(&function_call_text);

                                            if let Err(e) = on_chunk(&function_call_text) {
                                                return Err(anyhow::anyhow!("Failed to output function call: {}", e));
                                            }
                                            accumulated_response.push_str(&function_call_text);
                                            has_valid_content = true;
                                            self.metrics.total_chunks += 1;
                                        }
                                        Err(e) => {
                                            return Err(anyhow::anyhow!("Failed to serialize function call: {}", e));
                                        }
                                    }
                                }
                                Part::FunctionResponse { .. } => {
                                    has_valid_content = true;
                                }
                            }
                        }
                    } else if i == 0 {
                        // No candidates in first response - this might be an error
                        return Err(anyhow::anyhow!("No candidates found in streaming response"));
                    }
                }

                // Update first chunk time if not set
                if self.metrics.first_chunk_time.is_none() && has_valid_content {
                    self.metrics.first_chunk_time = Some(Instant::now());
                }
            }
            Err(parse_err) => {
                // If JSON parsing fails, try to extract text content manually
                eprintln!("Warning: Failed to parse streaming response as JSON: {}", parse_err);

                // Try to extract text content from raw response
                if let Some(extracted_text) = self.extract_text_from_raw_response(&body_text) {
                    if !extracted_text.trim().is_empty() {
                        partial_content.push_str(&extracted_text);

                        if let Err(e) = on_chunk(&extracted_text) {
                            return Err(anyhow::anyhow!("Failed to output extracted text: {}", e));
                        }
                        accumulated_response.push_str(&extracted_text);
                        has_valid_content = true;
                        self.metrics.total_chunks += 1;
                        self.metrics.total_bytes += extracted_text.len();
                    }
                } else {
                    // Last resort: try to extract any text content
                    if let Some(fallback_text) = self.extract_text_fallback(&body_text) {
                        if !fallback_text.trim().is_empty() {
                            partial_content.push_str(&fallback_text);

                            if let Err(e) = on_chunk(&fallback_text) {
                                return Err(anyhow::anyhow!("Failed to output fallback text: {}", e));
                            }
                            accumulated_response.push_str(&fallback_text);
                            has_valid_content = true;
                            self.metrics.total_chunks += 1;
                            self.metrics.total_bytes += fallback_text.len();
                        }
                    }
                }

                if !has_valid_content {
                    return Err(anyhow::anyhow!("Failed to parse response and extract content: {}", parse_err));
                }
            }
        }

        // If no valid content was found, return an error
        if !has_valid_content || accumulated_response.trim().is_empty() {
            return Err(anyhow::anyhow!("No valid content received from streaming API"));
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

    /// Extract text content from raw JSON response
    fn extract_text_from_raw_response(&self, body_text: &str) -> Option<String> {
        // Try to parse individual JSON objects from the response
        let mut extracted_text = String::new();

        // Split response by newlines (each line might be a JSON object)
        for line in body_text.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            // Try to parse this line as a JSON object
            if let Ok(response) = serde_json::from_str::<GenerateContentResponse>(line) {
                if let Some(candidate) = response.candidates.into_iter().next() {
                    for part in candidate.content.parts {
                        match part {
                            Part::Text { text } => {
                                if !text.trim().is_empty() {
                                    extracted_text.push_str(&text);
                                }
                            }
                            Part::FunctionCall { function_call } => {
                                if let Ok(function_call_json) = serde_json::to_string(&function_call) {
                                    let function_call_text = format!("[FUNCTION_CALL:{}]", function_call_json);
                                    extracted_text.push_str(&function_call_text);
                                }
                            }
                            Part::FunctionResponse { .. } => {
                                // Mark as having content
                            }
                        }
                    }
                }
            }
        }

        if extracted_text.is_empty() {
            None
        } else {
            Some(extracted_text)
        }
    }

    /// Fallback text extraction when all else fails
    fn extract_text_fallback(&self, body_text: &str) -> Option<String> {
        // Simple regex-like extraction of text content between quotes
        let mut extracted = String::new();
        let mut in_text = false;
        let mut escape_next = false;

        for ch in body_text.chars() {
            match ch {
                '"' if !escape_next => {
                    if in_text {
                        // End of text content
                        if !extracted.trim().is_empty() {
                            return Some(extracted.trim().to_string());
                        }
                        extracted.clear();
                    }
                    in_text = !in_text;
                }
                '\\' if in_text => {
                    escape_next = true;
                }
                _ if in_text => {
                    extracted.push(ch);
                    escape_next = false;
                }
                _ => {
                    escape_next = false;
                }
            }
        }

        if extracted.trim().is_empty() {
            None
        } else {
            Some(extracted.trim().to_string())
        }
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
