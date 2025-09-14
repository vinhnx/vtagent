# Migrating from console::style to anstyle

This document outlines the approach for migrating VTAgent's terminal styling from `console::style` to `anstyle` for better cross-platform compatibility and performance.

## Why Migrate to anstyle?

1. **Cross-platform compatibility**: anstyle works consistently across Unix and Windows terminals
2. **Environment variable support**: Automatically respects NO_COLOR, CLICOLOR, CLICOLOR_FORCE
3. **Better performance**: Zero-allocation options for efficient styling
4. **Library adapters**: Integration with popular crates like crossterm, termcolor, and owo-colors

## Migration Approach

### 1. Current State Analysis

VTAgent currently uses `console::style` for terminal styling with approximately 124 style references across the codebase:
- 70 instances in `src/main.rs`
- 54 instances in other files

### 2. Migration Strategy

#### Phase 1: Infrastructure Setup
- [x] Add anstyle dependencies to Cargo.toml
- [x] Create utility module in vtagent-core
- [x] Implement styled text output functions
- [x] Create styling guide

#### Phase 2: Proof of Concept
- [x] Demonstrate anstyle working in a simple example

#### Phase 3: Documentation and Tooling
- [ ] Document the migration approach
- [ ] Create migration script or tool

#### Phase 4: Gradual Migration
- [ ] Migrate less critical files first
- [ ] Test each migration
- [ ] Gradually work toward more critical components

### 3. Style Mapping

| console::style | anstyle equivalent |
|----------------|--------------------|
| `style("text").red()` | `Styles::error()` |
| `style("text").green()` | `Styles::success()` |
| `style("text").blue()` | `Styles::info()` |
| `style("text").yellow()` | `Styles::warning()` |
| `style("text").cyan()` | `Styles::debug()` |
| `style("text").magenta()` | `Styles::code()` |
| `style("text").bold()` | `Styles::bold()` |

### 4. Implementation Examples

#### Before (console::style):
```rust
use console::style;

println!("{}", style("Error message").red().bold());
println!("{}", style("Warning message").yellow().bold());
println!("{}", style("Success message").green());
```

#### After (anstyle):
```rust
use vtagent_core::ui::styled::*;

error("Error message");  // Uses red, bold styling
warning("Warning message");  // Uses yellow, bold styling
success("Success message");  // Uses green styling
```

Or for more control:
```rust
use vtagent_core::ui::styled::*;

println!("{}{}{}", Styles::bold_error().render(), "Error message", Styles::bold_error().render_reset());
println!("{}{}{}", Styles::bold_warning().render(), "Warning message", Styles::bold_warning().render_reset());
println!("{}{}{}", Styles::success().render(), "Success message", Styles::success().render_reset());
```

### 5. Migration Steps

1. **Identify all style() calls**: Use grep to find all instances
2. **Map to anstyle equivalents**: Use the style mapping table above
3. **Replace gradually**: Start with non-critical files
4. **Test thoroughly**: Verify styling works across different terminals
5. **Update imports**: Remove `use console::style;` and add `use vtagent_core::ui::styled::*;`

### 6. Considerations

1. **String formatting**: anstyle uses a different approach to formatting with render() and render_reset()
2. **Chaining**: console::style allows method chaining, while anstyle uses Style objects
3. **Performance**: anstyle can be more efficient, especially with the streaming approach
4. **Compatibility**: anstyle automatically handles terminal capability detection

### 7. Testing

After migration, test the following:
- Different terminal environments (Unix, Windows, macOS)
- With and without NO_COLOR environment variable
- With different CLICOLOR settings
- Performance comparison between old and new implementations