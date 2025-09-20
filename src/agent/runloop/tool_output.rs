use anstyle::Style;
use once_cell::sync::Lazy;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::process::Command;
use std::sync::{Arc, Mutex, OnceLock};
use syntect::easy::HighlightLines;
use syntect::highlighting::{Theme, ThemeSet};
use syntect::parsing::{SyntaxReference, SyntaxSet};
use syntect::util::LinesWithEndings;
use vtcode_core::config::loader::{SyntaxHighlightingConfig, VTCodeConfig};
use vtcode_core::utils::ansi::{AnsiRenderer, MessageStyle};

const BYTES_PER_MB: usize = 1024 * 1024;

static SYNTAX_SET: Lazy<SyntaxSet> = Lazy::new(|| SyntaxSet::load_defaults_newlines());
static THEME_SET: Lazy<ThemeSet> = Lazy::new(ThemeSet::load_defaults);
static THEME_CACHE: Lazy<Mutex<HashMap<String, Arc<Theme>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub(crate) fn render_tool_output(
    tool_name: Option<&str>,
    val: &Value,
    vt_cfg: Option<&VTCodeConfig>,
) {
    let mut renderer = AnsiRenderer::stdout();
    let syntax_cfg = vt_cfg.map(|cfg| &cfg.syntax_highlighting);
    let command_text = val.get("command").and_then(|value| value.as_str());
    let program = command_text.and_then(command_program);
    let is_terminal_tool = matches!(tool_name, Some("run_terminal_cmd") | Some("bash"));
    let is_git_command = is_terminal_tool && matches!(program, Some("git"));
    let is_ls_command = is_terminal_tool && matches!(program, Some("ls"));

    let git_styles = if is_git_command {
        Some(GitStyles::current())
    } else {
        None
    };
    let ls_styles = if is_ls_command || matches!(tool_name, Some("list_files")) {
        Some(LsStyles::current())
    } else {
        None
    };

    if let Some(content) = val.get("content").and_then(|value| value.as_str()) {
        if !content.trim().is_empty() {
            let _ = renderer.line(MessageStyle::Tool, "[content]");
            let highlight_hint = val.get("path").and_then(|value| value.as_str());
            let highlighted =
                render_highlighted_block(&mut renderer, content, highlight_hint, syntax_cfg);
            if !highlighted {
                render_plain_block(&mut renderer, MessageStyle::Output, content);
            }
        }
    }

    if let Some(stdout) = val.get("stdout").and_then(|value| value.as_str()) {
        if !stdout.trim().is_empty() {
            let _ = renderer.line(MessageStyle::Tool, "[stdout]");
            let syntax_hint = syntax_hint_from_command(command_text);
            let highlightable_commands = matches!(
                program,
                Some("cat") | Some("bat") | Some("sed") | Some("head") | Some("tail")
            );
            let should_highlight = highlightable_commands && syntax_hint.is_some();
            let highlighted = if should_highlight {
                render_highlighted_block(&mut renderer, stdout, syntax_hint.as_deref(), syntax_cfg)
            } else {
                false
            };
            if !highlighted {
                for line in stdout.lines() {
                    if let Some(style) = select_line_style(
                        tool_name,
                        line,
                        is_git_command,
                        is_ls_command,
                        git_styles,
                        ls_styles,
                    ) {
                        let indented = format!("  {}", line);
                        let _ = renderer.line_with_style(style, &indented);
                    } else {
                        render_plain_line(&mut renderer, MessageStyle::Output, line);
                    }
                }
            }
        }
    }

    if matches!(tool_name, Some("list_files")) {
        if let Some(items) = val.get("items").and_then(|value| value.as_array()) {
            render_list_items(&mut renderer, items, ls_styles);
        }
        if let Some(message) = val.get("message").and_then(|value| value.as_str()) {
            let _ = renderer.line(MessageStyle::Info, message);
        }
    }

    if let Some(stderr) = val.get("stderr").and_then(|value| value.as_str()) {
        if !stderr.trim().is_empty() {
            let _ = renderer.line(MessageStyle::Tool, "[stderr]");
            let formatted = stderr
                .lines()
                .map(|line| format!("  {}", line))
                .collect::<Vec<_>>()
                .join("\n");
            let _ = renderer.line(MessageStyle::Error, &formatted);
        }
    }
}

