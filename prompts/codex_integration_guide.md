# Codex Integration Implementation Guide

## Immediate Actions for VTAgent Enhancement

### 1. Update System Prompt with Codex Patterns

**Current Status**: ✅ Enhanced system prompt created
**Next Steps**: 
- Replace current system prompt with Codex-enhanced version
- Add structured output markers to all security/quality analysis functions
- Implement validation for JSON/patch outputs

### 2. Add New Tool Functions

#### Priority 1: Security Analysis Tools

```rust
// Add to vtagent-core/src/tools.rs

pub async fn security_scan(
    scan_type: String,
    output_format: String,
    severity_filter: Vec<String>
) -> Result<String, Box<dyn std::error::Error>> {
    // Implementation using existing AST and search tools
    // Returns structured security report with markers
}

pub async fn generate_security_patch(
    vulnerability_report: String,
    target_files: Vec<String>,
    patch_strategy: String
) -> Result<String, Box<dyn std::error::Error>> {
    // Generate git patches for security fixes
    // Validate with git apply --check
}

pub async fn extract_json_markers(
    input_text: String,
    begin_marker: String,
    end_marker: String,
    validate_json: bool
) -> Result<String, Box<dyn std::error::Error>> {
    // Extract and validate JSON between markers
    // Essential for Codex-style structured output
}
```

#### Priority 2: Code Quality Tools

```rust
pub async fn generate_code_quality_report(
    format: String,
    include_metrics: bool,
    severity_threshold: String
) -> Result<String, Box<dyn std::error::Error>> {
    // Generate GitLab/GitHub compatible quality reports
    // Use CodeClimate JSON format
}

pub async fn validate_patch(
    patch_file: String,
    dry_run: bool,
    check_syntax: bool
) -> Result<String, Box<dyn std::error::Error>> {
    // Validate patch applicability and safety
    // Use git apply --check and syntax validation
}
```

### 3. Enhanced Configuration Support

Add to `vtagent.toml`:

```toml
[security]
enable_vulnerability_scanning = true
auto_generate_patches = false
consolidate_findings = true
severity_threshold = "medium"
output_format = "gitlab-sast"

[code_quality]
enable_quality_reports = true
output_format = "codeclimate"
include_metrics = true
validate_outputs = true

[structured_output]
use_markers = true
validate_json = true
fallback_on_error = true
```

### 4. Tool Policy Updates

Update tool policies to handle new security tools:

```toml
[tools.policies]
security_scan = "prompt"
generate_security_patch = "prompt"  # Always require approval for patches
validate_patch = "allow"
generate_code_quality_report = "allow"
extract_json_markers = "allow"
```

## Implementation Phases

### Phase 1: Core Structured Output (Week 1)
- [ ] Implement `extract_json_markers` tool
- [ ] Add structured output validation to existing tools
- [ ] Update system prompt with marker patterns
- [ ] Test with existing AST and search tools

### Phase 2: Security Analysis Enhancement (Week 2)
- [ ] Implement `security_scan` using existing AST tools
- [ ] Add vulnerability pattern library
- [ ] Create security report templates
- [ ] Integrate with existing file operations

### Phase 3: Patch Generation (Week 3)
- [ ] Implement `generate_security_patch` tool
- [ ] Add `validate_patch` functionality
- [ ] Create patch templates for common vulnerabilities
- [ ] Integrate with git operations

### Phase 4: CI/CD Integration (Week 4)
- [ ] Add GitLab CI/CD configuration generation
- [ ] Create GitHub Actions workflow templates
- [ ] Implement quality report generation
- [ ] Add comprehensive testing

## Codex Pattern Examples

### Security Analysis with Structured Output

```rust
// Example usage in VTAgent
let security_analysis = r#"
=== BEGIN_SECURITY_REPORT ===
# Security Analysis Report

## Summary
- **Total Findings**: 5
- **Critical**: 2
- **High**: 2
- **Medium**: 1

## Critical Issues

### 1. SQL Injection in login.rs:34
- **CWE**: CWE-89
- **Evidence**: `format!("SELECT * FROM users WHERE email = '{}'", user_input)`
- **Exploitability**: 95/100
- **Remediation**: Use parameterized queries

### 2. Hardcoded API Key in config.rs:12
- **CWE**: CWE-798
- **Evidence**: `const API_KEY = "sk-1234567890abcdef"`
- **Exploitability**: 98/100
- **Remediation**: Move to environment variables

## Recommended Actions
1. Fix SQL injection with prepared statements
2. Remove hardcoded secrets
3. Add input validation
4. Implement security headers
5. Add authentication checks

=== END_SECURITY_REPORT ===
"#;
```

