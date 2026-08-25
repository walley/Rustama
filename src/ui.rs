use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Margin, Offset, Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::dimmed;
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Shadow};

#[derive(Debug, Clone)]
pub struct Theme {
    pub dialog_border: Color,
    pub dialog_bg: Color,
    pub list_selected_bg: Color,
    pub list_selected_fg: Color,
    pub dir_fg: Color,
    pub file_fg: Color,
    pub path_fg: Color,
    pub focus_fg: Color,
    pub accent: Color,
    pub thinking_fg: Color,
}

impl Theme {
    #[cfg(test)]
    pub fn default() -> Self {
        Theme {
            dialog_border: Color::Cyan,
            dialog_bg: Color::DarkGray,
            list_selected_bg: Color::White,
            list_selected_fg: Color::Black,
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
            list_selected_bg: Color::White,
            list_selected_fg: Color::Black,
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
    pub fn new(name: &str, x: u16, y: u16, active: bool, fg: Color, bg: Color) -> Self {
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
        let display = if let Some(hotkey) = self.hotkey {
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

#[derive(Debug, Clone)]
pub struct DialogLabel {
    pub text: String,
    pub fg: Option<Color>,
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
    Dropdown {
        label: String,
        items: Vec<String>,
        state: DialogDropdownState,
    },
    FileList {
        entries: Vec<(String, bool)>,
        selection: usize,
        scroll: usize,
        focused: bool,
    },
    Buttons(Vec<DialogButton>),
}

#[derive(Debug, Clone)]
pub struct DialogButton {
    pub label: String,
    pub focused: bool,
}

#[derive(Debug, Clone)]
pub enum DialogHit {
    Outside,
    TextInput,
    DropdownItem(usize, Option<usize>),
    Button(usize),
    FileListItem(usize),
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

    pub fn add_label_colored(&mut self, text: &str, fg: Color) {
        self.items.push(DialogItem::Label(DialogLabel {
            text: text.to_string(),
            fg: Some(fg),
        }));
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
        self.items.push(DialogItem::Dropdown {
            label: label.to_string(),
            items,
            state,
        });
    }

    pub fn add_file_list(
        &mut self,
        entries: Vec<(String, bool)>,
        selection: usize,
        scroll: usize,
        focused: bool,
    ) {
        self.items.push(DialogItem::FileList {
            entries,
            selection,
            scroll,
            focused,
        });
    }

    pub fn add_button(&mut self, label: &str, focused: bool) {
        match self.items.last_mut() {
            Some(DialogItem::Buttons(btns)) => {
                btns.push(DialogButton {
                    label: label.to_string(),
                    focused,
                });
            }
            _ => {
                self.items.push(DialogItem::Buttons(vec![DialogButton {
                    label: label.to_string(),
                    focused,
                }]));
            }
        }
    }

    fn item_height(item: &DialogItem) -> u16 {
        match item {
            DialogItem::Label(_) => 1,
            DialogItem::TextInput(_) => 2,
            DialogItem::Dropdown { items, state, .. } => {
                if state.expanded {
                    1 + items.len() as u16
                } else {
                    1
                }
            }
            DialogItem::FileList { entries, .. } => entries.len().min(12) as u16 + 2, // +2 for borders
            DialogItem::Buttons(_) => 1,
        }
    }

    /// Popup dimensions shared by `render` and `hit_test` so they always agree.
    pub fn dialog_size(area: Rect) -> (u16, u16) {
        ((area.width * 2 / 3).max(40), (area.height * 2 / 3).max(10))
    }

    /// Geometry of a bordered file list placed at `y_cursor` with the button
    /// row at `btn_bottom_y`. Returns `(box_height, visible_rows, first_entry)`.
    /// The box is clamped to the space above the button row, and the scroll
    /// offset is adjusted so the selection is always visible.
    fn file_list_geometry(
        entries_len: usize,
        selection: usize,
        scroll: usize,
        y_cursor: u16,
        btn_bottom_y: u16,
    ) -> (u16, usize, usize) {
        let max_box = btn_bottom_y.saturating_sub(y_cursor).max(2);
        let box_h = (entries_len.min(12) as u16 + 2).min(max_box).max(2);
        let visible = (box_h - 2) as usize;
        if visible == 0 || entries_len == 0 {
            return (box_h, visible, 0);
        }
        let max_start = entries_len.saturating_sub(visible);
        let mut start = scroll.min(max_start);
        if selection < start {
            start = selection;
        }
        if selection >= start + visible {
            start = selection + 1 - visible;
        }
        (box_h, visible, start)
    }

    pub fn render(&self, f: &mut Frame, area: Rect, theme: &Theme) {
        let (dialog_w, dialog_h) = Self::dialog_size(area);

        let popup = Rect {
            x: (area.width.saturating_sub(dialog_w)) / 2,
            y: (area.height.saturating_sub(dialog_h)) / 2,
            width: dialog_w,
            height: dialog_h,
        };

        let inner = popup.inner(Margin::new(1, 1));
        f.render_widget(Clear, popup);
        f.render_widget(dialog_block(&self.title, theme), popup);

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
                    let item_area = Rect {
                        x: inner.x,
                        y: y_cursor,
                        width: inner.width,
                        height: 1,
                    };
                    f.render_widget(
                        Paragraph::new(Line::from(Span::styled(
                            &label.text,
                            Style::default().fg(label.fg.unwrap_or(Color::White)),
                        ))),
                        item_area,
                    );
                }
                DialogItem::TextInput(ti) => {
                    let label_area = Rect {
                        x: inner.x,
                        y: y_cursor,
                        width: inner.width,
                        height: 1,
                    };
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
                    let input_area = Rect {
                        x: inner.x,
                        y: y_cursor + 1,
                        width: inner.width,
                        height: 1,
                    };
                    f.render_widget(
                        Paragraph::new(Line::from(Span::styled(
                            format!("  {}", ti.value),
                            path_style,
                        ))),
                        input_area,
                    );

                    if ti.focused {
                        let cursor_x = input_area.x + 2 + ti.cursor as u16;
                        f.set_cursor_position(Position::new(cursor_x, input_area.y));
                    }

                }
                DialogItem::Dropdown {
                    label,
                    items,
                    state,
                } => {
                    if !state.expanded {
                        let display = if items.is_empty() {
                            format!("  {}: []", label)
                        } else {
                            format!("  {}: [{} \u{25bc}]", label, items[state.selected])
                        };
                        let style = if state.focused {
                            Style::default()
                                .fg(theme.focus_fg)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(theme.accent)
                        };
                        let item_area = Rect {
                            x: inner.x,
                            y: y_cursor,
                            width: inner.width,
                            height: 1,
                        };
                        f.render_widget(
                            Paragraph::new(Line::from(Span::styled(display, style))),
                            item_area,
                        );
                    } else {
                        let header = format!("  {}: [{} \u{25b2}]", label, items[state.selected]);
                        let header_area = Rect {
                            x: inner.x,
                            y: y_cursor,
                            width: inner.width,
                            height: 1,
                        };
                        f.render_widget(
                            Paragraph::new(Line::from(Span::styled(
                                header,
                                Style::default().fg(theme.focus_fg),
                            ))),
                            header_area,
                        );

                        for (i, item_name) in items.iter().enumerate() {
                            let prefix = if i == state.selected {
                                "\u{2022} "
                            } else {
                                "\u{25cb} "
                            };
                            let display = format!("    {}{}", prefix, item_name);
                            let style = if i == state.selected {
                                Style::default()
                                    .fg(theme.focus_fg)
                                    .add_modifier(Modifier::BOLD)
                            } else {
                                Style::default().fg(Color::White)
                            };
                            let item_area = Rect {
                                x: inner.x,
                                y: y_cursor + 1 + i as u16,
                                width: inner.width,
                                height: 1,
                            };
                            f.render_widget(
                                Paragraph::new(Line::from(Span::styled(display, style))),
                                item_area,
                            );
                        }
                    }
                }
                DialogItem::FileList {
                    entries,
                    selection,
                    scroll,
                    focused,
                } => {
                    let (box_h, visible, start) = Self::file_list_geometry(
                        entries.len(),
                        *selection,
                        *scroll,
                        y_cursor,
                        btn_bottom_y,
                    );
                    let box_area = Rect {
                        x: inner.x,
                        y: y_cursor,
                        width: inner.width,
                        height: box_h,
                    };

                    let border_color = if *focused {
                        theme.focus_fg
                    } else {
                        theme.dialog_border
                    };
                    let block = Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(border_color))
                        .style(Style::default().bg(theme.dialog_bg));
                    f.render_widget(block, box_area);

                    let list_inner = box_area.inner(Margin::new(1, 1));
                    let end = (start + visible).min(entries.len());
                    let mut list_items: Vec<ListItem> = entries[start..end]
                        .iter()
                        .enumerate()
                        .map(|(i, (name, is_dir))| {
                            let real_idx = start + i;
                            let is_sel = real_idx == *selection;
                            let display = if *is_dir && name != ".." {
                                format!("{}/", name)
                            } else {
                                name.clone()
                            };
                            let name_style = if is_sel {
                                Style::default()
                                    .fg(theme.list_selected_fg)
                                    .bg(theme.list_selected_bg)
                                    .add_modifier(Modifier::BOLD)
                            } else if *is_dir {
                                Style::default()
                                    .fg(theme.dir_fg)
                                    .add_modifier(Modifier::BOLD)
                            } else {
                                Style::default().fg(theme.file_fg)
                            };
                            // Pad the selected row so the highlight spans the full width.
                            let text = if is_sel {
                                let pad = (list_inner.width as usize)
                                    .saturating_sub(display.chars().count());
                                format!("{}{}", display, " ".repeat(pad))
                            } else {
                                display
                            };
                            ListItem::new(Line::from(Span::styled(text, name_style)))
                        })
                        .collect();

                    list_items.resize_with(visible, || ListItem::new(Line::from(Span::raw(""))));
                    let list = ratatui::widgets::List::new(list_items);
                    f.render_widget(list, list_inner);
                }
                DialogItem::Buttons(_) => unreachable!(),
            }

            y_cursor += h;
        }

        if let Some(idx) = button_idx
            && let Some(DialogItem::Buttons(btns)) = self.items.get(idx)
        {
            let total_btn_width: u16 = btns
                .iter()
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
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Green)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Green)
                };
                spans.push(Span::styled(display, style));
            }
            let btn_area = Rect {
                x: inner.x,
                y: btn_bottom_y,
                width: inner.width,
                height: 1,
            };
            f.render_widget(Paragraph::new(Line::from(spans)), btn_area);
        }

    }

    pub fn hit_test(&self, col: u16, row: u16, area: Rect) -> DialogHit {
        let (dialog_w, dialog_h) = Self::dialog_size(area);

        let popup = Rect {
            x: (area.width.saturating_sub(dialog_w)) / 2,
            y: (area.height.saturating_sub(dialog_h)) / 2,
            width: dialog_w,
            height: dialog_h,
        };

        if col < popup.x
            || col >= popup.x + popup.width
            || row < popup.y
            || row >= popup.y + popup.height
        {
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
                        return DialogHit::TextInput;
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
                DialogItem::FileList {
                    entries,
                    selection,
                    scroll,
                    ..
                } => {
                    let (_box_h, visible, start) = Self::file_list_geometry(
                        entries.len(),
                        *selection,
                        *scroll,
                        y_cursor,
                        btn_bottom_y,
                    );
                    if row > y_cursor && row <= y_cursor + visible as u16 {
                        let list_idx = start + (row - y_cursor - 1) as usize;
                        if list_idx < entries.len() {
                            return DialogHit::FileListItem(list_idx);
                        }
                    }
                }
                DialogItem::Buttons(_) => unreachable!(),
            }

            y_cursor += h;
        }

        if let Some(idx) = button_item_idx
            && let Some(DialogItem::Buttons(btns)) = self.items.get(idx)
            && row == btn_bottom_y
        {
            let total_btn_width: u16 = btns
                .iter()
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
                    return DialogHit::Button(bi);
                }
                x_cursor += btn_w;
            }
        }

        DialogHit::None
    }
}

