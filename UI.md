# UI Widgets Documentation

This document describes the reusable UI widgets defined in [`src/ui.rs`](src/ui.rs).
All widgets are built on top of [ratatui](https://docs.rs/ratatui/) and are used by
`main.rs` (rendering) and `app.rs` (state / event handling).

## Overview

| Widget | Type | Purpose |
|---|---|---|
| [`Theme`](#theme) | struct | Color palette shared by all widgets |
| [`dialog_block`](#dialog_block) | function | Styled bordered block for popup dialogs |
| [`Button`](#button) | struct | Clickable button with optional hotkey |
| [`Dropdown`](#dropdown) | struct | Standalone expandable dropdown |
| [`FileActionDialog`](#fileactiondialog) | struct | Composite modal dialog builder |
| [`MainMenu`](#mainmenu) | struct | Top menu bar (File / Edit / View / Help) |
| [`MessageBox`](#messagebox) | struct | Simple modal message box with an OK button |
| [`ConfirmationBox`](#confirmationbox) | struct | Modal yes/no confirmation box ("confirmationbox" class) |

---

## Theme

Central color palette used by all widgets for a consistent look.

```rust
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
```

**Constructors:**

- `Theme::default()` — default palette (cyan accents, dark gray dialogs).
- `Theme::dark()` — dark variant with a near-black (`Rgb(20, 20, 20)`) dialog background.

---

## dialog_block

```rust
pub fn dialog_block(title: &str, theme: &Theme) -> Block<'static>
```

Helper that returns a pre-styled ratatui `Block` used as the frame of every popup:

- All borders enabled, title rendered as ` TITLE ` (padded with spaces).
- Border colored with `theme.dialog_border`, background with `theme.dialog_bg`.
- Drop shadow (offset `1,1`) via ratatui's `Shadow`.

**Usage:**

```rust
f.render_widget(Clear, popup_area);
f.render_widget(dialog_block("About", theme), popup_area);
```

---

## Button

A single-line clickable button rendered as `[name]`.

```rust
pub struct Button {
    pub name: String,
    pub x: u16,
    pub y: u16,
    pub width: u16,     // name.len() + 4
    pub active: bool,
    pub fg: Color,
    pub bg: Color,
    pub hotkey: Option<char>,
}
```

**Methods:**

- `Button::new(name, x, y, active, fg, bg) -> Button` — creates a button; width is computed automatically.
- `is_clicked(col, row) -> bool` — hit test for mouse clicks.
- `render() -> (String, Style)` — returns the display text and style; the caller renders it with a `Paragraph`.

**Behavior:**

- When `active` is `true`: black bold text on `bg` (highlighted).
- When `active` is `false`: plain text in `fg`.
- If `hotkey` is set and the character occurs in `name`, that letter is uppercased in the display, e.g. hotkey `c` on `"Cancel"` renders as `[Cancel]` with the `C` emphasized.

**Usage (send button in `main.rs`):**

```rust
let send_btn = Button::new("send", btn_x, btn_y, has_text, Color::DarkGray, Color::Green);
let (btn_text, btn_style) = send_btn.render();
f.render_widget(Paragraph::new(Line::from(Span::styled(btn_text, btn_style))), btn_area);
```

---

## Dropdown

A standalone expandable dropdown widget. (Note: dialogs usually use the
`DialogItem::Dropdown` variant of [`FileActionDialog`](#fileactiondialog) instead.)

```rust
pub struct Dropdown {
    pub label: String,
    pub items: Vec<String>,
    pub selected: usize,
    pub x: u16,
    pub y: u16,
    pub focused: bool,
    pub expanded: bool,
}
```

**Methods:**

- `Dropdown::new(label, items, x, y) -> Dropdown`
- `total_height() -> u16` — `1` when collapsed, `1 + items.len()` when expanded.
- `render_line(line_offset, theme) -> Vec<Line>` — produces the lines to render.
- `hit_test(col, row) -> Option<usize>` — returns the clicked item index (or current selection when the header is clicked).

**Behavior:**

- Collapsed: `  Label: [value ▼]`.
- Expanded: header with `▲` followed by items prefixed with `•` (selected) or `○`.
- Focused items are drawn with `theme.focus_fg` + bold; otherwise `theme.accent`.

---

## FileActionDialog

A composite, vertically stacked modal dialog. This is the main dialog framework of
the application (used for *Save Session*, *Export As*, etc.).

```rust
pub struct FileActionDialog {
    pub title: String,
    pub items: Vec<DialogItem>,
}
```

### Building a dialog

Items are appended top-to-bottom with the `add_*` methods:

| Method | Item type | Height |
|---|---|---|
| `add_label(text)` | Static text line (white) | 1 |
| `add_label_colored(text, color)` | Static text line in a custom color | 1 |
| `add_text_input(label, value, cursor, focused)` | Label + input line (shows cursor when focused) | 2 |
| `add_dropdown(label, items, state)` | Expandable dropdown (`DialogDropdownState`) | 1 or `1 + items.len()` |
| `add_file_list(entries, selection, scroll, focused)` | **Bordered**, scrollable file list, `entries: Vec<(String, bool /*is_dir*/)>` | up to 14 (12 rows + borders), clamped to fit above the button row |
| `add_button(label, focused)` | Button row (consecutive calls append to the same row) | 1 |

Buttons are special: they are always rendered right-aligned on a single row, 2 lines
above the bottom of the dialog's inner area, regardless of insertion order.

The file list is drawn inside its own bordered box (border in `theme.focus_fg`
when the list has keyboard focus, `theme.dialog_border` otherwise). There is no
selection arrow — the selected row is highlighted full-width with
`theme.list_selected_bg`, and directories are shown with a trailing `/`. The
visible window is clamped to the space above the button row, and the effective
scroll is adjusted at render/hit-test time so the selection is always visible
(place the file list last, just before the buttons).

### Supporting types

```rust
pub enum DialogItem {
    Label(DialogLabel),
    TextInput(DialogTextInput),
    Dropdown { label: String, items: Vec<String>, state: DialogDropdownState },
    FileList { entries: Vec<(String, bool)>, selection: usize, scroll: usize, focused: bool },
    Buttons(Vec<DialogButton>),
}

pub struct DialogLabel {
    pub text: String,
    pub fg: Option<Color>,   // None -> white
}

pub struct DialogDropdownState {
    pub focused: bool,
    pub expanded: bool,
    pub selected: usize,
}
```

### Rendering

```rust
pub fn render(&self, f: &mut Frame, area: Rect, theme: &Theme) -> DialogAreas
```

- Sizes itself to `2/3` of `area` (minimum 40×10), centered; the size formula is
  shared with `hit_test` via `FileActionDialog::dialog_size(area)`.
- Returns `DialogAreas { popup, inner, item_areas }` with the exact `Rect` of every
  rendered item — useful for mouse handling.

### Hit testing

```rust
pub fn hit_test(&self, col: u16, row: u16, area: Rect) -> DialogHit
```

Mirrors `render`'s layout and returns:

```rust
pub enum DialogHit {
    Outside,                          // click outside the popup
    TextInput(usize),                 // item index
    DropdownItem(usize, Option<usize>),// item index, optional sub-item index
    Button(usize, usize),             // item index, button index
    FileListItem(usize, usize),       // item index, entry index
    None,                             // inside popup, but on no widget
}
```

### Usage (from `main.rs`)

```rust
let mut d = FileActionDialog::new("Save Session");
d.add_text_input("Session file:", &app.save_dialog_path, app.save_dialog_cursor, focused);
d.add_button("Cancel", cancel_focused);
d.add_button("Save", save_focused);
let areas = d.render(f, area, &app.theme);

// mouse handling
match d.hit_test(col, row, area) {
    DialogHit::Button(_, bi) => { /* ... */ }
    DialogHit::TextInput(_) => { /* ... */ }
    _ => {}
}
```

---

## MainMenu

The classic top menu bar with drop-down submenus: **File**, **Edit**, **View**, **Help**.

```rust
pub struct MainMenu {
    pub active: ActiveMenu,   // which menu is open
    pub selection: usize,     // highlighted item in the open menu
}
```

### Supporting enums

```rust
pub enum ActiveMenu { None, File, Edit, View, Help }

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
```

Menu contents (defined in `item_names()`):

| Menu | Items |
|---|---|
| File | Load, Save, Export..., ─, Exit |
| Edit | Set Model, Agentic Mode, ─, Settings... |
| View | Terminal |
| Help | About |

(`─` is a non-selectable separator.)

### Methods

- `new()`, `is_open()`, `open(menu)`, `close()` — state management.
- `item_names()`, `max_items()`, `is_separator(index)` — menu content queries.
- `bar_col_to_menu(col) -> ActiveMenu` — maps a click column on the bar to a menu.
- `handle_key(key) -> MenuAction` — keyboard navigation:
  - `←` / `→` — switch to previous / next menu (wraps around).
  - `↑` / `↓` — move selection (skips separators, wraps around).
  - `Enter` — activate the selected item, close the menu, return its `MenuAction`.
  - `Esc` — close the menu.
- `handle_click(col, row) -> MenuAction` — mouse support: row `0` toggles/switches menus; clicks inside the submenu activate items; clicks elsewhere close the menu.
- `render_bar(f, area)` — renders the bar: the menu names on the left and a clock (`HH:MM`) on the right. Nothing else is drawn here — status text and the mode indicator (agentic/chat) belong to the status bar.
- `render_submenu(f, menu_bar_area)` — renders the open drop-down list under the active menu.

### Usage

```rust
// event handling (app.rs)
let action = self.main_menu.handle_key(key);
let action = self.main_menu.handle_click(col, row);

// rendering (main.rs)
app.main_menu.render_bar(f, menu_area);
if app.main_menu.is_open() {
    app.main_menu.render_submenu(f, menu_area);
}
```

---

## MessageBox

A simple centered modal showing a title, a (possibly long, multi-paragraph)
message, and an **OK** button.

```rust
pub struct MessageBox {
    pub title: String,
    pub message: String,
}
```

**Methods:**

- `MessageBox::new(title, message) -> MessageBox`
- `render(f, area, focused, theme)` — renders the box; `focused` highlights the OK button.
- `hit_test(col, row, area) -> bool` — returns `true` when the OK button was clicked.

**Layout behavior (multirow text support):**

- The message is **word-wrapped** to the box width; explicit `\n` newlines and
  blank lines are preserved, and words longer than the wrap width are hard-split.
- The width auto-sizes to the longest source line (including the 2-column text
  indent), capped at 46 inner columns (50 total), and shrinks to fit small terminals.
- Height = wrapped line count + 4 rows of chrome; the OK button is centered one
  row below the text.
- Both `render` and `hit_test` share a single internal `layout()` computation,
  so the clickable button area always matches what is drawn. When changing the
  layout, only `layout()` needs to be updated.

**Usage:**

```rust
let mb = MessageBox::new("Retry Paused", &app.retry_paused_message);
mb.render(f, area, true, &app.theme);

if mb.hit_test(col, row, area) {
    // OK clicked — dismiss
}
```

---

## ConfirmationBox

The **confirmationbox** class: a modal variant of [`MessageBox`](#messagebox) for
yes/no questions. Shows a title, a word-wrapped message, and two buttons —
confirm (`Yes`, green) and cancel (`No`, red) — centered as a pair below the
text. Used for the *Confirm Quit* dialog.

```rust
pub struct ConfirmationBox {
    pub title: String,
    pub message: String,
    pub confirm_label: String,   // default "Yes"
    pub cancel_label: String,    // default "No"
}
```

### Supporting types

```rust
pub enum ConfirmFocus { Yes, No }        // which button is focused
pub enum ConfirmHit { Outside, Yes, No, None }  // hit_test result
```

**Methods:**

- `ConfirmationBox::new(title, message) -> ConfirmationBox` — default `Yes`/`No` labels.
- `with_labels(confirm, cancel) -> ConfirmationBox` — builder override for the button labels.
- `render(f, area, focus, theme)` — renders the box; the focused button is highlighted.
- `hit_test(col, row, area) -> ConfirmHit` — `Outside` clicks can be used to cancel,
  `None` means inside the popup but on no button.

**Layout behavior:**

- Same conventions as `MessageBox`: the message is **word-wrapped** to the box
  width (explicit newlines and blank lines preserved, long words hard-split),
  width auto-sizes to the longest source line (capped at 46 inner columns) but
  always grows enough to fit both buttons, and height = wrapped line count + 4
  rows of chrome.
- The button pair is centered on one row, one row below the text, with the
  confirm button on the left and the cancel button on the right.
- Both `render` and `hit_test` share a single internal `layout()` computation —
  when changing the layout, only `layout()` needs to be updated.

**Keyboard conventions (implemented by the caller, see `app.rs`):**

- `←` / `→` / `Tab` / `Shift+Tab` — move focus between the two buttons.
- `Enter` — activate the focused button.
- `y` / `n` — direct hotkeys, `Esc` — cancel.

**Usage (quit confirmation, from `main.rs` / `app.rs`):**

```rust
// rendering
let cb = ConfirmationBox::new("Confirm Quit", "Are you sure you want to quit?");
cb.render(f, area, app.quit_confirm_focus, &app.theme);

// mouse handling
match cb.hit_test(col, row, area) {
    ConfirmHit::Yes => { /* confirmed */ }
    ConfirmHit::No | ConfirmHit::Outside => { /* dismissed */ }
    ConfirmHit::None => {}
}
```

---

## Status Bar

Not a `ui.rs` widget — rendered by `render_status_bar()` in `main.rs`. It is the
single line between the **Output** and **Input** areas and is always visible.

Format (left to right, `|` separated):

```
 MODE | message | tokens
```

- **Mode** — always present. `AGENTIC` (black bold on cyan) when agentic mode is
  on, `CHAT` (white bold) otherwise.
- **Message** — `app.status_message`, shown when non-empty. Yellow bold while a
  request is loading/retrying, white bold otherwise.
- **Tokens** — `app.format_token_stats()` (cyan), shown when token stats exist.

The keybar (bottom line) only shows key hints and the focus indicator; mode and
status text live exclusively in the status bar.

---

## Conventions

- All widgets are **immediately rendered** — they produce `Line`/`Span`/style values
  or draw directly onto a `Frame`; persistent state lives in the widget structs and
  in `App`.
- Widgets that support the mouse provide both a `render` (returning areas or using
  fixed layout math) and a matching `hit_test` that replicates the layout exactly.
  Keep the two in sync when modifying layout.
- Dialogs clear the background first (`f.render_widget(Clear, popup_area)`) so they
  render as opaque popups with a shadow.
