use crate::config::telemetry::{DEFAULT_HOTPATH_PERCENTILE, HotpathConfig, HotpathReportFormat};
use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(feature = "hotpath")]
use hotpath::Format;

static PROFILER_ACTIVE: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy, Debug)]
pub enum ProfilerScope {
    MainStartup,
    AgentRunLoop,
    UnifiedAgentRunLoop,
    PromptRefinement,
    GeminiContextTrim,
    UnifiedContextTrim,
    GeminiToolPrune,
    UnifiedToolPrune,
    GeminiGenerate,
    UnifiedGenerate,
    ToolExecution,
    ToolPolicyConstraints,
    ToolOutputNormalization,
    SinglePromptGenerate,
}

impl ProfilerScope {
    pub const fn label(self) -> &'static str {
        match self {
            Self::MainStartup => "telemetry.hotpath.main_startup",
            Self::AgentRunLoop => "telemetry.hotpath.agent_loop",
            Self::UnifiedAgentRunLoop => "telemetry.hotpath.unified_agent_loop",
            Self::PromptRefinement => "telemetry.hotpath.prompt_refinement",
            Self::GeminiContextTrim => "telemetry.hotpath.gemini_context_trim",
            Self::UnifiedContextTrim => "telemetry.hotpath.unified_context_trim",
            Self::GeminiToolPrune => "telemetry.hotpath.gemini_tool_prune",
            Self::UnifiedToolPrune => "telemetry.hotpath.unified_tool_prune",
            Self::GeminiGenerate => "telemetry.hotpath.gemini_generate",
            Self::UnifiedGenerate => "telemetry.hotpath.unified_generate",
            Self::ToolExecution => "telemetry.hotpath.tool_execution",
            Self::ToolPolicyConstraints => "telemetry.hotpath.tool_policy_constraints",
            Self::ToolOutputNormalization => "telemetry.hotpath.tool_output_normalization",
            Self::SinglePromptGenerate => "telemetry.hotpath.single_prompt_generate",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ProfilerEnvKey {
    Enabled,
    Percentiles,
    Format,
}

impl ProfilerEnvKey {
    pub const fn key(self) -> &'static str {
        match self {
            Self::Enabled => "VTAGENT_HOTPATH_ENABLED",
            Self::Percentiles => "VTAGENT_HOTPATH_PERCENTILES",
            Self::Format => "VTAGENT_HOTPATH_FORMAT",
        }
    }
}

#[derive(Debug)]
pub struct ProfilerGuard {
    active: bool,
    #[cfg(feature = "hotpath")]
    guard: Option<hotpath::HotPath>,
}

impl ProfilerGuard {
    pub fn new(config: &HotpathConfig, scope: ProfilerScope) -> Self {
        let enabled = env_enabled_override().unwrap_or(config.enabled);
        if !enabled {
            PROFILER_ACTIVE.store(false, Ordering::Relaxed);
            return Self {
                active: false,
                #[cfg(feature = "hotpath")]
                guard: None,
            };
        }

        let percentiles = sanitize_percentiles(
            env_percentiles_override().unwrap_or_else(|| config.percentiles.clone()),
        );
        let format = env_format_override().unwrap_or(config.report_format);

        PROFILER_ACTIVE.store(true, Ordering::Relaxed);

        #[cfg(feature = "hotpath")]
        {
            let guard = hotpath::init(
                scope.label().to_string(),
                percentiles.as_slice(),
                convert_format(format),
            );
            return Self {
                active: true,
                guard: Some(guard),
            };
        }

        #[cfg(not(feature = "hotpath"))]
        {
            let _ = (percentiles, format, scope);
            Self { active: true }
        }
    }

    pub fn is_active(&self) -> bool {
        self.active
    }
}

impl Drop for ProfilerGuard {
    fn drop(&mut self) {
        if self.active {
            PROFILER_ACTIVE.store(false, Ordering::Relaxed);
        }
    }
}

pub fn start_profiler(config: &HotpathConfig, scope: ProfilerScope) -> ProfilerGuard {
    ProfilerGuard::new(config, scope)
}

pub fn is_profiler_active() -> bool {
    PROFILER_ACTIVE.load(Ordering::Relaxed)
}

pub fn measure_sync_scope<T, F>(scope: ProfilerScope, block: F) -> T
where
    F: FnOnce() -> T,
{
    if is_profiler_active() {
        #[cfg(feature = "hotpath")]
        {
            return hotpath::measure_block!(scope.label(), block());
        }
    }

    #[cfg(not(feature = "hotpath"))]
    let _ = scope;

    block()
}

pub async fn measure_async_scope<T, F, Fut>(scope: ProfilerScope, factory: F) -> T
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = T>,
{
    if is_profiler_active() {
        #[cfg(feature = "hotpath")]
        {
            let future = factory();
            return hotpath::measure_block!(scope.label(), async { future.await }.await);
        }
    }

    #[cfg(not(feature = "hotpath"))]
    let _ = scope;

    factory().await
}

fn env_enabled_override() -> Option<bool> {
    std::env::var(ProfilerEnvKey::Enabled.key())
        .ok()
        .and_then(|value| parse_bool(&value))
}

fn env_percentiles_override() -> Option<Vec<u8>> {
    std::env::var(ProfilerEnvKey::Percentiles.key())
        .ok()
        .and_then(|value| {
            let parsed: Vec<u8> = value
                .split(|c| matches!(c, ',' | ';' | ' '))
                .filter_map(|segment| {
                    let trimmed = segment.trim();
                    if trimmed.is_empty() {
                        return None;
                    }
                    trimmed.parse::<u8>().ok()
                })
                .collect();
            if parsed.is_empty() {
                None
            } else {
                Some(parsed)
            }
        })
}

fn env_format_override() -> Option<HotpathReportFormat> {
    std::env::var(ProfilerEnvKey::Format.key())
        .ok()
        .and_then(|value| parse_format(value.trim()))
}

fn parse_bool(value: &str) -> Option<bool> {
    let trimmed = value.trim();
    if trimmed.eq_ignore_ascii_case("true")
        || trimmed.eq_ignore_ascii_case("yes")
        || trimmed.eq_ignore_ascii_case("on")
        || trimmed == "1"
    {
        Some(true)
    } else if trimmed.eq_ignore_ascii_case("false")
        || trimmed.eq_ignore_ascii_case("no")
        || trimmed.eq_ignore_ascii_case("off")
        || trimmed == "0"
    {
        Some(false)
    } else {
        None
    }
}

fn parse_format(value: &str) -> Option<HotpathReportFormat> {
    if value.eq_ignore_ascii_case("table") {
        Some(HotpathReportFormat::Table)
    } else if value.eq_ignore_ascii_case("json") {
        Some(HotpathReportFormat::Json)
    } else if value.eq_ignore_ascii_case("json-pretty") || value.eq_ignore_ascii_case("json_pretty")
    {
        Some(HotpathReportFormat::JsonPretty)
    } else {
        None
    }
}

fn sanitize_percentiles(values: Vec<u8>) -> Vec<u8> {
    let mut filtered: Vec<u8> = values.into_iter().filter(|value| *value <= 100).collect();
    filtered.sort_unstable();
    filtered.dedup();
    if filtered.is_empty() {
        filtered.push(DEFAULT_HOTPATH_PERCENTILE);
    }
    filtered
}

#[cfg(feature = "hotpath")]
fn convert_format(format: HotpathReportFormat) -> Format {
    match format {
        HotpathReportFormat::Table => Format::Table,
        HotpathReportFormat::Json => Format::Json,
        HotpathReportFormat::JsonPretty => Format::JsonPretty,
    }
}
