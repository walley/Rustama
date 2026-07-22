use ratatui::layout::{Margin, Offset, Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, ListItem, Paragraph, Shadow};
use ratatui::widgets::dimmed;
use ratatui::Frame;

#[derive(Debug, Clone)]
pub struct Theme {
    pub dialog_border: Color,
    pub dialog_bg: Color,
    pub button_active_bg: Color,
    pub button_inactive_fg: Color,
    pub list_selected_bg: Color,
    pub list_selected_fg: Color,
    pub list_selected_indicator_bg: Color,
    pub dir_fg: Color,
    pub file_fg: Color,
    pub path_fg: Color,
    pub focus_fg: Color,
    pub accent: Color,
    pub thinking_fg: Color,
}

impl Theme {
    pub fn default() -> Self {
        Theme {
            dialog_border: Color::Cyan,
            dialog_bg: Color::DarkGray,
            button_active_bg: Color::Green,
            button_inactive_fg: Color::Green,
            list_selected_bg: Color::White,
            list_selected_fg: Color::Black,
            list_selected_indicator_bg: Color::Cyan,
            dir_fg: Color::Blue,
            file_fg: Color::White,
            path_fg: Color::DarkGray,
            focus_fg: Color::Yellow,
            accent: Color::Cyan,
            thinking_fg: Color::Yellow,
        }
    }

    pub fn dark() -> Self {
        Theme {
            dialog_border: Color::Cyan,
            dialog_bg: Color::Rgb(20, 20, 20),
            button_active_bg: Color::Green,
            button_inactive_fg: Color::Green,
            list_selected_bg: Color::White,
            list_selected_fg: Color::Black,
            list_selected_indicator_bg: Color::Cyan,
            dir_fg: Color::LightBlue,
            file_fg: Color::White,
            path_fg: Color::DarkGray,
            focus_fg: Color::Yellow,
            accent: Color::Cyan,
            thinking_fg: Color::Yellow,
        }
    }
}

pub fn dialog_block(title: &str, theme: &Theme) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", title))
        .border_style(Style::default().fg(theme.dialog_border))
        .style(Style::default().bg(theme.dialog_bg))
        .shadow(Shadow::new(dimmed()).offset(Offset::new(1, 1)))
}

pub struct Button {
    pub name: String,
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub active: bool,
    pub fg: Color,
    pub bg: Color,
    pub hotkey: Option<char>,
}

impl Button {
    pub fn new(
        name: &str,
        x: u16,
        y: u16,
        active: bool,
        fg: Color,
        bg: Color,
    ) -> Self {
        let width = name.len() as u16 + 4;
        Button {
            name: name.to_string(),
            x,
            y,
            width,
            active,
            fg,
            bg,
            hotkey: None,
        }
    }

    pub fn is_clicked(&self, col: u16, row: u16) -> bool {
        col >= self.x && col < self.x + self.width && row == self.y
    }

    pub fn render(&self) -> (String, Style) {
        let display = if self.hotkey.is_some() {
            let hotkey = self.hotkey.unwrap();
            let idx = self.name.to_lowercase().chars().position(|c| c == hotkey);
            if let Some(idx) = idx {
                let before: String = self.name.chars().take(idx).collect();
                let at: String = self.name.chars().skip(idx).take(1).collect();
                let after: String = self.name.chars().skip(idx + 1).collect();
                format!("[{}{}{}]", before, at.to_uppercase(), after)
            } else {
                format!("[{}]", self.name)
            }
        } else {
            format!("[{}]", self.name)
        };

        let style = if self.active {
            Style::default()
                .fg(Color::Black)
                .bg(self.bg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(self.fg)
        };

        (display, style)
    }
}

pub struct Dropdown {
    pub label: String,
    pub items: Vec<String>,
    pub selected: usize,
    pub x: u16,
    pub y: u16,
    pub focused: bool,
    pub expanded: bool,
}

impl Dropdown {
    pub fn new(label: &str, items: Vec<String>, x: u16, y: u16) -> Self {
        Dropdown {
            label: label.to_string(),
            items,
            selected: 0,
            x,
            y,
            focused: false,
            expanded: false,
        }
    }

