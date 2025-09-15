use serde::Serialize;
use std::fs::{OpenOptions, create_dir_all};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct TrajectoryLogger {
    path: PathBuf,
    enabled: bool,
}

impl TrajectoryLogger {
    pub fn new(workspace: &Path) -> Self {
        let dir = workspace.join("logs");
        let _ = create_dir_all(&dir);
        let path = dir.join("trajectory.jsonl");
        Self {
            path,
            enabled: true,
        }
    }

    pub fn disabled() -> Self {
        Self {
            path: PathBuf::from("/dev/null"),
            enabled: false,
        }
    }

    pub fn log<T: Serialize>(&self, record: &T) {
        if !self.enabled {
            return;
        }
        if let Ok(line) = serde_json::to_string(record) {
            if let Ok(mut f) = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)
            {
                let _ = writeln!(f, "{}", line);
            }
        }
    }

    pub fn log_route(
        &self,
        turn: usize,
        selected_model: &str,
        class: &str,
        input_preview: &str,
    ) {
        #[derive(Serialize)]
        struct RouteRec<'a> {
            kind: &'static str,
            turn: usize,
            selected_model: &'a str,
            class: &'a str,
            input_preview: &'a str,
            ts: i64,
        }
        let rec = RouteRec {
            kind: "route",
            turn,
            selected_model,
            class,
            input_preview,
            ts: chrono::Utc::now().timestamp(),
        };
        self.log(&rec);
    }

    pub fn log_tool_call(&self, turn: usize, name: &str, args: &serde_json::Value, ok: bool) {
        #[derive(Serialize)]
        struct ToolRec<'a> {
            kind: &'static str,
            turn: usize,
            name: &'a str,
            args: serde_json::Value,
            ok: bool,
            ts: i64,
        }
        let rec = ToolRec {
            kind: "tool",
            turn,
            name,
            args: args.clone(),
            ok,
            ts: chrono::Utc::now().timestamp(),
        };
        self.log(&rec);
    }
}
