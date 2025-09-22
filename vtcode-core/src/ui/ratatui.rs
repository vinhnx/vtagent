use crate::ui::slash::{SlashCommandInfo, suggestions_for};
use crate::ui::theme;
use anstyle::{AnsiColor, Color as AnsiColorEnum, Effects, Style as AnsiStyle};
use anyhow::{Context, Result};
use chrono::Local;
use crossterm::{
    ExecutableCommand, cursor,
    event::{
        DisableMouseCapture, EnableMouseCapture, Event as CrosstermEvent, EventStream, KeyCode,
        KeyEvent, KeyEventKind, KeyModifiers, MouseEvent, MouseEventKind,
    },
    terminal::{Clear, ClearType, disable_raw_mode, enable_raw_mode},
};
use futures::StreamExt;
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Clear as ClearWidget, List, ListItem, ListState, Paragraph,
    },
};
use serde_json::Value;
use std::cmp;
use std::io;
use std::mem;
use std::time::{Duration, Instant};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio::time::{Interval, MissedTickBehavior, interval};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

const ESCAPE_DOUBLE_MS: u64 = 750;
const REDRAW_INTERVAL_MS: u64 = 33;
const MESSAGE_INDENT: usize = 2;
const NAVIGATION_HINT_TEXT: &str = "↵ send · Esc cancel · Ctrl+Shift+M mouse";

#[derive(Clone, Default)]
pub struct RatatuiTextStyle {
    pub color: Option<Color>,
    pub bold: bool,
    pub italic: bool,
}

impl RatatuiTextStyle {
    pub fn merge_color(mut self, fallback: Option<Color>) -> Self {
        if self.color.is_none() {
            self.color = fallback;
        }
        self
    }

    fn to_style(&self, fallback: Option<Color>) -> Style {
        let mut style = Style::default();
        if let Some(color) = self.color.or(fallback) {
            style = style.fg(color);
        }
        if self.bold {
            style = style.add_modifier(Modifier::BOLD);
        }
        if self.italic {
            style = style.add_modifier(Modifier::ITALIC);
        }
        style
    }
}

#[derive(Clone, Default)]
pub struct RatatuiSegment {
    pub text: String,
    pub style: RatatuiTextStyle,
}

#[derive(Clone, Default)]
struct StyledLine {
    segments: Vec<RatatuiSegment>,
}

impl StyledLine {
    fn push_segment(&mut self, segment: RatatuiSegment) {
        if segment.text.is_empty() {
            return;
        }
        self.segments.push(segment);
    }

    fn has_visible_content(&self) -> bool {
        self.segments
            .iter()
            .any(|segment| segment.text.chars().any(|ch| !ch.is_whitespace()))
    }
}

#[derive(Clone)]
pub struct RatatuiTheme {
    pub background: Option<Color>,
    pub foreground: Option<Color>,
    pub primary: Option<Color>,
    pub secondary: Option<Color>,
}

