use once_cell::sync::Lazy;

/// Metadata describing a slash command supported by the chat interface.
#[derive(Clone, Copy, Debug)]
pub struct SlashCommandInfo {
    pub name: &'static str,
    pub description: &'static str,
}

/// Collection of slash command definitions in the order they should be displayed.
pub static SLASH_COMMANDS: Lazy<Vec<SlashCommandInfo>> = Lazy::new(|| {
    vec![
        SlashCommandInfo {
            name: "theme",
            description: "Switch UI theme (usage: /theme <theme-id>)",
        },
        SlashCommandInfo {
            name: "list-themes",
            description: "List all available UI themes",
        },
        SlashCommandInfo {
            name: "command",
            description: "Run a terminal command (usage: /command <program> [args...])",
        },
        SlashCommandInfo {
            name: "help",
            description: "Show slash command help",
        },
        SlashCommandInfo {
            name: "exit",
            description: "Exit the session",
        },
    ]
});

/// Returns slash command metadata that match the provided prefix (case insensitive).
pub fn suggestions_for(prefix: &str) -> Vec<&'static SlashCommandInfo> {
    if prefix.is_empty() {
        return SLASH_COMMANDS.iter().collect();
    }
    let query = prefix.to_ascii_lowercase();
    let mut matches: Vec<&SlashCommandInfo> = SLASH_COMMANDS
        .iter()
        .filter(|info| info.name.starts_with(&query))
        .collect();
    if matches.is_empty() {
        SLASH_COMMANDS.iter().collect()
    } else {
        matches.sort_by(|a, b| a.name.cmp(b.name));
        matches
    }
}
