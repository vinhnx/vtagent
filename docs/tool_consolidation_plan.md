# VTAgent Tool Consolidation Action Plan

## Overview
This document provides specific implementation steps to address redundancy and integration issues identified in the tool compatibility assessment.

## Phase 1: Security Tool Implementation (Priority: Critical)

### 1.1 Complete `security_scan` Implementation
**Current State**: Placeholder returning empty results
**Required Implementation**:

```rust
async fn security_scan(&self, args: Value) -> Result<Value> {
    // Implement actual SAST scanning using:
    // 1. AST-grep patterns for structural vulnerabilities
    // 2. Regex patterns for common security anti-patterns
    // 3. Dependency analysis integration
    
    let patterns = SecurityPatterns::load_default();
    let mut findings = Vec::new();
    
    for pattern in patterns {
        let matches = self.ast_grep_search(pattern.to_ast_query()).await?;
        findings.extend(SecurityFinding::from_matches(matches, pattern));
    }
    
    Ok(SecurityReport::new(findings).to_json())
}
```

### 1.2 Implement `generate_security_patch`
**Dependencies**: Requires git integration and patch generation logic
**Implementation Strategy**:
- Use `git diff` for patch generation
- Implement template-based fixes for common vulnerabilities
- Add validation before patch application

### 1.3 Complete `analyze_dependency_vulnerabilities`
**Required Integration**:
- Parse `Cargo.toml`, `package.json`, etc.
- Query vulnerability databases (OSV, GitHub Advisory)
- Implement caching for vulnerability data

## Phase 2: Search Tool Consolidation (Priority: High)

### 2.1 Enhanced `rp_search` Interface
**New Parameters**:
```rust
#[derive(Debug, Deserialize)]
struct EnhancedRgInput {
    pattern: String,
    path: String,
    
    // Existing parameters
    case_sensitive: Option<bool>,
    literal: Option<bool>,
    
    // New consolidated parameters
    search_mode: Option<SearchMode>, // "regex", "fuzzy", "multi_pattern"
    patterns: Option<Vec<String>>,   // For multi-pattern search
    logic: Option<LogicMode>,        // "AND", "OR", "NOT"
    similarity_threshold: Option<f64>, // For fuzzy search
    
    // File filtering
    glob_pattern: Option<String>,
    file_type: Option<String>,
    include_hidden: Option<bool>,
    
    // Output control
    context_lines: Option<usize>,
    max_results: Option<usize>,
}

#[derive(Debug, Deserialize)]
enum SearchMode {
    Regex,
    Literal, 
    Fuzzy,
    MultiPattern,
    Similarity,
}

#[derive(Debug, Deserialize)]
enum LogicMode {
    And,
    Or,
    Not,
}
```

### 2.2 Implementation Strategy
```rust
async fn rp_search_enhanced(&self, input: &EnhancedRgInput) -> Result<Value> {
    match input.search_mode.unwrap_or(SearchMode::Regex) {
        SearchMode::Regex | SearchMode::Literal => {
            self.rg_search_with_ripgrep(input).await
        },
        SearchMode::Fuzzy => {
            self.fuzzy_search_impl(input).await
        },
        SearchMode::MultiPattern => {
            self.multi_pattern_search_impl(input).await
        },
        SearchMode::Similarity => {
            self.similarity_search_impl(input).await
        },
    }
}
```

### 2.3 Deprecation Strategy
**Tools to Deprecate**:
- `code_search` → Add alias pointing to `rp_search`
- `codebase_search` → Add alias pointing to `rp_search`
- `fuzzy_search` → Merge into `rp_search` with `search_mode: "fuzzy"`
- `multi_pattern_search` → Merge into `rp_search` with `search_mode: "multi_pattern"`
- `similarity_search` → Merge into `rp_search` with `search_mode: "similarity"`

**Backward Compatibility**:
```rust
async fn code_search(&self, args: Value) -> Result<Value> {
    // Convert old format to new enhanced format
    let legacy_input: RgInput = serde_json::from_value(args)?;
    let enhanced_input = EnhancedRgInput::from_legacy(legacy_input);
    self.rp_search_enhanced(&enhanced_input).await
}
```

## Phase 3: File Discovery Consolidation (Priority: Medium)

### 3.1 Enhanced `list_files` Interface
**New Parameters**:
```rust
#[derive(Debug, Deserialize)]
struct EnhancedListInput {
    path: String,
    
    // Existing parameters
    max_items: usize,
    include_hidden: bool,
    
    // New consolidated parameters
    recursive: Option<bool>,           // Enable recursive search
    name_pattern: Option<String>,      // File name pattern matching
    content_pattern: Option<String>,   // Search within file content
    file_extensions: Option<Vec<String>>, // Filter by extensions
    
    // AST filtering
    ast_grep_pattern: Option<String>,
}
```