impl Default for RatatuiTheme {
    fn default() -> Self {
        Self {
            background: None,
            foreground: None,
            primary: None,
            secondary: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RatatuiMessageKind {
    Agent,
    Error,
    Info,
    Policy,
    Pty,
    Tool,
    User,
}

pub enum RatatuiCommand {
    AppendLine {
        kind: RatatuiMessageKind,
        segments: Vec<RatatuiSegment>,
    },
    Inline {
        kind: RatatuiMessageKind,
        segment: RatatuiSegment,
    },
    ReplaceLast {
        count: usize,
        kind: RatatuiMessageKind,
        lines: Vec<Vec<RatatuiSegment>>,
    },
    SetPrompt {
        prefix: String,
        style: RatatuiTextStyle,
    },
    SetPlaceholder {
        hint: Option<String>,
    },
    SetTheme {
        theme: RatatuiTheme,
    },
    UpdateStatusBar {
        left: Option<String>,
        center: Option<String>,
        right: Option<String>,
    },
    Shutdown,
}

#[derive(Debug, Clone)]
pub enum RatatuiEvent {
    Submit(String),
    Cancel,
    Exit,
    Interrupt,
    ScrollLineUp,
    ScrollLineDown,
    ScrollPageUp,
    ScrollPageDown,
}

#[derive(Clone)]
pub struct RatatuiHandle {
    sender: UnboundedSender<RatatuiCommand>,
}

impl RatatuiHandle {
    pub fn append_line(&self, kind: RatatuiMessageKind, segments: Vec<RatatuiSegment>) {
        if segments.is_empty() {
            let _ = self.sender.send(RatatuiCommand::AppendLine {
                kind,
                segments: vec![RatatuiSegment::default()],
            });
        } else {
            let _ = self
                .sender
                .send(RatatuiCommand::AppendLine { kind, segments });
        }
    }

    pub fn inline(&self, kind: RatatuiMessageKind, segment: RatatuiSegment) {
        let _ = self.sender.send(RatatuiCommand::Inline { kind, segment });
    }

    pub fn replace_last(
        &self,
        count: usize,
        kind: RatatuiMessageKind,
        lines: Vec<Vec<RatatuiSegment>>,
    ) {
        let _ = self
            .sender
            .send(RatatuiCommand::ReplaceLast { count, kind, lines });
    }

    pub fn set_prompt(&self, prefix: String, style: RatatuiTextStyle) {
        let _ = self
            .sender
            .send(RatatuiCommand::SetPrompt { prefix, style });
    }

    pub fn set_placeholder(&self, hint: Option<String>) {
        let _ = self.sender.send(RatatuiCommand::SetPlaceholder { hint });
    }

    pub fn set_theme(&self, theme: RatatuiTheme) {
        let _ = self.sender.send(RatatuiCommand::SetTheme { theme });
    }

    pub fn update_status_bar(
        &self,
        left: Option<String>,
        center: Option<String>,
        right: Option<String>,
    ) {
        let _ = self.sender.send(RatatuiCommand::UpdateStatusBar {
            left,
            center,
            right,
        });
    }

    pub fn shutdown(&self) {
        let _ = self.sender.send(RatatuiCommand::Shutdown);
    }
}

pub struct RatatuiSession {
    pub handle: RatatuiHandle,
    pub events: UnboundedReceiver<RatatuiEvent>,
}

pub fn spawn_session(theme: RatatuiTheme, placeholder: Option<String>) -> Result<RatatuiSession> {
    let (command_tx, command_rx) = mpsc::unbounded_channel();
    let (event_tx, event_rx) = mpsc::unbounded_channel();

    tokio::spawn(async move {
        if let Err(err) = run_ratatui(command_rx, event_tx, theme, placeholder).await {
            tracing::error!(error = ?err, "ratatui session terminated unexpectedly");
        }
    });

    Ok(RatatuiSession {
        handle: RatatuiHandle { sender: command_tx },
        events: event_rx,
    })
}

async fn run_ratatui(
    commands: UnboundedReceiver<RatatuiCommand>,
    events: UnboundedSender<RatatuiEvent>,
    theme: RatatuiTheme,
    placeholder: Option<String>,
) -> Result<()> {
    let mut stdout = io::stdout();
    let backend = CrosstermBackend::new(&mut stdout);
    let mut terminal = Terminal::new(backend).context("failed to initialize ratatui terminal")?;
    let _guard = TerminalGuard::new().context("failed to configure terminal for ratatui")?;
    terminal
        .clear()
        .context("failed to clear terminal for ratatui")?;

    let mut app = RatatuiLoop::new(theme, placeholder);
    let mut command_rx = commands;
    let mut event_stream = EventStream::new();
    let mut redraw = true;
    let mut ticker = create_ticker();
    loop {
        if redraw {
            terminal
                .draw(|frame| app.draw(frame))
                .context("failed to draw ratatui frame")?;
            redraw = false;
        }

        tokio::select! {
            Some(cmd) = command_rx.recv() => {
                if app.handle_command(cmd) {
                    redraw = true;
                }
                if app.should_exit() {
                    break;
                }
            }
            event = event_stream.next() => {
                match event {
                    Some(Ok(evt)) => {
                        if matches!(evt, CrosstermEvent::Resize(_, _)) {
                            terminal
                                .autoresize()
                                .context("failed to autoresize terminal viewport")?;
                        }
                        if app.handle_event(evt, &events)? {
                            redraw = true;
                        }
                        if app.should_exit() {
                            break;
                        }
                    }
                    Some(Err(_)) => {
                        redraw = true;
                    }
                    None => {}
                }
            }
            _ = ticker.tick() => {
                if app.needs_tick() {
                    redraw = true;
                }
            }
        }

        if app.should_exit() {
            break;
        }
    }

    terminal.show_cursor().ok();
    terminal
        .clear()
        .context("failed to clear terminal after ratatui session")?;

    Ok(())
}

struct TerminalGuard;

impl TerminalGuard {
    fn new() -> Result<Self> {
        enable_raw_mode().context("failed to enable raw mode")?;
        let mut stdout = io::stdout();
        stdout
            .execute(cursor::Hide)
            .context("failed to hide cursor")?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let mut stdout = io::stdout();
        let _ = stdout.execute(DisableMouseCapture);
        let _ = stdout.execute(cursor::Show);
        let _ = stdout.execute(Clear(ClearType::FromCursorDown));
    }
}

#[derive(Default)]
struct InputState {
    value: String,
    cursor: usize,
}

impl InputState {
    fn clear(&mut self) {
        self.value.clear();
        self.cursor = 0;
    }

    fn insert(&mut self, ch: char) {
        self.value.insert(self.cursor, ch);
        self.cursor += ch.len_utf8();
    }

    fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let new_cursor = self.value[..self.cursor]
            .chars()
            .next_back()
            .map(|ch| self.cursor - ch.len_utf8())
            .unwrap_or(0);
        self.value.replace_range(new_cursor..self.cursor, "");
        self.cursor = new_cursor;
    }

    fn delete(&mut self) {
        if self.cursor >= self.value.len() {
            return;
        }
        let len = self.value[self.cursor..]
            .chars()
            .next()
            .map(|ch| ch.len_utf8())
            .unwrap_or(0);
        let end = self.cursor + len;
        self.value.replace_range(self.cursor..end, "");
    }

    fn move_left(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let new_cursor = self.value[..self.cursor]
            .chars()
            .next_back()
            .map(|ch| self.cursor - ch.len_utf8())
            .unwrap_or(0);
        self.cursor = new_cursor;
    }

    fn move_right(&mut self) {
        if self.cursor >= self.value.len() {
            return;
        }
        let advance = self.value[self.cursor..]
            .chars()
            .next()
            .map(|ch| ch.len_utf8())
            .unwrap_or(0);
        self.cursor += advance;
    }

    fn move_home(&mut self) {
        self.cursor = 0;
    }

    fn move_end(&mut self) {
        self.cursor = self.value.len();
    }

    fn take(&mut self) -> String {
        let mut result = String::new();
        mem::swap(&mut result, &mut self.value);
        self.cursor = 0;
        result
    }

    fn value(&self) -> &str {
        &self.value
    }

    fn width_before_cursor(&self) -> usize {
        UnicodeWidthStr::width(&self.value[..self.cursor])
    }
}

#[derive(Clone)]
struct MessageBlock {
    kind: RatatuiMessageKind,
    lines: Vec<StyledLine>,
    timestamp: Option<String>,
    pty_command: Option<String>,
}

#[derive(Clone)]
struct RenderedMessage {
    lines: Vec<Line<'static>>,
    decoration: MessageDecoration,
    height: u16,
    cursor: Option<(u16, u16)>,
}

impl RenderedMessage {
    fn plain(lines: Vec<Line<'static>>) -> Self {
        Self::with_decoration(lines, MessageDecoration::None, None)
    }

    fn block(
        lines: Vec<Line<'static>>,
        border_color: Color,
        border_type: BorderType,
        title: Option<Line<'static>>,
    ) -> Self {
        Self::with_decoration(
            lines,
            MessageDecoration::Block {
                border_color,
                border_type,
                title,
            },
            None,
        )
    }

    fn block_with_cursor(
        lines: Vec<Line<'static>>,
        border_color: Color,
        border_type: BorderType,
        title: Option<Line<'static>>,
        cursor: Option<(u16, u16)>,
    ) -> Self {
        Self::with_decoration(
            lines,
            MessageDecoration::Block {
                border_color,
                border_type,
                title,
            },
            cursor,
        )
    }

    fn spacer() -> Self {
        Self::plain(vec![Line::default()])
    }

    fn with_decoration(
        mut lines: Vec<Line<'static>>,
        decoration: MessageDecoration,
        cursor: Option<(u16, u16)>,
    ) -> Self {
        if lines.is_empty() {
            lines.push(Line::default());
        }
        let mut height = lines.len() as u16;
        if decoration.is_block() {
            height = height.saturating_add(2);
        }
        Self {
            lines,
            decoration,
            height: height.max(1),
            cursor,
        }
    }

    fn height(&self) -> u16 {
        self.height
    }

    fn render(&self, frame: &mut Frame, area: Rect, base_style: Style) {
        let mut paragraph = Paragraph::new(self.lines.clone())
            .alignment(Alignment::Left)
            .style(base_style.clone());
        if let MessageDecoration::Block {
            border_color,
            border_type,
            title,
        } = &self.decoration
        {
            let mut block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(*border_color))
                .border_type(*border_type);
            if let Some(line) = title.clone() {
                block = block.title(line);
            }
            paragraph = paragraph.block(block);
        }
        frame.render_widget(paragraph, area);
    }

    fn cursor(&self) -> Option<(u16, u16)> {
        self.cursor
    }
}

#[derive(Clone)]
enum MessageDecoration {
    None,
    Block {
        border_color: Color,
        border_type: BorderType,
        title: Option<Line<'static>>,
    },
}

impl MessageDecoration {
    fn is_block(&self) -> bool {
        matches!(self, MessageDecoration::Block { .. })
    }
}

#[derive(Clone, Default)]
struct StatusBarContent {
    left: String,
    center: String,
    right: String,
}

impl StatusBarContent {
    fn new() -> Self {
        Self {
            left: "exit · /help · /theme".to_string(),
            center: String::new(),
            right: NAVIGATION_HINT_TEXT.to_string(),
        }
    }

