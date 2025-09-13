//! Simple markdown-based storage system
//!
//! This module provides simple storage capabilities using markdown files
//! instead of complex databases. Perfect for storing project metadata,
//! search results, and other simple data structures.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Simple markdown storage manager
#[derive(Clone)]
pub struct MarkdownStorage {
    storage_dir: PathBuf,
}

impl MarkdownStorage {
    /// Create a new markdown storage instance
    pub fn new(storage_dir: PathBuf) -> Self {
        Self { storage_dir }
    }

    /// Initialize storage directory
    pub fn init(&self) -> Result<()> {
        fs::create_dir_all(&self.storage_dir)?;
        Ok(())
    }

    /// Store data as markdown
    pub fn store<T: Serialize>(&self, key: &str, data: &T, title: &str) -> Result<()> {
        let file_path = self.storage_dir.join(format!("{}.md", key));
        let markdown = self.serialize_to_markdown(data, title)?;
        fs::write(file_path, markdown)?;
        Ok(())
    }

    /// Load data from markdown
    pub fn load<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<T> {
        let file_path = self.storage_dir.join(format!("{}.md", key));
        let content = fs::read_to_string(file_path)?;
        self.deserialize_from_markdown(&content)
    }

    /// List all stored items
    pub fn list(&self) -> Result<Vec<String>> {
        let mut items = Vec::new();

        for entry in fs::read_dir(&self.storage_dir)? {
            let entry = entry?;
            if let Some(file_name) = entry.path().file_stem() {
                if let Some(name) = file_name.to_str() {
                    items.push(name.to_string());
                }
            }
        }

        Ok(items)
    }

    /// Delete stored item
    pub fn delete(&self, key: &str) -> Result<()> {
        let file_path = self.storage_dir.join(format!("{}.md", key));
        if file_path.exists() {
            fs::remove_file(file_path)?;
        }
        Ok(())
    }

    /// Check if item exists
    pub fn exists(&self, key: &str) -> bool {
        let file_path = self.storage_dir.join(format!("{}.md", key));
        file_path.exists()
    }

    // Helper methods

    fn serialize_to_markdown<T: Serialize>(&self, data: &T, title: &str) -> Result<String> {
        let json = serde_json::to_string_pretty(data)?;
        let yaml = serde_yaml::to_string(data)?;

        let markdown = format!(
            "# {}\n\n\
            ## JSON\n\n\
            ```json\n\
            {}\n\
            ```\n\n\
            ## YAML\n\n\
            ```yaml\n\
            {}\n\
            ```\n\n\
            ## Raw Data\n\n\
            {}\n",
            title,
            json,
            yaml,
            self.format_raw_data(data)
        );

        Ok(markdown)
    }

    fn deserialize_from_markdown<T: for<'de> Deserialize<'de>>(&self, content: &str) -> Result<T> {
        // Try to extract JSON from markdown code blocks
        if let Some(json_block) = self.extract_code_block(content, "json") {
            return serde_json::from_str(json_block).context("Failed to parse JSON from markdown");
        }

        // Try to extract YAML from markdown code blocks
        if let Some(yaml_block) = self.extract_code_block(content, "yaml") {
            return serde_yaml::from_str(yaml_block).context("Failed to parse YAML from markdown");
        }

        Err(anyhow::anyhow!("No valid JSON or YAML found in markdown"))
    }

    fn extract_code_block<'a>(&self, content: &'a str, language: &str) -> Option<&'a str> {
        let start_pattern = format!("```{}", language);
        let end_pattern = "```";

        if let Some(start_idx) = content.find(&start_pattern) {
            let code_start = start_idx + start_pattern.len();
            if let Some(end_idx) = content[code_start..].find(end_pattern) {
                let code_end = code_start + end_idx;
                return Some(content[code_start..code_end].trim());
            }
        }

        None
    }

    fn format_raw_data<T: Serialize>(&self, data: &T) -> String {
        match serde_json::to_value(data) {
            Ok(serde_json::Value::Object(map)) => {
                let mut lines = Vec::new();
                for (key, value) in map {
                    lines.push(format!("- **{}**: {}", key, self.format_value(&value)));
                }
                lines.join("\n")
            }
            _ => "Complex data structure".to_string(),
        }
    }

    fn format_value(&self, value: &serde_json::Value) -> String {
        match value {
            serde_json::Value::String(s) => format!("\"{}\"", s),
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::Bool(b) => b.to_string(),
            serde_json::Value::Array(arr) => format!("[{} items]", arr.len()),
            serde_json::Value::Object(obj) => format!("{{{} fields}}", obj.len()),
            serde_json::Value::Null => "null".to_string(),
        }
    }
}

/// Simple key-value storage using markdown
pub struct SimpleKVStorage {
    storage: MarkdownStorage,
}

impl SimpleKVStorage {
    pub fn new(storage_dir: PathBuf) -> Self {
        Self {
            storage: MarkdownStorage::new(storage_dir),
        }
    }

    pub fn init(&self) -> Result<()> {
        self.storage.init()
    }

    pub fn put(&self, key: &str, value: &str) -> Result<()> {
        let data = HashMap::from([("value".to_string(), value.to_string())]);
        self.storage.store(key, &data, &format!("Key-Value: {}", key))
    }

    pub fn get(&self, key: &str) -> Result<String> {
        let data: HashMap<String, String> = self.storage.load(key)?;
        data.get("value")
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Value not found for key: {}", key))
    }

    pub fn delete(&self, key: &str) -> Result<()> {
        self.storage.delete(key)
    }

    pub fn list_keys(&self) -> Result<Vec<String>> {
        self.storage.list()
    }
}

/// Simple project metadata storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectData {
    pub name: String,
    pub description: Option<String>,
    pub version: String,
    pub tags: Vec<String>,
    pub metadata: HashMap<String, String>,
}

impl ProjectData {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            description: None,
            version: "1.0.0".to_string(),
            tags: vec![],
            metadata: HashMap::new(),
        }
    }
}

/// Project storage using markdown
#[derive(Clone)]
pub struct ProjectStorage {
    storage: MarkdownStorage,
}

impl ProjectStorage {
    pub fn new(storage_dir: PathBuf) -> Self {
        Self {
            storage: MarkdownStorage::new(storage_dir),
        }
    }

    pub fn init(&self) -> Result<()> {
        self.storage.init()
    }

    pub fn save_project(&self, project: &ProjectData) -> Result<()> {
        self.storage.store(
            &project.name,
            project,
            &format!("Project: {}", project.name)
        )
    }

    pub fn load_project(&self, name: &str) -> Result<ProjectData> {
        self.storage.load(name)
    }

    pub fn list_projects(&self) -> Result<Vec<String>> {
        self.storage.list()
    }

    pub fn delete_project(&self, name: &str) -> Result<()> {
        self.storage.delete(name)
    }
}