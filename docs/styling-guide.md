# VTAgent Styling Guide

This guide documents the styling conventions used in VTAgent for consistent terminal output across different platforms.

## Color Palette

VTAgent uses the following color conventions for different types of messages:

- **Error messages**: Red (`AnsiColor::Red`) with bold styling
- **Warning messages**: Yellow (`AnsiColor::Yellow`) with bold styling
- **Success messages**: Green (`AnsiColor::Green`) with bold styling
- **Informational messages**: Blue (`AnsiColor::Blue`) with optional bold styling
- **Debug messages**: Cyan (`AnsiColor::Cyan`) with optional dim styling
- **Headers**: Blue (`AnsiColor::Blue`) with bold styling
- **Code/Technical content**: Magenta (`AnsiColor::Magenta`) 

## Style Functions

The `vtagent-core/src/ui/styled.rs` module provides the following preset styles:

- `Styles::error()` - Red text for errors
- `Styles::warning()` - Yellow text for warnings
- `Styles::success()` - Green text for success messages
- `Styles::info()` - Blue text for informational messages
- `Styles::debug()` - Cyan text for debug messages
- `Styles::bold()` - Bold text styling
- `Styles::bold_error()` - Red, bold text for critical errors
- `Styles::bold_success()` - Green, bold text for important success messages
- `Styles::bold_warning()` - Yellow, bold text for important warnings
- `Styles::header()` - Blue, bold text for headers
- `Styles::code()` - Magenta text for code snippets

## Usage Examples

To use these styles in your code:

```rust
use vtagent_core::ui::styled::*;

// Print a simple error message
error("This is an error message");

// Print a bold success message
println!("{}{}{}", Styles::bold_success().render(), "Operation completed!", Styles::bold_success().render_reset());

// Print a custom styled message
custom("Custom message", Styles::header());
```

## Best Practices

1. Always use the provided style functions rather than creating custom styles directly
2. Ensure proper reset of styles by using both `.render()` and `.render_reset()`
3. Use appropriate style levels for different message types
4. Test styling across different terminal environments
5. Respect environment variables like NO_COLOR, CLICOLOR, etc.