fn render_plain_line(renderer: &mut AnsiRenderer, style: MessageStyle, line: &str) {
    let indented = format!("  {}", line);
    let _ = renderer.line(style, &indented);
}

fn render_plain_block(renderer: &mut AnsiRenderer, style: MessageStyle, text: &str) {
    for line in text.lines() {
        render_plain_line(renderer, style, line);
    }
}

fn render_list_items(renderer: &mut AnsiRenderer, items: &[Value], ls_styles: Option<&LsStyles>) {
    let _ = renderer.line(MessageStyle::Tool, "[items]");
    for item in items {
        if let Some(display) = format_list_item(item, ls_styles) {
            if let Some(style) = display.style {
                let _ = renderer.line_with_style(style, &display.text);
            } else {
                let _ = renderer.line(MessageStyle::Output, &display.text);
            }
        }
    }
}

struct ListItemDisplay {
    text: String,
    style: Option<Style>,
}

fn format_list_item(item: &Value, ls_styles: Option<&LsStyles>) -> Option<ListItemDisplay> {
    let name = item.get("name").and_then(|value| value.as_str())?;
    let mut display_name = name.to_string();
    let mut style = None;

    if let Some(entry_type) = item.get("type").and_then(|value| value.as_str()) {
        if entry_type == "directory" {
            display_name.push('/');
            if let Some(ls) = ls_styles {
                style = ls.directory_style();
            }
        }
    }

    let mut details = Vec::new();
    if let Some(size) = item.get("size").and_then(|value| value.as_u64()) {
        details.push(format!("{} bytes", size));
    }
    if let Some(path) = item.get("path").and_then(|value| value.as_str()) {
        if path != name {
            details.push(path.to_string());
        }
    }

    let mut line = format!("  {}", display_name);
    if !details.is_empty() {
        line.push_str("  (");
        line.push_str(&details.join(", "));
        line.push(')');
    }

    Some(ListItemDisplay { text: line, style })
}

fn render_highlighted_block(
    renderer: &mut AnsiRenderer,
    content: &str,
    hint: Option<&str>,
    cfg: Option<&SyntaxHighlightingConfig>,
) -> bool {
    let config = match cfg {
        Some(cfg) if cfg.enabled => cfg,
        _ => return false,
    };

    if !renderer.supports_color() {
        return false;
    }

    if config.max_file_size_mb > 0 {
        let max_bytes = config.max_file_size_mb.saturating_mul(BYTES_PER_MB);
        if content.as_bytes().len() > max_bytes {
            return false;
        }
    }

    let theme = match load_theme(&config.theme, config.cache_themes) {
        Some(theme) => theme,
        None => return false,
    };

    let syntax_set: &SyntaxSet = &SYNTAX_SET;
    let syntax = match choose_syntax(syntax_set, hint, content) {
        Some(syntax) => syntax,
        None => return false,
    };

    if !language_allowed(config, syntax) {
        return false;
    }

    let mut highlighter = HighlightLines::new(syntax, theme.as_ref());
    for line in LinesWithEndings::from(content) {
        let Ok(ranges) = highlighter.highlight_line(line, syntax_set) else {
            return false;
        };
        if ranges.is_empty() {
            let _ = renderer.line(MessageStyle::Output, "  ");
            continue;
        }
        let mut styled = Vec::with_capacity(ranges.len());
        for (style, segment) in ranges {
            styled.push((anstyle_syntect::to_anstyle(style), segment));
        }
        if renderer
            .line_segments(Some((Style::new(), "  ")), &styled)
            .is_err()
        {
            return false;
        }
    }

    true
}

