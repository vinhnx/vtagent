//! CLI command implementations with modular architecture

pub mod chat;
pub mod analyze;
pub mod create_project;
pub mod init;
pub mod init_project;
pub mod config;

// Re-export command handlers for backward compatibility
pub use chat::handle_chat_command;
pub use analyze::handle_analyze_command;
pub use create_project::handle_create_project_command;
pub use init::handle_init_command;
pub use init_project::handle_init_project_command;
pub use config::handle_config_command;