    pub fn total_height(&self) -> u16 {
        if self.expanded {
            1 + self.items.len() as u16
        } else {
            1
        }
    }

    pub fn render_line(&self, line_offset: u16, theme: &Theme) -> Vec<Line<'static>> {
        if !self.expanded {
            if line_offset != 0 {
                return Vec::new();
            }
            let display = if self.items.is_empty() {
                format!("  {}: []", self.label)
            } else {
                format!("  {}: [{} \u{25bc}]", self.label, self.items[self.selected])
            };
            let style = if self.focused {
                Style::default().fg(theme.focus_fg).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.accent)
            };
            vec![Line::from(Span::styled(display, style))]
        } else {
            let mut lines = Vec::new();
            if line_offset == 0 {
                let display = if self.items.is_empty() {
                    format!("  {}: [] \u{25b2}", self.label)
                } else {
                    format!("  {}: [{} \u{25b2}]", self.label, self.items[self.selected])
                };
                lines.push(Line::from(Span::styled(display, Style::default().fg(theme.focus_fg))));
            }
            for (i, item) in self.items.iter().enumerate() {
                let row = 1 + i as u16;
                if row < line_offset {
                    continue;
                }
                let prefix = if i == self.selected { "\u{2022} " } else { "\u{25cb} " };
                let display = format!("    {}{}", prefix, item);
                let style = if i == self.selected {
                    Style::default().fg(theme.focus_fg).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                lines.push(Line::from(Span::styled(display, style)));
            }
            lines
        }
    }

    pub fn hit_test(&self, col: u16, row: u16) -> Option<usize> {
        if self.expanded {
            if col >= self.x + 4 && col < self.x + 30 && row > self.y && row < self.y + 1 + self.items.len() as u16 {
                let idx = (row - self.y - 1) as usize;
                if idx < self.items.len() {
                    return Some(idx);
                }
            }
            if col >= self.x && col < self.x + 30 && row == self.y {
                return Some(self.selected);
            }
        } else {
            if col >= self.x && col < self.x + 30 && row == self.y {
                return Some(self.selected);
            }
        }
        None
    }
}

#[derive(Debug, Clone)]
pub struct DialogLabel {
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct DialogTextInput {
    pub label: String,
    pub value: String,
    pub cursor: usize,
    pub focused: bool,
}

#[derive(Debug, Clone)]
pub struct DialogDropdownState {
    pub focused: bool,
    pub expanded: bool,
    pub selected: usize,
}

#[derive(Debug, Clone)]
pub enum DialogItem {
    Label(DialogLabel),
    TextInput(DialogTextInput),
    Dropdown { label: String, items: Vec<String>, state: DialogDropdownState },
    FileList { entries: Vec<(String, bool)>, selection: usize, scroll: usize, focused: bool },
    Buttons(Vec<DialogButton>),
}

#[derive(Debug, Clone)]
pub struct DialogButton {
    pub label: String,
    pub focused: bool,
}

#[derive(Debug, Clone)]
pub struct DialogAreas {
    pub popup: Rect,
    pub inner: Rect,
    pub item_areas: Vec<(usize, Rect)>,
}

#[derive(Debug, Clone)]
pub enum DialogHit {
    Outside,
    TextInput(usize),
    DropdownItem(usize, Option<usize>),
    Button(usize, usize),
    FileListItem(usize, usize),
    None,
}

pub struct FileActionDialog {
    pub title: String,
    pub items: Vec<DialogItem>,
}

impl FileActionDialog {
    pub fn new(title: &str) -> Self {
        FileActionDialog {
            title: title.to_string(),
            items: Vec::new(),
        }
    }

    pub fn add_label(&mut self, text: &str) {
        self.items.push(DialogItem::Label(DialogLabel { text: text.to_string() }));
    }

