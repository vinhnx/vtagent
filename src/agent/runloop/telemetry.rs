use std::path::Path;

use vtcode_core::config::loader::VTCodeConfig;
use vtcode_core::core::trajectory::TrajectoryLogger;

pub(crate) fn build_trajectory_logger(
    workspace: &Path,
    vt_cfg: Option<&VTCodeConfig>,
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