fn load_theme(name: &str, cache: bool) -> Option<Arc<Theme>> {
    if cache {
        if let Ok(mut guard) = THEME_CACHE.lock() {
            if let Some(theme) = guard.get(name) {
                return Some(Arc::clone(theme));
            }
            let theme = theme_from_defaults(name)?;
            guard.insert(name.to_string(), Arc::clone(&theme));
            return Some(theme);
        }
    }

    theme_from_defaults(name)
}

fn theme_from_defaults(name: &str) -> Option<Arc<Theme>> {
    let default_theme_name = SyntaxHighlightingConfig::default().theme;
    let theme = THEME_SET
        .themes
        .get(name)
        .cloned()
        .or_else(|| THEME_SET.themes.get(default_theme_name.as_str()).cloned())?;
    Some(Arc::new(theme))
}

fn choose_syntax<'a>(
    syntax_set: &'a SyntaxSet,
    hint: Option<&str>,
    content: &str,
) -> Option<&'a SyntaxReference> {
    if let Some(hint) = hint {
        let cleaned = hint.trim_matches(|ch: char| "\"'`()[]{}".contains(ch));
        if !cleaned.is_empty() {
            let path = Path::new(cleaned);
            if let Ok(Some(syntax)) = syntax_set.find_syntax_for_file(path) {
                if !is_plain_text(syntax) {
                    return Some(syntax);
                }
            }
            if let Some(ext) = path.extension().and_then(|ext| ext.to_str()) {
                if let Some(syntax) = syntax_set.find_syntax_by_extension(ext) {
                    if !is_plain_text(syntax) {
                        return Some(syntax);
                    }
                }
            }
            if let Some(syntax) = syntax_set.find_syntax_by_token(cleaned) {
                if !is_plain_text(syntax) {
                    return Some(syntax);
                }
            }
        }
    }

    if let Some(first_line) = content.lines().next() {
        if let Some(syntax) = syntax_set.find_syntax_by_first_line(first_line) {
            if !is_plain_text(syntax) {
                return Some(syntax);
            }
        }
    }

    None
}

fn is_plain_text(syntax: &SyntaxReference) -> bool {
    syntax.name.eq_ignore_ascii_case("Plain Text")
}

fn language_allowed(cfg: &SyntaxHighlightingConfig, syntax: &SyntaxReference) -> bool {
    if cfg.enabled_languages.is_empty() {
        return true;
    }

    let mut allowed = HashSet::new();
    for lang in &cfg.enabled_languages {
        allowed.insert(lang.to_ascii_lowercase());
    }

    let syntax_name = syntax.name.to_ascii_lowercase();
    if allowed.contains(&syntax_name) {
        return true;
    }

    for ext in &syntax.file_extensions {
        if allowed.contains(&ext.to_ascii_lowercase()) {
            return true;
        }
    }

    false
}

fn syntax_hint_from_command(command: Option<&str>) -> Option<String> {
    let command = command?;
    let mut hint = None;
    for token in command.split_whitespace() {
        if token.starts_with('-') {
            continue;
        }
        let cleaned = clean_command_token(token);
        if cleaned.is_empty() {
            continue;
        }
        if cleaned.contains('.') || cleaned.contains('/') {
            hint = Some(cleaned);
        }
    }
    hint
}

fn clean_command_token(token: &str) -> String {
    token
        .trim_matches(|ch: char| "\"'`()[]{}".contains(ch))
        .trim_end_matches(|ch: char| matches!(ch, ',' | ';' | ':' | '|' | '&'))
        .to_string()
}

fn command_program(command: &str) -> Option<&str> {
    command.split_whitespace().next()
}

fn select_line_style(
    tool_name: Option<&str>,
    line: &str,
    is_git_command: bool,
    is_ls_command: bool,
    git: Option<&GitStyles>,
    ls: Option<&LsStyles>,
) -> Option<Style> {
    if !matches!(tool_name, Some("run_terminal_cmd") | Some("bash")) {
        return None;
    }

    let trimmed = line.trim_start();

    if is_git_command {
        if let Some(styles) = git {
            if (trimmed.starts_with("diff --") || trimmed.starts_with("index "))
                && styles.meta.is_some()
            {
                return styles.meta;
            }
            if trimmed.starts_with("@@") {
                return styles.frag.or(styles.meta);
            }
            if trimmed.starts_with('+') && !trimmed.starts_with("+++") {
                return styles.add;
            }
            if trimmed.starts_with('-') && !trimmed.starts_with("---") {
                return styles.remove;
            }
        }
    }

    if is_ls_command {
        if let Some(styles) = ls {
            if let Some(style) = styles.style_for_line(trimmed) {
                return Some(style);
            }
        }
    }

    None
}