    fn update(&mut self, left: Option<String>, center: Option<String>, right: Option<String>) {
        if let Some(value) = left {
            self.left = value;
        }
        if let Some(value) = center {
            self.center = value;
        }
        if let Some(value) = right {
            self.right = value;
        }
    }
}

#[derive(Default)]
struct SlashSuggestionState {
    items: Vec<&'static SlashCommandInfo>,
    list_state: ListState,
}

impl SlashSuggestionState {
    fn clear(&mut self) {
        self.items.clear();
        self.list_state.select(None);
    }

    fn update(&mut self, query: &str) {
        self.items = suggestions_for(query);
        if self.items.is_empty() {
            self.list_state.select(None);
        } else {
            self.list_state.select(Some(0));
        }
    }

    fn is_visible(&self) -> bool {
        !self.items.is_empty()
    }

    fn items(&self) -> &[&'static SlashCommandInfo] {
        &self.items
    }

    fn list_state(&mut self) -> &mut ListState {
        &mut self.list_state
    }

    fn selected_index(&self) -> Option<usize> {
        self.list_state.selected()
    }

    fn select_previous(&mut self) -> bool {
        if self.items.is_empty() {
            return false;
        }
        let current = self.list_state.selected().unwrap_or(0);
        let len = self.items.len();
        let next = if current == 0 {
            len.saturating_sub(1)
        } else {
            current.saturating_sub(1)
        };
        if len == 0 {
            self.list_state.select(None);
            return false;
        }
        if current != next {
            self.list_state.select(Some(next));
        } else {
            self.list_state.select(Some(next));
        }
        true
    }

    fn select_next(&mut self) -> bool {
        if self.items.is_empty() {
            return false;
        }
        let len = self.items.len();
        let current = self.list_state.selected().unwrap_or(0);
        let next = if current + 1 >= len { 0 } else { current + 1 };
        self.list_state.select(Some(next));
        true
    }

    fn selected(&self) -> Option<&'static SlashCommandInfo> {
        let index = self.list_state.selected()?;
        self.items.get(index).copied()
    }
}

struct RatatuiLoop {
    messages: Vec<MessageBlock>,
    current_line: StyledLine,
    current_kind: Option<RatatuiMessageKind>,
    current_active: bool,
    prompt_prefix: String,
    prompt_style: RatatuiTextStyle,
    input: InputState,
    placeholder_hint: Option<String>,
    show_placeholder: bool,
    should_exit: bool,
    theme: RatatuiTheme,
    last_escape: Option<Instant>,
    transcript_area: Option<Rect>,
    slash_suggestions: SlashSuggestionState,
    status_bar: StatusBarContent,
    mouse_capture_enabled: bool,
    pending_pty_command: Option<String>,
}

impl RatatuiLoop {
    fn new(theme: RatatuiTheme, placeholder: Option<String>) -> Self {
        let mut instance = Self {
            messages: Vec::new(),
            current_line: StyledLine::default(),
            current_kind: None,
            current_active: false,
            prompt_prefix: "❯ ".to_string(),
            prompt_style: RatatuiTextStyle::default(),
            input: InputState::default(),
            placeholder_hint: placeholder.clone(),
            show_placeholder: placeholder.is_some(),
            should_exit: false,
            theme,
            last_escape: None,
            transcript_area: None,
            slash_suggestions: SlashSuggestionState::default(),
            status_bar: StatusBarContent::new(),
            mouse_capture_enabled: false,
            pending_pty_command: None,
        };
        instance.status_bar.center = Self::mouse_capture_text(false);
        instance
    }

    fn should_exit(&self) -> bool {
        self.should_exit
    }

    fn needs_tick(&self) -> bool {
        false
    }

    fn mouse_capture_text(enabled: bool) -> String {
        if enabled {
            "Mouse capture on · pointer events".to_string()
        } else {
            "Mouse capture off · drag to select".to_string()
        }
    }

    fn set_mouse_capture(&mut self, enabled: bool) -> Result<()> {
        if self.mouse_capture_enabled == enabled {
            return Ok(());
        }

        let mut stdout = io::stdout();
        if enabled {
            stdout
                .execute(EnableMouseCapture)
                .context("failed to enable mouse capture")?;
        } else {
            stdout
                .execute(DisableMouseCapture)
                .context("failed to disable mouse capture")?;
        }
        self.mouse_capture_enabled = enabled;
        self.status_bar.center = Self::mouse_capture_text(enabled);
        Ok(())
    }

    fn toggle_mouse_capture(&mut self) -> Result<()> {
        let enabled = !self.mouse_capture_enabled;
        self.set_mouse_capture(enabled)
    }

