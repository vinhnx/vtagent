//! Rate limiter for API requests and tool calls to prevent abuse and rate limiting

use anyhow::Result;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Rate limiter to prevent API abuse and rate limiting
#[derive(Debug)]
pub struct RateLimiter {
    /// Maximum requests per minute
    requests_per_minute: usize,
    /// Timestamp of recent requests (for sliding window)
    request_times: Arc<Mutex<Vec<Instant>>>,
    /// Current tool call count
    tool_call_count: Arc<AtomicUsize>,
    /// Maximum tool calls allowed
    max_tool_calls: usize,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new(requests_per_minute: usize, max_tool_calls: usize) -> Self {
        Self {
            requests_per_minute,
            request_times: Arc::new(Mutex::new(Vec::new())),
            tool_call_count: Arc::new(AtomicUsize::new(0)),
            max_tool_calls,
        }
    }

    /// Check if we can make an API request, blocking if necessary
    pub async fn wait_for_api_request(&self) -> Result<()> {
        let mut request_times = self.request_times.lock().unwrap();

        // Remove old requests (older than 1 minute)
        let one_minute_ago = Instant::now() - Duration::from_secs(60);
        request_times.retain(|&time| time > one_minute_ago);

        // If we're at the limit, wait until the oldest request expires
        if request_times.len() >= self.requests_per_minute {
            let oldest_request = request_times[0];
            let wait_time = Duration::from_secs(60) - oldest_request.elapsed();

            if wait_time > Duration::from_secs(0) {
                tokio::time::sleep(wait_time).await;
                // After waiting, remove expired requests again
                let one_minute_ago = Instant::now() - Duration::from_secs(60);
                request_times.retain(|&time| time > one_minute_ago);
            }
        }

        // Add current request time
        request_times.push(Instant::now());

        Ok(())
    }

    /// Check if we can make a tool call
    pub fn can_make_tool_call(&self) -> bool {
        self.tool_call_count.load(Ordering::Relaxed) < self.max_tool_calls
    }

    /// Increment the tool call count
    pub fn increment_tool_call(&self) {
        self.tool_call_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Get the current tool call count
    pub fn get_tool_call_count(&self) -> usize {
        self.tool_call_count.load(Ordering::Relaxed)
    }

    /// Reset tool call count for new session
    pub fn reset_tool_calls(&self) {
        self.tool_call_count.store(0, Ordering::Relaxed);
    }

    /// Get the current request count in the sliding window
    pub fn get_current_request_count(&self) -> usize {
        let request_times = self.request_times.lock().unwrap();
        let one_minute_ago = Instant::now() - Duration::from_secs(60);
        request_times.iter().filter(|&&time| time > one_minute_ago).count()
    }
}
