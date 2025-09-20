use anyhow::{Context, Result};
use iocraft::prelude::*;
use parking_lot::Mutex;
use std::cmp;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio::sync::oneshot;
use tokio::task::yield_now;

const ESCAPE_DOUBLE_MS: u64 = 750;

#[derive(Clone, Default)]
pub struct IocraftTextStyle {
    pub color: Option<Color>,
    pub weight: Weight,
    pub italic: bool,
}

impl IocraftTextStyle {
    pub fn merge_color(mut self, fallback: Option<Color>) -> Self {
        if self.color.is_none() {
            self.color = fallback;
        }
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IocraftMessageKind {
    System,
    User,
    Agent,
    Tool,
    Error,
    Reasoning,
}

impl Default for IocraftMessageKind {
    fn default() -> Self {
        Self::System
    }
}

#[derive(Clone, Default)]
pub struct IocraftSegment {
    pub text: String,
    pub style: IocraftTextStyle,
    pub kind: IocraftMessageKind,
}

#[derive(Clone, Default)]
struct StyledLine {
    segments: Vec<IocraftSegment>,
    kind: IocraftMessageKind,
}

impl StyledLine {
    fn push_segment(&mut self, segment: IocraftSegment) {
        if segment.text.is_empty() {
            return;
        }
        if matches!(self.kind, IocraftMessageKind::System) {
            self.kind = segment.kind;
        }
        self.segments.push(segment);
    }
}

#[derive(Clone)]
struct ToolPermissionPrompt {
    tool: String,
    description: Option<String>,
    responder: Arc<Mutex<Option<oneshot::Sender<bool>>>>,
}

impl ToolPermissionPrompt {
    fn new(tool: String, description: Option<String>, responder: oneshot::Sender<bool>) -> Self {
        Self {
            tool,
            description,
            responder: Arc::new(Mutex::new(Some(responder))),
        }
    }

    fn respond(&self, approved: bool) {
        if let Some(sender) = self.responder.lock().take() {
            let _ = sender.send(approved);
        }
    }
}

#[derive(Clone)]
pub struct IocraftTheme {
    pub background: Option<Color>,
    pub foreground: Option<Color>,
    pub primary: Option<Color>,
    pub secondary: Option<Color>,
    pub alert: Option<Color>,
}

impl Default for IocraftTheme {
    fn default() -> Self {
        Self {
            background: None,
            foreground: None,
            primary: None,
            secondary: None,
            alert: None,
        }
    }
}

pub enum IocraftCommand {
    AppendLine {
        segments: Vec<IocraftSegment>,
    },
    Inline {
        segment: IocraftSegment,
    },
    SetPrompt {
        prefix: String,
        style: IocraftTextStyle,
    },
    SetPlaceholder {
        hint: Option<String>,
    },
    SetTheme {
        theme: IocraftTheme,
    },
    RequestToolPermission {
        tool: String,
        description: Option<String>,
        responder: oneshot::Sender<bool>,
    },
    Shutdown,
}

#[derive(Debug, Clone)]
pub enum IocraftEvent {
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
pub struct IocraftHandle {
    sender: UnboundedSender<IocraftCommand>,
}

impl IocraftHandle {
    pub fn append_line(&self, segments: Vec<IocraftSegment>) {
        if segments.is_empty() {
            let _ = self.sender.send(IocraftCommand::AppendLine {
                segments: vec![IocraftSegment::default()],
            });
        } else {
            let _ = self.sender.send(IocraftCommand::AppendLine { segments });
        }
    }

    pub fn inline(&self, segment: IocraftSegment) {
        let _ = self.sender.send(IocraftCommand::Inline { segment });
    }

    pub fn set_prompt(&self, prefix: String, style: IocraftTextStyle) {
        let _ = self
            .sender
            .send(IocraftCommand::SetPrompt { prefix, style });
    }

    pub fn set_placeholder(&self, hint: Option<String>) {
        let _ = self.sender.send(IocraftCommand::SetPlaceholder { hint });
    }

    pub fn set_theme(&self, theme: IocraftTheme) {
        let _ = self.sender.send(IocraftCommand::SetTheme { theme });
    }

    pub async fn request_tool_permission(
        &self,
        tool: impl Into<String>,
        description: Option<String>,
    ) -> bool {
        let (tx, rx) = oneshot::channel();
        let command = IocraftCommand::RequestToolPermission {
            tool: tool.into(),
            description,
            responder: tx,
        };
        if self.sender.send(command).is_err() {
            return false;
        }
        rx.await.unwrap_or(false)
    }

    pub fn shutdown(&self) {
        let _ = self.sender.send(IocraftCommand::Shutdown);
    }
}

pub struct IocraftSession {
    pub handle: IocraftHandle,
    pub events: UnboundedReceiver<IocraftEvent>,
}

pub fn spawn_session(theme: IocraftTheme, placeholder: Option<String>) -> Result<IocraftSession> {
    let (command_tx, command_rx) = mpsc::unbounded_channel();
    let (event_tx, event_rx) = mpsc::unbounded_channel();

    tokio::spawn(async move {
        if let Err(err) = run_iocraft(command_rx, event_tx, theme, placeholder).await {
            tracing::error!(error = ?err, "iocraft session terminated unexpectedly");
        }
    });

    Ok(IocraftSession {
        handle: IocraftHandle { sender: command_tx },
        events: event_rx,
    })
}

async fn run_iocraft(
    commands: UnboundedReceiver<IocraftCommand>,
    events: UnboundedSender<IocraftEvent>,
    theme: IocraftTheme,
    placeholder: Option<String>,
) -> Result<()> {
    element! {
        SessionRoot(
            commands: commands,
            events: events,
            theme: theme,
            placeholder: placeholder,
        )
    }
    .render_loop()
    .await
    .context("iocraft render loop failed")
}

#[derive(Default, Props)]
struct SessionRootProps {
    commands: Option<UnboundedReceiver<IocraftCommand>>,
    events: Option<UnboundedSender<IocraftEvent>>,
    theme: IocraftTheme,
    placeholder: Option<String>,
}

#[component]
fn SessionRoot(props: &mut SessionRootProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let (width, height) = hooks.use_terminal_size();
    let mut system = hooks.use_context_mut::<SystemContext>();
    let (stdout, _) = hooks.use_output();

    hooks.use_future(async move {
        let _ = stdout.println("VT Code terminal ready");
    });

    let lines = hooks.use_state(Vec::<StyledLine>::default);
    let current_line = hooks.use_state(StyledLine::default);
    let current_active = hooks.use_state(|| false);
    let prompt_prefix = hooks.use_state(|| "❯ ".to_string());
    let prompt_style = hooks.use_state(IocraftTextStyle::default);
    let mut input_value = hooks.use_state(String::new);
    let placeholder_hint = hooks.use_state(|| props.placeholder.clone().unwrap_or_default());
    let show_placeholder = hooks.use_state(|| props.placeholder.is_some());
    let mut should_exit = hooks.use_state(|| false);
    let theme_state = hooks.use_state(|| props.theme.clone());
    let command_state = hooks.use_state(|| props.commands.take());
    let line_count_state = hooks.use_state(|| 0usize);
    let scroll_offset_state = hooks.use_state(|| 0usize);
    let manual_scroll_state = hooks.use_state(|| false);
    let tool_prompt_state = hooks.use_state(|| None::<ToolPermissionPrompt>);

    let estimated_view_capacity = cmp::max(height.saturating_sub(12) as usize, 8);

    let tool_prompt_for_commands = tool_prompt_state.clone();
    hooks.use_future({
        let mut command_slot = command_state;
        let mut lines_state = lines;
        let mut current_line_state = current_line;
        let mut current_active_state = current_active;
        let mut prompt_prefix_state = prompt_prefix;
        let mut prompt_style_state = prompt_style;
        let mut placeholder_state = placeholder_hint;
        let mut placeholder_visible_state = show_placeholder;
        let mut exit_state = should_exit;
        let mut theme_handle_state = theme_state;
        let mut line_count = line_count_state;
        let mut scroll_offset = scroll_offset_state;
        let manual_scroll = manual_scroll_state;
        let mut tool_prompt_store = tool_prompt_for_commands;
        async move {
            let receiver = loop {
                if let Some(mut guard) = command_slot.try_write() {
                    let extracted = guard.take();
                    drop(guard);
                    break extracted;
                }

                if command_slot.try_read().is_none() {
                    tracing::warn!("iocraft command receiver missing; terminating command loop");
                    return;
                }

                yield_now().await;
            };

            let Some(mut rx) = receiver else {
                tracing::warn!("iocraft command channel closed before loop start");
                return;
            };

            while let Some(cmd) = rx.recv().await {
                match cmd {
                    IocraftCommand::AppendLine { segments } => {
                        let was_active = current_active_state.get();
                        flush_current_line(
                            &mut current_line_state,
                            &mut current_active_state,
                            &mut lines_state,
                            was_active,
                        );
                        if let Some(mut lines_guard) = lines_state.try_write() {
                            let line_kind = segments
                                .iter()
                                .find(|segment| !segment.text.is_empty())
                                .map(|segment| segment.kind)
                                .unwrap_or_default();
                            lines_guard.push(StyledLine {
                                segments,
                                kind: line_kind,
                            });
                        }
                        if !manual_scroll.get() {
                            scroll_offset.set(0);
                        }
                    }
                    IocraftCommand::Inline { segment } => {
                        append_inline_segment(
                            &mut current_line_state,
                            &mut current_active_state,
                            &mut lines_state,
                            segment,
                        );
                        if !manual_scroll.get() {
                            scroll_offset.set(0);
                        }
                    }
                    IocraftCommand::SetPrompt { prefix, style } => {
                        prompt_prefix_state.set(prefix);
                        prompt_style_state.set(style);
                    }
                    IocraftCommand::SetPlaceholder { hint } => {
                        placeholder_state.set(hint.clone().unwrap_or_default());
                        placeholder_visible_state.set(hint.is_some());
                    }
                    IocraftCommand::SetTheme { theme } => {
                        theme_handle_state.set(theme);
                    }
                    IocraftCommand::RequestToolPermission {
                        tool,
                        description,
                        responder,
                    } => {
                        let existing_prompt = tool_prompt_store.read().clone();
                        if let Some(active) = existing_prompt {
                            active.respond(false);
                        }
                        tool_prompt_store.set(Some(ToolPermissionPrompt::new(
                            tool,
                            description,
                            responder,
                        )));
                    }
                    IocraftCommand::Shutdown => {
                        exit_state.set(true);
                        break;
                    }
                }

                let mut total = lines_state.read().len();
                if current_active_state.get() {
                    if let Some(line) = current_line_state.try_read() {
                        if !line.segments.is_empty() {
                            total += 1;
                        }
                    }
                }
                line_count.set(total);
            }
        }
    });

    if should_exit.get() {
        system.exit();
    }

    let events_tx = match props.events.clone() {
        Some(tx) => tx,
        None => {
            tracing::warn!("iocraft events sender missing; rendering fallback view");
            let theme_snapshot = theme_state.read().clone();
            let fallback_background =
                theme_snapshot
                    .background
                    .unwrap_or(Color::Rgb { r: 0, g: 0, b: 0 });
            let fallback_foreground = theme_snapshot.foreground.unwrap_or(Color::White);

            return element! {
                View(
                    width,
                    height,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    background_color: Some(fallback_background),
                    padding_left: 2u16,
                    padding_right: 2u16,
                ) {
                    Text(
                        content: "Interactive controls unavailable: event channel not initialized.",
                        color: Some(fallback_foreground),
                        wrap: TextWrap::Wrap,
                    )
                }
            };
        }
    };
    let mut last_escape = hooks.use_state(|| None::<Instant>);
    let mut placeholder_toggle = show_placeholder;
    let mut scroll_handle = scroll_offset_state;
    let mut manual_scroll_toggle = manual_scroll_state;
    let line_count_snapshot = line_count_state;
    let mut prompt_state_for_events = tool_prompt_state.clone();

    hooks.use_terminal_events(move |event| {
        if let TerminalEvent::Key(KeyEvent {
            code,
            kind,
            modifiers,
            ..
        }) = event
        {
            if kind == KeyEventKind::Release {
                return;
            }

            let active_prompt_opt = prompt_state_for_events.read().clone();
            if let Some(active_prompt) = active_prompt_opt {
                match code {
                    KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                        active_prompt.respond(true);
                        prompt_state_for_events.set(None);
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                        active_prompt.respond(false);
                        prompt_state_for_events.set(None);
                    }
                    _ => {}
                }
                return;
            }

            match code {
                KeyCode::Enter => {
                    let text = input_value.to_string();
                    input_value.set(String::new());
                    last_escape.set(None);
                    placeholder_toggle.set(false);
                    manual_scroll_toggle.set(false);
                    scroll_handle.set(0);
                    let _ = events_tx.send(IocraftEvent::Submit(text));
                }
                KeyCode::Esc => {
                    let now = Instant::now();
                    if last_escape
                        .get()
                        .and_then(|prev| now.checked_duration_since(prev))
                        .map(|elapsed| elapsed <= Duration::from_millis(ESCAPE_DOUBLE_MS))
                        .unwrap_or(false)
                    {
                        let _ = events_tx.send(IocraftEvent::Exit);
                        should_exit.set(true);
                    } else {
                        last_escape.set(Some(now));
                        let _ = events_tx.send(IocraftEvent::Cancel);
                    }
                }
                KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                    let _ = events_tx.send(IocraftEvent::Interrupt);
                    should_exit.set(true);
                }
                KeyCode::Up => {
                    manual_scroll_toggle.set(true);
                    let total = line_count_snapshot.get();
                    let next_offset = cmp::min(
                        scroll_handle.get().saturating_add(1),
                        total.saturating_sub(1),
                    );
                    scroll_handle.set(next_offset);
                    let _ = events_tx.send(IocraftEvent::ScrollLineUp);
                }
                KeyCode::Down => {
                    let current = scroll_handle.get();
                    if current > 0 {
                        let new_offset = current.saturating_sub(1);
                        scroll_handle.set(new_offset);
                        if new_offset == 0 {
                            manual_scroll_toggle.set(false);
                        }
                    }
                    let _ = events_tx.send(IocraftEvent::ScrollLineDown);
                }
                KeyCode::PageUp => {
                    manual_scroll_toggle.set(true);
                    let total = line_count_snapshot.get();
                    let next_offset = cmp::min(
                        scroll_handle.get().saturating_add(estimated_view_capacity),
                        total.saturating_sub(1),
                    );
                    scroll_handle.set(next_offset);
                    let _ = events_tx.send(IocraftEvent::ScrollPageUp);
                }
                KeyCode::PageDown => {
                    let current = scroll_handle.get();
                    let step = estimated_view_capacity;
                    if current > 0 {
                        let new_offset = current.saturating_sub(step);
                        scroll_handle.set(new_offset);
                        if new_offset == 0 {
                            manual_scroll_toggle.set(false);
                        }
                    }
                    let _ = events_tx.send(IocraftEvent::ScrollPageDown);
                }
                KeyCode::End => {
                    manual_scroll_toggle.set(false);
                    scroll_handle.set(0);
                }
                KeyCode::Char('k')
                    if modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                {
                    manual_scroll_toggle.set(true);
                    let total = line_count_snapshot.get();
                    let next_offset = cmp::min(
                        scroll_handle.get().saturating_add(1),
                        total.saturating_sub(1),
                    );
                    scroll_handle.set(next_offset);
                    let _ = events_tx.send(IocraftEvent::ScrollLineUp);
                }
                KeyCode::Char('j')
                    if modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                {
                    let current = scroll_handle.get();
                    if current > 0 {
                        let new_offset = current.saturating_sub(1);
                        scroll_handle.set(new_offset);
                        if new_offset == 0 {
                            manual_scroll_toggle.set(false);
                        }
                    }
                    let _ = events_tx.send(IocraftEvent::ScrollLineDown);
                }
                _ => {}
            }
        }
    });

    let mut transcript_lines = lines.read().clone();
    if let Some(current) = current_line.try_read() {
        if current_active.get() && !current.segments.is_empty() {
            transcript_lines.push(current.clone());
        }
    }

    let prompt_prefix_value = prompt_prefix.to_string();
    let prompt_style_value = prompt_style.read().clone();
    let input_value_string = input_value.to_string();
    let placeholder_text = placeholder_hint.to_string();
    let placeholder_visible = show_placeholder.get() && !placeholder_text.is_empty();
    let prompt_snapshot = tool_prompt_state.read().clone();
    let prompt_active = prompt_snapshot.is_some();

    let total_lines = transcript_lines.len();
    let manual_scroll_active = manual_scroll_state.get();
    let applied_offset = cmp::min(scroll_offset_state.get(), total_lines.saturating_sub(1));
    let end_index = total_lines.saturating_sub(applied_offset);
    let start_index = end_index.saturating_sub(estimated_view_capacity).max(0);

    let visible_lines: Vec<StyledLine> = transcript_lines
        .into_iter()
        .skip(start_index)
        .take(estimated_view_capacity)
        .collect();

    let theme_value = theme_state.read().clone();

    let background = theme_value
        .background
        .unwrap_or(Color::Rgb { r: 0, g: 0, b: 0 });
    let foreground = theme_value.foreground.unwrap_or(Color::White);
    let primary = theme_value.primary.unwrap_or(foreground);
    let secondary = theme_value.secondary.unwrap_or(primary);
    let alert = theme_value.alert.unwrap_or(Color::Red);

    let surface = lighten_color(background, 0.06);
    let transcript_surface = lighten_color(background, 0.04);
    let header_background = mix_colors(primary, background, 0.35);
    let header_border = mix_colors(primary, background, 0.6);
    let footer_background = mix_colors(secondary, background, 0.3);
    let frame_color = mix_colors(primary, background, 0.45);
    let input_surface = lighten_color(background, 0.1);
    let input_border = mix_colors(secondary, background, 0.4);
    let input_inner_surface = lighten_color(background, 0.14);
    let logo_background = mix_colors(primary, background, 0.55);

    let root_padding_x = safe_padding(width, 2);
    let root_padding_y = safe_padding(height, 1);
    let interior_width = width.saturating_sub(root_padding_x.saturating_mul(2));
    let header_padding_x = safe_padding(interior_width, 3);
    let header_padding_y = safe_padding(height, 1);
    let transcript_padding_x = safe_padding(interior_width, 2);
    let transcript_padding_y = safe_padding(height, 1);
    let transcript_inner_width =
        interior_width.saturating_sub(transcript_padding_x.saturating_mul(2));
    let bubble_padding_x = safe_padding(transcript_inner_width, 2);
    let bubble_padding_y = safe_padding(height, 1);
    let bubble_min_width_value = u32::from(bubble_padding_x) * 2 + 1;
    let bubble_min_width = Size::Length(bubble_min_width_value);
    let bubble_max_width =
        if u32::from(transcript_inner_width) > bubble_min_width_value.saturating_mul(2) {
            Size::Percent(90.0)
        } else {
            Size::Auto
        };
    let overlay_padding_x = safe_padding(interior_width, 4);
    let overlay_min_width = Size::Length(u32::from(overlay_padding_x) * 2 + 1);
    let prompt_card_padding_x = safe_padding(interior_width, 3);
    let prompt_card_padding_y = safe_padding(height, 2);
    let prompt_card_min_width_value = u32::from(prompt_card_padding_x) * 2 + 1;
    let prompt_card_min_width = Size::Length(prompt_card_min_width_value);
    let prompt_card_max_width =
        if u32::from(interior_width) > prompt_card_min_width_value.saturating_mul(2) {
            Size::Percent(70.0)
        } else {
            Size::Auto
        };
    let placeholder_padding = safe_padding(transcript_inner_width, 2);
    let scroll_indicator_padding_x = safe_padding(transcript_inner_width, 2);
    let scroll_indicator_padding_y = safe_padding(height, 1);
    let footer_padding_x = safe_padding(interior_width, 2);
    let footer_padding_y = safe_padding(height, 1);
    let input_padding_x = safe_padding(transcript_inner_width, 2);
    let input_padding_y = safe_padding(height, 1);
    let input_inner_padding_x = safe_padding(transcript_inner_width, 1);

    let user_bubble_bg = mix_colors(primary, background, 0.3);
    let agent_bubble_bg = mix_colors(secondary, background, 0.35);
    let system_bubble_bg = lighten_color(background, 0.12);
    let tool_bubble_bg = mix_colors(secondary, background, 0.45);
    let error_bubble_bg = mix_colors(alert, background, 0.4);
    let reasoning_bubble_bg = mix_colors(secondary, background, 0.5);
    let overlay_scrim = mix_colors(background, Color::Black, 0.65);
    let prompt_card_bg = lighten_color(background, 0.18);
    let prompt_card_border = mix_colors(primary, background, 0.5);
    let approve_button_color = mix_colors(primary, background, 0.25);
    let deny_button_color = mix_colors(alert, background, 0.25);
    let button_trim_color = mix_colors(foreground, background, 0.3);

    let placeholder_color = theme_value.secondary.or(Some(foreground));
    let placeholder_element = placeholder_visible.then(|| {
        element! {
            Text(
                content: placeholder_text.clone(),
                color: placeholder_color,
                italic: true,
                wrap: TextWrap::Wrap,
            )
        }
    });

    let mut transcript_rows: Vec<AnyElement<'static>> = visible_lines
        .into_iter()
        .map(|line| {
            if line.segments.is_empty() {
                return element! {
                    View(height: 1u16) {
                        Text(content: "", wrap: TextWrap::NoWrap)
                    }
                }
                .into();
            }

            let (bubble_bg, border_color, text_color) = match line.kind {
                IocraftMessageKind::User => (user_bubble_bg, primary, foreground),
                IocraftMessageKind::Agent => (agent_bubble_bg, secondary, foreground),
                IocraftMessageKind::Tool => (tool_bubble_bg, secondary, foreground),
                IocraftMessageKind::Error => (error_bubble_bg, alert, foreground),
                IocraftMessageKind::Reasoning => (reasoning_bubble_bg, secondary, foreground),
                IocraftMessageKind::System => (system_bubble_bg, frame_color, foreground),
            };

            let alignment = match line.kind {
                IocraftMessageKind::User => AlignItems::FlexEnd,
                _ => AlignItems::FlexStart,
            };

            let label_color = mix_colors(border_color, foreground, 0.2);

            let message_segments = line.segments.into_iter().map(|segment| {
                let color = segment.style.color.unwrap_or(text_color);
                element! {
                    Text(
                        content: segment.text,
                        color: Some(color),
                        weight: segment.style.weight,
                        italic: segment.style.italic,
                        wrap: TextWrap::Wrap,
                    )
                }
            });

            element! {
                View(
                    flex_direction: FlexDirection::Column,
                    align_items: alignment,
                    width: 100pct,
                    gap: 0u16,
                ) {
                    Text(
                        content: message_label(line.kind).to_string(),
                        color: Some(label_color),
                        weight: Weight::Bold,
                        italic: false,
                        wrap: TextWrap::NoWrap,
                    )
                    View(
                        background_color: Some(bubble_bg),
                        border_style: BorderStyle::Round,
                        border_color: Some(border_color),
                        padding_left: bubble_padding_x,
                        padding_right: bubble_padding_x,
                        padding_top: bubble_padding_y,
                        padding_bottom: bubble_padding_y,
                        gap: 0u16,
                        min_width: bubble_min_width,
                        max_width: bubble_max_width,
                    ) {
                        #(message_segments)
                    }
                }
            }
            .into()
        })
        .collect();

    if transcript_rows.is_empty() {
        transcript_rows.push(
            element! {
                View(
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    padding: placeholder_padding,
                ) {
                    Text(
                        content: "Welcome! Start by typing a prompt or load a project.",
                        color: Some(foreground),
                        weight: Weight::Bold,
                        wrap: TextWrap::Wrap,
                    )
                }
            }
            .into(),
        );
    }

    if manual_scroll_active {
        transcript_rows.insert(
            0,
            element! {
                View(
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    border_style: BorderStyle::Classic,
                    border_color: Some(frame_color),
                    background_color: Some(surface),
                    padding_left: scroll_indicator_padding_x,
                    padding_right: scroll_indicator_padding_x,
                    padding_top: scroll_indicator_padding_y,
                    padding_bottom: scroll_indicator_padding_y,
                ) {
                    Text(
                        content: format!("Viewing history (offset {})", applied_offset),
                        color: Some(foreground),
                        weight: Weight::Bold,
                        wrap: TextWrap::NoWrap,
                    )
                }
            }
            .into(),
        );
    }

    let input_value_state = input_value;
    let prompt_state_for_buttons = tool_prompt_state.clone();

    let prompt_overlay: Option<AnyElement<'static>> = prompt_snapshot.clone().map(|prompt| {
        let tool_name = prompt.tool.clone();
        let description = prompt
            .description
            .clone()
            .unwrap_or_else(|| format!("Allow the agent to use '{tool_name}'?"));
        let mut approve_state = prompt_state_for_buttons.clone();
        let mut deny_state = prompt_state_for_buttons.clone();
        let helper_color = mix_colors(foreground, background, 0.35);

        element! {
            View(
                position: Position::Absolute,
                left: 0i16,
                top: 0i16,
                width: 100pct,
                height: 100pct,
                background_color: Some(overlay_scrim),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                padding_left: overlay_padding_x,
                padding_right: overlay_padding_x,
                min_width: overlay_min_width,
            ) {
                View(
                    flex_direction: FlexDirection::Column,
                    gap: 1u16,
                    background_color: Some(prompt_card_bg),
                    border_style: BorderStyle::Round,
                    border_color: Some(prompt_card_border),
                    padding_left: prompt_card_padding_x,
                    padding_right: prompt_card_padding_x,
                    padding_top: prompt_card_padding_y,
                    padding_bottom: prompt_card_padding_y,
                    min_width: prompt_card_min_width,
                    max_width: prompt_card_max_width,
                ) {
                    Text(
                        content: "Tool permission required",
                        color: Some(foreground),
                        weight: Weight::Bold,
                        wrap: TextWrap::Wrap,
                    )
                    Text(
                        content: format!("Tool: {tool_name}"),
                        color: Some(theme_value.secondary.unwrap_or(foreground)),
                        weight: Weight::Bold,
                        wrap: TextWrap::Wrap,
                    )
                    Text(
                        content: description,
                        color: Some(foreground),
                        wrap: TextWrap::Wrap,
                    )
                    View(
                        flex_direction: FlexDirection::Row,
                        gap: 2u16,
                        margin_top: 1u16,
                    ) {
                        Button(
                            handler: move |_| {
                                let active_opt = approve_state.read().clone();
                                if let Some(active) = active_opt {
                                    active.respond(true);
                                    approve_state.set(None);
                                }
                            },
                        ) {
                            View(
                                border_style: BorderStyle::Round,
                                border_color: Some(button_trim_color),
                                background_color: Some(approve_button_color),
                                padding_left: 3u16,
                                padding_right: 3u16,
                                padding_top: 1u16,
                                padding_bottom: 1u16,
                            ) {
                                Text(
                                    content: "Approve",
                                    color: Some(foreground),
                                    weight: Weight::Bold,
                                    wrap: TextWrap::NoWrap,
                                )
                            }
                        }
                        Button(
                            handler: move |_| {
                                let active_opt = deny_state.read().clone();
                                if let Some(active) = active_opt {
                                    active.respond(false);
                                    deny_state.set(None);
                                }
                            },
                        ) {
                            View(
                                border_style: BorderStyle::Round,
                                border_color: Some(button_trim_color),
                                background_color: Some(deny_button_color),
                                padding_left: 3u16,
                                padding_right: 3u16,
                                padding_top: 1u16,
                                padding_bottom: 1u16,
                            ) {
                                Text(
                                    content: "Deny",
                                    color: Some(foreground),
                                    weight: Weight::Bold,
                                    wrap: TextWrap::NoWrap,
                                )
                            }
                        }
                    }
                    Text(
                        content: "Tip: Y/Enter to approve, N/Esc to deny",
                        color: Some(helper_color),
                        italic: true,
                        wrap: TextWrap::Wrap,
                    )
                }
            }
        }
        .into()
    });

    element! {
        View(
            width,
            height,
            flex_direction: FlexDirection::Column,
            background_color: Some(background),
            padding_top: root_padding_y,
            padding_bottom: root_padding_y,
            padding_left: root_padding_x,
            padding_right: root_padding_x,
            gap: 1u16,
        ) {
            View(
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                gap: 2u16,
                border_style: BorderStyle::Round,
                border_color: Some(header_border),
                background_color: Some(header_background),
                padding_left: header_padding_x,
                padding_right: header_padding_x,
                padding_top: header_padding_y,
                padding_bottom: header_padding_y,
            ) {
                View(
                    border_style: BorderStyle::Round,
                    border_color: Some(frame_color),
                    background_color: Some(logo_background),
                    padding_left: 2u16,
                    padding_right: 2u16,
                    padding_top: 0u16,
                    padding_bottom: 0u16,
                ) {
                    Text(
                        content: "VT",
                        color: Some(foreground),
                        weight: Weight::Bold,
                        wrap: TextWrap::NoWrap,
                    )
                }
                View(
                    flex_direction: FlexDirection::Column,
                    gap: 0u16,
                ) {
                    Text(
                        content: "VT Code",
                        color: Some(foreground),
                        weight: Weight::Bold,
                        wrap: TextWrap::NoWrap,
                    )
                    Text(
                        content: "Terminal workspace for building software",
                        color: Some(lighten_color(foreground, 0.2)),
                        wrap: TextWrap::Wrap,
                    )
                }
            }
            View(
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                gap: 1u16,
                border_style: BorderStyle::Round,
                border_color: Some(frame_color),
                background_color: Some(transcript_surface),
                padding_left: transcript_padding_x,
                padding_right: transcript_padding_x,
                padding_top: transcript_padding_y,
                padding_bottom: transcript_padding_y,
            ) {
                View(
                    flex_direction: FlexDirection::Column,
                    flex_grow: 1.0,
                    gap: 1u16,
                ) {
                    #(transcript_rows)
                }
                View(
                    flex_direction: FlexDirection::Column,
                    gap: 1u16,
                    border_style: BorderStyle::Round,
                    border_color: Some(input_border),
                    background_color: Some(input_surface),
                    padding_left: input_padding_x,
                    padding_right: input_padding_x,
                    padding_top: input_padding_y,
                    padding_bottom: input_padding_y,
                ) {
                    View(
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        gap: 1u16,
                    ) {
                        Text(
                            content: prompt_prefix_value.clone(),
                            color: prompt_style_value.color.or(theme_value.secondary),
                            weight: prompt_style_value.weight,
                            italic: prompt_style_value.italic,
                            wrap: TextWrap::NoWrap,
                        )
                        View(
                            flex_direction: FlexDirection::Column,
                            flex_grow: 1.0,
                            border_style: BorderStyle::Classic,
                            border_color: Some(input_border),
                            background_color: Some(input_inner_surface),
                            padding_left: input_inner_padding_x,
                            padding_right: input_inner_padding_x,
                            padding_top: 0u16,
                            padding_bottom: 0u16,
                        ) {
                            TextInput(
                                has_focus: !prompt_active,
                                value: input_value_string.clone(),
                                on_change: move |value| {
                                    let mut handle = input_value_state;
                                    handle.set(value);
                                },
                                color: Some(foreground),
                            )
                        }
                    }
                    #(placeholder_element.into_iter())
                }
            }
            View(
                flex_direction: FlexDirection::Row,
                border_style: BorderStyle::Classic,
                border_color: Some(frame_color),
                background_color: Some(footer_background),
                padding_left: footer_padding_x,
                padding_right: footer_padding_x,
                padding_top: footer_padding_y,
                padding_bottom: footer_padding_y,
            ) {
                Text(
                    content: "Enter: send • Esc Esc: exit • Ctrl+C: interrupt • PgUp/PgDn: scroll • /help for commands",
                    color: Some(foreground),
                    wrap: TextWrap::Wrap,
                )
            }
            #(prompt_overlay.into_iter())
        }
    }
}