#[derive(Clone, Copy, Default)]
struct GitStyles {
    add: Option<Style>,
    remove: Option<Style>,
    meta: Option<Style>,
    frag: Option<Style>,
}

impl GitStyles {
    fn current() -> &'static Self {
        static CACHE: OnceLock<GitStyles> = OnceLock::new();
        CACHE.get_or_init(Self::from_sources)
    }

    fn from_sources() -> Self {
        Self::from_git_config().unwrap_or_else(Self::defaults)
    }

    fn defaults() -> Self {
        Self {
            add: anstyle_git::parse("green").ok(),
            remove: anstyle_git::parse("red").ok(),
            meta: anstyle_git::parse("bold yellow").ok(),
            frag: anstyle_git::parse("cyan").ok(),
        }
    }

    fn from_git_config() -> Option<Self> {
        let add = git_config_style("color.diff.new");
        let remove = git_config_style("color.diff.old");
        let meta = git_config_style("color.diff.meta");
        let frag = git_config_style("color.diff.frag");

        if add.is_none() && remove.is_none() && meta.is_none() && frag.is_none() {
            None
        } else {
            Some(Self {
                add,
                remove,
                meta,
                frag,
            })
        }
    }
}

fn git_config_style(key: &str) -> Option<Style> {
    let output = Command::new("git")
        .args(["config", "--get", key])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8(output.stdout).ok()?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    anstyle_git::parse(trimmed).ok()
}

#[derive(Clone, Copy, Default)]
struct LsStyles {
    dir: Option<Style>,
    exec: Option<Style>,
    symlink: Option<Style>,
    pipe: Option<Style>,
    socket: Option<Style>,
    block: Option<Style>,
    char_device: Option<Style>,
    orphan: Option<Style>,
}

impl LsStyles {
    fn current() -> &'static Self {
        static CACHE: OnceLock<LsStyles> = OnceLock::new();
        CACHE.get_or_init(Self::from_sources)
    }

    fn from_sources() -> Self {
        let env_styles = Self::from_env();
        if env_styles.has_any() {
            env_styles
        } else {
            Self::defaults()
        }
    }

    fn has_any(&self) -> bool {
        self.dir.is_some()
            || self.exec.is_some()
            || self.symlink.is_some()
            || self.pipe.is_some()
            || self.socket.is_some()
            || self.block.is_some()
            || self.char_device.is_some()
            || self.orphan.is_some()
    }

    fn from_env() -> Self {
        if let Ok(ls_colors) = std::env::var("LS_COLORS") {
            let mut styles = Self::default();
            for part in ls_colors.split(':') {
                if let Some((key, value)) = part.split_once('=') {
                    let parsed = anstyle_ls::parse(value);
                    match key {
                        "di" => styles.dir = parsed,
                        "ex" => styles.exec = parsed,
                        "ln" => styles.symlink = parsed,
                        "pi" => styles.pipe = parsed,
                        "so" => styles.socket = parsed,
                        "bd" => styles.block = parsed,
                        "cd" => styles.char_device = parsed,
                        "or" => styles.orphan = parsed,
                        _ => {}
                    }
                }
            }
            if styles.has_any() {
                return styles;
            }
        }
        Self::defaults()
    }

    fn defaults() -> Self {
        Self {
            dir: anstyle_ls::parse("01;34"),
            exec: anstyle_ls::parse("01;32"),
            symlink: anstyle_ls::parse("01;36"),
            pipe: anstyle_ls::parse("33"),
            socket: anstyle_ls::parse("01;35"),
            block: anstyle_ls::parse("01;33"),
            char_device: anstyle_ls::parse("01;33"),
            orphan: anstyle_ls::parse("31"),
        }
    }

    fn style_for_line(&self, line: &str) -> Option<Style> {
        if line.is_empty() {
            return None;
        }
        if line.ends_with('/') {
            return self.dir;
        }
        if line.ends_with('*') {
            return self.exec;
        }
        if line.ends_with('@') {
            return self.symlink;
        }
        if line.ends_with('|') {
            return self.pipe;
        }
        if line.ends_with('=') {
            return self.socket;
        }
        if line.ends_with('%') {
            return self.char_device;
        }
        if line.ends_with('?') {
            return self.orphan;
        }

        if is_ls_long_format(line) {
            if let Some(first) = line.chars().next() {
                match first {
                    'd' => return self.dir,
                    'l' => return self.symlink.or(self.dir),
                    'p' => return self.pipe,
                    's' => return self.socket,
                    'c' => return self.char_device,
                    'b' => return self.block,
                    '-' => {
                        if has_execute_bits(&line[..10]) {
                            return self.exec;
                        }
                    }
                    _ => {}
                }
            }
        }

        None
    }
    fn directory_style(&self) -> Option<Style> {
        self.dir
    }
}

