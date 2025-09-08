#!/usr/bin/env rust-script
//! Test script for the consolidated search functionality

use serde_json::json;
use std::path::PathBuf;
use vtagent_core::tools::ToolRegistry;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing consolidated rp_search functionality...");
    
    let root = PathBuf::from(".");
    let registry = ToolRegistry::new(root).await?;
    
    // Test 1: Basic exact search
    println!("\n1. Testing exact search mode:");
    let result = registry.execute_tool(
        "rp_search",
        json!({
            "pattern": "fn main",
            "path": "src",
            "mode": "exact",
            "max_results": 5
        })
    ).await?;
    println!("Exact search result: {}", serde_json::to_string_pretty(&result)?);
    
    // Test 2: Fuzzy search
    println!("\n2. Testing fuzzy search mode:");
    let result = registry.execute_tool(
        "rp_search", 
        json!({
            "pattern": "main",
            "path": "src",
            "mode": "fuzzy",
            "max_results": 3
        })
    ).await?;
    println!("Fuzzy search result: {}", serde_json::to_string_pretty(&result)?);
    
    // Test 3: Multi-pattern search with AND logic
    println!("\n3. Testing multi-pattern search (AND):");
    let result = registry.execute_tool(
        "rp_search",
        json!({
            "pattern": "dummy", // Required but not used in multi mode
            "path": "src", 
            "mode": "multi",
            "patterns": ["use", "fn"],
            "logic": "AND",
            "max_results": 3
        })
    ).await?;
    println!("Multi-pattern AND result: {}", serde_json::to_string_pretty(&result)?);
    
    // Test 4: Multi-pattern search with OR logic
    println!("\n4. Testing multi-pattern search (OR):");
    let result = registry.execute_tool(
        "rp_search",
        json!({
            "pattern": "dummy", // Required but not used in multi mode
            "path": "src",
            "mode": "multi", 
            "patterns": ["async", "await"],
            "logic": "OR",
            "max_results": 3
        })
    ).await?;
    println!("Multi-pattern OR result: {}", serde_json::to_string_pretty(&result)?);
    
    println!("\n✅ All consolidated search tests completed successfully!");
    Ok(())
}
