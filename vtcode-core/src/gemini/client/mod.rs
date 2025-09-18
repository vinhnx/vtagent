pub mod config;
pub mod retry;

pub use config::ClientConfig;
pub use retry::RetryConfig;

use crate::gemini::models::{GenerateContentRequest, GenerateContentResponse};
use crate::gemini::streaming::{
    StreamingError, StreamingMetrics, StreamingProcessor, StreamingResponse,
};
use crate::llm::provider::StreamToken;
use anyhow::{Context, Result};
use futures::StreamExt;
use reqwest::Client as ReqwestClient;
use std::time::Instant;
use tokio::sync::mpsc;
use tokio_stream::{Stream, wrappers::ReceiverStream};

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

        if error_str.contains("timeout")
            || error_str.contains("connection")
            || error_str.contains("network")
        {
            StreamingError::NetworkError {
                message: error.to_string(),
                is_retryable: true,
            }
        } else if error_str.contains("rate limit") || error_str.contains("429") {
            StreamingError::ApiError {
                status_code: 429,
                message: "Rate limit exceeded".to_string(),
                is_retryable: true,
            }
        } else {
            StreamingError::NetworkError {
                message: error.to_string(),
                is_retryable: false,
            }
        }
    }

    /// Generate content with the Gemini API
    pub async fn generate(
        &mut self,
        request: &GenerateContentRequest,
    ) -> Result<GenerateContentResponse> {
        let start_time = Instant::now();

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model, self.api_key
        );

        let response = self
            .http
            .post(&url)
            .json(request)
            .send()
            .await
            .context("Failed to send request")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("API error {}: {}", status, error_text));
        }

        let response_data: GenerateContentResponse =
            response.json().await.context("Failed to parse response")?;

        self.metrics.total_requests += 1;
        self.metrics.total_response_time += start_time.elapsed();

        Ok(response_data)
    }

    /// Generate content with the Gemini API using streaming
    pub async fn generate_stream<F>(
        &mut self,
        request: &GenerateContentRequest,
        on_chunk: F,
    ) -> Result<StreamingResponse, StreamingError>
    where
        F: FnMut(&str) -> Result<(), StreamingError>,
    {
        let start_time = Instant::now();

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent?key={}",
            self.model, self.api_key
        );

        let response = self
            .http
            .post(&url)
            .json(request)
            .send()
            .await
            .map_err(|e| {
                let error = anyhow::Error::new(e);
                self.classify_error(&error)
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();

            let is_retryable = match status.as_u16() {
                429 | 500 | 502 | 503 | 504 => true,
                _ => false,
            };

            return Err(StreamingError::ApiError {
                status_code: status.as_u16(),
                message: error_text,
                is_retryable,
            });
        }

        // Process the streaming response
        let mut processor = StreamingProcessor::new();
        let result = processor.process_stream(response, on_chunk).await;

        self.metrics.total_requests += 1;
        self.metrics.total_response_time += start_time.elapsed();

        result
    }

    /// Generate content with the Gemini API using token-based streaming
    ///
    /// This method returns a stream of tokens that can be consumed for real-time
    /// display with animation support.
    ///
    /// # Arguments
    ///
    /// * `request` - The generate content request
    ///
    /// # Returns
    ///
    /// A stream of StreamToken results
    pub fn stream_tokens(
        &mut self,
        request: &GenerateContentRequest,
    ) -> impl Stream<Item = Result<StreamToken, StreamingError>> {
        let (tx, rx) = mpsc::channel(100);
        let client = self.http.clone();
        let api_key = self.api_key.clone();
        let model = self.model.clone();
        let request = request.clone();

        tokio::spawn(async move {
            let result = Self::fetch_streaming_tokens(client, api_key, model, request, tx).await;
            if let Err(e) = result {
                eprintln!("Streaming tokens error: {}", e);
            }
        });

        ReceiverStream::new(rx)
    }

    async fn fetch_streaming_tokens(
        client: ReqwestClient,
        api_key: String,
        model: String,
        request: GenerateContentRequest,
        tx: mpsc::Sender<Result<StreamToken, StreamingError>>,
    ) -> Result<(), StreamingError> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent?key={}",
            model, api_key
        );

        let response = client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| StreamingError::NetworkError {
                message: format!("Failed to send request: {}", e),
                is_retryable: true,
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();

            let is_retryable = match status.as_u16() {
                429 | 500 | 502 | 503 | 504 => true,
                _ => false,
            };

            return Err(StreamingError::ApiError {
                status_code: status.as_u16(),
                message: error_text,
                is_retryable,
            });
        }

        let mut stream = response.bytes_stream();
        let mut buffer = Vec::new();
        let mut usage_metadata = None;

        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(bytes) => {
                    buffer.extend_from_slice(&bytes);

                    // Process complete JSON objects from buffer
                    while let Some(end_pos) = Self::find_json_boundary(&buffer) {
                        let json_bytes = buffer.drain(..end_pos).collect::<Vec<_>>();

                        // Skip empty lines or non-JSON content
                        let json_str = String::from_utf8_lossy(&json_bytes).trim().to_string();
                        if json_str.is_empty() || !json_str.starts_with('{') {
                            continue;
                        }

                        match serde_json::from_str::<serde_json::Value>(&json_str) {
                            Ok(json_value) => {
                                // Extract usage metadata if present
                                if let Some(usage) = json_value.get("usageMetadata") {
                                    usage_metadata = Some(usage.clone());
                                }

                                // Extract candidates
                                if let Some(candidates) = json_value.get("candidates").and_then(|c| c.as_array()) {
                                    for candidate in candidates {
                                        if let Some(content) = candidate.get("content") {
                                            if let Some(parts) = content.get("parts").and_then(|p| p.as_array()) {
                                                for part in parts {
                                                    if let Some(text) = part.get("text").and_then(|t| t.as_str()) {
                                                        let is_final = candidate.get("finishReason").is_some();
                                                        let finish_reason = candidate
                                                            .get("finishReason")
                                                            .and_then(|fr| fr.as_str())
                                                            .map(|s| s.to_string());

                                                        let token = StreamToken {
                                                            text: text.to_string(),
                                                            is_final,
                                                            finish_reason,
                                                        };

                                                        if tx.send(Ok(token)).await.is_err() {
                                                            return Ok(());
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("Failed to parse JSON: {} - Content: {}", e, json_str);
                            }
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(Err(StreamingError::NetworkError {
                        message: format!("Failed to read chunk: {}", e),
                        is_retryable: true,
                    })).await;
                    break;
                }
            }
        }

        // Send final token if we haven't already
        let _ = tx.send(Ok(StreamToken {
            text: String::new(),
            is_final: true,
            finish_reason: Some("STOP".to_string()),
        })).await;

        Ok(())
    }

    fn find_json_boundary(buffer: &[u8]) -> Option<usize> {
        let s = String::from_utf8_lossy(buffer);
        let mut brace_count = 0;
        let mut in_string = false;
        let mut escape_next = false;
        let mut start_found = false;

        for (i, c) in s.char_indices() {
            if escape_next {
                escape_next = false;
                continue;
            }

            match c {
                '"' if !escape_next => in_string = !in_string,
                '\\' if in_string => escape_next = true,
                '{' if !in_string => {
                    brace_count += 1;
                    start_found = true;
                }
                '}' if !in_string => {
                    brace_count -= 1;
                    if start_found && brace_count == 0 {
                        return Some(i + c.len_utf8());
                    }
                }
                '\n' if !start_found => {
                    // Skip to next line if we haven't found a JSON start
                    return Some(i + c.len_utf8());
                }
                _ => {}
            }
        }
        None
    }
}
