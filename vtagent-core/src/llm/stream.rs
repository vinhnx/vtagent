use crate::config::constants::streaming;
use std::env;

const SSE_EVENT_SEPARATOR: &str = "\n\n";

/// Determine the chunk size for streaming outputs.
///
/// The value can be overridden by setting the `VTAGENT_STREAMING_CHARS_PER_CHUNK`
/// environment variable. Values are clamped between the configured minimum and
/// maximum bounds to avoid inefficient streaming behaviour.
pub fn resolve_chunk_size() -> usize {
    let env_value = env::var(streaming::CHUNK_SIZE_ENV)
        .ok()
        .and_then(|value| value.parse::<usize>().ok());

    match env_value {
        Some(value) => value.clamp(
            streaming::MIN_CHARS_PER_CHUNK,
            streaming::MAX_CHARS_PER_CHUNK,
        ),
        None => streaming::DEFAULT_CHARS_PER_CHUNK,
    }
}

/// Split the provided content into streaming-friendly chunks.
///
/// The function preserves character boundaries to avoid breaking UTF-8 glyphs
/// mid-sequence. Empty inputs return an empty vector and are handled by the
/// caller without producing unnecessary events.
pub fn chunk_text(content: &str) -> Vec<String> {
    let mut buffer = String::new();
    let mut chunks = Vec::new();
    let mut count = 0usize;
    let chunk_size = resolve_chunk_size();

    for ch in content.chars() {
        buffer.push(ch);
        count += 1;
        if count >= chunk_size {
            if !buffer.is_empty() {
                chunks.push(std::mem::take(&mut buffer));
            }
            count = 0;
        }
    }

    if !buffer.is_empty() {
        chunks.push(buffer);
    }

    chunks
}

/// Drain complete SSE events from the provided buffer and return their payloads.
///
/// The function accumulates data lines from SSE-formatted responses and returns
/// the extracted payloads without the leading `data:` prefix. Incomplete events
/// remain in the buffer for subsequent calls.
pub fn drain_sse_events(buffer: &mut String) -> Vec<String> {
    let mut events = Vec::new();

    loop {
        let Some(idx) = buffer.find(SSE_EVENT_SEPARATOR) else {
            break;
        };

        let raw_event = buffer[..idx].replace('\r', "");
        buffer.drain(..idx + SSE_EVENT_SEPARATOR.len());

        if raw_event.trim().is_empty() {
            continue;
        }

        if let Some(payload) = extract_event_payload(&raw_event) {
            if !payload.is_empty() {
                events.push(payload);
            }
        }
    }

    events
}

fn extract_event_payload(event: &str) -> Option<String> {
    let mut data_lines = Vec::new();

    for line in event.lines() {
        let trimmed = line.trim_end();
        if let Some(data) = trimmed.strip_prefix("data:") {
            data_lines.push(data.trim_start());
        }
    }

    if data_lines.is_empty() {
        None
    } else {
        Some(data_lines.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct EnvVarGuard {
        key: &'static str,
    }

    impl EnvVarGuard {
        fn new(key: &'static str, value: &str) -> Self {
            env::set_var(key, value);
            Self { key }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            env::remove_var(self.key);
        }
    }

    #[test]
    fn resolve_chunk_size_respects_defaults() {
        env::remove_var(streaming::CHUNK_SIZE_ENV);
        let size = resolve_chunk_size();
        assert_eq!(size, streaming::DEFAULT_CHARS_PER_CHUNK);
    }

    #[test]
    fn resolve_chunk_size_clamps_bounds() {
        let _guard_low = EnvVarGuard::new(streaming::CHUNK_SIZE_ENV, "1");
        let size_low = resolve_chunk_size();
        assert_eq!(size_low, streaming::MIN_CHARS_PER_CHUNK);
        drop(_guard_low);

        let _guard_high = EnvVarGuard::new(streaming::CHUNK_SIZE_ENV, "9999");
        let size_high = resolve_chunk_size();
        assert_eq!(size_high, streaming::MAX_CHARS_PER_CHUNK);
    }

    #[test]
    fn chunk_text_respects_configured_size() {
        let _guard = EnvVarGuard::new(streaming::CHUNK_SIZE_ENV, "4");
        let chunks = chunk_text("streaming");
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0], "stre");
        assert_eq!(chunks[1], "amin");
        assert_eq!(chunks[2], "g");
    }

    #[test]
    fn drain_sse_events_extracts_payloads() {
        let mut buffer = String::new();
        buffer.push_str("data: one\\n\\n");
        buffer.push_str("data: two\\n\\n");
        let events = drain_sse_events(&mut buffer);
        assert_eq!(events, vec!["one".to_string(), "two".to_string()]);
        assert!(buffer.is_empty());
    }

    #[test]
    fn drain_sse_events_handles_multiline_payloads() {
        let mut buffer = String::new();
        buffer.push_str("event: message\r\n");
        buffer.push_str("data: {\"a\":\"b\"}\r\n\r\n");
        buffer.push_str("data: [DONE]");

        let mut events = drain_sse_events(&mut buffer);
        assert_eq!(events.len(), 1);
        assert_eq!(events.remove(0), "{\"a\":\"b\"}");
        assert_eq!(buffer, "data: [DONE]");
    }
}