fn flush_current_line(
    current_line: &mut State<StyledLine>,
    current_active: &mut State<bool>,
    lines_state: &mut State<Vec<StyledLine>>,
    force: bool,
) {
    if !force && !current_active.get() {
        return;
    }

    if let Some(cur) = current_line.try_read() {
        if !cur.segments.is_empty() || force {
            if let Some(mut lines) = lines_state.try_write() {
                lines.push(cur.clone());
            }
        }
    }

    if let Some(mut cur) = current_line.try_write() {
        cur.segments.clear();
    }
    current_active.set(false);
}

fn append_inline_segment(
    current_line: &mut State<StyledLine>,
    current_active: &mut State<bool>,
    lines_state: &mut State<Vec<StyledLine>>,
    segment: IocraftSegment,
) {
    let text = segment.text;
    let style = segment.style;

    if text.is_empty() {
        return;
    }

    let mut parts = text.split('\n').peekable();
    let ends_with_newline = text.ends_with('\n');

    while let Some(part) = parts.next() {
        if !part.is_empty() {
            if let Some(mut cur) = current_line.try_write() {
                cur.push_segment(IocraftSegment {
                    text: part.to_string(),
                    style: style.clone(),
                    kind: segment.kind,
                });
            }
            current_active.set(true);
        }

        if parts.peek().is_some() {
            flush_current_line(current_line, current_active, lines_state, true);
        }
    }

    if ends_with_newline {
        flush_current_line(current_line, current_active, lines_state, true);
    }
}

