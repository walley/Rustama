use std::io;
use std::time::Duration;

use ratatui::crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, MouseButton, MouseEventKind,
};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};

use pulldown_cmark::{
    CodeBlockKind, Event as MdEvent, Options as MdOptions, Parser as MdParser, Tag, TagEnd,
};

use ratatui::prelude::*;
use ratatui::widgets::*;

mod app;
mod config;
mod primary_selection;
mod ui;
use app::{App, ChatMessage, Focus, InputMode, ModelDialogFocus, SaveDialogFocus, SettingsFocus};
use config::Config;
use ui::{Button, ConfirmationBox, FileActionDialog, dialog_block};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        original_hook(info);
    }));

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut app = App::new(Config::load());
    let result = run_app(&mut terminal, &mut app);

    crate::app::log_to_file(
        app.is_logging,
        &app.log_file,
        &app.session_id,
        "END",
        "Program exited",
    );

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(e) = result {
        eprintln!("Error: {}", e);
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()>
where
    io::Error: From<<B as Backend>::Error>,
{
    loop {
        terminal.draw(|f| ui(f, app))?;

        if event::poll(Duration::from_millis(50))? {
            let size = terminal.size()?;
            app.terminal_height = size.height;

            if !app.retrying {
                match event::read()? {
                    Event::Key(key) => {
                        app.handle_global_key(key);
                    }
                    Event::Mouse(mouse) => match mouse.kind {
                        MouseEventKind::ScrollUp => app.scroll_up(),
                        MouseEventKind::ScrollDown => app.scroll_down(),
                        MouseEventKind::Down(MouseButton::Left) => {
                            let size = terminal.size()?;
                            app.handle_click(mouse.column, mouse.row, size.width, size.height);
                        }
                        MouseEventKind::Up(MouseButton::Left) => {
                            app.handle_mouse_up();
                        }
                        MouseEventKind::Drag(MouseButton::Left) => {
                            let size = terminal.size()?;
                            let input_start = size.height.saturating_sub(6);
                            app.handle_mouse_drag(mouse.row, input_start);
                        }
                        _ => {}
                    },
                    _ => {}
                }
                while event::poll(Duration::ZERO)? {
                    match event::read()? {
                        Event::Key(key) => {
                            app.handle_global_key(key);
                        }
                        Event::Mouse(mouse) => match mouse.kind {
                            MouseEventKind::ScrollUp => app.scroll_up(),
                            MouseEventKind::ScrollDown => app.scroll_down(),
                            MouseEventKind::Down(MouseButton::Left) => {
                                let size = terminal.size()?;
                                app.handle_click(mouse.column, mouse.row, size.width, size.height);
                            }
                            MouseEventKind::Up(MouseButton::Left) => {
                                app.handle_mouse_up();
                            }
                            MouseEventKind::Drag(MouseButton::Left) => {
                                let size = terminal.size()?;
                                let input_start = size.height.saturating_sub(6);
                                app.handle_mouse_drag(mouse.row, input_start);
                            }
                            _ => {}
                        },
                        _ => {}
                    }
                }
            } else {
                // During retry: allow scrolling and text selection, block everything else
                if let Event::Mouse(mouse) = event::read()? {
                    match mouse.kind {
                        MouseEventKind::ScrollUp => app.scroll_up(),
                        MouseEventKind::ScrollDown => app.scroll_down(),
                        MouseEventKind::Down(MouseButton::Left) => {
                            let size = terminal.size()?;
                            app.handle_click(mouse.column, mouse.row, size.width, size.height);
                        }
                        MouseEventKind::Up(MouseButton::Left) => {
                            app.handle_mouse_up();
                        }
                        MouseEventKind::Drag(MouseButton::Left) => {
                            let size = terminal.size()?;
                            let input_start = size.height.saturating_sub(6);
                            app.handle_mouse_drag(mouse.row, input_start);
                        }
                        _ => {}
                    }
                }
                while event::poll(Duration::ZERO)? {
                    if let Event::Mouse(mouse) = event::read()? {
                        match mouse.kind {
                            MouseEventKind::ScrollUp => app.scroll_up(),
                            MouseEventKind::ScrollDown => app.scroll_down(),
                            MouseEventKind::Down(MouseButton::Left) => {
                                let size = terminal.size()?;
                                app.handle_click(mouse.column, mouse.row, size.width, size.height);
                            }
                            MouseEventKind::Up(MouseButton::Left) => {
                                app.handle_mouse_up();
                            }
                            MouseEventKind::Drag(MouseButton::Left) => {
                                let size = terminal.size()?;
                                let input_start = size.height.saturating_sub(6);
                                app.handle_mouse_drag(mouse.row, input_start);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        app.check_responses();
        app.check_model_responses();

        if app.should_quit {
            return Ok(());
        }
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    let area = f.area();

    let main_chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .split(area);

    app.main_menu.render_bar(f, main_chunks[0]);
    render_keybar(f, app, main_chunks[2]);

    let show_terminal = app.terminal_state.visible && app.terminal_state.is_running();

    // The status bar always has content (the mode indicator), so it is
    // always rendered between the output and input areas.
    if show_terminal {
        let content_chunks = Layout::horizontal([
            Constraint::Percentage(100 - app.terminal_state.width_pct),
            Constraint::Percentage(app.terminal_state.width_pct),
        ])
        .split(main_chunks[1]);

        let left_chunks = Layout::vertical([
            Constraint::Min(3),
            Constraint::Length(1),
            Constraint::Length(5),
        ])
        .split(content_chunks[0]);
        app.output_width = left_chunks[0].width;
        render_output(f, app, left_chunks[0]);
        render_status_bar(f, app, left_chunks[1]);
        render_input(f, app, left_chunks[2]);
        render_terminal_panel(f, app, content_chunks[1]);
    } else {
        let content_chunks = Layout::vertical([
            Constraint::Min(3),
            Constraint::Length(1),
            Constraint::Length(5),
        ])
        .split(main_chunks[1]);
        app.output_width = content_chunks[0].width;
        render_output(f, app, content_chunks[0]);
        render_status_bar(f, app, content_chunks[1]);
        render_input(f, app, content_chunks[2]);
    }

    if app.main_menu.is_open() {
        app.main_menu.render_submenu(f, main_chunks[0]);
    }

    if app.show_about {
        let mb = ui::MessageBox::new("About", &app.about_message);
        mb.render(f, area, true, &app.theme);
    }

    if app.show_quit_confirm {
        let cb = ConfirmationBox::new("Confirm Quit", "Are you sure you want to quit?");
        cb.render(f, area, app.quit_confirm_focus, &app.theme);
    }

    if app.show_model_dialog {
        render_model_dialog(f, app, area);
    }

    if app.show_load_dialog {
        render_load_dialog(f, app, area);
    }

    if app.show_file_dialog {
        render_file_dialog(f, app, area);
    }

    if app.show_save_dialog {
        render_save_dialog(f, app, area);
    }

    if app.show_settings_dialog {
        render_settings_dialog(f, app, area);
    }

    if app.show_retry_paused {
        let mb = ui::MessageBox::new("Retry Paused", &app.retry_paused_message);
        mb.render(f, area, true, &app.theme);
    }
}

fn render_output(f: &mut Frame, app: &mut App, area: Rect) {
    let streaming_len = if app.is_loading {
        app.streaming_text.len()
    } else {
        0
    };
    let streaming_thinking_len = if app.is_loading {
        app.streaming_thinking.len()
    } else {
        0
    };
    let output_width = area.width.saturating_sub(4) as usize;

    let history_changed =
        app.messages.len() != app.cached_msg_count || app.cached_width as usize != output_width;
    let streaming_changed = app.is_loading
        && (streaming_len != app.cached_streaming_len
            || streaming_thinking_len != app.cached_streaming_thinking_len);

    if history_changed {
        let mut hist: Vec<Line<'static>> = Vec::new();
        let mut tool_call_num: usize = 0;

        for msg in &app.messages {
            match msg {
                ChatMessage::User(text) => {
                    for line in text.lines() {
                        hist.push(Line::from(vec![
                            Span::styled(
                                " > ",
                                Style::default()
                                    .fg(Color::Cyan)
                                    .add_modifier(Modifier::BOLD),
                            ),
                            Span::styled(
                                line.to_string(),
                                Style::default()
                                    .fg(Color::Cyan)
                                    .add_modifier(Modifier::BOLD),
                            ),
                        ]));
                    }
                    hist.push(Line::from(""));
                }
                ChatMessage::Assistant(text) => {
                    let mut md_lines = render_markdown(text);
                    hist.append(&mut md_lines);
                }
                ChatMessage::System(text) => {
                    for text_line in text.lines() {
                        hist.push(Line::from(Span::styled(
                            format!("  {}", text_line),
                            Style::default()
                                .fg(Color::DarkGray)
                                .add_modifier(Modifier::ITALIC),
                        )));
                    }
                    hist.push(Line::from(""));
                }
                ChatMessage::App(text) => {
                    for text_line in text.lines() {
                        hist.push(Line::from(Span::styled(
                            format!("  {}", text_line),
                            Style::default()
                                .fg(Color::DarkGray)
                                .add_modifier(Modifier::ITALIC),
                        )));
                    }
                    hist.push(Line::from(""));
                }
                ChatMessage::Thinking(text) => {
                    let mut in_code = false;
                    for text_line in text.lines() {
                        let trimmed = text_line.trim();
                        if trimmed.starts_with("```") {
                            in_code = !in_code;
                        }
                        let style = if in_code {
                            Style::default()
                                .fg(app.theme.thinking_fg)
                                .bg(Color::Rgb(30, 60, 120))
                        } else {
                            Style::default()
                                .fg(app.theme.thinking_fg)
                                .add_modifier(Modifier::ITALIC)
                        };
                        hist.push(Line::from(Span::styled(format!("  {}", text_line), style)));
                    }
                    hist.push(Line::from(""));
                }
                ChatMessage::FileContent { name, content } => {
                    hist.push(Line::from(Span::styled(
                        format!("  \u{1F4C4} {}", name),
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )));
                    let mut md_lines = render_markdown(content);
                    hist.append(&mut md_lines);
                    hist.push(Line::from(""));
                }
                ChatMessage::ToolCall {
                    name, arguments, ..
                } => {
                    tool_call_num += 1;
                    hist.push(Line::from(vec![
                        Span::styled(
                            " \u{2699} ",
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            format!("#{} TOOL CALL: {}", tool_call_num, name),
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ]));
                    let pretty_args = serde_json::from_str::<serde_json::Value>(arguments)
                        .ok()
                        .and_then(|v| serde_json::to_string_pretty(&v).ok())
                        .unwrap_or_else(|| arguments.to_string());
                    for arg_line in pretty_args.lines() {
                        hist.push(Line::from(Span::styled(
                            format!("    {}", arg_line),
                            Style::default().fg(Color::DarkGray),
                        )));
                    }
                    hist.push(Line::from(""));
                }
                ChatMessage::ToolResult { name, content, .. } => {
                    hist.push(Line::from(vec![
                        Span::styled(
                            " \u{2714} ",
                            Style::default()
                                .fg(Color::Green)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            format!("RESULT: {}", name),
                            Style::default()
                                .fg(Color::Green)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ]));
                    let preview = if content.len() > 500 {
                        format!("{}... ({} bytes total)", &content[..500], content.len())
                    } else {
                        content.to_string()
                    };
                    for result_line in preview.lines().take(20) {
                        hist.push(Line::from(Span::styled(
                            format!("    {}", result_line),
                            Style::default().fg(Color::DarkGray),
                        )));
                    }
                    if content.lines().count() > 20 {
                        hist.push(Line::from(Span::styled(
                            format!("    ... ({} more lines)", content.lines().count() - 20),
                            Style::default().fg(Color::DarkGray),
                        )));
                    }
                    hist.push(Line::from(""));
                }
            }
        }

        app.cached_output = hist;
        app.cached_msg_count = app.messages.len();
        app.cached_width = area.width;
    }

    if history_changed || streaming_changed {
        let mut lines = app.cached_output.clone();
        if app.is_loading && !app.streaming_thinking.is_empty() {
            let mut in_code = false;
            for think_line in app.streaming_thinking.lines() {
                let trimmed = think_line.trim();
                if trimmed.starts_with("```") {
                    in_code = !in_code;
                }
                let style = if in_code {
                    Style::default()
                        .fg(app.theme.thinking_fg)
                        .bg(Color::Rgb(30, 60, 120))
                } else {
                    Style::default()
                        .fg(app.theme.thinking_fg)
                        .add_modifier(Modifier::ITALIC)
                };
                lines.push(Line::from(Span::styled(format!("  {}", think_line), style)));
            }
            lines.push(Line::from(""));
        }
        if app.is_loading && !app.streaming_text.is_empty() {
            let mut md_lines = render_markdown(&app.streaming_text);
            lines.append(&mut md_lines);
        } else if app.is_loading && app.streaming_thinking.is_empty() {
            lines.push(Line::from(Span::styled(
                "  Streaming response...",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::SLOW_BLINK),
            )));
        }
        app.cached_streaming_len = streaming_len;
        app.cached_streaming_thinking_len = streaming_thinking_len;
        app.cached_wrapped = wrap_and_justify_lines(&lines, output_width, app.justify);
    }

    let lines = &app.cached_wrapped;

    let total_lines = lines.len() as u16;
    let visible_height = area.height.saturating_sub(2);
    let max_scroll = total_lines.saturating_sub(visible_height);
    if app.auto_scroll {
        app.scroll_offset = max_scroll;
    } else {
        let scroll = app.scroll_offset.min(max_scroll);
        app.scroll_offset = scroll;
    }
    let scroll = app.scroll_offset;

    let selected_lines: Vec<Line<'static>>;
    let render_lines: &[Line<'static>] =
        if let (Some(s), Some(e)) = (app.selection_start, app.selection_end) {
            if !app.cached_wrapped.is_empty() {
                let s_raw = s.min(app.cached_wrapped.len() - 1);
                let e_raw = e.min(app.cached_wrapped.len() - 1);
                let (s, e) = if s_raw <= e_raw {
                    (s_raw, e_raw)
                } else {
                    (e_raw, s_raw)
                };
                selected_lines = app
                    .cached_wrapped
                    .iter()
                    .enumerate()
                    .map(|(i, l)| {
                        if i >= s && i <= e {
                            let mut hl = l.clone();
                            for span in &mut hl.spans {
                                span.style = span.style.add_modifier(Modifier::REVERSED);
                            }
                            hl
                        } else {
                            l.clone()
                        }
                    })
                    .collect();
                &selected_lines
            } else {
                lines
            }
        } else {
            lines
        };

    let focus_style = if app.focus == Focus::Output {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let available = area.width.saturating_sub(2) as usize;
    let model_display = if app.model_name.chars().count() > available {
        let model_budget = available.saturating_sub(3);
        if model_budget > 0 {
            let truncated: String = app.model_name.chars().take(model_budget).collect();
            format!("{}...", truncated)
        } else {
            String::new()
        }
    } else {
        app.model_name.clone()
    };

    let bottom_title = Line::from(Span::styled(
        format!(" {} ", model_display),
        Style::default().fg(Color::DarkGray),
    ));

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Output ")
        .title_bottom(bottom_title)
        .border_style(focus_style);

    let paragraph = Paragraph::new(render_lines)
        .block(block)
        .scroll((scroll, 0));

    let content_area = Rect {
        x: area.x,
        y: area.y,
        width: area.width.saturating_sub(1),
        height: area.height,
    };
    f.render_widget(paragraph, content_area);

    let scrollbar_area = Rect {
        x: area.x + area.width - 1,
        y: area.y,
        width: 1,
        height: area.height,
    };

    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("▲"))
        .end_symbol(Some("▼"))
        .track_symbol(Some("│"))
        .thumb_style(
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .track_style(Style::default().fg(Color::DarkGray));

    let mut scrollbar_state = ScrollbarState::new(max_scroll as usize).position(scroll as usize);
    f.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
}

fn render_input(f: &mut Frame, app: &App, area: Rect) {
    let (title, border_style) = match (&app.input_mode, &app.focus) {
        (InputMode::Input, Focus::Input) => {
            (" Input ".to_string(), Style::default().fg(Color::Green))
        }
        (_, Focus::Input) => (
            " Input (i or Enter to type) ".to_string(),
            Style::default().fg(Color::Yellow),
        ),
        _ => (
            " Input (i or Enter to type) ".to_string(),
            Style::default().fg(Color::DarkGray),
        ),
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(border_style);

    let mut textarea = app.textarea.clone();
    textarea.set_cursor_line_style(Style::default());
    textarea.set_block(block);
    f.render_widget(&textarea, area);
    render_send_button(f, app, area);
}

fn render_terminal_panel(f: &mut Frame, app: &App, area: Rect) {
    let b = app.terminal_state.buffer.lock().unwrap();
    let content = if b.content.len() > 4000 {
        &b.content[b.content.len() - 4000..]
    } else {
        &b.content
    };
    let lines: Vec<Line> = content
        .lines()
        .map(|l| {
            Line::from(Span::styled(
                l.to_string(),
                Style::default().fg(Color::Green).bg(Color::Black),
            ))
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .style(Style::default().bg(Color::Black))
        .title(format!(" {} ", app.terminal_state.command));

    let total_lines = lines.len() as u16;
    let visible_lines = area.height.saturating_sub(2);
    let scroll_y = total_lines.saturating_sub(visible_lines);

    let paragraph = Paragraph::new(lines)
        .block(block)
        .style(Style::default().bg(Color::Black))
        .scroll((scroll_y, 0));

    f.render_widget(paragraph, area);
}

fn render_send_button(f: &mut Frame, app: &App, area: Rect) {
    let has_text = !app.textarea.lines().join("").trim().is_empty();
    let btn_x = area.x + area.width.saturating_sub(13);
    let btn_y = area.y + area.height.saturating_sub(1);

    let send_btn = Button::new(
        "send",
        btn_x,
        btn_y,
        has_text,
        Color::DarkGray,
        Color::Green,
    );

    let (btn_text, btn_style) = send_btn.render();
    let btn_area = Rect {
        x: btn_x,
        y: btn_y,
        width: send_btn.width,
        height: 1,
    };

    f.render_widget(Clear, btn_area);
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(btn_text, btn_style))),
        btn_area,
    );
}

fn render_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let activity = app.status_message.trim().to_string();
    let token_info = app.format_token_stats();

    // Mode indicator always leads the bar: "mode | messages | tokens".
    let (mode_label, mode_style) = if app.agentic_mode {
        (
            " AGENTIC ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        (
            " CHAT ",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
    };

    let sep_style = Style::default().fg(Color::Gray);
    let mut spans: Vec<Span> = vec![Span::styled(mode_label, mode_style)];

    if !activity.is_empty() {
        spans.push(Span::styled("|", sep_style));
        let style = if app.is_loading || app.retrying {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD)
        };
        spans.push(Span::styled(format!(" {} ", activity), style));
    }

    if !token_info.is_empty() {
        spans.push(Span::styled("|", sep_style));
        spans.push(Span::styled(
            format!(" {} ", token_info),
            Style::default().fg(Color::Cyan),
        ));
    }

    let line = Line::from(spans);
    let bar = Paragraph::new(line).style(Style::default().bg(Color::DarkGray));
    f.render_widget(bar, area);
}

fn render_keybar(f: &mut Frame, app: &App, area: Rect) {
    let focus_label = match app.focus {
        Focus::Output => " [OUTPUT] ",
        Focus::Input => " [INPUT] ",
    };

    let terminal_hint = if app.terminal_state.is_running() {
        " Ctrl+T "
    } else {
        ""
    };

    let resize_hint = if app.terminal_state.is_running() && app.terminal_state.visible {
        " \u{2190}\u{2192} "
    } else {
        ""
    };

    let text = match app.input_mode {
        InputMode::Normal => format!(
            " F9:Menu  F10:Quit  Ctrl+S:Save  Mouse:Scroll{}{}{}",
            focus_label, terminal_hint, resize_hint
        ),
        InputMode::Input => format!(" Enter:Send  Alt+Enter:Newline{}", terminal_hint),
        InputMode::Menu => {
            " \u{2190}\u{2192}:Navigate  \u{2191}\u{2193}:Select  Enter:Open  Esc:Close".to_string()
        }
    };

    let keybar = Paragraph::new(Line::from(Span::styled(
        text,
        Style::default().fg(Color::White).bg(Color::DarkGray),
    )))
    .style(Style::default().bg(Color::DarkGray));

    f.render_widget(keybar, area);
}

fn render_model_dialog(f: &mut Frame, app: &App, area: Rect) {
    let model_count = app.available_models.len().min(12) as u16;
    let dialog_w = 56u16;
    let dialog_h = model_count + 7;

    let popup_area = Rect {
        x: (area.width.saturating_sub(dialog_w)) / 2,
        y: (area.height.saturating_sub(dialog_h)) / 2,
        width: dialog_w,
        height: dialog_h,
    };

    let inner = popup_area.inner(Margin::new(1, 1));
    f.render_widget(Clear, popup_area);
    f.render_widget(dialog_block("Select Model", &app.theme), popup_area);

    if app.available_models.is_empty() {
        let loading = Paragraph::new(Line::from(Span::styled(
            "  Loading models...",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::SLOW_BLINK),
        )));
        f.render_widget(loading, inner);
        return;
    }

    let list_height = model_count;
    let list_area = Rect {
        x: inner.x,
        y: inner.y,
        width: inner.width,
        height: list_height,
    };

    let items: Vec<ListItem> = app
        .available_models
        .iter()
        .enumerate()
        .take(model_count as usize)
        .map(|(i, name)| {
            let is_current = name == &app.model_name;
            let is_selected = i == app.model_dialog_selection;

            let mut spans = Vec::new();
            if is_selected {
                spans.push(Span::styled(
                    " > ",
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ));
            } else {
                spans.push(Span::styled("   ", Style::default()));
            }

            let name_style = if is_current {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else if is_selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let display_name = if name.len() > 46 {
                format!("{}...", &name[..43])
            } else {
                name.clone()
            };

            spans.push(Span::styled(display_name, name_style));

            if is_current {
                spans.push(Span::styled(
                    " (current)",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::ITALIC),
                ));
            }

            ListItem::new(Line::from(spans))
        })
        .collect();

    let list = List::new(items);
    f.render_widget(list, list_area);

    let btn_y = inner.y + list_height + 1;
    let confirm_btn = Button::new(
        "Confirm",
        inner.x + 10,
        btn_y,
        app.model_dialog_focus == ModelDialogFocus::Confirm,
        Color::Green,
        Color::Green,
    );
    let cancel_btn = Button::new(
        "Cancel",
        inner.x + 24,
        btn_y,
        app.model_dialog_focus == ModelDialogFocus::Cancel,
        Color::Red,
        Color::Red,
    );

    let (confirm_text, confirm_style) = confirm_btn.render();
    let (cancel_text, cancel_style) = cancel_btn.render();

    let buttons = Line::from(vec![
        Span::raw("          "),
        Span::styled(confirm_text, confirm_style),
        Span::raw("   "),
        Span::styled(cancel_text, cancel_style),
        Span::raw("          "),
    ]);

    let btn_area = Rect {
        x: inner.x,
        y: btn_y,
        width: inner.width,
        height: 1,
    };
    f.render_widget(Paragraph::new(buttons), btn_area);
}

fn render_file_dialog(f: &mut Frame, app: &App, area: Rect) {
    let dlg = app.build_file_action_dialog(area);
    dlg.render(f, area, &app.theme);
}

fn render_save_dialog(f: &mut Frame, app: &App, area: Rect) {
    use ui::DialogDropdownState;

    let dlg = match app.save_dialog_mode {
        app::SaveDialogMode::SaveSession => {
            let mut d = FileActionDialog::new("Save Session");
            d.add_text_input(
                "Session file:",
                &app.save_dialog_path,
                app.save_dialog_cursor,
                app.save_dialog_focus == SaveDialogFocus::Path,
            );
            d.add_button("Cancel", app.save_dialog_focus == SaveDialogFocus::Cancel);
            d.add_button("Save", app.save_dialog_focus == SaveDialogFocus::Save);
            d
        }
        app::SaveDialogMode::ExportChat => {
            let mut d = FileActionDialog::new("Export As");
            d.add_text_input(
                "File path:",
                &app.save_dialog_path,
                app.save_dialog_cursor,
                app.save_dialog_focus == SaveDialogFocus::Path,
            );
            let fmt_state = DialogDropdownState {
                focused: app.save_dialog_focus == SaveDialogFocus::Format,
                expanded: app.show_format_dropdown,
                selected: if app.export_format == app::ExportFormat::Markdown {
                    0
                } else {
                    1
                },
            };
            d.add_dropdown(
                "Format",
                vec!["Markdown".to_string(), "Plain Text".to_string()],
                fmt_state,
            );
            d.add_button("Cancel", app.save_dialog_focus == SaveDialogFocus::Cancel);
            d.add_button("Export", app.save_dialog_focus == SaveDialogFocus::Save);
            d
        }
    };

    dlg.render(f, area, &app.theme);
}

fn render_load_dialog(f: &mut Frame, app: &App, area: Rect) {
    let dialog_w: u16 = 60;
    let dialog_h: u16 = 6;

    let popup_area = Rect {
        x: (area.width.saturating_sub(dialog_w)) / 2,
        y: (area.height.saturating_sub(dialog_h)) / 2,
        width: dialog_w,
        height: dialog_h,
    };

    f.render_widget(Clear, popup_area);
    f.render_widget(dialog_block("Load Session", &app.theme), popup_area);

    let inner = popup_area.inner(Margin::new(1, 1));

    let label = Paragraph::new(Line::from(Span::styled(
        "  Session file path:",
        Style::default().fg(Color::White),
    )));
    let label_area = Rect {
        x: inner.x,
        y: inner.y,
        width: inner.width,
        height: 1,
    };
    f.render_widget(label, label_area);

    let path_line = Paragraph::new(Line::from(Span::styled(
        format!("  {}", app.load_dialog_path),
        Style::default().fg(Color::White).bg(Color::Rgb(40, 40, 40)),
    )));
    let path_area = Rect {
        x: inner.x,
        y: inner.y + 2,
        width: inner.width,
        height: 1,
    };
    f.render_widget(path_line, path_area);

    let cursor_x = path_area.x + 2 + app.load_dialog_cursor as u16;
    f.set_cursor_position(Position::new(cursor_x, path_area.y));

    let btn_y = inner.y + 4;
    let load_btn = Button::new(
        "Load",
        inner.x + 10,
        btn_y,
        true,
        Color::Green,
        Color::Green,
    );
    let cancel_btn = Button::new("Cancel", inner.x + 22, btn_y, true, Color::Red, Color::Red);

    let (load_text, load_style) = load_btn.render();
    let (cancel_text, cancel_style) = cancel_btn.render();

    let buttons = Line::from(vec![
        Span::raw("          "),
        Span::styled(load_text, load_style),
        Span::raw("     "),
        Span::styled(cancel_text, cancel_style),
        Span::raw("          "),
    ]);

    let btn_area = Rect {
        x: inner.x,
        y: btn_y,
        width: inner.width,
        height: 1,
    };
    f.render_widget(Paragraph::new(buttons), btn_area);
}

fn render_settings_dialog(f: &mut Frame, app: &App, area: Rect) {
    let dialog_w: u16 = 60;
    let dialog_h: u16 = 28;

    let popup_area = Rect {
        x: (area.width.saturating_sub(dialog_w)) / 2,
        y: (area.height.saturating_sub(dialog_h)) / 2,
        width: dialog_w,
        height: dialog_h,
    };

    f.render_widget(Clear, popup_area);
    f.render_widget(dialog_block("Settings", &app.theme), popup_area);

    let inner = popup_area.inner(Margin::new(2, 1));

    let field_labels = [
        ("Proxy URL:", SettingsFocus::Proxy),
        ("Ollama URL:", SettingsFocus::OllamaUrl),
        ("Temperature:", SettingsFocus::Temperature),
        ("Top-P:", SettingsFocus::TopP),
        ("Top-K:", SettingsFocus::TopK),
        ("Freq Penalty:", SettingsFocus::FrequencyPenalty),
        ("Pres Penalty:", SettingsFocus::PresencePenalty),
        ("Max Tokens:", SettingsFocus::MaxTokens),
        ("Effort:", SettingsFocus::ReasoningEffort),
        ("Max Rounds:", SettingsFocus::MaxToolRounds),
        ("Max Retries:", SettingsFocus::MaxRetries),
        ("Justify:", SettingsFocus::Justify),
    ];

    for (i, (label, focus)) in field_labels.iter().enumerate() {
        let field_y = inner.y + i as u16 * 2;

        let label_style = if *focus == app.settings_focus {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };

        let label_para = Paragraph::new(Line::from(Span::styled(
            format!("  {:<13}", label),
            label_style,
        )));
        let label_area = Rect {
            x: inner.x,
            y: field_y,
            width: 15,
            height: 1,
        };
        f.render_widget(label_para, label_area);

        if *focus == SettingsFocus::Justify {
            let checked = app.settings_justify;
            let toggle_text = if checked { "[X]" } else { "[ ]" };
            let toggle_style = if *focus == app.settings_focus {
                Style::default().fg(Color::Black).bg(Color::White)
            } else {
                Style::default().fg(Color::White).bg(Color::Rgb(40, 40, 40))
            };
            let toggle_para = Paragraph::new(Line::from(Span::styled(
                format!(" {:<5} ", toggle_text),
                toggle_style,
            )));
            let toggle_area = Rect {
                x: inner.x + 14,
                y: field_y,
                width: 7,
                height: 1,
            };
            f.render_widget(toggle_para, toggle_area);
        } else {
            let value = match *focus {
                SettingsFocus::Proxy => &app.settings_proxy,
                SettingsFocus::OllamaUrl => &app.settings_ollama_url,
                SettingsFocus::Temperature => &app.settings_temperature,
                SettingsFocus::TopP => &app.settings_top_p,
                SettingsFocus::TopK => &app.settings_top_k,
                SettingsFocus::FrequencyPenalty => &app.settings_frequency_penalty,
                SettingsFocus::PresencePenalty => &app.settings_presence_penalty,
                SettingsFocus::MaxTokens => &app.settings_max_tokens,
                SettingsFocus::ReasoningEffort => &app.settings_reasoning_effort,
                SettingsFocus::MaxToolRounds => &app.settings_max_tool_rounds,
                SettingsFocus::MaxRetries => &app.settings_max_retries,
                _ => "",
            };

            let value_style = if *focus == app.settings_focus {
                Style::default().fg(Color::Black).bg(Color::White)
            } else {
                Style::default().fg(Color::White).bg(Color::Rgb(40, 40, 40))
            };

            let max_w = inner.width.saturating_sub(16);
            let display_val = if value.len() > max_w as usize {
                format!("{}...", &value[..(max_w as usize - 3)])
            } else {
                value.to_string()
            };

            let value_para = Paragraph::new(Line::from(Span::styled(
                format!(
                    " {:<width$} ",
                    display_val,
                    width = max_w.saturating_sub(1) as usize
                ),
                value_style,
            )));
            let value_area = Rect {
                x: inner.x + 14,
                y: field_y,
                width: max_w + 2,
                height: 1,
            };
            f.render_widget(value_para, value_area);

            if *focus == app.settings_focus {
                let cursor_x = value_area.x + 1 + app.settings_cursor as u16;
                f.set_cursor_position(Position::new(cursor_x, field_y));
            }
        }
    }

    let num_fields = 12u16;
    let btn_y = inner.y + num_fields * 2 + 1;
    let save_label = "Save";
    let cancel_label = "Cancel";
    let save_w = save_label.len() as u16 + 4;
    let cancel_w = cancel_label.len() as u16 + 4;
    let gap: u16 = 4;
    let total_btn_w = save_w + gap + cancel_w;
    let btn_start_x = inner.x + (inner.width.saturating_sub(total_btn_w)) / 2;

    let save_btn = Button::new(
        save_label,
        btn_start_x,
        btn_y,
        app.settings_focus == SettingsFocus::Save,
        Color::Green,
        Color::Green,
    );
    let cancel_btn = Button::new(
        cancel_label,
        btn_start_x + save_w + gap,
        btn_y,
        app.settings_focus == SettingsFocus::Cancel,
        Color::Red,
        Color::Red,
    );

    let (save_text, save_style) = save_btn.render();
    let (cancel_text, cancel_style) = cancel_btn.render();

    let buttons = Line::from(vec![
        Span::styled(save_text, save_style),
        Span::raw(" ".repeat(gap as usize)),
        Span::styled(cancel_text, cancel_style),
    ]);

    let btn_area = Rect {
        x: btn_start_x,
        y: btn_y,
        width: total_btn_w,
        height: 1,
    };
    f.render_widget(Paragraph::new(buttons), btn_area);
}

fn flush_line(lines: &mut Vec<Line<'static>>, spans: &mut Vec<Span<'static>>) {
    if !spans.is_empty() {
        let collected: Vec<Span> = std::mem::take(spans);
        lines.push(Line::from(collected));
    }
}

fn render_markdown(text: &str) -> Vec<Line<'static>> {
    let mut md_options = MdOptions::empty();
    md_options.insert(MdOptions::ENABLE_STRIKETHROUGH);
    md_options.insert(MdOptions::ENABLE_TABLES);

    let filtered: Vec<&str> = text
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return true;
            }
            if trimmed.starts_with('+')
                && trimmed.ends_with('+')
                && trimmed.chars().all(|c| c == '+' || c == '-' || c == ' ')
            {
                return false;
            }
            true
        })
        .collect();

    let cleaned = normalize_code_fences(&filtered).join("\n");

    let parser = MdParser::new_ext(&cleaned, md_options);
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut current_spans: Vec<Span<'static>> = Vec::new();
    let mut bold = false;
    let mut italic = false;
    let mut in_code_block = false;
    let mut code_block_lang: Option<String> = None;
    let mut code_block_text = String::new();
    let mut in_table = false;
    let mut table_headers: Vec<String> = Vec::new();
    let mut table_rows: Vec<Vec<String>> = Vec::new();
    let mut current_row: Vec<String> = Vec::new();
    let mut is_header_row = false;

    for event in parser {
        match event {
            MdEvent::Start(tag) => match tag {
                Tag::Heading { level, .. } => {
                    flush_line(&mut lines, &mut current_spans);
                    let prefix = match level {
                        pulldown_cmark::HeadingLevel::H1 => "# ",
                        pulldown_cmark::HeadingLevel::H2 => "## ",
                        pulldown_cmark::HeadingLevel::H3 => "### ",
                        pulldown_cmark::HeadingLevel::H4 => "#### ",
                        pulldown_cmark::HeadingLevel::H5 => "##### ",
                        pulldown_cmark::HeadingLevel::H6 => "###### ",
                    };
                    current_spans.push(Span::styled(
                        prefix.to_string(),
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ));
                    bold = true;
                }
                Tag::CodeBlock(kind) => {
                    flush_line(&mut lines, &mut current_spans);
                    in_code_block = true;
                    code_block_text.clear();
                    code_block_lang = match kind {
                        CodeBlockKind::Fenced(lang) if !lang.is_empty() => Some(lang.to_string()),
                        _ => None,
                    };
                    let header = match &code_block_lang {
                        Some(lang) => format!("[{}]", lang.to_uppercase()),
                        None => "[CODE]".to_string(),
                    };
                    lines.push(Line::from(Span::styled(
                        header,
                        Style::default()
                            .fg(Color::Yellow)
                            .bg(Color::Rgb(30, 60, 120)),
                    )));
                }
                Tag::Table(_alignment) => {
                    flush_line(&mut lines, &mut current_spans);
                    in_table = true;
                    table_headers.clear();
                    table_rows.clear();
                    current_row.clear();
                    is_header_row = true;
                }
                Tag::TableHead => {
                    is_header_row = true;
                    current_row.clear();
                }
                Tag::TableRow => {
                    current_row.clear();
                }
                Tag::TableCell => {}
                Tag::Emphasis => italic = true,
                Tag::Strong => bold = true,
                Tag::Item => {
                    flush_line(&mut lines, &mut current_spans);
                    current_spans.push(Span::styled("  • ", Style::default().fg(Color::Green)));
                }
                _ => {}
            },
            MdEvent::End(tag_end) => match tag_end {
                TagEnd::Paragraph => {
                    flush_line(&mut lines, &mut current_spans);
                    lines.push(Line::from(""));
                }
                TagEnd::Heading(_) => {
                    bold = false;
                    flush_line(&mut lines, &mut current_spans);
                    lines.push(Line::from(""));
                }
                TagEnd::CodeBlock => {
                    in_code_block = false;
                    let code_lines = highlight_code(&code_block_text, code_block_lang.as_deref());
                    for line in code_lines {
                        lines.push(line);
                    }
                    lines.push(Line::from(""));
                }
                TagEnd::Table => {
                    if !table_headers.is_empty() {
                        let col_widths: Vec<usize> = table_headers
                            .iter()
                            .enumerate()
                            .map(|(i, h)| {
                                let data_max = table_rows
                                    .iter()
                                    .filter_map(|r| r.get(i))
                                    .map(|c| c.len())
                                    .max()
                                    .unwrap_or(0);
                                h.len().max(data_max).max(3)
                            })
                            .collect();

                        let border_style = Style::default().fg(Color::DarkGray);

                        // Top border: ┌───┬───┬───┐
                        let mut top = String::from("┌");
                        for (i, &w) in col_widths.iter().enumerate() {
                            top.push_str(&"─".repeat(w + 2));
                            if i < col_widths.len() - 1 {
                                top.push('┬');
                            }
                        }
                        top.push('┐');
                        lines.push(Line::from(Span::styled(top, border_style)));

                        // Header row: │ a │ b │ c │
                        let mut header_spans: Vec<Span> = Vec::new();
                        for (i, h) in table_headers.iter().enumerate() {
                            let w = col_widths[i];
                            let padded = format!("{:<width$}", h, width = w);
                            header_spans.push(Span::styled(
                                format!("│ {} ", padded),
                                Style::default()
                                    .fg(Color::White)
                                    .bg(Color::Rgb(60, 60, 80))
                                    .add_modifier(Modifier::BOLD),
                            ));
                        }
                        header_spans.push(Span::styled("│", border_style));
                        lines.push(Line::from(header_spans));

                        // Separator: ├───┼───┼───┤
                        let mut sep = String::from("├");
                        for (i, &w) in col_widths.iter().enumerate() {
                            sep.push_str(&"─".repeat(w + 2));
                            if i < col_widths.len() - 1 {
                                sep.push('┼');
                            }
                        }
                        sep.push('┤');
                        lines.push(Line::from(Span::styled(sep, border_style)));

                        // Data rows: │ x │ y │ z │
                        for row in &table_rows {
                            let mut row_spans: Vec<Span> = Vec::new();
                            for (i, cell) in row.iter().enumerate() {
                                let w = col_widths.get(i).copied().unwrap_or(10);
                                let padded = format!("{:<width$}", cell, width = w);
                                row_spans
                                    .push(Span::styled(format!("│ {} ", padded), Style::default()));
                            }
                            row_spans.push(Span::styled("│", border_style));
                            lines.push(Line::from(row_spans));
                        }

                        // Bottom border: └───┴───┴───┘
                        let mut bottom = String::from("└");
                        for (i, &w) in col_widths.iter().enumerate() {
                            bottom.push_str(&"─".repeat(w + 2));
                            if i < col_widths.len() - 1 {
                                bottom.push('┴');
                            }
                        }
                        bottom.push('┘');
                        lines.push(Line::from(Span::styled(bottom, border_style)));
                        lines.push(Line::from(""));
                    }
                    in_table = false;
                }
                TagEnd::TableHead => {
                    table_headers = current_row.clone();
                    is_header_row = false;
                }
                TagEnd::TableRow => {
                    if !is_header_row {
                        table_rows.push(current_row.clone());
                    }
                    is_header_row = false;
                }
                TagEnd::TableCell => {
                    let cell_text = current_spans
                        .iter()
                        .map(|s| s.content.to_string())
                        .collect::<String>();
                    current_row.push(cell_text);
                    current_spans.clear();
                }
                TagEnd::Emphasis => italic = false,
                TagEnd::Strong => bold = false,
                TagEnd::Item => {
                    flush_line(&mut lines, &mut current_spans);
                }
                _ => {}
            },
            MdEvent::Text(text) => {
                if in_code_block {
                    code_block_text.push_str(&text);
                } else {
                    let style = if in_table && is_header_row {
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        let mut s = Style::default();
                        if bold {
                            s = s.add_modifier(Modifier::BOLD);
                        }
                        if italic {
                            s = s.add_modifier(Modifier::ITALIC);
                        }
                        s
                    };
                    current_spans.push(Span::styled(text.to_string(), style));
                }
            }
            MdEvent::Code(code) => {
                current_spans.push(Span::styled(
                    format!("`{}`", code),
                    Style::default().fg(Color::Cyan).bg(Color::Rgb(40, 40, 60)),
                ));
            }
            MdEvent::SoftBreak | MdEvent::HardBreak => {
                if in_code_block {
                    code_block_text.push('\n');
                } else if in_table {
                    current_spans.clear();
                } else {
                    flush_line(&mut lines, &mut current_spans);
                }
            }
            MdEvent::Rule => {
                flush_line(&mut lines, &mut current_spans);
                lines.push(Line::from(Span::styled(
                    "─────────────────────────────────────",
                    Style::default().fg(Color::DarkGray),
                )));
                lines.push(Line::from(""));
            }
            _ => {}
        }
    }

    flush_line(&mut lines, &mut current_spans);

    if lines.is_empty() {
        lines.push(Line::from(""));
    }

    lines
}

fn span_display_width(span: &Span) -> usize {
    span.content.chars().count()
}

fn wrap_and_justify_lines(
    lines: &[Line<'static>],
    width: usize,
    justify: bool,
) -> Vec<Line<'static>> {
    let mut result = Vec::new();
    let mut para: Vec<Line<'static>> = Vec::new();

    for line in lines {
        let w: usize = line.spans.iter().map(|s| span_display_width(s)).sum();
        let is_code = line
            .spans
            .iter()
            .any(|s| s.style.bg == Some(Color::Rgb(30, 60, 120)));
        let is_table = line.spans.iter().any(|s| {
            s.content.contains('│')
                || s.content.contains('┌')
                || s.content.contains('└')
                || s.content.contains('├')
                || s.content.contains('┬')
                || s.content.contains('┴')
                || s.content.contains('┼')
        });
        if w == 0 || is_code || is_table {
            if !para.is_empty() {
                if justify {
                    justify_paragraph_into(&mut result, &para, width);
                } else {
                    result.append(&mut para);
                }
                para.clear();
            }
            result.push(line.clone());
        } else if w <= width {
            para.push(line.clone());
        } else {
            para.extend(wrap_single_line(line.clone(), width));
        }
    }
    if !para.is_empty() {
        if justify {
            justify_paragraph_into(&mut result, &para, width);
        } else {
            result.extend(para);
        }
    }
    result
}

fn justify_paragraph_into(out: &mut Vec<Line<'static>>, lines: &[Line<'static>], width: usize) {
    let last = lines.len() - 1;
    for (i, line) in lines.iter().enumerate() {
        let w: usize = line.spans.iter().map(|s| span_display_width(s)).sum();
        let is_indented = line
            .spans
            .first()
            .map(|s| s.content.starts_with(' '))
            .unwrap_or(false);
        if i < last && w < width && !is_indented {
            out.push(Line::from(distribute_spaces(&line.spans, w, width)));
        } else {
            out.push(line.clone());
        }
    }
}

fn distribute_spaces(
    spans: &[Span<'static>],
    current_w: usize,
    target_w: usize,
) -> Vec<Span<'static>> {
    let extra = target_w - current_w;

    let normalized: Vec<Span<'static>> = if spans.len() == 1 && extra > 0 {
        let text = spans[0].content.as_ref();
        let style = spans[0].style;
        let mut parts: Vec<Span<'static>> = Vec::new();
        let mut current = String::new();
        for ch in text.chars() {
            if ch == ' ' {
                if !current.is_empty() {
                    parts.push(Span::styled(std::mem::take(&mut current), style));
                }
                parts.push(Span::styled(" ", style));
            } else {
                current.push(ch);
            }
        }
        if !current.is_empty() {
            parts.push(Span::styled(current, style));
        }
        parts
    } else {
        spans.to_vec()
    };

    let gaps: Vec<usize> = normalized
        .iter()
        .enumerate()
        .filter(|(_, s)| s.content.chars().all(|c| c == ' '))
        .map(|(i, _)| i)
        .collect();

    if gaps.is_empty() {
        return normalized;
    }

    let per_gap = extra / gaps.len();
    let remainder = extra % gaps.len();
    let mut result = Vec::with_capacity(normalized.len());
    let mut gap_idx = 0;

    for (i, span) in normalized.iter().enumerate() {
        if gaps.contains(&i) {
            let add = if gap_idx < remainder { 1 } else { 0 };
            let total = 1 + per_gap + add;
            result.push(Span::styled(" ".repeat(total), span.style));
            gap_idx += 1;
        } else {
            result.push(span.clone());
        }
    }
    result
}

fn wrap_single_line(line: Line<'static>, width: usize) -> Vec<Line<'static>> {
    let mut tokens: Vec<(String, Style)> = Vec::new();
    for span in &line.spans {
        let text = span.content.to_string();
        let style = span.style;
        for (i, part) in text.split(' ').enumerate() {
            if i > 0 {
                tokens.push((" ".to_string(), style));
            }
            if !part.is_empty() {
                tokens.push((part.to_string(), style));
            }
        }
    }

    let mut lines = Vec::new();
    let mut cur: Vec<Span<'static>> = Vec::new();
    let mut cur_w = 0;

    for (text, style) in tokens {
        let tw = text.chars().count();
        if cur_w + tw > width && cur_w > 0 {
            lines.push(Line::from(std::mem::take(&mut cur)));
            cur_w = 0;
        }
        cur.push(Span::styled(text, style));
        cur_w += tw;
    }
    if !cur.is_empty() {
        lines.push(Line::from(cur));
    }
    lines
}

fn highlight_code(code: &str, lang: Option<&str>) -> Vec<Line<'static>> {
    use syntect::easy::HighlightLines;
    use syntect::highlighting::ThemeSet;
    use syntect::parsing::SyntaxSet;

    use std::sync::OnceLock;
    static SS: OnceLock<SyntaxSet> = OnceLock::new();
    static TS: OnceLock<ThemeSet> = OnceLock::new();

    let ss = SS.get_or_init(SyntaxSet::load_defaults_newlines);
    let ts = TS.get_or_init(ThemeSet::load_defaults);

    let syntax = lang
        .and_then(|l| ss.find_syntax_by_token(l))
        .unwrap_or_else(|| ss.find_syntax_plain_text());

    let mut h = HighlightLines::new(syntax, &ts.themes["base16-ocean.dark"]);

    let bg = Color::Rgb(30, 60, 120);
    let mut result: Vec<Line<'static>> = Vec::new();

    for line in code.lines() {
        let ranges = h.highlight_line(line, ss).unwrap_or_default();
        let mut spans: Vec<Span<'static>> = Vec::new();
        for (style, text) in ranges {
            let fg = Color::Rgb(style.foreground.r, style.foreground.g, style.foreground.b);
            spans.push(Span::styled(
                text.to_string(),
                Style::default().fg(fg).bg(bg),
            ));
        }
        result.push(Line::from(spans));
    }

    if code.ends_with('\n') || code.is_empty() {
        result.push(Line::from(Span::styled(" ", Style::default().bg(bg))));
    }

    result
}

fn normalize_code_fences(lines: &[&str]) -> Vec<String> {
    let mut result = Vec::new();
    let mut in_code_block = false;
    let mut fence_indent: usize = 0;

    for line in lines {
        if in_code_block {
            let trimmed = line.trim_start();
            let current_indent = line.len() - trimmed.len();

            if trimmed.starts_with("```") && current_indent <= fence_indent {
                in_code_block = false;
                result.push(format!("{}{}", " ".repeat(fence_indent), trimmed));
                continue;
            }

            if line.trim().is_empty() {
                result.push(format!("{}{}", " ".repeat(fence_indent), ""));
            } else if current_indent < fence_indent {
                result.push(format!("{}{}", " ".repeat(fence_indent), trimmed));
            } else {
                result.push(line.to_string());
            }
        } else {
            let trimmed = line.trim_start();
            if trimmed.starts_with("```") {
                let indent = line.len() - trimmed.len();
                if indent > 0 {
                    in_code_block = true;
                    fence_indent = indent;
                    result.push(line.to_string());
                    continue;
                }
            }
            result.push(line.to_string());
        }
    }
    result
}
