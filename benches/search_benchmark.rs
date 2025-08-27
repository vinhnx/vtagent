use criterion::{black_box, criterion_group, criterion_main, Criterion};
use serde_json::json;
use std::env;
use tempfile::TempDir;
use vtagent::tools::ToolRegistry;

/// Benchmark search performance across different file sizes and patterns
fn benchmark_search_performance(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let original_cwd = env::current_dir().unwrap();

    env::set_current_dir(&temp_dir).unwrap();

    // Create test files of different sizes
    create_test_files(&temp_dir);

    let mut registry = ToolRegistry::new(temp_dir.path().to_path_buf());

    let mut group = c.benchmark_group("search");

    // Benchmark simple pattern search
    group.bench_function("simple_pattern", |b| {
        b.iter(|| {
            let args = json!({
                "pattern": "fn main",
                "path": ".",
                "type": "regex"
            });
            let _result = registry.execute("grep_search", args);
        });
    });

    // Benchmark word boundary search
    group.bench_function("word_boundary", |b| {
        b.iter(|| {
            let args = json!({
                "pattern": "function",
                "path": ".",
                "type": "word"
            });
            let _result = registry.execute("grep_search", args);
        });
    });

    // Benchmark case-insensitive search
    group.bench_function("case_insensitive", |b| {
        b.iter(|| {
            let args = json!({
                "pattern": "FUNCTION",
                "path": ".",
                "case_sensitive": false
            });
            let _result = registry.execute("grep_search", args);
        });
    });

    // Benchmark search with context lines
    group.bench_function("with_context", |b| {
        b.iter(|| {
            let args = json!({
                "pattern": "fn main",
                "path": ".",
                "context_lines": 3
            });
            let _result = registry.execute("grep_search", args);
        });
    });

    // Benchmark glob pattern filtering
    group.bench_function("glob_filter", |b| {
        b.iter(|| {
            let args = json!({
                "pattern": "function",
                "path": ".",
                "glob_pattern": "*.rs"
            });
            let _result = registry.execute("grep_search", args);
        });
    });

    group.finish();

    // Restore original directory
    let _ = env::set_current_dir(original_cwd);
}

/// Benchmark file operations performance
fn benchmark_file_operations(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let original_cwd = env::current_dir().unwrap();

    env::set_current_dir(&temp_dir).unwrap();
    create_test_files(&temp_dir);

    let mut registry = ToolRegistry::new(temp_dir.path().to_path_buf());

    let mut group = c.benchmark_group("file_operations");

    // Benchmark file reading
    group.bench_function("read_file", |b| {
        b.iter(|| {
            let args = json!({
                "path": "large_file.txt"
            });
            let _result = registry.execute("read_file", args);
        });
    });

    // Benchmark directory listing
    group.bench_function("list_files", |b| {
        b.iter(|| {
            let args = json!({
                "path": "."
            });
            let _result = registry.execute("list_files", args);
        });
    });

    // Benchmark file writing
    group.bench_function("write_file", |b| {
        b.iter(|| {
            let args = json!({
                "path": "benchmark_write.txt",
                "content": "benchmark content",
                "overwrite": true
            });
            let _result = registry.execute("write_file", args);
        });
    });

    group.finish();

    // Restore original directory
    let _ = env::set_current_dir(original_cwd);
}

fn create_test_files(temp_dir: &TempDir) {
    // Create a large file for performance testing
    let large_content = format!(
        "fn main() {{\n    println!(\"Hello, world!\");\n}}\n\n{}",
        "line content\n".repeat(1000)
    );
    std::fs::write(temp_dir.path().join("large_file.txt"), large_content).unwrap();

    // Create multiple files with different patterns
    for i in 0..10 {
        let content = format!(
            r#"// File number {}
fn function_{}() {{
    println!("Function {}", i);
}}

struct Struct{} {{
    field: i32,
}}

impl Struct{} {{
    fn method(&self) {{
        todo!();
    }}
}}
"#,
            i, i, i, i, i
        );
        std::fs::write(temp_dir.path().join(format!("file_{}.rs", i)), content).unwrap();
    }

    // Create Python files
    for i in 0..5 {
        let content = format!(
            r#"# Python file {}
def function_{}(param):
    """Function {} documentation"""
    print(f"Function {0}", param)
    return param * 2

class Class{}(object):
    def __init__(self):
        self.value = {}

    def method(self):
        return self.value
"#,
            i, i, i, i, i
        );
        std::fs::write(temp_dir.path().join(format!("script_{}.py", i)), content).unwrap();
    }

    // Create JavaScript files
    for i in 0..5 {
        let content = format!(
            r#"// JavaScript file {}
function function{}(param) {{
    console.log('Function {}', param);
    return param * 2;
}}

class Class{} {{
    constructor() {{
        this.value = {};
    }}

    method() {{
        return this.value;
    }}
}}
"#,
            i, i, i, i, i
        );
        std::fs::write(temp_dir.path().join(format!("script_{}.js", i)), content).unwrap();
    }
}

criterion_group!(
    benches,
    benchmark_search_performance,
    benchmark_file_operations
);
criterion_main!(benches);
