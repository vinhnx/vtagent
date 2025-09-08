pub mod request;
pub mod response;

pub use request::GenerateContentRequest;
pub use response::{Candidate, GenerateContentResponse};

use serde::{Deserialize, Serialize};
use serde_json::Value;

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
        function_call: crate::gemini::function_calling::FunctionCall,
    },
    #[serde(rename_all = "camelCase")]
    FunctionResponse {
        function_response: crate::gemini::function_calling::FunctionResponse,
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
pub struct Tool {
    #[serde(rename = "functionDeclarations")]
    pub function_declarations: Vec<FunctionDeclaration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDeclaration {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolConfig {
    #[serde(rename = "functionCallingConfig")]
    pub function_calling_config: crate::gemini::function_calling::FunctionCallingConfig,
}

impl ToolConfig {
    pub fn auto() -> Self {
        Self {
            function_calling_config: crate::gemini::function_calling::FunctionCallingConfig::auto(),
        }
    }
}
