use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Tools configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ToolsConfig {
    /// Default policy for tools not explicitly listed
    #[serde(default = "default_tool_policy")]
    pub default_policy: ToolPolicy,

    /// Specific tool policies
    #[serde(default)]
    pub policies: IndexMap<String, ToolPolicy>,
}

impl Default for ToolsConfig {
    fn default() -> Self {
        Self {
            default_policy: default_tool_policy(),
            policies: IndexMap::new(),
        }
    }
}

/// Tool execution policy
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ToolPolicy {
    /// Allow execution without confirmation
    Allow,
    /// Prompt user for confirmation
    Prompt,
    /// Deny execution
    Deny,
}

fn default_tool_policy() -> ToolPolicy {
    ToolPolicy::Prompt
}