    fn handle_command(&mut self, command: RatatuiCommand) -> bool {
        match command {
            RatatuiCommand::AppendLine { kind, segments } => {
                let plain = Self::collect_plain_text(&segments);
                self.track_pty_metadata(kind, &plain);
                let was_active = self.current_active;
                self.flush_current_line(was_active);
                self.push_line(kind, StyledLine { segments });
                true
            }
            RatatuiCommand::Inline { kind, segment } => {
                self.append_inline_segment(kind, segment);
                true
            }
            RatatuiCommand::ReplaceLast { count, kind, lines } => {
                let was_active = self.current_active;
                self.flush_current_line(was_active);
                if kind == RatatuiMessageKind::Tool {
                    if let Some(first_line) = lines.first() {
                        let plain = Self::collect_plain_text(first_line);
                        self.track_pty_metadata(kind, &plain);
                    }
                }
                self.remove_last_lines(count);
                for segments in lines {
                    self.push_line(kind, StyledLine { segments });
                }
                true
            }
            RatatuiCommand::SetPrompt { prefix, style } => {
                self.prompt_prefix = prefix;
                self.prompt_style = style;
                true
            }
            RatatuiCommand::SetPlaceholder { hint } => {
                self.placeholder_hint = hint.clone();
                self.update_input_state();
                true
            }
            RatatuiCommand::SetTheme { theme } => {
                self.theme = theme;
                self.status_bar.center = Self::mouse_capture_text(self.mouse_capture_enabled);
                true
            }
            RatatuiCommand::UpdateStatusBar {
                left,
                center,
                right,
            } => {
                self.status_bar.update(left, center, right);
                true
            }
            RatatuiCommand::Shutdown => {
                self.should_exit = true;
                true
            }
        }
    }

    fn collect_plain_text(segments: &[RatatuiSegment]) -> String {
        segments
            .iter()
            .map(|segment| segment.text.as_str())
            .collect::<String>()
    }

    fn append_inline_segment(&mut self, kind: RatatuiMessageKind, segment: RatatuiSegment) {
        let text = segment.text;
        let style = segment.style;
        if text.is_empty() {
            return;
        }

        if self.current_kind != Some(kind) {
            if self.current_active {
                self.flush_current_line(true);
            }
            self.current_kind = Some(kind);
        }

        let mut parts = text.split('\n').peekable();
        let ends_with_newline = text.ends_with('\n');

        while let Some(part) = parts.next() {
            if !part.is_empty() {
                if self.current_kind != Some(kind) {
                    self.current_kind = Some(kind);
                }
                self.current_line.push_segment(RatatuiSegment {
                    text: part.to_string(),
                    style: style.clone(),
                });
                self.current_active = true;
            }

            if parts.peek().is_some() {
                self.flush_current_line(true);
                self.current_kind = Some(kind);
            }
        }

        if ends_with_newline {
            self.flush_current_line(true);
            self.current_kind = Some(kind);
        }
    }

    fn flush_current_line(&mut self, force: bool) {
        if !force && !self.current_active {
            return;
        }

        if let Some(kind) = self.current_kind {
            if !self.current_line.segments.is_empty() || force {
                let line = mem::take(&mut self.current_line);
                self.push_line(kind, line);
            } else {
                self.current_line = StyledLine::default();
            }
        }

        self.current_line = StyledLine::default();
        self.current_active = false;
        self.current_kind = None;
    }

    fn update_input_state(&mut self) {
        self.show_placeholder = self.placeholder_hint.is_some() && self.input.value().is_empty();
        self.refresh_slash_suggestions();
    }

    fn refresh_slash_suggestions(&mut self) {
        if let Some(rest) = self.input.value().strip_prefix('/') {
            let trimmed = rest.trim_start();
            if trimmed.chars().any(char::is_whitespace) {
                self.slash_suggestions.clear();
                return;
            }
            let query = trimmed.trim_end();
            self.slash_suggestions.update(query);
        } else {
            self.slash_suggestions.clear();
        }
    }

    fn set_input_text(&mut self, value: String) {
        self.input.value = value;
        self.input.cursor = self.input.value.len();
        self.update_input_state();
    }

    fn apply_selected_suggestion(&mut self) -> bool {
        let Some(selected) = self.slash_suggestions.selected() else {
            return false;
        };
        let raw = self.input.value().to_string();
        let remainder = raw
            .strip_prefix('/')
            .and_then(|rest| {
                rest.char_indices()
                    .find(|(_, ch)| ch.is_whitespace())
                    .map(|(idx, _)| rest[idx..].trim_start().to_string())
            })
            .unwrap_or_default();

        let mut new_value = format!("/{}", selected.name);
        if remainder.is_empty() {
            new_value.push(' ');
        } else {
            new_value.push(' ');
            new_value.push_str(&remainder);
        }
        self.set_input_text(new_value);
        true
    }

    fn push_line(&mut self, kind: RatatuiMessageKind, line: StyledLine) {
        if let Some(block) = self.messages.last_mut() {
            if block.kind == kind {
                if kind == RatatuiMessageKind::Pty && block.pty_command.is_none() {
                    block.pty_command = self.pending_pty_command.clone();
                }
                block.lines.push(line);
                return;
            }
        }

        let timestamp = if kind == RatatuiMessageKind::User {
            Some(Local::now().format("%H:%M:%S").to_string())
        } else {
            None
        };
        self.messages.push(MessageBlock {
            kind,
            lines: vec![line],
            timestamp,
            pty_command: if kind == RatatuiMessageKind::Pty {
                self.pending_pty_command.clone()
            } else {
                None
            },
        });
    }

    fn remove_last_lines(&mut self, mut count: usize) {
        while count > 0 {
            let Some(block) = self.messages.last_mut() else {
                break;
            };

            if block.lines.len() <= count {
                count -= block.lines.len();
                self.messages.pop();
            } else {
                let new_len = block.lines.len() - count;
                block.lines.truncate(new_len);
                count = 0;
            }
        }
    }

    fn track_pty_metadata(&mut self, kind: RatatuiMessageKind, plain: &str) {
        if kind != RatatuiMessageKind::Tool {
            return;
        }
        let trimmed = plain.trim();
        if let Some(rest) = trimmed.strip_prefix("[TOOL]") {
            let command_text = rest.trim_start();
            if let Some(json_part) = command_text.strip_prefix("run_terminal_cmd") {
                if let Some(command) = Self::parse_run_command(json_part.trim()) {
                    self.pending_pty_command = Some(command.join(" "));
                }
            } else if !command_text.starts_with('[') {
                self.pending_pty_command = None;
            }
        }
    }

