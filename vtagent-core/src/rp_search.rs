//! Helper that owns the debounce/cancellation logic for `rp_search` operations.
//!
//! This module manages the orchestration of ripgrep searches, implementing
//! debounce and cancellation logic to ensure responsive and efficient searches.
//!
//! It works as follows:
//! 1. First query starts a debounce timer.
//! 2. While the timer is pending, the latest query from the user is stored.
//! 3. When the timer fires, it is cleared, and a search is done for the most
//!    recent query.
//! 4. If there is an in-flight search that is not a prefix of the latest thing
//!    the user typed, it is cancelled.

use anyhow::Result;
use serde_json;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;

/// Maximum number of search results to return
const MAX_SEARCH_RESULTS: NonZeroUsize = NonZeroUsize::new(100).unwrap();

/// Number of threads to use for searching
const NUM_SEARCH_THREADS: NonZeroUsize = NonZeroUsize::new(2).unwrap();

/// How long to wait after a keystroke before firing the first search when none
/// is currently running. Keeps early queries more meaningful.
const SEARCH_DEBOUNCE: Duration = Duration::from_millis(150);

/// Poll interval when waiting for an active search to complete
const ACTIVE_SEARCH_COMPLETE_POLL_INTERVAL: Duration = Duration::from_millis(20);

/// Input parameters for ripgrep search
#[derive(Debug, Clone)]
pub struct RpSearchInput {
    pub pattern: String,
    pub path: String,
    pub case_sensitive: Option<bool>,
    pub literal: Option<bool>,
    pub glob_pattern: Option<String>,
    pub context_lines: Option<usize>,
    pub include_hidden: Option<bool>,
    pub max_results: Option<usize>,
}

/// Result of a ripgrep search
#[derive(Debug, Clone)]
pub struct RpSearchResult {
    pub query: String,
    pub matches: Vec<serde_json::Value>,
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
            if let Some(active_search) = &st.active_search
                && !query.starts_with(&active_search.query)
            {
                active_search
                    .cancellation_token
                    .store(true, Ordering::Relaxed);
                st.active_search = None;
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
            thread::sleep(SEARCH_DEBOUNCE);
            loop {
                #[expect(clippy::unwrap_used)]
                if state.lock().unwrap().active_search.is_none() {
                    break;
                }
                thread::sleep(ACTIVE_SEARCH_COMPLETE_POLL_INTERVAL);
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

            RpSearchManager::spawn_rp_search(query, search_dir, cancellation_token, state);
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
                if let Some(active_search) = &st.active_search
                    && Arc::ptr_eq(&active_search.cancellation_token, &cancellation_token)
                {
                    st.active_search = None;
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
}
