use anyhow::{Result, anyhow};

use super::ToolRegistry;

impl ToolRegistry {
    pub fn pty_config(&self) -> &crate::config::PtyConfig {
        &self.pty_config
    }

    pub fn can_start_pty_session(&self) -> bool {
        if !self.pty_config.enabled {
            return false;
        }
        self.active_pty_sessions
            .load(std::sync::atomic::Ordering::SeqCst)
            < self.pty_config.max_sessions
    }

    pub fn start_pty_session(&self) -> Result<()> {
        if !self.can_start_pty_session() {
            return Err(anyhow!(
                "Maximum PTY sessions ({}) exceeded. Current active sessions: {}",
                self.pty_config.max_sessions,
                self.active_pty_sessions
                    .load(std::sync::atomic::Ordering::SeqCst)
            ));
        }
        self.active_pty_sessions
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }

    pub fn end_pty_session(&self) {
        let current = self
            .active_pty_sessions
            .load(std::sync::atomic::Ordering::SeqCst);
        if current > 0 {
            self.active_pty_sessions
                .fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
        }
    }

    pub fn active_pty_sessions(&self) -> usize {
        self.active_pty_sessions
            .load(std::sync::atomic::Ordering::SeqCst)
    }
}
