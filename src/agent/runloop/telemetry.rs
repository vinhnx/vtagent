use std::path::Path;

use vtagent_core::config::loader::VTAgentConfig;
use vtagent_core::core::trajectory::TrajectoryLogger;

pub(crate) fn build_trajectory_logger(
    workspace: &Path,
    vt_cfg: Option<&VTAgentConfig>,
) -> TrajectoryLogger {
    vt_cfg
        .map(|cfg| cfg.telemetry.trajectory_enabled)
        .map(|enabled| {
            if enabled {
                TrajectoryLogger::new(workspace)
            } else {
                TrajectoryLogger::disabled()
            }
        })
        .unwrap_or_else(|| TrajectoryLogger::new(workspace))
}