    pub fn add_text_input(&mut self, label: &str, value: &str, cursor: usize, focused: bool) {
        self.items.push(DialogItem::TextInput(DialogTextInput {
            label: label.to_string(),
            value: value.to_string(),
            cursor,
            focused,
        }));
    }

    pub fn add_dropdown(&mut self, label: &str, items: Vec<String>, state: DialogDropdownState) {
        self.items.push(DialogItem::Dropdown { label: label.to_string(), items, state });
    }

    pub fn add_file_list(&mut self, entries: Vec<(String, bool)>, selection: usize, scroll: usize, focused: bool) {
        self.items.push(DialogItem::FileList { entries, selection, scroll, focused });
    }

    pub fn add_button(&mut self, label: &str, focused: bool) {
        match self.items.last_mut() {
            Some(DialogItem::Buttons(btns)) => {
                btns.push(DialogButton { label: label.to_string(), focused });
            }
            _ => {
                self.items.push(DialogItem::Buttons(vec![
                    DialogButton { label: label.to_string(), focused },
                ]));
            }
        }
    }

    fn item_height(item: &DialogItem) -> u16 {
        match item {
            DialogItem::Label(_) => 1,
            DialogItem::TextInput(_) => 2,
            DialogItem::Dropdown { items, state, .. } => {
                if state.expanded { 1 + items.len() as u16 } else { 1 }
            }
            DialogItem::FileList { entries, .. } => entries.len().min(12) as u16,
            DialogItem::Buttons(_) => 1,
        }
    }

    fn total_content_height(items: &[DialogItem]) -> u16 {
        items.iter().map(Self::item_height).sum::<u16>()
    }