// ─── Main Menu ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ActiveMenu {
    None,
    File,
    Edit,
    View,
    Help,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MenuAction {
    None,
    LoadSession,
    SaveSession,
    ExportChat,
    Quit,
    OpenModelDialog,
    ToggleAgenticMode(bool),
    ToggleTerminal,
    OpenSettingsDialog,
    ShowAbout,
}

pub struct MainMenu {
    pub active: ActiveMenu,
    pub selection: usize,
}

impl MainMenu {
    pub fn new() -> Self {
        MainMenu {
            active: ActiveMenu::None,
            selection: 0,
        }
    }

    pub fn is_open(&self) -> bool {
        self.active != ActiveMenu::None
    }

    pub fn open(&mut self, menu: ActiveMenu) {
        self.active = menu;
        self.selection = 0;
    }

    pub fn close(&mut self) {
        self.active = ActiveMenu::None;
    }

    pub fn item_names(&self) -> Vec<&'static str> {
        match self.active {
            ActiveMenu::File => vec!["Load", "Save", "Export...", "\u{2500}", "Exit"],
            ActiveMenu::Edit => vec!["Set Model", "Agentic Mode", "\u{2500}", "Settings..."],
            ActiveMenu::View => vec!["Terminal"],
            ActiveMenu::Help => vec!["About"],
            ActiveMenu::None => vec![],
        }
    }

    pub fn max_items(&self) -> usize {
        self.item_names().len()
    }

    pub fn is_separator(&self, index: usize) -> bool {
        self.item_names()
            .get(index)
            .is_some_and(|name| *name == "\u{2500}")
    }

    fn x_offset_for(menu: ActiveMenu) -> u16 {
        match menu {
            ActiveMenu::File => 0,
            ActiveMenu::Edit => 7,
            ActiveMenu::View => 14,
            ActiveMenu::Help => 21,
            ActiveMenu::None => 0,
        }
    }

    pub fn bar_col_to_menu(col: u16) -> ActiveMenu {
        match col {
            0..=6 => ActiveMenu::File,
            7..=13 => ActiveMenu::Edit,
            14..=20 => ActiveMenu::View,
            _ => ActiveMenu::Help,
        }
    }

    fn next_menu(menu: ActiveMenu) -> ActiveMenu {
        match menu {
            ActiveMenu::File => ActiveMenu::Edit,
            ActiveMenu::Edit => ActiveMenu::View,
            ActiveMenu::View => ActiveMenu::Help,
            ActiveMenu::Help => ActiveMenu::File,
            ActiveMenu::None => ActiveMenu::File,
        }
    }

    fn prev_menu(menu: ActiveMenu) -> ActiveMenu {
        match menu {
            ActiveMenu::File => ActiveMenu::Help,
            ActiveMenu::Edit => ActiveMenu::File,
            ActiveMenu::View => ActiveMenu::Edit,
            ActiveMenu::Help => ActiveMenu::View,
            ActiveMenu::None => ActiveMenu::File,
        }
    }

    fn action_for(&self, menu: ActiveMenu, index: usize) -> MenuAction {
        match (menu, index) {
            (ActiveMenu::File, 0) => MenuAction::LoadSession,
            (ActiveMenu::File, 1) => MenuAction::SaveSession,
            (ActiveMenu::File, 2) => MenuAction::ExportChat,
            (ActiveMenu::File, 4) => MenuAction::Quit,
            (ActiveMenu::Edit, 0) => MenuAction::OpenModelDialog,
            (ActiveMenu::Edit, 1) => MenuAction::ToggleAgenticMode(true),
            (ActiveMenu::Edit, 3) => MenuAction::OpenSettingsDialog,
            (ActiveMenu::View, 0) => MenuAction::ToggleTerminal,
            (ActiveMenu::Help, 0) => MenuAction::ShowAbout,
            _ => MenuAction::None,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> MenuAction {
        match key.code {
            KeyCode::Esc => {
                self.close();
                MenuAction::None
            }
            KeyCode::Left => {
                self.active = Self::prev_menu(self.active);
                self.selection = self.first_selectable();
                MenuAction::None
            }
            KeyCode::Right => {
                self.active = Self::next_menu(self.active);
                self.selection = self.first_selectable();
                MenuAction::None
            }
            KeyCode::Up => {
                self.selection = self.prev_selectable(self.selection);
                MenuAction::None
            }
            KeyCode::Down => {
                self.selection = self.next_selectable(self.selection);
                MenuAction::None
            }
            KeyCode::Enter => {
                let action = self.action_for(self.active, self.selection);
                self.close();
                action
            }
            _ => MenuAction::None,
        }
    }

    fn first_selectable(&self) -> usize {
        let count = self.max_items();
        for i in 0..count {
            if !self.is_separator(i) {
                return i;
            }
        }
        0
    }

    fn prev_selectable(&self, current: usize) -> usize {
        let max = self.max_items();
        let mut new = current;
        loop {
            if new == 0 {
                new = max;
            }
            new -= 1;
            if !self.is_separator(new) {
                return new;
            }
        }
    }

    fn next_selectable(&self, current: usize) -> usize {
        let max = self.max_items();
        let mut new = current;
        loop {
            new += 1;
            if new >= max {
                new = 0;
            }
            if !self.is_separator(new) {
                return new;
            }
        }
    }

    pub fn handle_click(&mut self, col: u16, row: u16) -> MenuAction {
        if row == 0 {
            let menu = Self::bar_col_to_menu(col);
            if self.active == menu {
                self.close();
                return MenuAction::None;
            } else {
                self.open(menu);
                return MenuAction::None;
            }
        }

        if self.is_open() {
            let x_offset = Self::x_offset_for(self.active);
            let item_count = self.max_items() as u16;
            if row >= 2 && row < 2 + item_count && col >= x_offset && col < x_offset + 20 {
                let idx = (row - 2) as usize;
                if !self.is_separator(idx) {
                    self.selection = idx;
                    let action = self.action_for(self.active, self.selection);
                    self.close();
                    return action;
                }
                return MenuAction::None;
            }
            self.close();
            return MenuAction::None;
        }

        MenuAction::None
    }

    /// Renders the top menu bar: the menu names on the left and the clock
    /// (`HH:MM`) on the right. Status text and mode indicators belong to the
    /// status bar / keybar, not here.
    pub fn render_bar(&self, f: &mut Frame, area: Rect) {
        let normal_style = Style::default().fg(Color::White).bg(Color::DarkGray);
        let selected_style = Style::default().fg(Color::Black).bg(Color::White);

        let file_style = if self.active == ActiveMenu::File {
            selected_style
        } else {
            normal_style
        };
        let edit_style = if self.active == ActiveMenu::Edit {
            selected_style
        } else {
            normal_style
        };
        let view_style = if self.active == ActiveMenu::View {
            selected_style
        } else {
            normal_style
        };
        let help_style = if self.active == ActiveMenu::Help {
            selected_style
        } else {
            normal_style
        };

        let now = chrono::Local::now();
        let clock = format!("  {}  ", now.format("%H:%M"));
        let clock_len = clock.len() as u16;
        let used = 6 + 1 + 6 + 1 + 6 + 1 + 5 + 1;
        let pad = area.width.saturating_sub(used + clock_len);
        let menu_bar_spans = vec![
            Span::styled(" File ", file_style),
            Span::styled(" ", normal_style),
            Span::styled(" Edit ", edit_style),
            Span::styled(" ", normal_style),
            Span::styled(" View ", view_style),
            Span::styled(" ", normal_style),
            Span::styled(" Help ", help_style),
            Span::styled(" ".repeat(pad as usize), normal_style),
            Span::styled(clock, normal_style),
        ];
        let menu_bar = Line::from(menu_bar_spans);

        f.render_widget(Paragraph::new(menu_bar).style(normal_style), area);
    }

    pub fn render_submenu(&self, f: &mut Frame, menu_bar_area: Rect) {
        let items = self.item_names();
        if items.is_empty() {
            return;
        }

        let menu_width = items.iter().map(|s| s.len()).max().unwrap_or(10) as u16 + 4;
        let x_offset = Self::x_offset_for(self.active);

        let popup_area = Rect {
            x: menu_bar_area.x + x_offset,
            y: menu_bar_area.y + 1,
            width: menu_width,
            height: (items.len() as u16) + 2,
        };

        let inner_width = menu_width.saturating_sub(2) as usize;
        let list_items: Vec<ListItem> = items
            .iter()
            .enumerate()
            .map(|(i, name)| {
                if *name == "\u{2500}" {
                    let line = "\u{2500}".repeat(inner_width);
                    ListItem::new(Line::from(Span::styled(
                        line,
                        Style::default().fg(Color::DarkGray),
                    )))
                } else {
                    let style = if i == self.selection {
                        Style::default().fg(Color::Black).bg(Color::White)
                    } else {
                        Style::default()
                    };
                    ListItem::new(Line::from(Span::styled(format!(" {} ", name), style)))
                }
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
}

pub struct MessageBox {
    pub title: String,
    pub message: String,
}

impl MessageBox {
    pub fn new(title: &str, message: &str) -> Self {
        MessageBox {
            title: title.to_string(),
            message: message.to_string(),
        }
    }

    /// Word-wrap `message` to `width` columns. Explicit newlines are kept,
    /// blank lines are preserved, and over-long words are hard-split.
    fn wrap_message(message: &str, width: usize) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for raw_line in message.lines() {
            if raw_line.is_empty() {
                out.push(String::new());
                continue;
            }
            let mut current = String::new();
            for word in raw_line.split_whitespace() {
                let candidate = if current.is_empty() {
                    word.to_string()
                } else {
                    format!("{} {}", current, word)
                };
                if candidate.chars().count() <= width {
                    current = candidate;
                } else {
                    if !current.is_empty() {
                        out.push(std::mem::take(&mut current));
                    }
                    // Hard-split words longer than the wrap width.
                    let mut rest = word;
                    while rest.chars().count() > width {
                        let split_at = rest
                            .char_indices()
                            .nth(width)
                            .map(|(i, _)| i)
                            .unwrap_or(rest.len());
                        out.push(rest[..split_at].to_string());
                        rest = &rest[split_at..];
                    }
                    current = rest.to_string();
                }
            }
            if !current.is_empty() {
                out.push(current);
            }
        }
        out
    }

    /// Single source of truth for the popup layout so `render` and
    /// `hit_test` always agree. Returns `(popup_area, wrapped_lines, button_rect)`.
    fn layout(&self, area: Rect) -> (Rect, Vec<String>, Rect) {
        // Margin::new(2, 1) -> 4 columns of horizontal chrome, 2 rows vertical.
        let max_inner_w = (area.width.saturating_sub(8)) as usize;
        let content_w = self
            .message
            .lines()
            .map(|l| l.chars().count() + 2) // 2-column text indent
            .max()
            .unwrap_or(0);
        let inner_w = content_w.min(46).min(max_inner_w).max(10);
        let lines = Self::wrap_message(&self.message, inner_w.saturating_sub(2).max(1));
        let msg_height = lines.len() as u16;
        let dialog_w = (inner_w as u16 + 4)
            .min(area.width.saturating_sub(4))
            .max(16);
        let dialog_h = 4 + msg_height;
        let popup_area = Rect {
            x: area.x + (area.width.saturating_sub(dialog_w)) / 2,
            y: area.y + (area.height.saturating_sub(dialog_h)) / 2,
            width: dialog_w,
            height: dialog_h.min(area.height),
        };
        let inner = popup_area.inner(Margin::new(2, 1));
        let btn_y = inner.y + msg_height + 1;
        let btn_w = "OK".len() as u16 + 4;
        let btn_area = Rect {
            x: inner.x + (inner.width.saturating_sub(btn_w)) / 2,
            y: btn_y,
            width: btn_w,
            height: 1,
        };
        (popup_area, lines, btn_area)
    }

    pub fn render(&self, f: &mut Frame, area: Rect, focused: bool, theme: &Theme) {
        let (popup_area, lines, btn_area) = self.layout(area);

        f.render_widget(Clear, popup_area);
        f.render_widget(dialog_block(&self.title, theme), popup_area);

        let inner = popup_area.inner(Margin::new(2, 1));
        for (i, line) in lines.iter().enumerate() {
            let para = Paragraph::new(Line::from(Span::styled(
                format!("  {}", line),
                Style::default().fg(Color::White),
            )));
            let line_area = Rect {
                x: inner.x,
                y: inner.y + i as u16,
                width: inner.width,
                height: 1,
            };
            f.render_widget(para, line_area);
        }

        let ok_btn = Button::new(
            "OK",
            btn_area.x,
            btn_area.y,
            focused,
            Color::Green,
            Color::Green,
        );
        let (display, style) = ok_btn.render();
        let btn_para = Paragraph::new(Line::from(Span::styled(display, style)));
        f.render_widget(btn_para, btn_area);
    }

    pub fn hit_test(&self, col: u16, row: u16, area: Rect) -> bool {
        let (_, _, btn_area) = self.layout(area);
        let ok_btn = Button::new(
            "OK",
            btn_area.x,
            btn_area.y,
            true,
            Color::Green,
            Color::Green,
        );
        ok_btn.is_clicked(col, row)
    }
}

/// Which button of a [`ConfirmationBox`] currently has focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmFocus {
    Yes,
    No,
}

/// Result of a [`ConfirmationBox::hit_test`] mouse query.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmHit {
    /// Click outside the popup.
    Outside,
    /// Click on the confirm (left) button.
    Yes,
    /// Click on the cancel (right) button.
    No,
    /// Click inside the popup, but on no button.
    None,
}

/// A modal confirmation box ("confirmationbox" class): title, a wrapped
/// message, and two buttons — confirm (default `Yes`) and cancel
/// (default `No`). Same widget family as [`MessageBox`], sharing its
/// layout conventions: both `render` and `hit_test` go through a single
/// internal `layout()` so they always agree.
pub struct ConfirmationBox {
    pub title: String,
    pub message: String,
    pub confirm_label: String,
    pub cancel_label: String,
}

impl ConfirmationBox {
    /// Creates a confirmation box with the default `Yes` / `No` buttons.
    pub fn new(title: &str, message: &str) -> Self {
        ConfirmationBox {
            title: title.to_string(),
            message: message.to_string(),
            confirm_label: "Yes".to_string(),
            cancel_label: "No".to_string(),
        }
    }

    /// Overrides the button labels (confirm first, cancel second).
    /// Part of the "confirmationbox" class API for reuse beyond the quit dialog.
    #[allow(dead_code)]
    pub fn with_labels(mut self, confirm: &str, cancel: &str) -> Self {
        self.confirm_label = confirm.to_string();
        self.cancel_label = cancel.to_string();
        self
    }

    /// Single source of truth for the popup layout so `render` and
    /// `hit_test` always agree. Returns
    /// `(popup_area, wrapped_lines, confirm_button_rect, cancel_button_rect)`.
    fn layout(&self, area: Rect) -> (Rect, Vec<String>, Rect, Rect) {
        // Margin::new(2, 1) -> 4 columns of horizontal chrome, 2 rows vertical.
        let max_inner_w = (area.width.saturating_sub(8)) as usize;
        let content_w = self
            .message
            .lines()
            .map(|l| l.chars().count() + 2) // 2-column text indent
            .max()
            .unwrap_or(0);
        let inner_w = content_w.min(46).min(max_inner_w).max(10);
        let lines = MessageBox::wrap_message(&self.message, inner_w.saturating_sub(2).max(1));
        let msg_height = lines.len() as u16;

        // Make sure both buttons (centered as a pair) always fit.
        let confirm_w = self.confirm_label.len() as u16 + 4;
        let cancel_w = self.cancel_label.len() as u16 + 4;
        let btn_gap: u16 = 2;
        let buttons_w = confirm_w + btn_gap + cancel_w;
        let min_inner_w = buttons_w + 4; // a little breathing room

        let dialog_w = ((inner_w as u16).max(min_inner_w) + 4)
            .min(area.width.saturating_sub(4))
            .max(16);
        let dialog_h = 4 + msg_height;
        let popup_area = Rect {
            x: area.x + (area.width.saturating_sub(dialog_w)) / 2,
            y: area.y + (area.height.saturating_sub(dialog_h)) / 2,
            width: dialog_w,
            height: dialog_h.min(area.height),
        };
        let inner = popup_area.inner(Margin::new(2, 1));
        let btn_y = inner.y + msg_height + 1;
        let btn_start_x = inner.x + (inner.width.saturating_sub(buttons_w)) / 2;
        let confirm_area = Rect {
            x: btn_start_x,
            y: btn_y,
            width: confirm_w,
            height: 1,
        };
        let cancel_area = Rect {
            x: btn_start_x + confirm_w + btn_gap,
            y: btn_y,
            width: cancel_w,
            height: 1,
        };
        (popup_area, lines, confirm_area, cancel_area)
    }

    pub fn render(&self, f: &mut Frame, area: Rect, focus: ConfirmFocus, theme: &Theme) {
        let (popup_area, lines, confirm_area, cancel_area) = self.layout(area);

        f.render_widget(Clear, popup_area);
        f.render_widget(dialog_block(&self.title, theme), popup_area);

        let inner = popup_area.inner(Margin::new(2, 1));
        for (i, line) in lines.iter().enumerate() {
            let para = Paragraph::new(Line::from(Span::styled(
                format!("  {}", line),
                Style::default().fg(Color::White),
            )));
            let line_area = Rect {
                x: inner.x,
                y: inner.y + i as u16,
                width: inner.width,
                height: 1,
            };
            f.render_widget(para, line_area);
        }

        let yes_btn = Button::new(
            &self.confirm_label,
            confirm_area.x,
            confirm_area.y,
            focus == ConfirmFocus::Yes,
            Color::Green,
            Color::Green,
        );
        let no_btn = Button::new(
            &self.cancel_label,
            cancel_area.x,
            cancel_area.y,
            focus == ConfirmFocus::No,
            Color::Red,
            Color::Red,
        );

        let (yes_display, yes_style) = yes_btn.render();
        let (no_display, no_style) = no_btn.render();

        let yes_para = Paragraph::new(Line::from(Span::styled(yes_display, yes_style)));
        f.render_widget(yes_para, confirm_area);
        let no_para = Paragraph::new(Line::from(Span::styled(no_display, no_style)));
        f.render_widget(no_para, cancel_area);
    }

    pub fn hit_test(&self, col: u16, row: u16, area: Rect) -> ConfirmHit {
        let (popup_area, _, confirm_area, cancel_area) = self.layout(area);
        if col < popup_area.x
            || col >= popup_area.x + popup_area.width
            || row < popup_area.y
            || row >= popup_area.y + popup_area.height
        {
            return ConfirmHit::Outside;
        }
        let yes_btn = Button::new(
            &self.confirm_label,
            confirm_area.x,
            confirm_area.y,
            true,
            Color::Green,
            Color::Green,
        );
        let no_btn = Button::new(
            &self.cancel_label,
            cancel_area.x,
            cancel_area.y,
            true,
            Color::Red,
            Color::Red,
        );
        if yes_btn.is_clicked(col, row) {
            return ConfirmHit::Yes;
        }
        if no_btn.is_clicked(col, row) {
            return ConfirmHit::No;
        }
        ConfirmHit::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_short_lines_unchanged() {
        let lines = MessageBox::wrap_message("hello\nworld", 46);
        assert_eq!(lines, vec!["hello", "world"]);
    }

    #[test]
    fn wrap_preserves_blank_lines() {
        let lines = MessageBox::wrap_message("a\n\nb", 46);
        assert_eq!(lines, vec!["a", "", "b"]);
    }

    #[test]
    fn wrap_long_line_splits_on_words() {
        let msg = "Supports markdown rendering, agentic tools, and saving.";
        let lines = MessageBox::wrap_message(msg, 20);
        assert!(lines.len() > 1, "expected wrapping, got {:?}", lines);
        for l in &lines {
            assert!(l.chars().count() <= 20, "line too long: {:?}", l);
        }
        // No words lost: rejoined text contains all original words.
        let joined = lines.join(" ");
        for word in msg.split_whitespace() {
            assert!(joined.contains(word), "missing word {}", word);
        }
    }

    #[test]
    fn wrap_hard_splits_overlong_word() {
        let lines = MessageBox::wrap_message("abcdefghijklmnopqrstuvwxyz", 10);
        assert_eq!(lines, vec!["abcdefghij", "klmnopqrst", "uvwxyz"]);
    }

    #[test]
    fn layout_button_below_message_and_hit_test_agrees() {
        let mb = MessageBox::new("About", "line one\n\nline two is a bit longer");
        let area = Rect::new(0, 0, 120, 40);
        let (popup, lines, btn) = mb.layout(area);
        // Button row is below the last message line and inside the popup.
        let last_text_row = popup.y + 1 + lines.len() as u16; // inner.y + msg lines
        assert!(btn.y > last_text_row);
        assert!(btn.y < popup.y + popup.height);
        // Button is horizontally centered-ish inside the popup.
        assert!(btn.x >= popup.x);
        assert!(btn.x + btn.width <= popup.x + popup.width);
        // hit_test hits the button center, misses a corner of the popup.
        assert!(mb.hit_test(btn.x + btn.width / 2, btn.y, area));
        assert!(!mb.hit_test(popup.x, popup.y, area));
    }

    #[test]
    fn layout_fits_small_terminal() {
        let mb = MessageBox::new(
            "About",
            "A terminal AI coding agent for Ollama LLMs.\nSupports markdown rendering, agentic tools, and saving.",
        );
        let area = Rect::new(0, 0, 40, 12);
        let (popup, _lines, btn) = mb.layout(area);
        assert!(popup.width <= area.width);
        assert!(popup.height <= area.height);
        assert!(btn.x + btn.width <= popup.x + popup.width);
    }

    #[test]
    fn file_list_geometry_fits_above_buttons() {
        // 20 entries, tall terminal: capped at 12 visible rows + 2 border rows.
        let (box_h, visible, start) = FileActionDialog::file_list_geometry(20, 0, 0, 5, 40);
        assert_eq!(visible, 12);
        assert_eq!(box_h, 14);
        assert_eq!(start, 0);
    }

    #[test]
    fn file_list_geometry_clamps_to_available_space() {
        // List starts at row 10, buttons at row 15 -> box can be at most 5 rows.
        let (box_h, visible, _start) = FileActionDialog::file_list_geometry(20, 0, 0, 10, 15);
        assert_eq!(box_h, 5);
        assert_eq!(visible, 3);
    }

    #[test]
    fn file_list_geometry_keeps_selection_visible() {
        // Scroll hint is stale; selection beyond the window must scroll into view.
        let (_box_h, visible, start) = FileActionDialog::file_list_geometry(20, 10, 0, 0, 30);
        assert!(start <= 10 && 10 < start + visible);
        // Selection above the window pulls scroll up.
        let (_b, visible2, start2) = FileActionDialog::file_list_geometry(20, 2, 15, 0, 30);
        assert!(start2 <= 2 && 2 < start2 + visible2);
    }

    #[test]
    fn file_list_geometry_empty_entries() {
        let (box_h, visible, start) = FileActionDialog::file_list_geometry(0, 0, 0, 0, 30);
        assert_eq!(box_h, 2); // just the borders
        assert_eq!(visible, 0);
        assert_eq!(start, 0);
    }

    #[test]
    fn file_dialog_render_and_hit_test_agree() {
        let entries: Vec<(String, bool)> =
            (0..20).map(|i| (format!("file{}.txt", i), false)).collect();
        let mut d = FileActionDialog::new("Load File");
        d.add_label_colored("/tmp", Color::DarkGray);
        d.add_file_list(entries.clone(), 5, 0, true);
        d.add_button("Cancel", false);
        d.add_button("Open", true);

        let area = Rect::new(0, 0, 80, 24);
        // Walk every row of the popup; FileListItem hits must be inside the
        // bordered box, and the index must account for clamped scroll.
        let mut saw_list_item = false;
        for row in 0..area.height {
            for col in 0..area.width {
                if let DialogHit::FileListItem(idx) = d.hit_test(col, row, area) {
                    saw_list_item = true;
                    assert!(idx < entries.len());
                }
            }
        }
        assert!(saw_list_item, "expected some file list hits");
    }

    #[test]
    fn file_dialog_visual_smoke_test() {
        use ratatui::Terminal;
        use ratatui::backend::TestBackend;

        // Mirrors App::build_file_action_dialog.
        let entries: Vec<(String, bool)> = vec![
            ("..".to_string(), true),
            ("docs".to_string(), true),
            ("src".to_string(), true),
        ]
        .into_iter()
        .chain((0..15).map(|i| (format!("notes{}.session.rustama", i), false)))
        .collect();

        let mut d = FileActionDialog::new("Load Session");
        d.add_label_colored("/home/user/sessions", Color::DarkGray);
        d.add_file_list(entries, 3, 0, true);
        d.add_button("Cancel", false);
        d.add_button("Load", true);

        let theme = Theme::default();
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                let area = f.area();
                d.render(f, area, &theme);
            })
            .unwrap();

        let buf = terminal.backend().buffer().clone();
        let lines: Vec<String> = (0..buf.area.height)
            .map(|y| (0..buf.area.width).map(|x| buf[(x, y)].symbol()).collect())
            .collect();
        let view = lines.join("\n");
        println!("\n{}", view);

        // 80x24: popup 53x16 at (13,4); inner x=14,y=5,w=51,h=14;
        // label row 5, list box rows 6..17, buttons row 17.
        let (dx, dy, dw, dh) = (13u16, 4u16, 53u16, 16u16);
        assert_eq!(buf[(dx, dy)].symbol(), "┌", "popup top-left corner");
        assert_eq!(
            buf[(dx + dw - 1, dy + dh - 1)].symbol(),
            "┘",
            "popup bottom-right corner"
        );

        // Bordered box around the file list: top border at row 6, bottom at row 16.
        let (lx, lt, lb) = (dx + 1, dy + 2, dy + dh - 4);
        assert_eq!(buf[(lx, lt)].symbol(), "┌", "list box top-left corner");
        assert_eq!(buf[(lx, lb)].symbol(), "└", "list box bottom-left corner");
        assert_eq!(
            buf[(lx + dw - 3, lt)].symbol(),
            "┐",
            "list box top-right corner"
        );
        for row in (lt + 1)..lb {
            assert_eq!(
                buf[(lx, row)].symbol(),
                "│",
                "list box left border row {}",
                row
            );
            assert_eq!(
                buf[(lx + dw - 3, row)].symbol(),
                "│",
                "list box right border row {}",
                row
            );
        }

        // No selection arrow anywhere inside the list box.
        for row in (lt + 1)..lb {
            for col in (lx + 1)..(lx + dw - 3) {
                assert_ne!(buf[(col, row)].symbol(), ">", "arrow at ({}, {})", col, row);
            }
        }

        // Directories shown with a trailing slash.
        assert!(
            view.contains("docs/"),
            "dir should have trailing slash:\n{}",
            view
        );
        assert!(
            view.contains("src/"),
            "dir should have trailing slash:\n{}",
            view
        );

        // Selection is row index 3 -> first visible file row; highlight spans
        // the full inner width of the box.
        let sel_row = lt + 1 + 3;
        for col in (lx + 1)..(lx + dw - 3) {
            assert_eq!(
                buf[(col, sel_row)].bg,
                theme.list_selected_bg,
                "selected row not fully highlighted at col {}",
                col
            );
        }

        // Buttons on the last-but-two inner row, right-aligned: [Cancel]  [Load].
        let btn_row = &lines[(dy + dh - 3) as usize];
        let cancel_at = btn_row.find("[Cancel]").expect("Cancel button missing");
        let load_at = btn_row.find("[Load]").expect("Load button missing");
        assert!(cancel_at < load_at, "Cancel should be left of Load");
        assert!(
            load_at + "[Load]".len() >= (dx + dw - 4) as usize,
            "buttons should be near the right corner"
        );
    }
}