    fn parse_run_command(json_segment: &str) -> Option<Vec<String>> {
        let value: Value = serde_json::from_str(json_segment).ok()?;
        let array = value.get("command")?.as_array()?;
        let mut command = Vec::with_capacity(array.len());
        for entry in array {
            if let Some(text) = entry.as_str() {
                command.push(text.to_string());
            }
        }
        if command.is_empty() {
            None
        } else {
            Some(command)
        }
    }

    fn render_slash_suggestions(&mut self, frame: &mut Frame, area: Rect) {
        if !self.slash_suggestions.is_visible() {
            return;
        }
        if area.width <= 4 || area.height == 0 {
            return;
        }

        const MAX_VISIBLE: usize = 6;
        let items: Vec<&SlashCommandInfo> = self
            .slash_suggestions
            .items()
            .iter()
            .take(MAX_VISIBLE)
            .copied()
            .collect();
        if items.is_empty() {
            return;
        }

        let visible_len = items.len();
        if let Some(selected) = self.slash_suggestions.selected_index() {
            if visible_len > 0 && selected >= visible_len {
                self.slash_suggestions
                    .list_state
                    .select(Some(visible_len.saturating_sub(1)));
            }
        }

        let max_name_len = items.iter().map(|info| info.name.len()).max().unwrap_or(0);
        let entries: Vec<String> = items
            .iter()
            .map(|info| {
                let mut line = format!("/{:<width$}", info.name, width = max_name_len);
                line.push(' ');
                line.push_str(info.description);
                line
            })
            .collect();

        let max_width = entries
            .iter()
            .map(|value| UnicodeWidthStr::width(value.as_str()))
            .max()
            .unwrap_or(0);
        let required_width = cmp::max(4, (max_width + 4).min(area.width as usize)) as u16;
        let visible_height = entries.len().min(MAX_VISIBLE) as u16 + 2;
        if visible_height > area.height {
            return;
        }

        let suggestion_area = Rect::new(
            area.x,
            area.y + area.height.saturating_sub(visible_height),
            required_width,
            visible_height,
        );
        frame.render_widget(ClearWidget, suggestion_area);

        let list_items: Vec<ListItem> = entries.into_iter().map(ListItem::new).collect();
        let border_style = Style::default().fg(self.theme.primary.unwrap_or(Color::LightBlue));
        let list = List::new(list_items)
            .block(
                Block::default()
                    .title(Line::from("? help · / commands"))
                    .borders(Borders::ALL)
                    .border_style(border_style),
            )
            .highlight_style(
                Style::default()
                    .fg(self.theme.primary.unwrap_or(Color::LightBlue))
                    .add_modifier(Modifier::BOLD),
            );
        frame.render_stateful_widget(list, suggestion_area, self.slash_suggestions.list_state());
    }

    fn handle_event(
        &mut self,
        event: CrosstermEvent,
        events: &UnboundedSender<RatatuiEvent>,
    ) -> Result<bool> {
        match event {
            CrosstermEvent::Key(key) => self.handle_key_event(key, events),
            CrosstermEvent::Resize(_, _) => Ok(true),
            CrosstermEvent::Mouse(mouse) => self.handle_mouse_event(mouse, events),
            CrosstermEvent::FocusGained | CrosstermEvent::FocusLost | CrosstermEvent::Paste(_) => {
                Ok(false)
            }
        }
    }