    pub fn render(&self, f: &mut Frame, area: Rect, theme: &Theme) -> DialogAreas {
        let dialog_w = (area.width * 2 / 3).max(40);
        let dialog_h = (area.height * 2 / 3).max(10);

        let popup = Rect {
            x: (area.width.saturating_sub(dialog_w)) / 2,
            y: (area.height.saturating_sub(dialog_h)) / 2,
            width: dialog_w,
            height: dialog_h,
        };

        let inner = popup.inner(Margin::new(1, 1));
        f.render_widget(Clear, popup);
        f.render_widget(dialog_block(&self.title, theme), popup);

        let mut item_areas = Vec::new();
        let mut y_cursor = inner.y;

        let btn_bottom_y = inner.y + inner.height - 2;
        let mut button_idx = None;

        for (idx, item) in self.items.iter().enumerate() {
            let h = Self::item_height(item);

            if matches!(item, DialogItem::Buttons(_)) {
                button_idx = Some(idx);
                continue;
            }

            match item {
                DialogItem::Label(label) => {
                    let item_area = Rect { x: inner.x, y: y_cursor, width: inner.width, height: 1 };
                    f.render_widget(
                        Paragraph::new(Line::from(Span::styled(&label.text, Style::default().fg(Color::White)))),
                        item_area,
                    );
                    item_areas.push((idx, item_area));
                }
                DialogItem::TextInput(ti) => {
                    let label_area = Rect { x: inner.x, y: y_cursor, width: inner.width, height: 1 };
                    f.render_widget(
                        Paragraph::new(Line::from(Span::styled(
                            format!("  {}", ti.label),
                            Style::default().fg(Color::White),
                        ))),
                        label_area,
                    );

                    let path_style = if ti.focused {
                        Style::default().fg(Color::White).bg(Color::Black)
                    } else {
                        Style::default().fg(Color::White).bg(Color::Rgb(40, 40, 40))
                    };
                    let input_area = Rect { x: inner.x, y: y_cursor + 1, width: inner.width, height: 1 };
                    f.render_widget(
                        Paragraph::new(Line::from(Span::styled(format!("  {}", ti.value), path_style))),
                        input_area,
                    );

                    if ti.focused {
                        let cursor_x = input_area.x + 2 + ti.cursor as u16;
                        f.set_cursor_position(Position::new(cursor_x, input_area.y));
                    }

                    item_areas.push((idx, label_area));
                    item_areas.push((idx, input_area));
                }
                DialogItem::Dropdown { label, items, state } => {
                    if !state.expanded {
                        let display = if items.is_empty() {
                            format!("  {}: []", label)
                        } else {
                            format!("  {}: [{} \u{25bc}]", label, items[state.selected])
                        };
                        let style = if state.focused {
                            Style::default().fg(theme.focus_fg).add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(theme.accent)
                        };
                        let item_area = Rect { x: inner.x, y: y_cursor, width: inner.width, height: 1 };
                        f.render_widget(Paragraph::new(Line::from(Span::styled(display, style))), item_area);
                        item_areas.push((idx, item_area));
                    } else {
                        let header = format!("  {}: [{} \u{25b2}]", label, items[state.selected]);
                        let header_area = Rect { x: inner.x, y: y_cursor, width: inner.width, height: 1 };
                        f.render_widget(
                            Paragraph::new(Line::from(Span::styled(header, Style::default().fg(theme.focus_fg)))),
                            header_area,
                        );
                        item_areas.push((idx, header_area));

                        for (i, item_name) in items.iter().enumerate() {
                            let prefix = if i == state.selected { "\u{2022} " } else { "\u{25cb} " };
                            let display = format!("    {}{}", prefix, item_name);
                            let style = if i == state.selected {
                                Style::default().fg(theme.focus_fg).add_modifier(Modifier::BOLD)
                            } else {
                                Style::default().fg(Color::White)
                            };
                            let item_area = Rect { x: inner.x, y: y_cursor + 1 + i as u16, width: inner.width, height: 1 };
                            f.render_widget(
                                Paragraph::new(Line::from(Span::styled(display, style))),
                                item_area,
                            );
                            item_areas.push((idx, item_area));
                        }
                    }
                }
                DialogItem::FileList { entries, selection, scroll, focused: _ } => {
                    let visible = h.min(entries.len() as u16);
                    let list_area = Rect { x: inner.x, y: y_cursor, width: inner.width, height: visible };
                    let mut list_items: Vec<ListItem> = entries[*scroll..][..visible as usize]
                        .iter()
                        .enumerate()
                        .map(|(i, (name, is_dir))| {
                            let real_idx = *scroll + i;
                            let is_sel = real_idx == *selection;
                            let mut spans = Vec::new();
                            if is_sel {
                                spans.push(Span::styled(" > ", Style::default().fg(theme.list_selected_fg).bg(theme.list_selected_indicator_bg).add_modifier(Modifier::BOLD)));
                            } else {
                                spans.push(Span::styled("   ", Style::default()));
                            }
                            let name_style = if is_sel {
                                Style::default().fg(theme.list_selected_fg).bg(theme.list_selected_bg).add_modifier(Modifier::BOLD)
                            } else if *is_dir {
                                Style::default().fg(theme.dir_fg).add_modifier(Modifier::BOLD)
                            } else {
                                Style::default().fg(theme.file_fg)
                            };
                            spans.push(Span::styled(name.clone(), name_style));
                            ListItem::new(Line::from(spans))
                        })
                        .collect();

                    list_items.resize_with(visible as usize, || ListItem::new(Line::from(Span::raw(""))));
                    let list = ratatui::widgets::List::new(list_items).style(Style::default().bg(Color::Black));
                    f.render_widget(list, list_area);
                    item_areas.push((idx, list_area));
                }
                DialogItem::Buttons(_) => unreachable!(),
            }

            y_cursor += h;
        }

        if let Some(idx) = button_idx {
            if let Some(DialogItem::Buttons(btns)) = self.items.get(idx) {
                let total_btn_width: u16 = btns.iter()
                    .map(|b| b.label.len() as u16 + 4)
                    .sum::<u16>()
                    .saturating_sub(1);
                let left_pad = inner.width.saturating_sub(total_btn_width + 2).max(1);
                let mut spans = vec![Span::raw(" ".repeat(left_pad as usize))];
                for (i, btn) in btns.iter().enumerate() {
                    if i > 0 {
                        spans.push(Span::raw("  "));
                    }
                    let display = format!("[{}]", btn.label);
                    let style = if btn.focused {
                        Style::default().fg(Color::Black).bg(Color::Green).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::Green)
                    };
                    spans.push(Span::styled(display, style));
                }
                let btn_area = Rect { x: inner.x, y: btn_bottom_y, width: inner.width, height: 1 };
                f.render_widget(Paragraph::new(Line::from(spans)), btn_area);
                item_areas.push((idx, btn_area));
            }
        }