fn message_label(kind: IocraftMessageKind) -> &'static str {
    match kind {
        IocraftMessageKind::User => "You",
        IocraftMessageKind::Agent => "VT Code",
        IocraftMessageKind::Tool => "Tool Output",
        IocraftMessageKind::Error => "Alert",
        IocraftMessageKind::Reasoning => "Thinking",
        IocraftMessageKind::System => "System",
    }
}

fn safe_padding(available: u16, desired: u16) -> u16 {
    if available <= desired.saturating_mul(2) {
        available.saturating_sub(1) / 2
    } else {
        desired
    }
}

fn mix_colors(base: Color, target: Color, ratio: f32) -> Color {
    let (br, bg, bb) = color_to_rgb_components(base);
    let (tr, tg, tb) = color_to_rgb_components(target);
    let ratio = ratio.clamp(0.0, 1.0);
    let blend = |start: u8, end: u8| -> u8 {
        let start = start as f32;
        let end = end as f32;
        ((start + (end - start) * ratio).round()).clamp(0.0, 255.0) as u8
    };
    Color::Rgb {
        r: blend(br, tr),
        g: blend(bg, tg),
        b: blend(bb, tb),
    }
}

fn lighten_color(color: Color, ratio: f32) -> Color {
    mix_colors(
        color,
        Color::Rgb {
            r: 255,
            g: 255,
            b: 255,
        },
        ratio,
    )
}