    fn handle_key_event(
        &mut self,
        key: KeyEvent,
        events: &UnboundedSender<RatatuiEvent>,
    ) -> Result<bool> {
        if key.kind == KeyEventKind::Release {
            return Ok(false);
        }

        let suggestions_active = self.slash_suggestions.is_visible();
        if suggestions_active {
            match key.code {
                KeyCode::Up => {
                    if self.slash_suggestions.select_previous() {
                        return Ok(true);
                    }
                }
                KeyCode::Down => {
                    if self.slash_suggestions.select_next() {
                        return Ok(true);
                    }
                }
                KeyCode::Char('k') if key.modifiers.is_empty() => {
                    self.slash_suggestions.select_previous();
                    return Ok(true);
                }
                KeyCode::Char('j') if key.modifiers.is_empty() => {
                    self.slash_suggestions.select_next();
                    return Ok(true);
                }
                KeyCode::Enter | KeyCode::Tab => {
                    if self.apply_selected_suggestion() {
                        return Ok(true);
                    }
                }
                _ => {}
            }
        }

        match key.code {
            KeyCode::Enter => {
                let text = self.input.take();
                self.update_input_state();
                self.last_escape = None;
                let _ = events.send(RatatuiEvent::Submit(text));
                Ok(true)
            }
            KeyCode::Char('m')
                if key
                    .modifiers
                    .contains(KeyModifiers::CONTROL | KeyModifiers::SHIFT) =>
            {
                self.toggle_mouse_capture()?;
                Ok(true)
            }
            KeyCode::Esc => {
                if self.input.value().is_empty() {
                    let now = Instant::now();
                    let double_escape = self
                        .last_escape
                        .map(|last| {
                            now.duration_since(last).as_millis() <= u128::from(ESCAPE_DOUBLE_MS)
                        })
                        .unwrap_or(false);
                    self.last_escape = Some(now);
                    if double_escape {
                        let _ = events.send(RatatuiEvent::Exit);
                        self.should_exit = true;
                    } else {
                        let _ = events.send(RatatuiEvent::Cancel);
                    }
                } else {
                    self.input.clear();
                    self.update_input_state();
                }
                Ok(true)
            }
            KeyCode::Char('c') | KeyCode::Char('d') | KeyCode::Char('z')
                if key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                self.input.clear();
                self.update_input_state();
                match key.code {
                    KeyCode::Char('c') => {
                        let _ = events.send(RatatuiEvent::Interrupt);
                    }
                    KeyCode::Char('d') => {
                        let _ = events.send(RatatuiEvent::Exit);
                        self.should_exit = true;
                    }
                    KeyCode::Char('z') => {
                        let _ = events.send(RatatuiEvent::Cancel);
                    }
                    _ => {}
                }
                Ok(true)
            }
            KeyCode::Char('?') if key.modifiers.is_empty() => {
                self.set_input_text("/help".to_string());
                Ok(true)
            }
            KeyCode::Backspace => {
                self.input.backspace();
                self.update_input_state();
                Ok(true)
            }
            KeyCode::Delete => {
                self.input.delete();
                self.update_input_state();
                Ok(true)
            }
            KeyCode::Left => {
                self.input.move_left();
                Ok(true)
            }
            KeyCode::Right => {
                self.input.move_right();
                Ok(true)
            }
            KeyCode::Home => {
                self.input.move_home();
                Ok(true)
            }
            KeyCode::End => {
                self.input.move_end();
                Ok(true)
            }
            KeyCode::Char(ch) => {
                if key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
                {
                    return Ok(false);
                }
                self.input.insert(ch);
                self.update_input_state();
                self.last_escape = None;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn is_in_transcript_area(&self, column: u16, row: u16) -> bool {
        self.transcript_area
            .map(|area| {
                let within_x = column >= area.x && column < area.x.saturating_add(area.width);
                let within_y = row >= area.y && row < area.y.saturating_add(area.height);
                within_x && within_y
            })
            .unwrap_or(false)
    }

    fn handle_mouse_event(
        &mut self,
        mouse: MouseEvent,
        _events: &UnboundedSender<RatatuiEvent>,
    ) -> Result<bool> {
        if !self.mouse_capture_enabled {
            return Ok(false);
        }

        if !self.is_in_transcript_area(mouse.column, mouse.row) {
            return Ok(false);
        }

        if matches!(
            mouse.kind,
            MouseEventKind::ScrollUp | MouseEventKind::ScrollDown
        ) {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn draw(&mut self, frame: &mut Frame) {
        let area = frame.area();
        if area.width == 0 || area.height == 0 {
            return;
        }

        let (body_area, status_area) = if area.height > 1 {
            let segments = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(1)])
                .split(area);
            (segments[0], Some(segments[1]))
        } else {
            (area, None)
        };

        self.transcript_area = Some(body_area);

        let base_style = self
            .theme
            .foreground
            .map(|fg| Style::default().fg(fg))
            .unwrap_or_else(Style::default);
        let accent_style = self
            .theme
            .primary
            .map(|color| Style::default().fg(color))
            .unwrap_or_else(|| base_style.clone());

        let prompt_inner_width = body_area.width.saturating_sub(2) as usize;
        let (prompt_lines, prompt_cursor) = if body_area.width > 0 && body_area.height > 0 {
            self.build_prompt_block(prompt_inner_width)
        } else {
            (vec![Line::default()], None)
        };

        let mut rendered = if body_area.width > 0 && body_area.height > 0 {
            self.build_rendered_messages(body_area.width)
        } else {
            Vec::new()
        };

        if !rendered.is_empty() {
            rendered.push(RenderedMessage::spacer());
        }

        let prompt_border_color = self
            .theme
            .primary
            .or(self.theme.foreground)
            .unwrap_or_else(|| self.kind_color(RatatuiMessageKind::User));
        let prompt_cursor_offset = prompt_cursor.map(|(row, col)| {
            (
                (row as u16).saturating_add(1),
                (col as u16).saturating_add(1),
            )
        });
        let prompt_message = RenderedMessage::block_with_cursor(
            prompt_lines.clone(),
            prompt_border_color,
            BorderType::Rounded,
            None,
            prompt_cursor_offset,
        );
        rendered.push(prompt_message);

        let available_height = body_area.height;
        let mut start_index = rendered.len();
        let mut used_height = 0u16;

        for idx in (0..rendered.len()).rev() {
            let message_height = rendered[idx].height().min(available_height);
            if used_height > 0 && used_height + message_height > available_height {
                break;
            }
            used_height = (used_height + message_height).min(available_height);
            start_index = idx;
            if used_height >= available_height {
                break;
            }
        }

        let visible = &rendered[start_index..];
        let mut y = body_area.y;
        let max_y = body_area.y.saturating_add(body_area.height);
        let mut cursor_position: Option<(u16, u16)> = None;
        let mut prompt_area: Option<Rect> = None;

        for message in visible {
            if y >= max_y {
                break;
            }
            let remaining = max_y.saturating_sub(y);
            let height = message.height().min(remaining);
            if height == 0 {
                continue;
            }
            let area = Rect::new(body_area.x, y, body_area.width, height);
            message.render(frame, area, base_style.clone());
            if let Some((row, col)) = message.cursor() {
                let cursor_x = area.x.saturating_add(col);
                let cursor_y = area.y.saturating_add(row);
                cursor_position = Some((cursor_x, cursor_y));
                prompt_area = Some(area);
            }
            y = y.saturating_add(height);
        }

        if let Some((cursor_x, cursor_y)) = cursor_position {
            frame.set_cursor_position((cursor_x, cursor_y));
        }

        if let Some(area) = prompt_area {
            self.render_slash_suggestions(frame, area);
        } else {
            self.render_slash_suggestions(frame, body_area);
        }

        if let Some(status_area) = status_area {
            if status_area.width > 0 {
                let left_text = self.status_bar.left.clone();
                let center_text = self.status_bar.center.clone();
                let right_text = self.status_bar.right.clone();

                let mut left_len = UnicodeWidthStr::width(left_text.as_str()) as u16;
                let mut right_len = UnicodeWidthStr::width(right_text.as_str()) as u16;
                if left_len > status_area.width {
                    left_len = status_area.width;
                }
                if right_len > status_area.width.saturating_sub(left_len) {
                    right_len = status_area.width.saturating_sub(left_len);
                }
                let center_len = status_area.width.saturating_sub(left_len + right_len);
                let sections = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Length(left_len),
                        Constraint::Length(center_len),
                        Constraint::Length(right_len),
                    ])
                    .split(status_area);

                if let Some(area) = sections.get(0) {
                    if area.width > 0 {
                        let left = Paragraph::new(Line::from(vec![Span::styled(
                            left_text.clone(),
                            accent_style.clone(),
                        )]))
                        .alignment(Alignment::Left)
                        .style(accent_style.clone());
                        frame.render_widget(left, *area);
                    }
                }
                if let Some(area) = sections.get(1) {
                    if area.width > 0 {
                        let center = Paragraph::new(Line::from(vec![Span::styled(
                            center_text.clone(),
                            accent_style.clone(),
                        )]))
                        .alignment(Alignment::Center)
                        .style(accent_style.clone());
                        frame.render_widget(center, *area);
                    }
                }
                if let Some(area) = sections.get(2) {
                    if area.width > 0 {
                        let right = Paragraph::new(Line::from(vec![Span::styled(
                            right_text.clone(),
                            base_style.clone(),
                        )]))
                        .alignment(Alignment::Right)
                        .style(base_style.clone());
                        frame.render_widget(right, *area);
                    }
                }
            }
        }
    }

