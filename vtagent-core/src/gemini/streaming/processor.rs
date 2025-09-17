//! Streaming processor for handling real-time responses from the Gemini API
//!
//! This module provides functionality to process streaming responses from the Gemini API,
//! parse them in real-time, and provide callbacks for handling each chunk of data.

use crate::gemini::models::Part;
use crate::gemini::streaming::{
    StreamingCandidate, StreamingError, StreamingMetrics, StreamingResponse,
};
use crate::llm::stream;
use futures::stream::StreamExt;
use reqwest::Response;
use std::collections::HashMap;
use std::time::Instant;
use tokio::time::{Duration, timeout};

/// Configuration for the streaming processor
#[derive(Debug, Clone)]
pub struct StreamingConfig {
    /// Timeout for reading each chunk
    pub chunk_timeout: Duration,
    /// Maximum time to wait for the first chunk
    pub first_chunk_timeout: Duration,
    /// Buffer size for chunk processing
    pub buffer_size: usize,
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            chunk_timeout: Duration::from_secs(30),
            first_chunk_timeout: Duration::from_secs(60),
            buffer_size: 1024,
        }
    }
}

/// Streaming processor for handling real-time responses from the Gemini API
pub struct StreamingProcessor {
    config: StreamingConfig,
    metrics: StreamingMetrics,
    candidate_text: HashMap<usize, String>,
}

impl StreamingProcessor {
    /// Create a new streaming processor with default configuration
    pub fn new() -> Self {
        Self {
            config: StreamingConfig::default(),
            metrics: StreamingMetrics::default(),
            candidate_text: HashMap::new(),
        }
    }

    /// Create a new streaming processor with custom configuration
    pub fn with_config(config: StreamingConfig) -> Self {
        Self {
            config,
            metrics: StreamingMetrics::default(),
            candidate_text: HashMap::new(),
        }
    }

    /// Process a streaming response from the Gemini API
    ///
    /// This method takes a response and processes it in real-time, calling the provided
    /// callback for each chunk of content received.
    ///
    /// # Arguments
    ///
    /// * `response` - The HTTP response from the Gemini API
    /// * `on_chunk` - A callback function that will be called for each text chunk received
    ///
    /// # Returns
    ///
    /// A result containing the final accumulated response or a streaming error
    pub async fn process_stream<F>(
        &mut self,
        response: Response,
        mut on_chunk: F,
    ) -> Result<StreamingResponse, StreamingError>
    where
        F: FnMut(&str) -> Result<(), StreamingError>,
    {
        self.metrics.request_start_time = Some(Instant::now());
        self.metrics.total_requests += 1;
        self.candidate_text.clear();

        // Get the response stream
        let mut stream = response.bytes_stream();

        let mut accumulated_response = StreamingResponse {
            candidates: Vec::new(),
            usage_metadata: None,
        };

        let mut _has_valid_content = false;
        let mut buffer = String::new();

        // Wait for the first chunk with a longer timeout
        let first_chunk_result = timeout(self.config.first_chunk_timeout, stream.next()).await;

        match first_chunk_result {
            Ok(Some(Ok(bytes))) => {
                self.metrics.first_chunk_time = Some(Instant::now());
                self.metrics.total_bytes += bytes.len();

                // Process the first chunk
                buffer.push_str(&String::from_utf8_lossy(&bytes));
                match self.process_buffer(&mut buffer, &mut accumulated_response, &mut on_chunk) {
                    Ok(valid) => _has_valid_content = valid,
                    Err(e) => return Err(e),
                }
            }
            Ok(Some(Err(e))) => {
                self.metrics.error_count += 1;
                return Err(StreamingError::NetworkError {
                    message: format!("Failed to read first chunk: {}", e),
                    is_retryable: true,
                });
            }
            Ok(None) => {
                return Err(StreamingError::StreamingError {
                    message: "Empty streaming response".to_string(),
                    partial_content: None,
                });
            }
            Err(_) => {
                self.metrics.error_count += 1;
                return Err(StreamingError::TimeoutError {
                    operation: "first_chunk".to_string(),
                    duration: self.config.first_chunk_timeout,
                });
            }
        }

        // Process subsequent chunks
        while let Some(result) = stream.next().await {
            match result {
                Ok(bytes) => {
                    self.metrics.total_bytes += bytes.len();

                    // Add to buffer
                    buffer.push_str(&String::from_utf8_lossy(&bytes));

                    // Process buffer
                    match self.process_buffer(&mut buffer, &mut accumulated_response, &mut on_chunk)
                    {
                        Ok(valid) => {
                            if valid {
                                _has_valid_content = true;
                            }
                        }
                        Err(e) => return Err(e),
                    }
                }
                Err(e) => {
                    self.metrics.error_count += 1;
                    return Err(StreamingError::NetworkError {
                        message: format!("Failed to read chunk: {}", e),
                        is_retryable: true,
                    });
                }
            }

            self.metrics.total_chunks += 1;
        }

        // Process any remaining data in the buffer
        if !buffer.is_empty() {
            match self.process_remaining_buffer(
                &mut buffer,
                &mut accumulated_response,
                &mut on_chunk,
            ) {
                Ok(valid) => {
                    if valid {
                        _has_valid_content = true;
                    }
                }
                Err(e) => return Err(e),
            }
        }

        if !_has_valid_content {
            return Err(StreamingError::ContentError {
                message: "No valid content received from streaming API".to_string(),
            });
        }

        Ok(accumulated_response)
    }