fn color_to_rgb_components(color: Color) -> (u8, u8, u8) {
    match color {
        Color::Black => (0, 0, 0),
        Color::DarkGrey => (128, 128, 128),
        Color::Grey => (192, 192, 192),
        Color::White => (255, 255, 255),
        Color::DarkRed => (128, 0, 0),
        Color::Red => (255, 0, 0),
        Color::DarkGreen => (0, 128, 0),
        Color::Green => (0, 255, 0),
        Color::DarkYellow => (128, 128, 0),
        Color::Yellow => (255, 255, 0),
        Color::DarkBlue => (0, 0, 128),
        Color::Blue => (0, 0, 255),
        Color::DarkMagenta => (128, 0, 128),
        Color::Magenta => (255, 0, 255),
        Color::DarkCyan => (0, 128, 128),
        Color::Cyan => (0, 255, 255),
        Color::AnsiValue(value) => ansi_value_to_rgb(value),
        Color::Rgb { r, g, b } => (r, g, b),
        Color::Reset => (255, 255, 255),
    }
}

fn ansi_value_to_rgb(value: u8) -> (u8, u8, u8) {
    match value {
        0 => (0, 0, 0),
        1 => (128, 0, 0),
        2 => (0, 128, 0),
        3 => (128, 128, 0),
        4 => (0, 0, 128),
        5 => (128, 0, 128),
        6 => (0, 128, 128),
        7 => (192, 192, 192),
        8 => (128, 128, 128),
        9 => (255, 0, 0),
        10 => (0, 255, 0),
        11 => (255, 255, 0),
        12 => (0, 0, 255),
        13 => (255, 0, 255),
        14 => (0, 255, 255),
        15 => (255, 255, 255),
        16..=231 => {
            let index = value - 16;
            let r = index / 36;
            let g = (index % 36) / 6;
            let b = index % 6;
            (
                rgb_component_from_cube(r),
                rgb_component_from_cube(g),
                rgb_component_from_cube(b),
            )
        }
        232..=255 => {
            let shade = (value - 232) * 10 + 8;
            (shade, shade, shade)
        }
    }
}

