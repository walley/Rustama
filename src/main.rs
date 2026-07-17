use std::io;
use std::time::Duration;

use crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, Event, MouseButton, MouseEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};

use pulldown_cmark::{
    CodeBlockKind, Event as MdEvent, Options as MdOptions, Parser as MdParser, Tag, TagEnd,
};

use ratatui::prelude::*;
use ratatui::layout::Offset;
use ratatui::widgets::*;

mod app;
mod config;
mod ui;
use app::{ActiveMenu, App, ChatMessage, FileDialogFocus, Focus, InputMode, ModelDialogFocus, SaveDialogFocus};
use config::Config;
use ui::Button;

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
            match event::read()? {
                Event::Key(key) => {
                    app.handle_key(key);
                }
                Event::Mouse(mouse) => match mouse.kind {
                    MouseEventKind::ScrollUp => app.scroll_up(),
                    MouseEventKind::ScrollDown => app.scroll_down(),
                    MouseEventKind::Down(MouseButton::Left) => {
                        let size = terminal.size()?;
                        app.handle_click(mouse.column, mouse.row, size.width, size.height);
                    }
                    _ => {}
                },
                _ => {}
            }
            while event::poll(Duration::ZERO)? {
                match event::read()? {
                    Event::Key(key) => {
                        app.handle_key(key);
                    }
                    Event::Mouse(mouse) => match mouse.kind {
                        MouseEventKind::ScrollUp => app.scroll_up(),
                        MouseEventKind::ScrollDown => app.scroll_down(),
                        MouseEventKind::Down(MouseButton::Left) => {
                            let size = terminal.size()?;
                            app.handle_click(mouse.column, mouse.row, size.width, size.height);
                        }
                        _ => {}
                    },
                    _ => {}
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

    render_menu_bar(f, app, main_chunks[0]);
    render_keybar(f, app, main_chunks[2]);

    let content_chunks =
        Layout::vertical([Constraint::Min(3), Constraint::Length(5)]).split(main_chunks[1]);

    render_output(f, app, content_chunks[0]);
    render_input(f, app, content_chunks[1]);

    if app.active_menu != ActiveMenu::None {
        render_submenu(f, app, main_chunks[0]);
    }

    if app.show_about {
        render_about_popup(f, area);
    }

    if app.show_model_dialog {
        render_model_dialog(f, app, area);
    }

    if app.show_file_dialog {
        render_file_dialog(f, app, area);
    }

    if app.show_save_dialog {
        render_save_dialog(f, app, area);
    }
}

fn render_menu_bar(f: &mut Frame, app: &App, area: Rect) {
    let normal_style = Style::default().fg(Color::White).bg(Color::DarkGray);
    let selected_style = Style::default().fg(Color::Black).bg(Color::White);

    let file_style = if app.active_menu == ActiveMenu::File {
        selected_style
    } else {
        normal_style
    };
    let options_style = if app.active_menu == ActiveMenu::Options {
        selected_style
    } else {
        normal_style
    };
    let help_style = if app.active_menu == ActiveMenu::Help {
        selected_style
    } else {
        normal_style
    };

    let agentic_indicator = if app.agentic_mode {
        Span::styled(
            " [AGENTIC] ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(" ", normal_style)
    };

    let menu_bar = Line::from(vec![
        Span::styled(" File ", file_style),
        Span::styled(" ", normal_style),
        Span::styled(" Options ", options_style),
        Span::styled(" ", normal_style),
        Span::styled(" Help ", help_style),
        agentic_indicator,
    ]);

    f.render_widget(Paragraph::new(menu_bar).style(normal_style), area);
}

fn render_output(f: &mut Frame, app: &mut App, area: Rect) {
    let mut lines: Vec<Line<'static>> = Vec::new();

    for msg in &app.messages {
        match msg {
            ChatMessage::User(text) => {
                lines.push(Line::from(vec![
                    Span::styled(
                        " > ",
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        text.clone(),
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));
                lines.push(Line::from(""));
            }
            ChatMessage::Assistant(text) => {
                let mut md_lines = render_markdown(text);
                lines.append(&mut md_lines);
            }
            ChatMessage::System(text) => {
                for text_line in text.lines() {
                    lines.push(Line::from(Span::styled(
                        format!("  {}", text_line),
                        Style::default()
                            .fg(Color::DarkGray)
                            .add_modifier(Modifier::ITALIC),
                    )));
                }
                lines.push(Line::from(""));
            }
            ChatMessage::App(text) => {
                for text_line in text.lines() {
                    lines.push(Line::from(Span::styled(
                        format!("  {}", text_line),
                        Style::default()
                            .fg(Color::DarkGray)
                            .add_modifier(Modifier::ITALIC),
                    )));
                }
                lines.push(Line::from(""));
            }
            ChatMessage::FileContent { name, content } => {
                lines.push(Line::from(Span::styled(
                    format!("  \u{1F4C4} {}", name),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )));
                let mut md_lines = render_markdown(content);
                lines.append(&mut md_lines);
                lines.push(Line::from(""));
            }
            ChatMessage::ToolCall { name, arguments } => {
                lines.push(Line::from(vec![
                    Span::styled(
                        " \u{2699} ",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("TOOL CALL: {}", name),
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
                    lines.push(Line::from(Span::styled(
                        format!("    {}", arg_line),
                        Style::default().fg(Color::DarkGray),
                    )));
                }
                lines.push(Line::from(""));
            }
            ChatMessage::ToolResult { name, content } => {
                lines.push(Line::from(vec![
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
                    lines.push(Line::from(Span::styled(
                        format!("    {}", result_line),
                        Style::default().fg(Color::DarkGray),
                    )));
                }
                if content.lines().count() > 20 {
                    lines.push(Line::from(Span::styled(
                        format!("    ... ({} more lines)", content.lines().count() - 20),
                        Style::default().fg(Color::DarkGray),
                    )));
                }
                lines.push(Line::from(""));
            }
        }
    }

    if app.is_loading && !app.streaming_text.is_empty() {
        let mut md_lines = render_markdown(&app.streaming_text);
        lines.append(&mut md_lines);
    } else if app.is_loading {
        lines.push(Line::from(Span::styled(
            "  Streaming response...",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::SLOW_BLINK),
        )));
    }

    let output_width = area.width.saturating_sub(4) as usize;
    let lines = wrap_and_justify_lines(lines, output_width);

    let total_lines = lines.len() as u16;
    let visible_height = area.height.saturating_sub(2);
    let max_scroll = total_lines.saturating_sub(visible_height);
    let scroll = app.scroll_offset.min(max_scroll);
    app.scroll_offset = scroll;

    let focus_style = if app.focus == Focus::Output {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Output ")
        .border_style(focus_style);

    let paragraph = Paragraph::new(lines).block(block).scroll((scroll, 0));

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
    let stats_str = if app.token_stats.prompt_tokens > 0 || app.token_stats.response_tokens > 0 {
        format!(
            " in:{} out:{} {}ms ",
            app.token_stats.prompt_tokens,
            app.token_stats.response_tokens,
            app.token_stats.total_duration_ms,
        )
    } else {
        String::new()
    };

    let (title, border_style) = match (&app.input_mode, &app.focus) {
        (InputMode::Input, Focus::Input) => (
            format!(" Input (Alt+Enter: send){} ", stats_str),
            Style::default().fg(Color::Green),
        ),
        (_, Focus::Input) => (
            format!(" Input (i or Enter to type){} ", stats_str),
            Style::default().fg(Color::Yellow),
        ),
        _ => (
            format!(" Input (i or Enter to type){} ", stats_str),
            Style::default().fg(Color::DarkGray),
        ),
    };

    let inner_h = area.height.saturating_sub(2) as usize;

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(border_style);

    if app.input_text.is_empty() && app.input_mode != InputMode::Input {
        let placeholder = Paragraph::new(vec![
            Line::from(Span::styled(
                "Type a message...",
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC),
            )),
        ])
        .block(block);
        f.render_widget(placeholder, area);
        render_send_button(f, app, area);
        return;
    }

    let lines = app.input_text.lines().collect::<Vec<_>>();
    let (cur_row, cur_col) = app.cursor_row_col();
    let total_lines = lines.len();

    let scroll_row = if total_lines > inner_h && inner_h > 0 {
        if cur_row + 1 > inner_h {
            cur_row + 2 - inner_h
        } else {
            0
        }
    } else {
        0
    };

    let display_lines: Vec<Line> = lines
        .iter()
        .enumerate()
        .skip(scroll_row)
        .take(inner_h)
        .map(|(row_idx, line)| {
            let chars: Vec<char> = line.chars().collect();
            let mut spans: Vec<Span> = Vec::new();

            let sel_range = app.selection_range();

            for (col_idx, c) in chars.iter().enumerate() {
                let char_pos = app.pos_from_row_col(row_idx, col_idx);
                let is_cursor = app.input_mode == InputMode::Input
                    && row_idx == cur_row
                    && col_idx == cur_col;
                let is_selected = sel_range
                    .map_or(false, |(start, end)| char_pos >= start && char_pos < end);

                let style = if is_cursor && is_selected {
                    Style::default().bg(Color::Rgb(100, 100, 255)).fg(Color::White)
                } else if is_cursor {
                    Style::default().bg(Color::White).fg(Color::Black)
                } else if is_selected {
                    Style::default().bg(Color::Rgb(50, 50, 150)).fg(Color::White)
                } else {
                    Style::default()
                };
                spans.push(Span::styled(c.to_string(), style));
            }

            if app.input_mode == InputMode::Input && row_idx == cur_row && cur_col == chars.len()
            {
                spans.push(Span::styled(" ", Style::default().bg(Color::White)));
            }

            Line::from(spans)
        })
        .collect();

    let paragraph = Paragraph::new(display_lines).block(block);

    f.render_widget(paragraph, area);
    render_send_button(f, app, area);
}

fn render_send_button(f: &mut Frame, app: &App, area: Rect) {
    let has_text = !app.input_text.trim().is_empty();
    let btn_x = area.x + area.width.saturating_sub(13);
    let btn_y = area.y + area.height.saturating_sub(1);

    let send_btn = Button::new("send", btn_x, btn_y, has_text, Color::DarkGray, Color::Green);

    let (btn_text, btn_style) = send_btn.render();
    let btn_area = Rect {
        x: btn_x,
        y: btn_y,
        width: send_btn.width,
        height: 1,
    };

    f.render_widget(Clear, btn_area);
    f.render_widget(Paragraph::new(Line::from(Span::styled(btn_text, btn_style))), btn_area);
}

fn render_keybar(f: &mut Frame, app: &App, area: Rect) {
    let agentic_label = if app.agentic_mode {
        " AGENTIC "
    } else {
        ""
    };

    let focus_label = match app.focus {
        Focus::Output => " [OUTPUT] ",
        Focus::Input => " [INPUT] ",
    };

    let text = match app.input_mode {
        InputMode::Normal => {
            if !app.status_message.is_empty() {
                format!(
                    " {} | q:Quit i:Input Tab:Menu Ctrl+S:Save{}{}",
                    app.status_message, agentic_label, focus_label
                )
            } else {
                format!(
                    " q:Quit  i:Input  F9/Tab:Menu  Ctrl+S:Save  Mouse:Click/Scroll{}{}",
                    agentic_label, focus_label
                )
            }
        }
        InputMode::Input => {
            format!(
                " Alt+Enter:Send  Enter:Newline  \u{2190}\u{2191}\u{2193}\u{2192}:Cursor{}",
                agentic_label
            )
        }
        InputMode::Menu => " \u{2190}\u{2192}:Navigate  \u{2191}\u{2193}:Select  Enter:Open  Esc:Close"
            .to_string(),
    };

    let keybar = Paragraph::new(Line::from(Span::styled(
        text,
        Style::default().fg(Color::White).bg(Color::DarkGray),
    )))
    .style(Style::default().bg(Color::DarkGray));

    f.render_widget(keybar, area);
}

fn render_submenu(f: &mut Frame, app: &App, menu_bar_area: Rect) {
    let items = app.menu_item_names();
    if items.is_empty() {
        return;
    }

    let menu_width = items.iter().map(|s| s.len()).max().unwrap_or(10) as u16 + 4;
    let x_offset = match app.active_menu {
        ActiveMenu::File => 0,
        ActiveMenu::Options => 7,
        ActiveMenu::Help => 17,
        _ => 0,
    };

    let popup_area = Rect {
        x: menu_bar_area.x + x_offset,
        y: menu_bar_area.y + 1,
        width: menu_width,
        height: (items.len() as u16) + 2,
    };

    let list_items: Vec<ListItem> = items
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let style = if i == app.menu_selection {
                Style::default().fg(Color::Black).bg(Color::White)
            } else {
                Style::default()
            };
            ListItem::new(Line::from(Span::styled(format!(" {} ", name), style)))
        })
        .collect();

    let list = List::new(list_items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::White))
            .style(Style::default().bg(Color::Black)),
    );

    f.render_widget(Clear, popup_area);
    f.render_widget(list, popup_area);
}

fn dialog_block(title: &str) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", title))
        .border_style(Style::default().fg(Color::Cyan))
        .style(Style::default().bg(Color::Black))
        .shadow(Shadow::new(dimmed()).offset(Offset::new(1, 1)))
}

fn render_about_popup(f: &mut Frame, area: Rect) {
    let popup_width = 50.min(area.width.saturating_sub(4));
    let popup_height = 10.min(area.height.saturating_sub(4));
    let popup_area = Rect {
        x: (area.width.saturating_sub(popup_width)) / 2,
        y: (area.height.saturating_sub(popup_height)) / 2,
        width: popup_width,
        height: popup_height,
    };

    f.render_widget(Clear, popup_area);
    f.render_widget(dialog_block("About"), popup_area);

    let inner = popup_area.inner(Margin::new(1, 1));

    let about_text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Rustama",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("  A terminal interface for Ollama LLM."),
        Line::from("  Supports markdown rendering and saving."),
        Line::from(""),
        Line::from("  Built with ratatui + crossterm"),
        Line::from(""),
    ];

    let text_area = Rect {
        x: inner.x,
        y: inner.y,
        width: inner.width,
        height: inner.height - 1,
    };
    f.render_widget(Paragraph::new(about_text), text_area);

    let btn_y = inner.y + inner.height - 1;
    let ok_btn = Button::new(
        "OK",
        inner.x + 20,
        btn_y,
        true,
        Color::Cyan,
        Color::Cyan,
    );

    let (ok_text, ok_style) = ok_btn.render();
    let buttons = Line::from(vec![
        Span::raw("                    "),
        Span::styled(ok_text, ok_style),
        Span::raw("                    "),
    ]);

    let btn_area = Rect {
        x: inner.x,
        y: btn_y,
        width: inner.width,
        height: 1,
    };
    f.render_widget(Paragraph::new(buttons), btn_area);
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
    f.render_widget(dialog_block("Select Model"), popup_area);

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
    let entry_count = app.file_dialog_entries.len().min(12) as u16;
    let dialog_w = 60u16;
    let dialog_h = entry_count + 5;

    let popup_area = Rect {
        x: (area.width.saturating_sub(dialog_w)) / 2,
        y: (area.height.saturating_sub(dialog_h)) / 2,
        width: dialog_w,
        height: dialog_h,
    };

    f.render_widget(Clear, popup_area);
    f.render_widget(dialog_block("Load File"), popup_area);

    let inner = popup_area.inner(Margin::new(1, 1));

    let path_display = app.file_dialog_path.display().to_string();
    let path_str = if path_display.len() > (inner.width as usize) {
        format!("...{}", &path_display[path_display.len() - inner.width as usize + 3..])
    } else {
        path_display
    };
    let path_line = Paragraph::new(Line::from(Span::styled(
        format!("  {}", path_str),
        Style::default().fg(Color::DarkGray),
    )));
    let path_area = Rect {
        x: inner.x,
        y: inner.y,
        width: inner.width,
        height: 1,
    };
    f.render_widget(path_line, path_area);

    let list_y = inner.y + 1;
    let list_h = entry_count;
    let list_area = Rect {
        x: inner.x,
        y: list_y,
        width: inner.width,
        height: list_h,
    };

    let visible_start = app.file_dialog_scroll;
    let visible_end = (visible_start + list_h as usize).min(app.file_dialog_entries.len());

    let items: Vec<ListItem> = app.file_dialog_entries[visible_start..visible_end]
        .iter()
        .enumerate()
        .map(|(i, (name, is_dir))| {
            let real_idx = visible_start + i;
            let is_selected = real_idx == app.file_dialog_selection;

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

            let icon = if name == ".." {
                "  "
            } else if *is_dir {
                "/ "
            } else {
                "  "
            };

            let name_style = if is_selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else if *is_dir {
                Style::default()
                    .fg(Color::Blue)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            spans.push(Span::styled(icon, name_style));
            spans.push(Span::styled(name.clone(), name_style));

            ListItem::new(Line::from(spans))
        })
        .collect();

    let list = List::new(items);
    f.render_widget(list, list_area);

    let btn_y = inner.y + list_h + 2;
    let open_btn = Button::new(
        "Open",
        inner.x + 8,
        btn_y,
        app.file_dialog_focus == FileDialogFocus::Open,
        Color::Green,
        Color::Green,
    );
    let cancel_btn = Button::new(
        "Cancel",
        inner.x + 21,
        btn_y,
        app.file_dialog_focus == FileDialogFocus::Cancel,
        Color::Red,
        Color::Red,
    );

    let (open_text, open_style) = open_btn.render();
    let (cancel_text, cancel_style) = cancel_btn.render();

    let buttons = Line::from(vec![
        Span::raw("        "),
        Span::styled(open_text, open_style),
        Span::raw("     "),
        Span::styled(cancel_text, cancel_style),
        Span::raw("        "),
    ]);

    let btn_area = Rect {
        x: inner.x,
        y: btn_y,
        width: inner.width,
        height: 1,
    };
    f.render_widget(Paragraph::new(buttons), btn_area);
}

fn render_save_dialog(f: &mut Frame, app: &App, area: Rect) {
    let dialog_w: u16 = 60;
    let dialog_h: u16 = 7;

    let popup_area = Rect {
        x: (area.width.saturating_sub(dialog_w)) / 2,
        y: (area.height.saturating_sub(dialog_h)) / 2,
        width: dialog_w,
        height: dialog_h,
    };

    f.render_widget(Clear, popup_area);
    f.render_widget(dialog_block("Save As"), popup_area);

    let inner = popup_area.inner(Margin::new(1, 1));

    let label = Paragraph::new(Line::from(Span::styled(
        "  File path:",
        Style::default().fg(Color::White),
    )));
    let label_area = Rect {
        x: inner.x,
        y: inner.y,
        width: inner.width,
        height: 1,
    };
    f.render_widget(label, label_area);

    let path_style = if app.save_dialog_focus == SaveDialogFocus::Path {
        Style::default()
            .fg(Color::Black)
            .bg(Color::White)
    } else {
        Style::default().fg(Color::White).bg(Color::Rgb(40, 40, 40))
    };

    let path_display = if app.save_dialog_path.is_empty() && app.save_dialog_focus == SaveDialogFocus::Path {
        format!("{} ", " ".repeat(app.save_dialog_cursor))
    } else {
        let chars: Vec<char> = app.save_dialog_path.chars().collect();
        let mut display = String::new();
        for (i, c) in chars.iter().enumerate() {
            if i == app.save_dialog_cursor && app.save_dialog_focus == SaveDialogFocus::Path {
                display.push_str(&format!("[{}]", c));
            } else {
                display.push(*c);
            }
        }
        if app.save_dialog_cursor == chars.len() && app.save_dialog_focus == SaveDialogFocus::Path {
            display.push(' ');
        }
        display
    };

    let path_line = Paragraph::new(Line::from(Span::styled(
        format!("  {}", path_display),
        path_style,
    )));
    let path_area = Rect {
        x: inner.x,
        y: inner.y + 2,
        width: inner.width,
        height: 1,
    };
    f.render_widget(path_line, path_area);

    let btn_y = inner.y + 4;
    let save_btn = Button::new(
        "Save",
        inner.x + 10,
        btn_y,
        app.save_dialog_focus == SaveDialogFocus::Save,
        Color::Green,
        Color::Green,
    );
    let cancel_btn = Button::new(
        "Cancel",
        inner.x + 23,
        btn_y,
        app.save_dialog_focus == SaveDialogFocus::Cancel,
        Color::Red,
        Color::Red,
    );

    let (save_text, save_style) = save_btn.render();
    let (cancel_text, cancel_style) = cancel_btn.render();

    let buttons = Line::from(vec![
        Span::raw("          "),
        Span::styled(save_text, save_style),
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

fn flush_line(lines: &mut Vec<Line<'static>>, spans: &mut Vec<Span<'static>>) {
    if !spans.is_empty() {
        let collected: Vec<Span> = spans.drain(..).collect();
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
                        CodeBlockKind::Fenced(lang) if !lang.is_empty() => {
                            Some(lang.to_string())
                        }
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
                    current_spans.push(Span::styled(
                        "  • ",
                        Style::default().fg(Color::Green),
                    ));
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
                                row_spans.push(Span::styled(
                                    format!("│ {} ", padded),
                                    Style::default(),
                                ));
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
                    Style::default()
                        .fg(Color::Cyan)
                        .bg(Color::Rgb(40, 40, 60)),
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

fn wrap_and_justify_lines(lines: Vec<Line<'static>>, width: usize) -> Vec<Line<'static>> {
    let mut result = Vec::new();
    let mut para: Vec<Line<'static>> = Vec::new();

    for line in lines {
        let w: usize = line.spans.iter().map(|s| span_display_width(s)).sum();
        let is_code = line.spans.iter().any(|s| s.style.bg == Some(Color::Rgb(30, 60, 120)));
        if w == 0 {
            if !para.is_empty() {
                justify_paragraph_into(&mut result, &para, width);
                para.clear();
            }
            result.push(line);
        } else if is_code {
            if !para.is_empty() {
                justify_paragraph_into(&mut result, &para, width);
                para.clear();
            }
            result.push(line);
        } else if w <= width {
            para.push(line);
        } else {
            para.extend(wrap_single_line(line, width));
        }
    }
    if !para.is_empty() {
        justify_paragraph_into(&mut result, &para, width);
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
    let gaps: Vec<usize> = spans
        .iter()
        .enumerate()
        .filter(|(_, s)| s.content.chars().all(|c| c == ' '))
        .map(|(i, _)| i)
        .collect();

    if gaps.is_empty() {
        return spans.to_vec();
    }

    let per_gap = extra / gaps.len();
    let remainder = extra % gaps.len();
    let mut result = Vec::with_capacity(spans.len());
    let mut gap_idx = 0;

    for (i, span) in spans.iter().enumerate() {
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

    let ss = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();

    let syntax = lang
        .and_then(|l| ss.find_syntax_by_token(l))
        .unwrap_or_else(|| ss.find_syntax_plain_text());

    let mut h = HighlightLines::new(syntax, &ts.themes["base16-ocean.dark"]);

    let bg = Color::Rgb(30, 60, 120);
    let mut result: Vec<Line<'static>> = Vec::new();

    for line in code.lines() {
        let ranges = h.highlight_line(line, &ss).unwrap_or_default();
        let mut spans: Vec<Span<'static>> = Vec::new();
        for (style, text) in ranges {
            let fg = Color::Rgb(
                style.foreground.r,
                style.foreground.g,
                style.foreground.b,
            );
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