    /// Process the buffer and extract complete JSON objects
    fn process_buffer<F>(
        &mut self,
        buffer: &mut String,
        accumulated_response: &mut StreamingResponse,
        on_chunk: &mut F,
    ) -> Result<bool, StreamingError>
    where
        F: FnMut(&str) -> Result<(), StreamingError>,
    {
        let mut _has_valid_content = false;

        // First handle SSE-style payloads.
        let original_len = buffer.len();
        let mut scratch = buffer.clone();
        let events = stream::drain_sse_events(&mut scratch);
        let consumed = original_len.saturating_sub(scratch.len());
        if consumed > 0 {
            buffer.drain(..consumed);
        }

        for payload in events {
            match self.process_payload(&payload, accumulated_response, on_chunk) {
                Ok(valid) => {
                    if valid {
                        _has_valid_content = true;
                    }
                }
                Err(e) => return Err(e),
            }
        }

        // Handle newline-delimited JSON payloads (fallback for non-SSE streams).
        loop {
            if let Some(newline_idx) = buffer.find('\n') {
                let line = buffer[..newline_idx].replace('\r', "");
                buffer.drain(..=newline_idx);
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                match self.process_payload(trimmed, accumulated_response, on_chunk) {
                    Ok(valid) => {
                        if valid {
                            _has_valid_content = true;
                        }
                    }
                    Err(e) => return Err(e),
                }
            } else {
                break;
            }
        }

        Ok(_has_valid_content)
    }

    /// Process any remaining data in the buffer after streaming is complete
    fn process_remaining_buffer<F>(
        &mut self,
        buffer: &mut String,
        accumulated_response: &mut StreamingResponse,
        on_chunk: &mut F,
    ) -> Result<bool, StreamingError>
    where
        F: FnMut(&str) -> Result<(), StreamingError>,
    {
        let mut _has_valid_content = false;

        if buffer.trim().is_empty() {
            buffer.clear();
            return Ok(false);
        }

        // Ensure any trailing SSE payload is processed.
        if buffer.contains("data:") && !buffer.contains("\n\n") {
            buffer.push_str("\n\n");
        }

        match self.process_buffer(buffer, accumulated_response, on_chunk) {
            Ok(valid) => {
                if valid {
                    _has_valid_content = true;
                }
            }
            Err(e) => return Err(e),
        }

        if !buffer.trim().is_empty() {
            let remaining = buffer.trim().to_string();
            buffer.clear();
            match self.process_payload(&remaining, accumulated_response, on_chunk) {
                Ok(valid) => {
                    if valid {
                        _has_valid_content = true;
                    }
                }
                Err(e) => return Err(e),
            }
        } else {
            buffer.clear();
        }

        Ok(_has_valid_content)
    }