fn rgb_component_from_cube(value: u8) -> u8 {
    match value {
        0 => 0,
        1 => 95,
        2 => 135,
        3 => 175,
        4 => 215,
        _ => 255,
    }
}

pub fn convert_style(style: anstyle::Style) -> IocraftTextStyle {
    let color = style.get_fg_color().and_then(|color| convert_color(color));
    let effects = style.get_effects();
    let weight = if effects.contains(anstyle::Effects::BOLD) {
        Weight::Bold
    } else {
        Weight::Normal
    };
    let italic = effects.contains(anstyle::Effects::ITALIC);

    IocraftTextStyle {
        color,
        weight,
        italic,
    }
}

pub fn convert_color(color: anstyle::Color) -> Option<Color> {
    match color {
        anstyle::Color::Ansi(ansi) => Some(match ansi {
            anstyle::AnsiColor::Black => Color::Black,
            anstyle::AnsiColor::Red => Color::DarkRed,
            anstyle::AnsiColor::Green => Color::DarkGreen,
            anstyle::AnsiColor::Yellow => Color::DarkYellow,
            anstyle::AnsiColor::Blue => Color::DarkBlue,
            anstyle::AnsiColor::Magenta => Color::DarkMagenta,
            anstyle::AnsiColor::Cyan => Color::DarkCyan,
            anstyle::AnsiColor::White => Color::Grey,
            anstyle::AnsiColor::BrightBlack => Color::DarkGrey,
            anstyle::AnsiColor::BrightRed => Color::Red,
            anstyle::AnsiColor::BrightGreen => Color::Green,
            anstyle::AnsiColor::BrightYellow => Color::Yellow,
            anstyle::AnsiColor::BrightBlue => Color::Blue,
            anstyle::AnsiColor::BrightMagenta => Color::Magenta,
            anstyle::AnsiColor::BrightCyan => Color::Cyan,
            anstyle::AnsiColor::BrightWhite => Color::White,
        }),
        anstyle::Color::Ansi256(value) => Some(Color::AnsiValue(value.index())),
        anstyle::Color::Rgb(rgb) => Some(Color::Rgb {
            r: rgb.r(),
            g: rgb.g(),
            b: rgb.b(),
        }),
    }
}

pub fn theme_from_styles(styles: &crate::ui::theme::ThemeStyles) -> IocraftTheme {
    IocraftTheme {
        background: convert_color(styles.background),
        foreground: convert_style(styles.output).color,
        primary: convert_style(styles.primary).color,
        secondary: convert_style(styles.secondary).color,
        alert: convert_style(styles.error).color,
    }
}