        DialogAreas { popup, inner, item_areas }
    }

    pub fn hit_test(&self, col: u16, row: u16, area: Rect) -> DialogHit {
        let dialog_w = (area.width * 2 / 3).max(40);
        let dialog_h = (area.height * 2 / 3).max(10);

        let popup = Rect {
            x: (area.width.saturating_sub(dialog_w)) / 2,
            y: (area.height.saturating_sub(dialog_h)) / 2,
            width: dialog_w,
            height: dialog_h,
        };

        if col < popup.x || col >= popup.x + popup.width || row < popup.y || row >= popup.y + popup.height {
            return DialogHit::Outside;
        }

        let inner = popup.inner(Margin::new(1, 1));
        let mut y_cursor = inner.y;

        let btn_bottom_y = inner.y + inner.height - 2;
        let mut button_item_idx = None;

        for (idx, item) in self.items.iter().enumerate() {
            if matches!(item, DialogItem::Buttons(_)) {
                button_item_idx = Some(idx);
                continue;
            }

            let h = Self::item_height(item);

            match item {
                DialogItem::Label(_) => {
                    if row == y_cursor {
                        return DialogHit::None;
                    }
                }
                DialogItem::TextInput(_) => {
                    if row >= y_cursor && row <= y_cursor + 1 {
                        return DialogHit::TextInput(idx);
                    }
                }
                DialogItem::Dropdown { items, state, .. } => {
                    if state.expanded {
                        if row >= y_cursor && row < y_cursor + 1 + items.len() as u16 {
                            if col >= inner.x + 4 && row > y_cursor {
                                let item_idx = (row - y_cursor - 1) as usize;
                                if item_idx < items.len() {
                                    return DialogHit::DropdownItem(idx, Some(item_idx));
                                }
                            }
                            return DialogHit::DropdownItem(idx, None);
                        }
                    } else {
                        if row == y_cursor {
                            return DialogHit::DropdownItem(idx, None);
                        }
                    }
                }
                DialogItem::FileList { entries, selection: _, scroll, .. } => {
                    let visible = h.min(entries.len() as u16);
                    if row >= y_cursor && row < y_cursor + visible {
                        let list_idx = (row - y_cursor + *scroll as u16) as usize;
                        if list_idx < entries.len() {
                            return DialogHit::FileListItem(idx, list_idx);
                        }
                    }
                }
                DialogItem::Buttons(_) => unreachable!(),
            }

            y_cursor += h;
        }

        if let Some(idx) = button_item_idx {
            if let Some(DialogItem::Buttons(btns)) = self.items.get(idx) {
                if row == btn_bottom_y {
                    let total_btn_width: u16 = btns.iter()
                        .map(|b| b.label.len() as u16 + 4)
                        .sum::<u16>()
                        .saturating_sub(1);
                    let left_pad = inner.width.saturating_sub(total_btn_width + 2).max(1);
                    let mut x_cursor = inner.x + left_pad;
                    for (bi, btn) in btns.iter().enumerate() {
                        if bi > 0 {
                            x_cursor += 2;
                        }
                        let btn_w = btn.label.len() as u16 + 4;
                        if col >= x_cursor && col < x_cursor + btn_w {
                            return DialogHit::Button(idx, bi);
                        }
                        x_cursor += btn_w;
                    }
                }
            }
        }

        DialogHit::None
    }
}