    /// Process a JSON payload extracted from the stream
    fn process_payload<F>(
        &mut self,
        payload: &str,
        accumulated_response: &mut StreamingResponse,
        on_chunk: &mut F,
    ) -> Result<bool, StreamingError>
    where
        F: FnMut(&str) -> Result<(), StreamingError>,
    {
        let trimmed = payload.trim();
        if trimmed.is_empty() || trimmed == "[DONE]" {
            return Ok(false);
        }

        let value: serde_json::Value =
            serde_json::from_str(trimmed).map_err(|err| StreamingError::ParseError {
                message: format!("Failed to parse streaming payload: {}", err),
                raw_response: trimmed.to_string(),
            })?;

        if let Some(error_value) = value.get("error") {
            let status_code = error_value
                .get("code")
                .and_then(|code| code.as_u64())
                .map(|code| code as u16)
                .unwrap_or(500);

            let message = error_value
                .get("message")
                .and_then(|msg| msg.as_str())
                .map(str::to_string)
                .unwrap_or_else(|| error_value.to_string());

            let is_retryable = matches!(status_code, 429 | 500 | 502 | 503 | 504);

            return Err(StreamingError::ApiError {
                status_code,
                message,
                is_retryable,
            });
        }

        let response: StreamingResponse =
            serde_json::from_value(value).map_err(|err| StreamingError::ParseError {
                message: format!("Failed to deserialize streaming payload: {}", err),
                raw_response: trimmed.to_string(),
            })?;

        let mut _has_valid_content = false;

        for candidate in &response.candidates {
            match self.process_candidate(candidate, on_chunk) {
                Ok(valid) => {
                    if valid {
                        _has_valid_content = true;
                    }
                }
                Err(e) => return Err(e),
            }
        }

        if !response.candidates.is_empty() {
            accumulated_response.candidates = response.candidates.clone();
        }

        if response.usage_metadata.is_some() {
            accumulated_response.usage_metadata = response.usage_metadata.clone();
        }

        Ok(_has_valid_content)
    }

    /// Process a streaming candidate and extract content
    fn process_candidate<F>(
        &mut self,
        candidate: &StreamingCandidate,
        on_chunk: &mut F,
    ) -> Result<bool, StreamingError>
    where
        F: FnMut(&str) -> Result<(), StreamingError>,
    {
        let mut has_valid_content = false;
        let index = candidate.index.unwrap_or(0);
        let entry = self.candidate_text.entry(index).or_default();
        let mut aggregated = entry.clone();

        for part in &candidate.content.parts {
            match part {
                Part::Text { text } => {
                    if text.is_empty() {
                        continue;
                    }

                    has_valid_content = true;

                    if text == &aggregated {
                        continue;
                    }

                    if text.starts_with(&aggregated) {
                        let delta = &text[aggregated.len()..];
                        if !delta.is_empty() {
                            on_chunk(delta)?;
                            aggregated.push_str(delta);
                        }
                        continue;
                    }

                    if aggregated.starts_with(text) {
                        aggregated = text.clone();
                        continue;
                    }

                    on_chunk(text)?;
                    aggregated.push_str(text);
                }
                Part::FunctionCall { .. } | Part::FunctionResponse { .. } => {
                    has_valid_content = true;
                }
            }
        }

        *entry = aggregated;

        Ok(has_valid_content)
    }

    /// Get current streaming metrics
    pub fn metrics(&self) -> &StreamingMetrics {
        &self.metrics
    }

    /// Reset streaming metrics
    pub fn reset_metrics(&mut self) {
        self.metrics = StreamingMetrics::default();
    }
}

