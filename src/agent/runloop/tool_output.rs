use serde_json::Value;
use vtagent_core::utils::ansi::{AnsiRenderer, MessageStyle};

pub(crate) fn render_tool_output(val: &Value) {
    let mut renderer = AnsiRenderer::stdout();
    if let Some(stdout) = val.get("stdout").and_then(|value| value.as_str())
        && !stdout.trim().is_empty()
    {
        let _ = renderer.line(MessageStyle::Info, "[stdout]");
        let _ = renderer.line(MessageStyle::Output, stdout);
    }
    if let Some(stderr) = val.get("stderr").and_then(|value| value.as_str())
        && !stderr.trim().is_empty()
    {
        let _ = renderer.line(MessageStyle::Error, "[stderr]");
        let _ = renderer.line(MessageStyle::Error, stderr);
    }
}
