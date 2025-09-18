use anyhow::Error;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionError {
    pub tool_name: String,
    pub error_type: ToolErrorType,
    pub message: String,
    pub is_recoverable: bool,
    pub recovery_suggestions: Vec<String>,
    pub original_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolErrorType {
    InvalidParameters,
    ToolNotFound,
    PermissionDenied,
    ResourceNotFound,
    NetworkError,
    Timeout,
    ExecutionError,
    PolicyViolation,
}

impl ToolExecutionError {
    pub fn new(tool_name: String, error_type: ToolErrorType, message: String) -> Self {
        let (is_recoverable, recovery_suggestions) = generate_recovery_info(&error_type);

        Self {
            tool_name,
            error_type,
            message,
            is_recoverable,
            recovery_suggestions,
            original_error: None,
        }
    }

    pub fn with_original_error(
        tool_name: String,
        error_type: ToolErrorType,
        message: String,
        original_error: String,
    ) -> Self {
        let mut error = Self::new(tool_name, error_type, message);
        error.original_error = Some(original_error);
        error
    }

    pub fn to_json_value(&self) -> Value {
        json!({
            "error": {
                "tool_name": self.tool_name,
                "error_type": format!("{:?}", self.error_type),
                "message": self.message,
                "is_recoverable": self.is_recoverable,
                "recovery_suggestions": self.recovery_suggestions,
                "original_error": self.original_error,
            }
        })
    }
}

pub fn classify_error(error: &Error) -> ToolErrorType {
    let error_msg = error.to_string().to_lowercase();

    if error_msg.contains("permission") || error_msg.contains("access denied") {
        ToolErrorType::PermissionDenied
    } else if error_msg.contains("not found") || error_msg.contains("no such file") {
        ToolErrorType::ResourceNotFound
    } else if error_msg.contains("timeout") || error_msg.contains("timed out") {
        ToolErrorType::Timeout
    } else if error_msg.contains("network") || error_msg.contains("connection") {
        ToolErrorType::NetworkError
    } else if error_msg.contains("invalid") || error_msg.contains("malformed") {
        ToolErrorType::InvalidParameters
    } else if error_msg.contains("policy") || error_msg.contains("denied") {
        ToolErrorType::PolicyViolation
    } else {
        ToolErrorType::ExecutionError
    }
}

fn generate_recovery_info(error_type: &ToolErrorType) -> (bool, Vec<String>) {
    match error_type {
        ToolErrorType::InvalidParameters => (
            true,
            vec![
                "Check parameter names and types against the tool schema".to_string(),
                "Ensure required parameters are provided".to_string(),
                "Verify parameter values are within acceptable ranges".to_string(),
            ],
        ),
        ToolErrorType::ToolNotFound => (
            false,
            vec![
                "Verify the tool name is spelled correctly".to_string(),
                "Check if the tool is available in the current context".to_string(),
                "Contact administrator if tool should be available".to_string(),
            ],
        ),
        ToolErrorType::PermissionDenied => (
            true,
            vec![
                "Check file permissions and access rights".to_string(),
                "Ensure workspace boundaries are respected".to_string(),
                "Try running with appropriate permissions".to_string(),
            ],
        ),
        ToolErrorType::ResourceNotFound => (
            true,
            vec![
                "Verify file paths and resource locations".to_string(),
                "Check if files exist and are accessible".to_string(),
                "Use list_dir to explore available resources".to_string(),
            ],
        ),
        ToolErrorType::NetworkError => (
            true,
            vec![
                "Check network connectivity".to_string(),
                "Retry the operation after a brief delay".to_string(),
                "Verify external service availability".to_string(),
            ],
        ),
        ToolErrorType::Timeout => (
            true,
            vec![
                "Increase timeout values if appropriate".to_string(),
                "Break large operations into smaller chunks".to_string(),
                "Check system resources and performance".to_string(),
            ],
        ),
        ToolErrorType::ExecutionError => (
            false,
            vec![
                "Review error details for specific issues".to_string(),
                "Check tool documentation for known limitations".to_string(),
                "Report the issue if it appears to be a bug".to_string(),
            ],
        ),
        ToolErrorType::PolicyViolation => (
            false,
            vec![
                "Review workspace policies and restrictions".to_string(),
                "Contact administrator for policy changes".to_string(),
                "Use alternative tools that comply with policies".to_string(),
            ],
        ),
    }
}
