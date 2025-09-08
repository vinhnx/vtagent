//! Multi-agent orchestration tools

use crate::core::agent::multi_agent::*;
use crate::core::agent::orchestrator::{ContextSearchCriteria, OrchestratorAgent};
use anyhow::{Result, anyhow};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};

/// Multi-agent system tools for orchestrator control
pub struct MultiAgentTools {
    /// Reference to the orchestrator agent
    orchestrator: Arc<Mutex<OrchestratorAgent>>,
}

impl MultiAgentTools {
    /// Create new multi-agent tools
    pub fn new(orchestrator: Arc<Mutex<OrchestratorAgent>>) -> Self {
        Self { orchestrator }
    }

    /// Create a new task for delegation
    pub async fn task_create(&self, params: Value) -> Result<Value> {
        let agent_type = params["agent_type"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing agent_type parameter"))?;

        let agent_type = match agent_type {
            "explorer" => AgentType::Explorer,
            "coder" => AgentType::Coder,
            _ => return Err(anyhow!("Invalid agent_type: {}", agent_type)),
        };

        let title = params["title"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing title parameter"))?
            .to_string();

        let description = params["description"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing description parameter"))?
            .to_string();

        let context_refs = if let Some(refs) = params["context_refs"].as_array() {
            refs.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect()
        } else {
            Vec::new()
        };

        let context_bootstrap = if let Some(bootstrap) = params["context_bootstrap"].as_array() {
            bootstrap
                .iter()
                .filter_map(|item| {
                    let path = item["path"].as_str()?;
                    let reason = item["reason"].as_str()?;
                    Some(ContextBootstrap {
                        path: path.to_string(),
                        reason: reason.to_string(),
                    })
                })
                .collect()
        } else {
            Vec::new()
        };

        let priority = if let Some(priority_str) = params["priority"].as_str() {
            match priority_str {
                "low" => TaskPriority::Low,
                "normal" => TaskPriority::Normal,
                "high" => TaskPriority::High,
                "critical" => TaskPriority::Critical,
                _ => TaskPriority::Normal,
            }
        } else {
            TaskPriority::Normal
        };

        let task_id = {
            let orchestrator = self.orchestrator.lock().unwrap();
            format!(
                "task_{}_{}",
                match agent_type {
                    AgentType::Explorer => "explorer",
                    AgentType::Coder => "coder",
                    AgentType::Orchestrator => "orchestrator",
                    AgentType::Single => "single",
                },
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            )
        };
        Ok(json!({
            "task_id": task_id,
            "status": "created",
            "agent_type": agent_type.to_string()
        }))
    }

    /// Launch a subagent to execute a task
    pub async fn launch_subagent(&self, params: Value) -> Result<Value> {
        let task_id = params["task_id"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing task_id parameter"))?;

        // Actually delegate to the orchestrator to launch the subagent
        let results = {
            let mut orchestrator = self.orchestrator.lock().unwrap();
            orchestrator.launch_subagent(task_id).await?
        };

        Ok(json!({
            "ok": true,
            "task_id": task_id,
            "status": "completed",
            "results": {
                "created_contexts": results.created_contexts,
                "modified_files": results.modified_files,
                "executed_commands": results.executed_commands,
                "summary": results.summary
            }
        }))
    }

    /// Add a context to the context store
    pub async fn add_context(&self, params: Value) -> Result<Value> {
        let id = params["id"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing id parameter"))?
            .to_string();

        let content = params["content"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing content parameter"))?
            .to_string();

        let context_type = if let Some(type_str) = params["type"].as_str() {
            match type_str {
                "environmental" => ContextType::Environmental,
                "diagnostic" => ContextType::Diagnostic,
                "implementation" => ContextType::Implementation,
                "analysis" => ContextType::Analysis,
                "strategic" => ContextType::Strategic,
                "verification" => ContextType::Verification,
                _ => ContextType::General,
            }
        } else {
            ContextType::General
        };

        let tags = if let Some(tags_array) = params["tags"].as_array() {
            tags_array
                .iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect()
        } else {
            Vec::new()
        };

        let related_files = if let Some(files_array) = params["related_files"].as_array() {
            files_array
                .iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect()
        } else {
            Vec::new()
        };

        let context = ContextItem {
            id: id.clone(),
            content,
            created_by: AgentType::Orchestrator,
            session_id: String::new(), // Will be set by the orchestrator
            created_at: std::time::SystemTime::now(),
            tags,
            context_type,
            related_files,
        };

        {
            let orchestrator = self.orchestrator.lock().unwrap();
            // Actually store the context
            orchestrator.add_context(context)?;
        }

        Ok(json!({
            "context_id": id,
            "status": "added"
        }))
    }

    /// Search the context store
    pub async fn context_search(&self, params: Value) -> Result<Value> {
        let query = params["query"].as_str();
        let tags = if let Some(tags_array) = params["tags"].as_array() {
            Some(
                tags_array
                    .iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect(),
            )
        } else {
            None
        };

        let context_type = if let Some(type_str) = params["type"].as_str() {
            match type_str {
                "environmental" => Some(ContextType::Environmental),
                "diagnostic" => Some(ContextType::Diagnostic),
                "implementation" => Some(ContextType::Implementation),
                "analysis" => Some(ContextType::Analysis),
                "strategic" => Some(ContextType::Strategic),
                "verification" => Some(ContextType::Verification),
                "general" => Some(ContextType::General),
                _ => None,
            }
        } else {
            None
        };

        let agent_type = if let Some(agent_str) = params["agent_type"].as_str() {
            match agent_str {
                "orchestrator" => Some(AgentType::Orchestrator),
                "explorer" => Some(AgentType::Explorer),
                "coder" => Some(AgentType::Coder),
                _ => None,
            }
        } else {
            None
        };

        let related_files = if let Some(files_array) = params["related_files"].as_array() {
            Some(
                files_array
                    .iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect(),
            )
        } else {
            None
        };

        let criteria = ContextSearchCriteria {
            tags,
            context_type,
            created_by: agent_type,
            related_files,
        };

        let contexts: Vec<crate::core::agent::multi_agent::ContextItem> = {
            let orchestrator = self.orchestrator.lock().unwrap();
            // Actually search the contexts
            orchestrator.search_contexts(criteria)?
        };

        let results: Vec<Value> = contexts
            .into_iter()
            .map(|ctx| {
                json!({
                    "id": ctx.id,
                    "content": ctx.content,
                    "type": match ctx.context_type {
                        ContextType::Environmental => "environmental",
                        ContextType::Diagnostic => "diagnostic",
                        ContextType::Implementation => "implementation",
                        ContextType::Analysis => "analysis",
                        ContextType::Strategic => "strategic",
                        ContextType::Verification => "verification",
                        ContextType::General => "general",
                    },
                    "created_by": ctx.created_by.to_string(),
                    "tags": ctx.tags,
                    "related_files": ctx.related_files,
                    "created_at": ctx.created_at
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs()
                })
            })
            .collect();

        Ok(json!({
            "query": query,
            "results": results,
            "count": results.len()
        }))
    }

    /// Check task status
    pub async fn task_status(&self, params: Value) -> Result<Value> {
        let task_id = params["task_id"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing task_id parameter"))?;

        let status = {
            let orchestrator = self.orchestrator.lock().unwrap();
            // Actually get the real task status
            orchestrator
                .get_task_status(task_id)?
                .map(|s| format!("{:?}", s))
                .unwrap_or_else(|| "not_found".to_string())
        };

        Ok(json!({
            "task_id": task_id,
            "status": status
        }))
    }

    /// Get all pending tasks
    pub async fn get_pending_tasks(&self, _params: Value) -> Result<Value> {
        let tasks: Vec<crate::core::agent::multi_agent::Task> = {
            let orchestrator = self.orchestrator.lock().unwrap();
            // Actually get the pending tasks from the orchestrator
            orchestrator.get_pending_tasks()?
        };

        let task_list: Vec<Value> = tasks
            .into_iter()
            .map(|task| {
                json!({
                    "id": task.id,
                    "agent_type": task.agent_type.to_string(),
                    "title": task.title,
                    "description": task.description,
                    "priority": match task.priority {
                        TaskPriority::Low => "low",
                        TaskPriority::Normal => "normal",
                        TaskPriority::High => "high",
                        TaskPriority::Critical => "critical",
                    },
                    "status": match task.status {
                        TaskStatus::Pending => "pending",
                        TaskStatus::InProgress => "in_progress",
                        TaskStatus::Completed => "completed",
                        TaskStatus::Failed => "failed",
                        TaskStatus::Cancelled => "cancelled",
                    },
                    "created_at": task.created_at
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs()
                })
            })
            .collect();

        Ok(json!({
            "pending_tasks": task_list,
            "count": task_list.len()
        }))
    }

    /// Finish the orchestration task
    pub async fn finish(&self, params: Value) -> Result<Value> {
        let message = params["message"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing message parameter"))?;

        let summary = params["summary"].as_str().unwrap_or("");

        Ok(json!({
            "status": "finished",
            "message": message,
            "summary": summary,
            "completion_time": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        }))
    }
}

/// Get available multi-agent tools for the orchestrator
pub fn get_multi_agent_function_declarations() -> Vec<serde_json::Value> {
    vec![
        json!({
            "name": "task_create",
            "description": "Create a new task for a subagent to execute",
            "parameters": {
                "type": "object",
                "properties": {
                    "agent_type": {
                        "type": "string",
                        "enum": ["explorer", "coder"],
                        "description": "Type of agent to execute the task"
                    },
                    "title": {
                        "type": "string",
                        "description": "Concise task title (max 7 words)"
                    },
                    "description": {
                        "type": "string",
                        "description": "Detailed instructions for the subagent"
                    },
                    "context_refs": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "List of context IDs to inject into subagent's initial state"
                    },
                    "context_bootstrap": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "path": {"type": "string"},
                                "reason": {"type": "string"}
                            },
                            "required": ["path", "reason"]
                        },
                        "description": "Files/directories to bootstrap into subagent context"
                    },
                    "priority": {
                        "type": "string",
                        "enum": ["low", "normal", "high", "critical"],
                        "description": "Task priority level"
                    }
                },
                "required": ["agent_type", "title", "description"]
            }
        }),
        json!({
            "name": "launch_subagent",
            "description": "Launch a subagent to execute a previously created task",
            "parameters": {
                "type": "object",
                "properties": {
                    "task_id": {
                        "type": "string",
                        "description": "The unique identifier of the task to execute"
                    }
                },
                "required": ["task_id"]
            }
        }),
        json!({
            "name": "add_context",
            "description": "Add synthesized context to the shared context store",
            "parameters": {
                "type": "object",
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "Unique, descriptive identifier using snake_case"
                    },
                    "content": {
                        "type": "string",
                        "description": "The actual context content"
                    },
                    "type": {
                        "type": "string",
                        "enum": ["environmental", "diagnostic", "implementation", "analysis", "strategic", "verification", "general"],
                        "description": "Context type classification"
                    },
                    "tags": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Tags for organization and search"
                    },
                    "related_files": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "File paths this context relates to"
                    }
                },
                "required": ["id", "content"]
            }
        }),
        json!({
            "name": "context_search",
            "description": "Search the context store for existing knowledge",
            "parameters": {
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query or criteria"
                    },
                    "tags": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Filter by specific tags"
                    },
                    "type": {
                        "type": "string",
                        "enum": ["environmental", "diagnostic", "implementation", "analysis", "strategic", "verification", "general"],
                        "description": "Filter by context type"
                    },
                    "agent_type": {
                        "type": "string",
                        "enum": ["orchestrator", "explorer", "coder"],
                        "description": "Filter by creator agent type"
                    },
                    "related_files": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Filter by related file paths"
                    }
                }
            }
        }),
        json!({
            "name": "task_status",
            "description": "Check the status of a task",
            "parameters": {
                "type": "object",
                "properties": {
                    "task_id": {
                        "type": "string",
                        "description": "The task identifier to check"
                    }
                },
                "required": ["task_id"]
            }
        }),
        json!({
            "name": "get_pending_tasks",
            "description": "Get all pending tasks in priority order",
            "parameters": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "finish",
            "description": "Signal completion of the entire high-level task",
            "parameters": {
                "type": "object",
                "properties": {
                    "message": {
                        "type": "string",
                        "description": "One sentence confirming completion"
                    },
                    "summary": {
                        "type": "string",
                        "description": "Brief summary of work accomplished"
                    }
                },
                "required": ["message"]
            }
        }),
    ]
}

/// Execute a multi-agent tool call
pub async fn execute_multi_agent_tool(
    tool_name: &str,
    params: Value,
    tools: &MultiAgentTools,
) -> Result<Value> {
    match tool_name {
        "task_create" => tools.task_create(params).await,
        "launch_subagent" => tools.launch_subagent(params).await,
        "add_context" => tools.add_context(params).await,
        "context_search" => tools.context_search(params).await,
        "task_status" => tools.task_status(params).await,
        "get_pending_tasks" => tools.get_pending_tasks(params).await,
        "finish" => tools.finish(params).await,
        _ => Err(anyhow!("Unknown multi-agent tool: {}", tool_name)),
    }
}