    fn build_rendered_messages(&self, width: u16) -> Vec<RenderedMessage> {
        if width == 0 {
            return Vec::new();
        }

        let mut rendered = Vec::new();
        let mut first_rendered = true;

        for block in &self.messages {
            if !self.block_has_visible_content(block) {
                continue;
            }

            if !first_rendered {
                rendered.push(RenderedMessage::spacer());
            }

            let message = match block.kind {
                RatatuiMessageKind::User => self.render_user_message(block, width),
                RatatuiMessageKind::Pty => self.render_pty_message(block, width),
                kind => self.render_standard_message(block, width, kind),
            };
            rendered.push(message);
            first_rendered = false;
        }

        rendered
    }

    fn render_standard_message(
        &self,
        block: &MessageBlock,
        width: u16,
        kind: RatatuiMessageKind,
    ) -> RenderedMessage {
        let width_usize = width as usize;
        let indent_width = MESSAGE_INDENT.min(width_usize);
        let mut lines = Vec::new();

        if let Some(header) = self.message_header_line(kind) {
            lines.push(header);
        }

        for line in &block.lines {
            let wrapped = self.wrap_segments(
                &line.segments,
                width_usize,
                indent_width,
                Some(self.kind_color(kind)),
            );
            lines.extend(wrapped);
        }

        RenderedMessage::plain(lines)
    }

    fn render_user_message(&self, block: &MessageBlock, width: u16) -> RenderedMessage {
        let border_color = self.kind_color(RatatuiMessageKind::User);
        let message_lines = self.user_message_lines(block, border_color);
        let width_usize = width as usize;

        if width < 4 {
            let mut lines = Vec::new();
            for line in &message_lines {
                let wrapped =
                    self.wrap_segments(&line.segments, width_usize, 0, Some(border_color));
                lines.extend(wrapped);
            }
            return RenderedMessage::plain(lines);
        }

        let inner_width = width.saturating_sub(2) as usize;
        let mut lines = Vec::new();
        for line in &message_lines {
            let wrapped = self.wrap_segments(&line.segments, inner_width, 0, Some(border_color));
            lines.extend(wrapped);
        }

        RenderedMessage::block(lines, border_color, BorderType::Rounded, None)
    }

    fn user_message_lines(&self, block: &MessageBlock, border_color: Color) -> Vec<StyledLine> {
        let timestamp = block
            .timestamp
            .clone()
            .unwrap_or_else(|| Local::now().format("%H:%M:%S").to_string());
        let mut prefix_style = RatatuiTextStyle::default();
        prefix_style.color = Some(border_color);
        prefix_style.bold = true;

        let mut message_lines = Vec::new();
        if let Some(first) = block.lines.first() {
            let mut first_line = first.clone();
            first_line.segments.insert(
                0,
                RatatuiSegment {
                    text: format!("❯ {} ", timestamp),
                    style: prefix_style.clone(),
                },
            );
            message_lines.push(first_line);
            message_lines.extend(block.lines.iter().skip(1).cloned());
        } else {
            let mut first_line = StyledLine::default();
            first_line.segments.push(RatatuiSegment {
                text: format!("❯ {} ", timestamp),
                style: prefix_style,
            });
            message_lines.push(first_line);
        }

        message_lines
    }

    fn render_pty_message(&self, block: &MessageBlock, width: u16) -> RenderedMessage {
        let border_color = self.kind_color(RatatuiMessageKind::Pty);
        let message_lines = self.pty_message_lines(block);
        let width_usize = width as usize;

        if width < 4 {
            let mut lines = Vec::new();
            for line in &message_lines {
                let wrapped =
                    self.wrap_segments(&line.segments, width_usize, 0, self.theme.foreground);
                lines.extend(wrapped);
            }
            return RenderedMessage::plain(lines);
        }

        let inner_width = width.saturating_sub(2) as usize;
        let mut lines = Vec::new();
        for line in &message_lines {
            let wrapped = self.wrap_segments(&line.segments, inner_width, 0, self.theme.foreground);
            lines.extend(wrapped);
        }

        let title = Line::from(vec![Span::styled(
            "Terminal".to_string(),
            Style::default()
                .fg(border_color)
                .add_modifier(Modifier::BOLD),
        )]);

        RenderedMessage::block(lines, border_color, BorderType::Rounded, Some(title))
    }

    fn pty_message_lines(&self, block: &MessageBlock) -> Vec<StyledLine> {
        let mut message_lines = Vec::new();
        if let Some(command) = block.pty_command.as_ref() {
            let mut prefix_style = RatatuiTextStyle::default();
            prefix_style.color = Some(self.kind_color(RatatuiMessageKind::Pty));
            prefix_style.bold = true;

            let mut command_style = RatatuiTextStyle::default();
            command_style.color = self.theme.primary.or(self.theme.foreground);

            let mut command_line = StyledLine::default();
            command_line.push_segment(RatatuiSegment {
                text: "$ ".to_string(),
                style: prefix_style,
            });
            command_line.push_segment(RatatuiSegment {
                text: command.clone(),
                style: command_style,
            });
            message_lines.push(command_line);
        }

        message_lines.extend(block.lines.clone());
        message_lines
    }

    fn build_prompt_block(
        &self,
        inner_width: usize,
    ) -> (Vec<Line<'static>>, Option<(usize, usize)>) {
        if inner_width == 0 {
            return (vec![Line::default()], None);
        }

        let segments = self.prompt_segments();
        let lines = self.wrap_segments(&segments, inner_width, 0, self.theme.foreground);

        if lines.is_empty() {
            return (vec![Line::default()], Some((0, 0)));
        }

        let prefix_width = UnicodeWidthStr::width(self.prompt_prefix.as_str());
        let input_width = if self.show_placeholder {
            0
        } else {
            self.input.width_before_cursor()
        };
        let placeholder_width = if self.show_placeholder {
            self.placeholder_hint
                .as_deref()
                .map(UnicodeWidthStr::width)
                .unwrap_or(0)
        } else {
            0
        };
        let cursor_width = prefix_width + input_width + placeholder_width;
        let line_width = inner_width.max(1);
        let row = cursor_width / line_width;
        let col = cursor_width % line_width;
        (lines, Some((row, col)))
    }

    fn block_has_visible_content(&self, block: &MessageBlock) -> bool {
        if matches!(
            block.kind,
            RatatuiMessageKind::Pty | RatatuiMessageKind::Tool | RatatuiMessageKind::Agent
        ) {
            return block.lines.iter().any(StyledLine::has_visible_content);
        }

        true
    }

