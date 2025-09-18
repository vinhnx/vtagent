use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::config::constants::defaults;

/// Tools configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ToolsConfig {
    /// Default policy for tools not explicitly listed
    #[serde(default = "default_tool_policy")]
    pub default_policy: ToolPolicy,

    /// Specific tool policies
    #[serde(default)]
    pub policies: IndexMap<String, ToolPolicy>,

    /// Maximum inner tool-call loops per user turn
    ///
    /// Prevents infinite tool-calling cycles in interactive chat. This limits how
    /// many back-and-forths the agent will perform executing tools and
    /// re-asking the model before returning a final answer.
    ///
    #[serde(default = "default_max_tool_loops")]
    pub max_tool_loops: usize,
}

impl Default for ToolsConfig {
    fn default() -> Self {
        let mut policies = IndexMap::new();
        policies.insert("run_terminal_cmd".to_string(), ToolPolicy::Allow);
        policies.insert("bash".to_string(), ToolPolicy::Allow);
        Self {
            default_policy: default_tool_policy(),
            policies,
            max_tool_loops: default_max_tool_loops(),
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

fn default_max_tool_loops() -> usize {
    defaults::DEFAULT_MAX_TOOL_LOOPS
}
