use crate::config::constants::tools;
use crate::config::types::CapabilityLevel;

use super::ToolRegistry;
use super::registration::ToolRegistration;

pub(super) fn register_builtin_tools(registry: &mut ToolRegistry) {
    for registration in builtin_tool_registrations() {
        if registration.name() == tools::AST_GREP_SEARCH && registry.ast_grep_engine.is_none() {
            continue;
        }

        let tool_name = registration.name();
        if let Err(err) = registry.register_tool(registration) {
            eprintln!("Warning: Failed to register tool '{}': {}", tool_name, err);
        }
    }
}

pub(super) fn builtin_tool_registrations() -> Vec<ToolRegistration> {
    vec![
        ToolRegistration::new(
            tools::GREP_SEARCH,
            CapabilityLevel::CodeSearch,
            false,
            ToolRegistry::grep_search_executor,
        ),
        ToolRegistration::new(
            tools::LIST_FILES,
            CapabilityLevel::FileListing,
            false,
            ToolRegistry::list_files_executor,
        ),
        ToolRegistration::new(
            tools::RUN_TERMINAL_CMD,
            CapabilityLevel::Bash,
            true,
            ToolRegistry::run_terminal_cmd_executor,
        ),
        ToolRegistration::new(
            tools::READ_FILE,
            CapabilityLevel::FileReading,
            false,
            ToolRegistry::read_file_executor,
        ),
        ToolRegistration::new(
            tools::WRITE_FILE,
            CapabilityLevel::Editing,
            false,
            ToolRegistry::write_file_executor,
        ),
        ToolRegistration::new(
            tools::EDIT_FILE,
            CapabilityLevel::Editing,
            false,
            ToolRegistry::edit_file_executor,
        ),
        ToolRegistration::new(
            tools::AST_GREP_SEARCH,
            CapabilityLevel::CodeSearch,
            false,
            ToolRegistry::ast_grep_executor,
        ),
        ToolRegistration::new(
            tools::SIMPLE_SEARCH,
            CapabilityLevel::CodeSearch,
            false,
            ToolRegistry::simple_search_executor,
        ),
        ToolRegistration::new(
            tools::BASH,
            CapabilityLevel::CodeSearch,
            true,
            ToolRegistry::bash_executor,
        ),
        ToolRegistration::new(
            tools::APPLY_PATCH,
            CapabilityLevel::Editing,
            false,
            ToolRegistry::apply_patch_executor,
        )
        .with_llm_visibility(false),
        ToolRegistration::new(
            tools::SRGN,
            CapabilityLevel::CodeSearch,
            false,
            ToolRegistry::srgn_executor,
        ),
    ]
}
