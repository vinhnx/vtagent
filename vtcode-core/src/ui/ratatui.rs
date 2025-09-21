use anyhow::{Context, Result};
use crossterm::{
    ExecutableCommand, cursor,
    event::{Event as CrosstermEvent, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    terminal::{Clear, ClearType, disable_raw_mode, enable_raw_mode},
};
use futures::StreamExt;
use ratatui::{
    Frame, Terminal, TerminalOptions, Viewport,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
};
use std::cmp::max;
use std::io;
use std::time::{Duration, Instant};
use tokio::runtime::Builder as TokioRuntimeBuilder;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio::time::{Interval, MissedTickBehavior, interval};
use unicode_width::UnicodeWidthStr;

const ESCAPE_DOUBLE_MS: u64 = 750;
const REDRAW_INTERVAL_MS: u64 = 33;

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

pub enum RatatuiCommand {
    AppendLine {
        segments: Vec<RatatuiSegment>,
    },
    Inline {
        segment: RatatuiSegment,
    },
    ReplaceLast {
        count: usize,
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
    pub fn append_line(&self, segments: Vec<RatatuiSegment>) {
        if segments.is_empty() {
            let _ = self.sender.send(RatatuiCommand::AppendLine {
                segments: vec![RatatuiSegment::default()],
            });
        } else {
            let _ = self.sender.send(RatatuiCommand::AppendLine { segments });
        }
    }

    pub fn inline(&self, segment: RatatuiSegment) {
        let _ = self.sender.send(RatatuiCommand::Inline { segment });
    }

    pub fn replace_last(&self, count: usize, lines: Vec<Vec<RatatuiSegment>>) {
        let _ = self
            .sender
            .send(RatatuiCommand::ReplaceLast { count, lines });
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

    std::thread::spawn(move || {
        match TokioRuntimeBuilder::new_current_thread().enable_all().build() {
            Ok(runtime) => {
                runtime.block_on(async move {
                    if let Err(err) = run_ratatui(command_rx, event_tx, theme, placeholder).await {
                        tracing::error!(error = ?err, "ratatui session terminated unexpectedly");
                    }
                });
            }
            Err(err) => {
                tracing::error!(error = ?err, "failed to build ratatui runtime");
            }
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
    let (_, rows) = crossterm::terminal::size().context("failed to query terminal size")?;
    let options = TerminalOptions {
        viewport: Viewport::Inline(rows),
    };
    let mut terminal = Terminal::with_options(backend, options)
        .context("failed to initialize ratatui terminal")?;
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
        self.cursor = (self.cursor + advance).min(self.value.len());
    }

    fn move_home(&mut self) {
        self.cursor = 0;
    }

    fn move_end(&mut self) {
        self.cursor = self.value.len();
    }

    fn value(&self) -> &str {
        &self.value
    }

    fn take(&mut self) -> String {
        let value = self.value.clone();
        self.clear();
        value
    }

    fn width_before_cursor(&self) -> usize {
        UnicodeWidthStr::width(&self.value[..self.cursor])
    }
}

struct RatatuiLoop {
    lines: Vec<StyledLine>,
    current_line: StyledLine,
    current_active: bool,
    prompt_prefix: String,
    prompt_style: RatatuiTextStyle,
    input: InputState,
    placeholder_hint: Option<String>,
    show_placeholder: bool,
    should_exit: bool,
    theme: RatatuiTheme,
    last_escape: Option<Instant>,
    scroll_offset: u16,
    needs_autoscroll: bool,
}

impl RatatuiLoop {
    fn new(theme: RatatuiTheme, placeholder: Option<String>) -> Self {
        Self {
            lines: Vec::new(),
            current_line: StyledLine::default(),
            current_active: false,
            prompt_prefix: "❯ ".to_string(),
            prompt_style: RatatuiTextStyle::default(),
            input: InputState::default(),
            placeholder_hint: placeholder.clone(),
            show_placeholder: placeholder.is_some(),
            should_exit: false,
            theme,
            last_escape: None,
            scroll_offset: 0,
            needs_autoscroll: true,
        }
    }

    fn should_exit(&self) -> bool {
        self.should_exit
    }

    fn needs_tick(&self) -> bool {
        false
    }

    fn handle_command(&mut self, command: RatatuiCommand) -> bool {
        match command {
            RatatuiCommand::AppendLine { segments } => {
                let was_active = self.current_active;
                self.flush_current_line(was_active);
                self.lines.push(StyledLine { segments });
                self.needs_autoscroll = true;
                true
            }
            RatatuiCommand::Inline { segment } => {
                self.append_inline_segment(segment);
                self.needs_autoscroll = true;
                true
            }
            RatatuiCommand::ReplaceLast { count, lines } => {
                let was_active = self.current_active;
                self.flush_current_line(was_active);
                let remove = count.min(self.lines.len());
                for _ in 0..remove {
                    self.lines.pop();
                }
                for segments in lines {
                    self.lines.push(StyledLine { segments });
                }
                self.needs_autoscroll = true;
                true
            }
            RatatuiCommand::SetPrompt { prefix, style } => {
                self.prompt_prefix = prefix;
                self.prompt_style = style;
                true
            }
            RatatuiCommand::SetPlaceholder { hint } => {
                self.placeholder_hint = hint.clone();
                self.show_placeholder = hint.is_some() && self.input.value().is_empty();
                true
            }
            RatatuiCommand::SetTheme { theme } => {
                self.theme = theme;
                true
            }
            RatatuiCommand::Shutdown => {
                self.should_exit = true;
                true
            }
        }
    }

    fn append_inline_segment(&mut self, segment: RatatuiSegment) {
        let text = segment.text;
        let style = segment.style;
        if text.is_empty() {
            return;
        }

        let mut parts = text.split('\n').peekable();
        let ends_with_newline = text.ends_with('\n');

        while let Some(part) = parts.next() {
            if !part.is_empty() {
                self.current_line.push_segment(RatatuiSegment {
                    text: part.to_string(),
                    style: style.clone(),
                });
                self.current_active = true;
            }

            if parts.peek().is_some() {
                self.flush_current_line(true);
            }
        }

        if ends_with_newline {
            self.flush_current_line(true);
        }
    }

    fn flush_current_line(&mut self, force: bool) {
        if !force && !self.current_active {
            return;
        }

        if !self.current_line.segments.is_empty() || force {
            self.lines.push(self.current_line.clone());
        }

        self.current_line = StyledLine::default();
        self.current_active = false;
    }

    fn handle_event(
        &mut self,
        event: CrosstermEvent,
        events: &UnboundedSender<RatatuiEvent>,
    ) -> Result<bool> {
        match event {
            CrosstermEvent::Key(key) => self.handle_key_event(key, events),
            CrosstermEvent::Resize(_, _) => {
                self.needs_autoscroll = true;
                Ok(true)
            }
            CrosstermEvent::FocusGained
            | CrosstermEvent::FocusLost
            | CrosstermEvent::Mouse(_)
            | CrosstermEvent::Paste(_) => Ok(false),
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

        match key.code {
            KeyCode::Enter => {
                let text = self.input.take();
                self.show_placeholder = self.placeholder_hint.is_some() && text.is_empty();
                self.last_escape = None;
                let _ = events.send(RatatuiEvent::Submit(text));
                Ok(true)
            }
            KeyCode::Esc => {
                let now = Instant::now();
                if self
                    .last_escape
                    .and_then(|previous| now.checked_duration_since(previous))
                    .map(|elapsed| elapsed <= Duration::from_millis(ESCAPE_DOUBLE_MS))
                    .unwrap_or(false)
                {
                    let _ = events.send(RatatuiEvent::Exit);
                    self.should_exit = true;
                } else {
                    self.last_escape = Some(now);
                    let _ = events.send(RatatuiEvent::Cancel);
                }
                Ok(true)
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                let _ = events.send(RatatuiEvent::Interrupt);
                self.should_exit = true;
                Ok(true)
            }
            KeyCode::Char('k')
                if key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                let _ = events.send(RatatuiEvent::ScrollLineUp);
                Ok(false)
            }
            KeyCode::Char('j')
                if key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                let _ = events.send(RatatuiEvent::ScrollLineDown);
                Ok(false)
            }
            KeyCode::Up => {
                let _ = events.send(RatatuiEvent::ScrollLineUp);
                Ok(false)
            }
            KeyCode::Down => {
                let _ = events.send(RatatuiEvent::ScrollLineDown);
                Ok(false)
            }
            KeyCode::PageUp => {
                let _ = events.send(RatatuiEvent::ScrollPageUp);
                Ok(false)
            }
            KeyCode::PageDown => {
                let _ = events.send(RatatuiEvent::ScrollPageDown);
                Ok(false)
            }
            KeyCode::Backspace => {
                self.input.backspace();
                self.show_placeholder =
                    self.placeholder_hint.is_some() && self.input.value().is_empty();
                Ok(true)
            }
            KeyCode::Delete => {
                self.input.delete();
                self.show_placeholder =
                    self.placeholder_hint.is_some() && self.input.value().is_empty();
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
                self.show_placeholder = false;
                self.last_escape = None;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn draw(&mut self, frame: &mut Frame) {
        let area = frame.area();
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([Constraint::Min(1)])
            .split(area);

        let transcript_area = layout[0];
        self.update_scroll_offset(transcript_area.height, transcript_area.width);
        let transcript_lines = self.collect_display_lines();
        let transcript_paragraph = Paragraph::new(transcript_lines)
            .wrap(Wrap { trim: false })
            .style(self.base_style())
            .scroll((self.scroll_offset, 0));
        frame.render_widget(transcript_paragraph, transcript_area);

        self.place_cursor(frame, transcript_area);
    }

    fn base_style(&self) -> Style {
        let mut style = Style::default();
        if let Some(bg) = self.theme.background {
            style = style.bg(bg);
        }
        if let Some(fg) = self.theme.foreground {
            style = style.fg(fg);
        }
        style
    }

    fn collect_display_lines(&self) -> Vec<Line<'static>> {
        let mut lines = self.collect_transcript_lines();
        if self.has_transcript_content() {
            lines.push(Line::default());
        }
        lines.push(Line::from(self.build_input_line()));
        lines
    }

    fn collect_transcript_lines(&self) -> Vec<Line<'static>> {
        let mut lines = Vec::new();
        for line in &self.lines {
            lines.push(self.convert_line(line));
        }
        if self.current_active && !self.current_line.segments.is_empty() {
            lines.push(self.convert_line(&self.current_line));
        }
        lines
    }

    fn has_transcript_content(&self) -> bool {
        !self.lines.is_empty() || (self.current_active && !self.current_line.segments.is_empty())
    }

    fn update_scroll_offset(&mut self, viewport_height: u16, viewport_width: u16) {
        if !self.needs_autoscroll {
            return;
        }

        let viewport = viewport_height.max(1) as usize;
        let transcript_rows = self.transcript_visual_rows(viewport_width);
        let input_rows = self.input_visual_rows(viewport_width);
        let total = transcript_rows.saturating_add(input_rows);
        if total <= viewport {
            self.scroll_offset = 0;
        } else {
            let offset = total.saturating_sub(viewport);
            self.scroll_offset = offset.min(u16::MAX as usize) as u16;
        }
        self.needs_autoscroll = false;
    }

    fn transcript_visual_rows(&self, viewport_width: u16) -> usize {
        if viewport_width == 0 {
            return 0;
        }

        let mut total = 0usize;
        for line in &self.lines {
            total = total.saturating_add(self.visual_height(line, viewport_width));
        }
        if self.current_active && !self.current_line.segments.is_empty() {
            total = total.saturating_add(self.visual_height(&self.current_line, viewport_width));
        }
        total
    }

    fn input_visual_rows(&self, viewport_width: u16) -> usize {
        if viewport_width == 0 {
            return 0;
        }

        let text_width = self.input_text_area_width(viewport_width);
        if text_width == 0 {
            return 0;
        }

        let display_width = self.input_display_width();
        let rows = (display_width + text_width - 1) / text_width;
        let input_rows = max(rows, 1);
        if self.has_transcript_content() {
            input_rows.saturating_add(1)
        } else {
            input_rows
        }
    }

    fn cursor_visual_position(&self, viewport_width: u16) -> Option<(usize, usize)> {
        if viewport_width == 0 {
            return None;
        }

        let text_width = self.input_text_area_width(viewport_width);
        if text_width == 0 {
            return None;
        }

        let prompt_width = self.prompt_width();
        let input_width = self.input.width_before_cursor();
        let cursor_column = prompt_width.saturating_add(input_width);
        let input_row = cursor_column / text_width;
        let input_col = cursor_column % text_width;
        let base_rows = self.transcript_visual_rows(viewport_width);
        let spacing = if self.has_transcript_content() { 1 } else { 0 };
        Some((
            base_rows.saturating_add(spacing).saturating_add(input_row),
            input_col,
        ))
    }

    fn place_cursor(&self, frame: &mut Frame, area: Rect) {
        let Some((row, col)) = self.cursor_visual_position(area.width) else {
            return;
        };

        let scroll = self.scroll_offset as usize;
        if row < scroll {
            return;
        }

        let visible_row = row - scroll;
        if visible_row >= area.height as usize {
            return;
        }

        let cursor_x = area.x + col as u16;
        let cursor_y = area.y + visible_row as u16;
        frame.set_cursor_position((cursor_x, cursor_y));
    }

    fn visual_height(&self, line: &StyledLine, viewport_width: u16) -> usize {
        if viewport_width == 0 {
            return 0;
        }

        let width = viewport_width as usize;
        let mut line_width = 0usize;
        for segment in &line.segments {
            line_width = line_width.saturating_add(UnicodeWidthStr::width(segment.text.as_str()));
        }

        if line_width == 0 {
            return 1;
        }

        let rows = (line_width + width - 1) / width;
        max(rows, 1)
    }

    fn convert_line(&self, line: &StyledLine) -> Line<'static> {
        let mut spans = Vec::with_capacity(line.segments.len());
        for segment in &line.segments {
            let style = segment.style.to_style(self.theme.foreground);
            spans.push(Span::styled(segment.text.clone(), style));
        }
        Line::from(spans)
    }

    fn build_input_line(&self) -> Vec<Span<'static>> {
        let mut spans = Vec::new();
        if !self.prompt_prefix.is_empty() {
            let style = self
                .prompt_style
                .to_style(self.theme.secondary.or(self.theme.foreground));
            spans.push(Span::styled(self.prompt_prefix.clone(), style));
        }
        let input_value = self.input.value().to_string();
        spans.push(Span::styled(
            input_value.clone(),
            Style::default().fg(self.theme.foreground.unwrap_or(Color::White)),
        ));
        if self.show_placeholder {
            if let Some(hint) = &self.placeholder_hint {
                if input_value.is_empty() {
                    spans.push(Span::raw(" "));
                    spans.push(Span::styled(
                        hint.clone(),
                        Style::default()
                            .fg(self
                                .theme
                                .secondary
                                .or(self.theme.foreground)
                                .unwrap_or(Color::White))
                            .add_modifier(Modifier::ITALIC),
                    ));
                }
            }
        }
        spans
    }

    fn prompt_width(&self) -> usize {
        UnicodeWidthStr::width(self.prompt_prefix.as_str())
    }

    fn input_text_area_width(&self, layout_width: u16) -> usize {
        usize::from(layout_width)
    }

    fn input_display_width(&self) -> usize {
        let mut width = self.prompt_width();
        let value = self.input.value();
        width = width.saturating_add(UnicodeWidthStr::width(value));
        if self.show_placeholder && value.is_empty() {
            if let Some(hint) = &self.placeholder_hint {
                width = width.saturating_add(1);
                width = width.saturating_add(UnicodeWidthStr::width(hint.as_str()));
            }
        }
        width
    }
}

fn create_ticker() -> Interval {
    let mut ticker = interval(Duration::from_millis(REDRAW_INTERVAL_MS));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
    ticker
}

pub fn convert_style(style: anstyle::Style) -> RatatuiTextStyle {
    let color = style.get_fg_color().and_then(|color| convert_color(color));
    let effects = style.get_effects();
    let bold = effects.contains(anstyle::Effects::BOLD);
    let italic = effects.contains(anstyle::Effects::ITALIC);

    RatatuiTextStyle {
        color,
        bold,
        italic,
    }
}

pub fn convert_color(color: anstyle::Color) -> Option<Color> {
    match color {
        anstyle::Color::Ansi(ansi) => Some(match ansi {
            anstyle::AnsiColor::Black => Color::Black,
            anstyle::AnsiColor::Red => Color::Red,
            anstyle::AnsiColor::Green => Color::Green,
            anstyle::AnsiColor::Yellow => Color::Yellow,
            anstyle::AnsiColor::Blue => Color::Blue,
            anstyle::AnsiColor::Magenta => Color::Magenta,
            anstyle::AnsiColor::Cyan => Color::Cyan,
            anstyle::AnsiColor::White => Color::Gray,
            anstyle::AnsiColor::BrightBlack => Color::DarkGray,
            anstyle::AnsiColor::BrightRed => Color::LightRed,
            anstyle::AnsiColor::BrightGreen => Color::LightGreen,
            anstyle::AnsiColor::BrightYellow => Color::LightYellow,
            anstyle::AnsiColor::BrightBlue => Color::LightBlue,
            anstyle::AnsiColor::BrightMagenta => Color::LightMagenta,
            anstyle::AnsiColor::BrightCyan => Color::LightCyan,
            anstyle::AnsiColor::BrightWhite => Color::White,
        }),
        anstyle::Color::Ansi256(value) => Some(Color::Indexed(value.index())),
        anstyle::Color::Rgb(rgb) => Some(Color::Rgb(rgb.r(), rgb.g(), rgb.b())),
    }
}

pub fn theme_from_styles(styles: &crate::ui::theme::ThemeStyles) -> RatatuiTheme {
    RatatuiTheme {
        background: convert_color(styles.background),
        foreground: convert_style(styles.output).color,
        primary: convert_style(styles.primary).color,
        secondary: convert_style(styles.secondary).color,
    }
}