fn is_ls_long_format(line: &str) -> bool {
    if line.len() < 10 {
        return false;
    }
    let perms = &line[..10];
    perms.chars().enumerate().all(|(idx, ch)| match idx {
        0 => matches!(ch, '-' | 'd' | 'l' | 'p' | 's' | 'c' | 'b'),
        _ => matches!(ch, '-' | 'r' | 'w' | 'x' | 's' | 't'),
    })
}

fn has_execute_bits(perms: &str) -> bool {
    perms
        .chars()
        .enumerate()
        .any(|(idx, ch)| idx > 0 && (idx % 3 == 0) && matches!(ch, 'x' | 's' | 't'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use anstyle::AnsiColor;

    #[test]
    fn detects_git_diff_styling() {
        let git = GitStyles::defaults();
        let added = select_line_style(
            Some("run_terminal_cmd"),
            "+added line",
            true,
            false,
            Some(&git),
            None,
        );
        assert_eq!(added, git.add);
        let removed = select_line_style(
            Some("run_terminal_cmd"),
            "-removed line",
            true,
            false,
            Some(&git),
            None,
        );
        assert_eq!(removed, git.remove);
        let header = select_line_style(
            Some("run_terminal_cmd"),
            "diff --git a/file b/file",
            true,
            false,
            Some(&git),
            None,
        );
        assert_eq!(header, git.meta);
    }

    #[test]
    fn detects_ls_styles_for_directories_and_executables() {
        let mut ls = LsStyles::default();
        let dir_style = Style::new().bold();
        let exec_style = Style::new().fg_color(Some(anstyle::Color::Ansi(AnsiColor::Green)));
        ls.dir = Some(dir_style);
        ls.exec = Some(exec_style);

        let directory = select_line_style(
            Some("run_terminal_cmd"),
            "folder/",
            false,
            true,
            None,
            Some(&ls),
        );
        assert_eq!(directory, Some(dir_style));
        let executable = select_line_style(
            Some("run_terminal_cmd"),
            "script*",
            false,
            true,
            None,
            Some(&ls),
        );
        assert_eq!(executable, Some(exec_style));
        let long_listing = select_line_style(
            Some("run_terminal_cmd"),
            "-rwxr-xr-x 1 user group 0 Jan  1 00:00 script",
            false,
            true,
            None,
            Some(&ls),
        );
        assert_eq!(long_listing, Some(exec_style));
    }

    #[test]
    fn non_terminal_tools_do_not_apply_special_styles() {
        let git = GitStyles::defaults();
        let result = select_line_style(Some("context7"), "+added", true, false, Some(&git), None);
        assert!(result.is_none());
    }

    #[test]
    fn extracts_syntax_hint_from_command() {
        let hint = syntax_hint_from_command(Some("cat src/main.rs"));
        assert_eq!(hint.as_deref(), Some("src/main.rs"));
    }
}
