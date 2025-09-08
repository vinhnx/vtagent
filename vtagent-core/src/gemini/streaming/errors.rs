use std::time::Duration;

/// Streaming error types for better error classification and handling
#[derive(Debug, Clone)]
pub enum StreamingError {
    /// Network-related errors (connection, timeout, DNS, etc.)
    NetworkError { message: String, is_retryable: bool },
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
    ContentError { message: String },
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
            StreamingError::ApiError {
                status_code,
                message,
                ..
            } => {
                write!(f, "API error ({}): {}", status_code, message)
            }
            StreamingError::ParseError { message, .. } => {
                write!(f, "Parse error: {}", message)
            }
            StreamingError::TimeoutError {
                operation,
                duration,
            } => {
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
