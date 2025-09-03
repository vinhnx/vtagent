//! Standalone implementation of rp_search with debounce/cancellation logic.
//!
//! This module provides a simplified implementation of rp_search that can be
//! used independently of the existing tool infrastructure.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;

/// Input parameters for ripgrep search
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RpSearchInput {
    pub pattern: String,
    #[serde(default = "default_search_path")]
    pub path: String,
    #[serde(default)]
    pub case_sensitive: Option<bool>,
    #[serde(default)]
    pub literal: Option<bool>,
    #[serde(default)]
    pub glob_pattern: Option<String>,
    #[serde(default)]
    pub context_lines: Option<usize>,
    #[serde(default)]
    pub include_hidden: Option<bool>,
    #[serde(default)]
    pub max_results: Option<usize>,
}

fn default_search_path() -> String {
    ".".to_string()
}

/// Result of a ripgrep search
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RpSearchResult {
    pub query: String,
    pub matches: Vec<Value>,
}

/// State machine for rp_search orchestration.
pub struct RpSearchManager {
    /// Unified state guarded by one mutex.
    state: Arc<Mutex<SearchState>>,

    search_dir: PathBuf,
}

struct SearchState {
    /// Latest query typed by user (updated every keystroke).
    latest_query: String,

    /// true if a search is currently scheduled.
    is_search_scheduled: bool,

    /// If there is an active search, this will be the query being searched.
    active_search: Option<ActiveSearch>,
}

struct ActiveSearch {
    query: String,
    cancellation_token: Arc<AtomicBool>,
}

impl RpSearchManager {
    pub fn new(search_dir: PathBuf) -> Self {
        Self {
            state: Arc::new(Mutex::new(SearchState {
                latest_query: String::new(),
                is_search_scheduled: false,
                active_search: None,
            })),
            search_dir,
        }
    }

    /// Call whenever the user edits the search query.
    pub fn on_user_query(&self, query: String) {
        {
            #[expect(clippy::unwrap_used)]
            let mut st = self.state.lock().unwrap();
            if query == st.latest_query {
                // No change, nothing to do.
                return;
            }

            // Update latest query.
            st.latest_query.clear();
            st.latest_query.push_str(&query);

            // If there is an in-flight search that is definitely obsolete,
            // cancel it now.
            if let Some(ref active_search) = st.active_search {
                if !query.starts_with(&active_search.query) {
                    active_search
                        .cancellation_token
                        .store(true, Ordering::Relaxed);
                    st.active_search = None;
                }
            }

            // Schedule a search to run after debounce.
            if !st.is_search_scheduled {
                st.is_search_scheduled = true;
            } else {
                return;
            }
        }

        // If we are here, we set `st.is_search_scheduled = true` before
        // dropping the lock. This means we are the only thread that can spawn a
        // debounce timer.
        let state = self.state.clone();
        let search_dir = self.search_dir.clone();
        thread::spawn(move || {
            // Always do a minimum debounce, but then poll until the
            // `active_search` is cleared.
            thread::sleep(Duration::from_millis(150));
            loop {
                #[expect(clippy::unwrap_used)]
                if state.lock().unwrap().active_search.is_none() {
                    break;
                }
                thread::sleep(Duration::from_millis(20));
            }

            // The debounce timer has expired, so start a search using the
            // latest query.
            let cancellation_token = Arc::new(AtomicBool::new(false));
            let token = cancellation_token.clone();
            let query = {
                #[expect(clippy::unwrap_used)]
                let mut st = state.lock().unwrap();
                let query = st.latest_query.clone();
                st.is_search_scheduled = false;
                st.active_search = Some(ActiveSearch {
                    query: query.clone(),
                    cancellation_token: token,
                });
                query
            };

            RpSearchManager::spawn_rp_search(
                query,
                search_dir,
                cancellation_token,
                state,
            );
        });
    }

    fn spawn_rp_search(
        _query: String,
        _search_dir: PathBuf,
        cancellation_token: Arc<AtomicBool>,
        search_state: Arc<Mutex<SearchState>>,
    ) {
        // In a real implementation, this would perform the actual ripgrep search
        // For now, we'll just simulate the search completion
        
        // Simulate search work
        thread::spawn(move || {
            // Simulate some work being done
            thread::sleep(Duration::from_millis(100));
            
            let is_cancelled = cancellation_token.load(Ordering::Relaxed);
            if !is_cancelled {
                // In a real implementation, this would send the search results
                // For now, we're just demonstrating the structure
                println!("Search completed");
            }

            // Reset the active search state. Do a pointer comparison to verify
            // that we are clearing the ActiveSearch that corresponds to the
            // cancellation token we were given.
            {
                #[expect(clippy::unwrap_used)]
                let mut st = search_state.lock().unwrap();
                if let Some(ref active_search) = &st.active_search {
                    if Arc::ptr_eq(&active_search.cancellation_token, &cancellation_token) {
                        st.active_search = None;
                    }
                }
            }
        });
    }
    
    /// Perform an actual ripgrep search with the given input parameters
    pub async fn perform_search(&self, _input: RpSearchInput) -> Result<RpSearchResult> {
        // This would be implemented to actually call ripgrep
        // For now, returning a placeholder result
        
        Ok(RpSearchResult {
            query: "test".to_string(),
            matches: vec![],
        })
    }
    
    /// Enhanced ripgrep search with debounce and cancellation
    pub async fn rp_search(&self, args: Value) -> Result<Value> {
        let input: RpSearchInput = serde_json::from_value(args)?;
        
        // Notify the search manager of the new query (for debounce logic)
        self.on_user_query(input.pattern.clone());
        
        // Perform the actual search
        let result = self.perform_search(input).await?;
        
        Ok(serde_json::to_value(result)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    
    #[tokio::test]
    async fn test_rp_search_basic() -> Result<()> {
        let manager = RpSearchManager::new(PathBuf::from("."));
        let args = json!({
            "pattern": "test",
            "path": "."
        });
        
        let result = manager.rp_search(args).await?;
        assert!(result.is_object());
        
        Ok(())
    }
}