### Patch Generation with Validation

```rust
// Example security patch
let security_patch = r#"
=== BEGIN_UNIFIED_DIFF ===
diff --git a/src/auth/login.rs b/src/auth/login.rs
index abc123..def456 100644
--- a/src/auth/login.rs
+++ b/src/auth/login.rs
@@ -31,7 +31,10 @@ pub async fn authenticate_user(email: &str, password: &str) -> Result<User, Aut
     // Validate input
     validate_email(email)?;
     
-    let query = format!("SELECT * FROM users WHERE email = '{}'", email);
+    // Use parameterized query to prevent SQL injection
+    let query = "SELECT * FROM users WHERE email = ?";
+    let params = vec![email];
+    
     let user = database::query_one(&query, params).await?;
     
     if verify_password(password, &user.password_hash)? {
=== END_UNIFIED_DIFF ===
"#;
```

### Code Quality Report Generation

```rust
// Example CodeClimate JSON output
let quality_report = r#"
=== BEGIN_CODE_QUALITY_JSON ===
[
  {
    "description": "SQL injection vulnerability in user authentication",
    "check_name": "security/sql-injection",
    "fingerprint": "abc123def456",
    "severity": "critical",
    "location": {
      "path": "src/auth/login.rs",
      "lines": {
        "begin": 34
      }
    }
  },
  {
    "description": "Hardcoded API key in configuration",
    "check_name": "security/hardcoded-secret",
    "fingerprint": "def456ghi789",
    "severity": "critical",
    "location": {
      "path": "src/config.rs",
      "lines": {
        "begin": 12
      }
    }
  }
]
=== END_CODE_QUALITY_JSON ===
"#;
```

## Testing Strategy

### Unit Tests for New Tools
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_extract_json_markers() {
        let input = "Some text\n=== BEGIN_JSON ===\n{\"test\": true}\n=== END_JSON ===\nMore text";
        let result = extract_json_markers(
            input.to_string(),
            "=== BEGIN_JSON ===".to_string(),
            "=== END_JSON ===".to_string(),
            true
        ).await.unwrap();
        
        assert_eq!(result, r#"{"test": true}"#);
    }

    #[tokio::test]
    async fn test_security_scan() {
        // Test security scanning with known vulnerable code
        let result = security_scan(
            "sast".to_string(),
            "gitlab".to_string(),
            vec!["critical".to_string(), "high".to_string()]
        ).await.unwrap();
        
        assert!(result.contains("=== BEGIN_SECURITY_REPORT ==="));
        assert!(result.contains("=== END_SECURITY_REPORT ==="));
    }
}
```

### Integration Tests
```rust
#[tokio::test]
async fn test_full_security_workflow() {
    // 1. Scan for vulnerabilities
    let scan_result = security_scan("all".to_string(), "gitlab".to_string(), vec!["high".to_string()]).await?;
    
    // 2. Generate patches for critical issues
    let patch_result = generate_security_patch(scan_result, vec!["src/main.rs".to_string()], "minimal".to_string()).await?;
    
    // 3. Validate patches
    let validation_result = validate_patch(patch_result, true, true).await?;
    
    assert!(validation_result.contains("Patch validation: PASSED"));
}
```

## Migration Path

### Backward Compatibility
- Keep existing tool functions unchanged
- Add new Codex-style tools alongside existing ones
- Use feature flags to enable/disable new functionality
- Provide migration guide for existing users

### Gradual Rollout
1. **Phase 1**: Add structured output support to existing tools
2. **Phase 2**: Introduce new security analysis tools
3. **Phase 3**: Add patch generation capabilities
4. **Phase 4**: Full CI/CD integration

### Documentation Updates
- Update tool documentation with Codex patterns
- Add security analysis examples
- Create CI/CD integration guides
- Provide migration examples

This implementation guide provides a clear path for integrating Codex patterns into VTAgent while maintaining compatibility with existing functionality and following security best practices.