    fn prompt_segments(&self) -> Vec<RatatuiSegment> {
        let mut segments = Vec::new();
        segments.push(RatatuiSegment {
            text: self.prompt_prefix.clone(),
            style: self.prompt_style.clone(),
        });

        if self.show_placeholder {
            if let Some(hint) = &self.placeholder_hint {
                let mut style = RatatuiTextStyle::default();
                style.italic = true;
                segments.push(RatatuiSegment {
                    text: hint.clone(),
                    style,
                });
            }
        } else {
            segments.push(RatatuiSegment {
                text: self.input.value().to_string(),
                style: RatatuiTextStyle::default(),
            });
        }

        segments
    }

    fn wrap_segments(
        &self,
        segments: &[RatatuiSegment],
        width: usize,
        indent: usize,
        fallback: Option<Color>,
    ) -> Vec<Line<'static>> {
        if width == 0 {
            return vec![Line::default()];
        }

        let mut lines = Vec::new();
        let indent_width = indent.min(width);
        let indent_text = " ".repeat(indent_width);
        let mut current = Vec::new();
        let mut current_width = indent_width;

        if indent_width > 0 {
            current.push(Span::raw(indent_text.clone()));
        }

        for segment in segments {
            let style = segment.style.to_style(fallback);
            let mut buffer = String::new();
            let mut buffer_width = 0usize;

            for ch in segment.text.chars() {
                if ch == '\n' {
                    if !buffer.is_empty() {
                        current.push(Span::styled(buffer.clone(), style));
                        buffer.clear();
                        buffer_width = 0;
                    }
                    lines.push(Line::from(current));
                    current = Vec::new();
                    if indent_width > 0 {
                        current.push(Span::raw(indent_text.clone()));
                    }
                    current_width = indent_width;
                    continue;
                }

                let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);
                if ch_width == 0 {
                    buffer.push(ch);
                    continue;
                }

                if current_width + buffer_width + ch_width > width
                    && current_width + buffer_width > indent_width
                {
                    if !buffer.is_empty() {
                        current.push(Span::styled(buffer.clone(), style));
                        buffer.clear();
                        buffer_width = 0;
                    }
                    lines.push(Line::from(current));
                    current = Vec::new();
                    if indent_width > 0 {
                        current.push(Span::raw(indent_text.clone()));
                    }
                    current_width = indent_width;
                }

                buffer.push(ch);
                buffer_width += ch_width;
            }

            if !buffer.is_empty() {
                current.push(Span::styled(buffer.clone(), style));
                current_width += buffer_width;
                buffer.clear();
            }
        }

        if current.is_empty() {
            if indent_width > 0 {
                current.push(Span::raw(indent_text));
            }
        }

        lines.push(Line::from(current));
        lines
    }

    fn message_header_line(&self, kind: RatatuiMessageKind) -> Option<Line<'static>> {
        let title = match kind {
            RatatuiMessageKind::Agent => "Agent",
            RatatuiMessageKind::Error => "Error",
            RatatuiMessageKind::Info => return None,
            RatatuiMessageKind::Policy => "Policy",
            RatatuiMessageKind::Pty => return None,
            RatatuiMessageKind::Tool => "Tool",
            RatatuiMessageKind::User => return None,
        };
        let style = Style::default()
            .fg(self.kind_color(kind))
            .add_modifier(Modifier::BOLD);
        Some(Line::from(vec![Span::styled(title.to_string(), style)]))
    }

    fn kind_color(&self, kind: RatatuiMessageKind) -> Color {
        match kind {
            RatatuiMessageKind::Agent => self.theme.primary.unwrap_or(Color::LightBlue),
            RatatuiMessageKind::User => self.theme.secondary.unwrap_or(Color::LightGreen),
            RatatuiMessageKind::Tool => Color::LightMagenta,
            RatatuiMessageKind::Pty => self.theme.primary.unwrap_or(Color::LightCyan),
            RatatuiMessageKind::Info => self.theme.primary.unwrap_or(Color::LightCyan),
            RatatuiMessageKind::Policy => Color::LightYellow,
            RatatuiMessageKind::Error => Color::Red,
        }
    }
}

fn convert_ansi_color(color: AnsiColorEnum) -> Option<Color> {
    match color {
        AnsiColorEnum::Ansi(ansi) => Some(match ansi {
            AnsiColor::Black => Color::Black,
            AnsiColor::Red => Color::Red,
            AnsiColor::Green => Color::Green,
            AnsiColor::Yellow => Color::Yellow,
            AnsiColor::Blue => Color::Blue,
            AnsiColor::Magenta => Color::Magenta,
            AnsiColor::Cyan => Color::Cyan,
            AnsiColor::White => Color::White,
            AnsiColor::BrightBlack => Color::DarkGray,
            AnsiColor::BrightRed => Color::LightRed,
            AnsiColor::BrightGreen => Color::LightGreen,
            AnsiColor::BrightYellow => Color::LightYellow,
            AnsiColor::BrightBlue => Color::LightBlue,
            AnsiColor::BrightMagenta => Color::LightMagenta,
            AnsiColor::BrightCyan => Color::LightCyan,
            AnsiColor::BrightWhite => Color::Gray,
        }),
        AnsiColorEnum::Ansi256(value) => Some(Color::Indexed(value.0)),
        AnsiColorEnum::Rgb(rgb) => Some(Color::Rgb(rgb.0, rgb.1, rgb.2)),
    }
}

fn convert_style_color(style: &AnsiStyle) -> Option<Color> {
    style.get_fg_color().and_then(convert_ansi_color)
}

pub fn convert_style(style: AnsiStyle) -> RatatuiTextStyle {
    let mut converted = RatatuiTextStyle::default();
    converted.color = convert_style_color(&style);
    let effects = style.get_effects();
    converted.bold = effects.contains(Effects::BOLD);
    converted.italic = effects.contains(Effects::ITALIC);
    converted
}

pub fn theme_from_styles(styles: &theme::ThemeStyles) -> RatatuiTheme {
    RatatuiTheme {
        background: convert_ansi_color(styles.background),
        foreground: convert_ansi_color(styles.foreground),
        primary: convert_style_color(&styles.primary),
        secondary: convert_style_color(&styles.secondary),
    }
}

fn create_ticker() -> Interval {
    let mut ticker = interval(Duration::from_millis(REDRAW_INTERVAL_MS));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
    ticker
}
