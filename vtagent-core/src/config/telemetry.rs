use serde::{Deserialize, Serialize};

pub const DEFAULT_HOTPATH_PERCENTILE: u8 = 99;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TelemetryConfig {
    #[serde(default = "default_true")]
    pub trajectory_enabled: bool,
    #[serde(default)]
    pub hotpath: HotpathConfig,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            trajectory_enabled: true,
            hotpath: HotpathConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HotpathConfig {
    #[serde(default = "default_hotpath_enabled")]
    pub enabled: bool,
    #[serde(default = "default_percentiles")]
    pub percentiles: Vec<u8>,
    #[serde(default)]
    pub report_format: HotpathReportFormat,
}

impl Default for HotpathConfig {
    fn default() -> Self {
        Self {
            enabled: default_hotpath_enabled(),
            percentiles: default_percentiles(),
            report_format: HotpathReportFormat::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HotpathReportFormat {
    Table,
    Json,
    JsonPretty,
}

impl Default for HotpathReportFormat {
    fn default() -> Self {
        Self::Table
    }
}

fn default_true() -> bool {
    true
}

fn default_hotpath_enabled() -> bool {
    false
}

fn default_percentiles() -> Vec<u8> {
    vec![DEFAULT_HOTPATH_PERCENTILE]
}