### 3.2 Tools to Consolidate
- `recursive_file_search` → Merge into `list_files` with `recursive: true`
- `find_file_by_name` → Merge into `list_files` with `name_pattern`
- Keep `search_files_with_content` as separate tool (unique functionality)

## Phase 4: Command Execution Unification (Priority: Low)

### 4.1 Unified Command Interface
```rust
#[derive(Debug, Deserialize)]
struct UnifiedCommandInput {
    command: Vec<String>,
    working_dir: Option<String>,
    timeout_secs: Option<u64>,
    
    // Execution mode
    mode: CommandMode, // "basic", "pty", "streaming"
    
    // PTY-specific options
    rows: Option<u16>,
    cols: Option<u16>,
}

#[derive(Debug, Deserialize)]
enum CommandMode {
    Basic,     // Current run_terminal_cmd
    Pty,       // Current run_pty_cmd  
    Streaming, // Current run_pty_cmd_streaming
}
```

### 4.2 Implementation
```rust
async fn run_command(&self, args: Value) -> Result<Value> {
    let input: UnifiedCommandInput = serde_json::from_value(args)?;
    
    match input.mode {
        CommandMode::Basic => self.run_terminal_command_impl(&input).await,
        CommandMode::Pty => self.run_pty_command_impl(&input).await,
        CommandMode::Streaming => self.run_pty_streaming_impl(&input).await,
    }
}
```

## Implementation Timeline

### Week 1: Security Tools
- [ ] Implement `security_scan` core functionality
- [ ] Add vulnerability pattern database
- [ ] Complete `analyze_dependency_vulnerabilities`
- [ ] Add proper error handling and validation

### Week 2: Search Consolidation - Phase 1
- [ ] Implement `EnhancedRgInput` structure
- [ ] Add fuzzy search logic to `rp_search`
- [ ] Add multi-pattern search logic to `rp_search`
- [ ] Create backward compatibility aliases

### Week 3: Search Consolidation - Phase 2  
- [ ] Add similarity search logic to `rp_search`
- [ ] Update function declarations
- [ ] Add deprecation warnings to old tools
- [ ] Update documentation

### Week 4: File Discovery Consolidation
- [ ] Implement `EnhancedListInput` structure
- [ ] Add recursive search to `list_files`
- [ ] Add name pattern matching to `list_files`
- [ ] Create migration path for deprecated tools

### Week 5: Command Execution Unification
- [ ] Implement `UnifiedCommandInput` structure
- [ ] Complete PTY implementation
- [ ] Add streaming support
- [ ] Create unified `run_command` tool

## Testing Strategy

### Unit Tests
```rust
#[cfg(test)]
mod consolidation_tests {
    #[tokio::test]
    async fn test_enhanced_rp_search_fuzzy_mode() {
        // Test fuzzy search functionality
    }
    
    #[tokio::test]
    async fn test_backward_compatibility_code_search() {
        // Ensure old code_search calls still work
    }
    
    #[tokio::test]
    async fn test_enhanced_list_files_recursive() {
        // Test recursive file listing
    }
}
```

### Integration Tests
- Test all deprecated tools still function correctly
- Verify performance improvements from consolidation
- Ensure security tools produce valid output

## Migration Guide for Users

### Search Tools Migration
```rust
// Old way
code_search { pattern: "fn main", path: "." }

// New way (both work during transition)
rp_search { pattern: "fn main", path: "." }
rp_search { pattern: "query", search_mode: "fuzzy", path: "." }
```

### File Discovery Migration
```rust
// Old way
recursive_file_search { pattern: "*.rs", path: "." }

// New way
list_files { path: ".", recursive: true, name_pattern: "*.rs" }
```

## Success Metrics

### Performance Improvements
- **Target**: 30% reduction in memory usage from tool consolidation
- **Target**: 20% improvement in cache hit rates
- **Target**: Reduced tool execution time through shared implementations

### Code Quality
- **Target**: 50% reduction in duplicate code across search tools
- **Target**: Unified error handling across all tools
- **Target**: Consistent parameter validation

### User Experience
- **Target**: Maintain 100% backward compatibility during transition
- **Target**: Improved documentation clarity
- **Target**: Reduced cognitive load with fewer, more powerful tools

## Risk Mitigation

### Backward Compatibility Risks
- **Mitigation**: Maintain aliases for all deprecated tools
- **Mitigation**: Gradual deprecation with clear migration timeline
- **Mitigation**: Comprehensive testing of legacy interfaces

### Performance Risks
- **Mitigation**: Benchmark before and after consolidation
- **Mitigation**: Implement performance monitoring
- **Mitigation**: Rollback plan if performance degrades

### Security Implementation Risks
- **Mitigation**: Thorough security review of new implementations
- **Mitigation**: Conservative approach with clear limitations documented
- **Mitigation**: External security audit of vulnerability detection logic

## Conclusion

This consolidation plan addresses the identified redundancy issues while maintaining system stability and user experience. The phased approach ensures critical security functionality is prioritized while systematically improving the overall tool ecosystem efficiency.
