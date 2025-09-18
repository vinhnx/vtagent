use serde_json::Value;
use vtcode_core::utils::ansi::{AnsiRenderer, MessageStyle};

pub(crate) fn render_tool_output(val: &Value) {
    let mut renderer = AnsiRenderer::stdout();
    if let Some(stdout) = val.get("stdout").and_then(|value| value.as_str())
        && !stdout.trim().is_empty()
    {
        let _ = renderer.line(MessageStyle::Tool, "[stdout]");
        let formatted = stdout
            .lines()
            .map(|line| format!("  {}", line))
            .collect::<Vec<_>>()
            .join("\n");
        let _ = renderer.line(MessageStyle::Output, &formatted);
    }
    if let Some(stderr) = val.get("stderr").and_then(|value| value.as_str())
        && !stderr.trim().is_empty()
    {
        let _ = renderer.line(MessageStyle::Tool, "[stderr]");
        let formatted = stderr
            .lines()
            .map(|line| format!("  {}", line))
            .collect::<Vec<_>>()
            .join("\n");
        let _ = renderer.line(MessageStyle::Error, &formatted);
    }
}
