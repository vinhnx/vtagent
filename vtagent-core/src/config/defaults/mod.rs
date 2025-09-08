use std::time::Duration;

/// Multi-agent system defaults
pub struct MultiAgentDefaults;

impl MultiAgentDefaults {
    pub fn max_agents() -> usize {
        5
    }
    pub fn max_context_size() -> usize {
        100000
    }
    pub fn compression_enabled() -> bool {
        true
    }

    // Constants for backward compatibility
    pub const ENABLE_MULTI_AGENT: bool = false;
    pub const ENABLE_TASK_MANAGEMENT: bool = true;
    pub const ENABLE_CONTEXT_SHARING: bool = true;
    pub const ENABLE_PERFORMANCE_MONITORING: bool = false;
    pub const MAX_CONCURRENT_SUBAGENTS: usize = 3;
    pub const CONTEXT_WINDOW_SIZE: usize = 100000;
    pub const MAX_CONTEXT_ITEMS: usize = 1000;
    pub const CONTEXT_STORE_ENABLED: bool = true;
    pub const DEBUG_MODE: bool = false;

    pub fn task_timeout() -> Duration {
        Duration::from_secs(300)
    }
}

/// Context store defaults
pub struct ContextStoreDefaults;

impl ContextStoreDefaults {
    pub fn max_size() -> usize {
        100000
    }
    pub fn compression() -> bool {
        true
    }

    // Constants for backward compatibility
    pub const MAX_CONTEXTS: usize = 1000;
    pub const AUTO_CLEANUP_DAYS: u32 = 7;
    pub const ENABLE_PERSISTENCE: bool = true;
    pub const COMPRESSION_ENABLED: bool = true;
    pub const STORAGE_DIR: &'static str = ".vtagent/context";
}

/// Performance defaults
pub struct PerformanceDefaults;

impl PerformanceDefaults {
    pub fn max_concurrent_operations() -> usize {
        10
    }
    pub fn timeout_seconds() -> u64 {
        30
    }
}

/// Scenario defaults
pub struct ScenarioDefaults;

impl ScenarioDefaults {
    pub fn max_scenarios() -> usize {
        10
    }
    pub fn default_timeout() -> u64 {
        300
    }

    // High performance scenario constants
    pub const HIGH_PERF_MAX_AGENTS: usize = 5;
    pub const HIGH_PERF_CONTEXT_WINDOW: usize = 200000;
    pub const HIGH_PERF_MAX_CONTEXTS: usize = 2000;

    // High quality scenario constants
    pub const HIGH_QUALITY_MAX_AGENTS: usize = 3;
    pub const HIGH_QUALITY_CONTEXT_WINDOW: usize = 150000;
    pub const HIGH_QUALITY_MAX_CONTEXTS: usize = 1500;

    // Balanced scenario constants
    pub const BALANCED_MAX_AGENTS: usize = 4;
    pub const BALANCED_CONTEXT_WINDOW: usize = 125000;
    pub const BALANCED_MAX_CONTEXTS: usize = 1250;

    pub fn high_perf_timeout() -> Duration {
        Duration::from_secs(180)
    }
    pub fn high_quality_timeout() -> Duration {
        Duration::from_secs(600)
    }
    pub fn balanced_timeout() -> Duration {
        Duration::from_secs(300)
    }
}