impl Default for StreamingProcessor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gemini::models::{Content, Part};
    use crate::gemini::streaming::StreamingCandidate;

    fn base_candidate(text: &str) -> StreamingCandidate {
        StreamingCandidate {
            content: Content {
                role: "model".to_string(),
                parts: vec![Part::Text {
                    text: text.to_string(),
                }],
            },
            finish_reason: None,
            index: Some(0),
        }
    }

    #[test]
    fn process_candidate_emits_incremental_chunks_for_cumulative_text() {
        let mut processor = StreamingProcessor::new();
        let mut emitted: Vec<String> = Vec::new();
        let mut on_chunk = |chunk: &str| -> Result<(), StreamingError> {
            emitted.push(chunk.to_string());
            Ok(())
        };

        let mut candidate = base_candidate("Hel");
        assert!(
            processor
                .process_candidate(&candidate, &mut on_chunk)
                .expect("first chunk succeeds")
        );
        assert_eq!(emitted, vec!["Hel".to_string()]);
        emitted.clear();

        candidate.content.parts = vec![Part::Text {
            text: "Hello".to_string(),
        }];
        assert!(
            processor
                .process_candidate(&candidate, &mut on_chunk)
                .expect("second chunk succeeds")
        );
        assert_eq!(emitted, vec!["lo".to_string()]);
        emitted.clear();

        candidate.content.parts = vec![Part::Text {
            text: "Hello world".to_string(),
        }];
        assert!(
            processor
                .process_candidate(&candidate, &mut on_chunk)
                .expect("third chunk succeeds")
        );
        assert_eq!(emitted, vec![" world".to_string()]);
        assert_eq!(
            processor.candidate_text.get(&0).map(String::as_str),
            Some("Hello world"),
        );
    }

    #[test]
    fn process_candidate_handles_delta_only_payloads() {
        let mut processor = StreamingProcessor::new();
        let mut emitted: Vec<String> = Vec::new();
        let mut on_chunk = |chunk: &str| -> Result<(), StreamingError> {
            emitted.push(chunk.to_string());
            Ok(())
        };

        let first = base_candidate("Hello");
        assert!(
            processor
                .process_candidate(&first, &mut on_chunk)
                .expect("delta chunk succeeds")
        );

        let second = base_candidate(" world");
        assert!(
            processor
                .process_candidate(&second, &mut on_chunk)
                .expect("delta append succeeds")
        );

        assert_eq!(emitted, vec!["Hello".to_string(), " world".to_string()]);
        assert_eq!(
            processor.candidate_text.get(&0).map(String::as_str),
            Some("Hello world"),
        );
    }

    #[test]
    fn process_candidate_preserves_whitespace_chunks() {
        let mut processor = StreamingProcessor::new();
        let mut emitted: Vec<String> = Vec::new();
        let mut on_chunk = |chunk: &str| -> Result<(), StreamingError> {
            emitted.push(chunk.to_string());
            Ok(())
        };

        let whitespace = base_candidate(" ");
        assert!(
            processor
                .process_candidate(&whitespace, &mut on_chunk)
                .expect("whitespace chunk succeeds")
        );

        assert_eq!(emitted, vec![" ".to_string()]);
        assert_eq!(
            processor.candidate_text.get(&0).map(String::as_str),
            Some(" "),
        );
    }

    #[test]
    fn test_streaming_processor_creation() {
        let processor = StreamingProcessor::new();
        assert_eq!(processor.metrics().total_requests, 0);
    }

    #[test]
    fn test_streaming_processor_with_config() {
        use std::time::Duration;

        let config = StreamingConfig {
            chunk_timeout: Duration::from_secs(10),
            first_chunk_timeout: Duration::from_secs(30),
            buffer_size: 512,
        };

        let processor = StreamingProcessor::with_config(config);
        assert_eq!(processor.metrics().total_requests, 0);
    }

    #[test]
    fn test_streaming_config_default() {
        let config = StreamingConfig::default();
        assert_eq!(config.buffer_size, 1024);
    }

    #[test]
    fn process_buffer_emits_chunks_for_sse_payloads() {
        let mut processor = StreamingProcessor::new();
        let mut buffer = String::from(
            "data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"Hel\"}]}}]}\n\n",
        );
        let mut aggregated = StreamingResponse {
            candidates: Vec::new(),
            usage_metadata: None,
        };
        let mut collected = Vec::new();

        {
            let mut on_chunk = |chunk: &str| {
                collected.push(chunk.to_string());
                Ok(())
            };

            let has_valid = processor
                .process_buffer(&mut buffer, &mut aggregated, &mut on_chunk)
                .expect("processor should not error");

            assert!(has_valid);
        }

        assert!(buffer.is_empty());
        assert_eq!(collected, vec![String::from("Hel")]);
        assert_eq!(aggregated.candidates.len(), 1);
    }

    #[test]
    fn process_buffer_ignores_done_events() {
        let mut processor = StreamingProcessor::new();
        let mut buffer = String::from("data: [DONE]\n\n");
        let mut aggregated = StreamingResponse {
            candidates: Vec::new(),
            usage_metadata: None,
        };
        let mut collected = Vec::new();

        let mut on_chunk = |chunk: &str| {
            collected.push(chunk.to_string());
            Ok(())
        };

        let has_valid = processor
            .process_buffer(&mut buffer, &mut aggregated, &mut on_chunk)
            .expect("processor should not error");

        assert!(!has_valid);
        assert!(buffer.is_empty());
        assert!(collected.is_empty());
        assert!(aggregated.candidates.is_empty());
    }

    #[test]
    fn process_remaining_buffer_handles_plain_json() {
        let mut processor = StreamingProcessor::new();
        let mut buffer = String::from(
            "{\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"Complete\"}]}}]}",
        );
        let mut aggregated = StreamingResponse {
            candidates: Vec::new(),
            usage_metadata: None,
        };
        let mut collected = Vec::new();

        {
            let mut on_chunk = |chunk: &str| {
                collected.push(chunk.to_string());
                Ok(())
            };

            let has_valid = processor
                .process_remaining_buffer(&mut buffer, &mut aggregated, &mut on_chunk)
                .expect("processor should not error");

            assert!(has_valid);
        }

        assert!(buffer.is_empty());
        assert_eq!(collected, vec![String::from("Complete")]);
        assert_eq!(aggregated.candidates.len(), 1);
    }
}
