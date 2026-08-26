use crate::ui::{
    ActiveMenu, Button, DialogDropdownState, DialogHit, FileActionDialog, MainMenu, MenuAction,
    Theme,
};
use arboard::Clipboard;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::text::Line;
use ratatui_textarea::TextArea;
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, mpsc};
use std::thread::JoinHandle;

use crate::config::{
    CloudModel, Config, ModelParams, ModelParamsStore, load_cloud_models_with_base,
    load_model_params, save_model_params,
};

#[derive(Debug, Clone, PartialEq)]
pub enum InputMode {
    Normal,
    Input,
    Menu,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Focus {
    Output,
    Input,
    /// Keyboard focus is on the embedded terminal: every key is forwarded
    /// to the PTY (except Ctrl+G, which leaves this mode).
    Terminal,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ModelDialogFocus {
    List,
    Confirm,
    Cancel,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileDialogFocus {
    List,
    Open,
    Cancel,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FileDialogMode {
    AttachFile,
    LoadSession,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SaveDialogMode {
    SaveSession,
    ExportChat,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SaveDialogFocus {
    Path,
    Format,
    Save,
    Cancel,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SettingsFocus {
    Proxy,
    OllamaUrl,
    Temperature,
    TopP,
    TopK,
    FrequencyPenalty,
    PresencePenalty,
    MaxTokens,
    ReasoningEffort,
    MaxToolRounds,
    MaxRetries,
    Justify,
    Save,
    Cancel,
}

impl SettingsFocus {
    /// Field order used by Tab/BackTab and Up/Down navigation.
    const ORDER: [SettingsFocus; 14] = [
        SettingsFocus::Proxy,
        SettingsFocus::OllamaUrl,
        SettingsFocus::Temperature,
        SettingsFocus::TopP,
        SettingsFocus::TopK,
        SettingsFocus::FrequencyPenalty,
        SettingsFocus::PresencePenalty,
        SettingsFocus::MaxTokens,
        SettingsFocus::ReasoningEffort,
        SettingsFocus::MaxToolRounds,
        SettingsFocus::MaxRetries,
        SettingsFocus::Justify,
        SettingsFocus::Save,
        SettingsFocus::Cancel,
    ];

    fn next(self) -> SettingsFocus {
        let idx = Self::ORDER.iter().position(|f| *f == self).unwrap_or(0);
        Self::ORDER[(idx + 1) % Self::ORDER.len()]
    }

    fn prev(self) -> SettingsFocus {
        let idx = Self::ORDER.iter().position(|f| *f == self).unwrap_or(0);
        Self::ORDER[(idx + Self::ORDER.len() - 1) % Self::ORDER.len()]
    }

    fn is_text_field(self) -> bool {
        matches!(
            self,
            SettingsFocus::Proxy
                | SettingsFocus::OllamaUrl
                | SettingsFocus::Temperature
                | SettingsFocus::TopP
                | SettingsFocus::TopK
                | SettingsFocus::FrequencyPenalty
                | SettingsFocus::PresencePenalty
                | SettingsFocus::MaxTokens
                | SettingsFocus::ReasoningEffort
                | SettingsFocus::MaxToolRounds
                | SettingsFocus::MaxRetries
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub enum ChatMessage {
    User(String),
    Assistant(String),
    System(String),
    App(String),
    Thinking(String),
    FileContent {
        name: String,
        content: String,
    },
    ToolCall {
        name: String,
        arguments: String,
        tool_call_id: Option<String>,
    },
    ToolResult {
        name: String,
        content: String,
        tool_call_id: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ExportFormat {
    Markdown,
    PlainText,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenStats {
    pub prompt_tokens: u64,
    pub response_tokens: u64,
    pub cached_tokens: u64,
    pub reasoning_tokens: u64,
    pub total_duration_ms: u64,
    pub tokens_per_sec: f64,
}

#[derive(Debug, Clone)]
pub enum StreamChunk {
    Text(String),
    Thinking(String),
    Done(TokenStats),
    Error(String),
    RetryPaused(String),
    StatusTick(String),
    ToolCalls(Vec<serde_json::Value>),
    Truncated(String),
    /// Token usage mid-stream (e.g. attached to the finish chunk before
    /// tool calls are executed). `check_responses` folds it into the stats
    /// that the next `Done`/`ToolCalls` transition reports.
    Stats(TokenStats),
}

const TERMINAL_RAW_LOG_MAX: usize = 256 * 1024;

/// Raw output log of the PTY. Kept alongside the vt100 screen state so the
/// model-facing `terminal_read` tool keeps its cursor-based incremental read
/// API (the vt100 screen alone cannot provide a stable byte cursor).
///
/// The cursor counts *characters*, not bytes: the PTY stream contains ANSI
/// escapes and multibyte UTF-8, and a byte-accurate cursor would require
/// either storing raw bytes (breaking the string API) or risking
/// mid-codepoint splits. Char-based cursors keep every offset a valid
/// boundary by construction.
pub struct TerminalBuffer {
    /// Raw output received from the PTY (ANSI escapes included), drained
    /// from the front when it exceeds [`TERMINAL_RAW_LOG_MAX`] chars.
    pub content: String,
    /// Total chars ever written (monotonic cursor).
    pub total_bytes_written: u64,
    /// Chars discarded from the front of `content` so far.
    pub buffer_start_offset: u64,
}

impl TerminalBuffer {
    pub fn new() -> Self {
        TerminalBuffer {
            content: String::new(),
            total_bytes_written: 0,
            buffer_start_offset: 0,
        }
    }

    fn push(&mut self, text: &str) {
        let chars = text.chars().count() as u64;
        self.total_bytes_written += chars;
        self.content.push_str(text);
        let len = self.content.chars().count() as u64;
        if len as usize > TERMINAL_RAW_LOG_MAX {
            let drain_to = (len as usize) - TERMINAL_RAW_LOG_MAX / 2;
            let byte_idx = self
                .content
                .char_indices()
                .nth(drain_to)
                .map_or(self.content.len(), |(i, _)| i);
            self.buffer_start_offset += drain_to as u64;
            self.content = self.content.split_off(byte_idx);
        }
    }

    /// Char offset → byte index (always on a char boundary, clamped).
    fn byte_idx(&self, char_off: usize) -> usize {
        self.content
            .char_indices()
            .nth(char_off)
            .map_or(self.content.len(), |(i, _)| i)
    }

    pub fn read_incremental(&self, cursor: Option<u64>) -> serde_json::Value {
        let total = self.total_bytes_written;
        let start = self.buffer_start_offset;
        let char_len = self.content.chars().count();

        let tail = |content: &str| -> String {
            let n = content.chars().count();
            if n > 4000 {
                let skip_bytes = content
                    .char_indices()
                    .nth(n - 4000)
                    .map_or(0, |(i, _)| i);
                content[skip_bytes..].to_string()
            } else {
                content.to_string()
            }
        };

        match cursor {
            None => {
                let output = tail(&self.content);
                serde_json::json!({
                    "output": output,
                    "cursor": total,
                    "gap": false,
                })
            }
            Some(c) => {
                if c > total {
                    // Cursor is ahead of what's been written — invalid/stale cursor
                    serde_json::json!({
                        "output": "",
                        "cursor": total,
                        "gap": false,
                        "error": format!("Invalid cursor {}: beyond total chars written ({})", c, total),
                    })
                } else if c == total {
                    // Exactly at the end — nothing new
                    serde_json::json!({
                        "output": "",
                        "cursor": total,
                        "gap": false,
                    })
                } else if c >= start {
                    let offset = (c - start) as usize;
                    let output = if offset < char_len {
                        self.content[self.byte_idx(offset)..].to_string()
                    } else {
                        String::new()
                    };
                    serde_json::json!({
                        "output": output,
                        "cursor": total,
                        "gap": false,
                    })
                } else {
                    let output = tail(&self.content);
                    serde_json::json!({
                        "output": output,
                        "cursor": total,
                        "gap": true,
                    })
                }
            }
        }
    }
}

/// Everything protected by the terminal mutex: the vt100 screen emulator
/// (what the user sees, and what the model reads as `screen`) plus the raw
/// output log (byte cursor for incremental reads).
pub struct TerminalCore {
    pub parser: vt100::Parser<TerminalQueries>,
    pub raw: TerminalBuffer,
}

impl TerminalCore {
    fn new(cols: u16, rows: u16) -> Self {
        TerminalCore {
            parser: vt100::Parser::new_with_callbacks(
                rows,
                cols,
                0,
                TerminalQueries::default(),
            ),
            raw: TerminalBuffer::new(),
        }
    }
}

/// vt100 callbacks: answers terminal *queries* the child application sends.
///
/// Real terminals answer these; without answers, TUI frameworks hang at
/// startup waiting for a reply (crossterm reads the cursor position —
/// `ESC[6n` — when entering raw mode, which is exactly what made Rustama
/// fail to start inside its own terminal).
///
/// The reply bytes are queued and drained by the UI loop, which writes
/// them to the PTY master.
#[derive(Default)]
pub struct TerminalQueries {
    pub pending_replies: Vec<u8>,
}

impl vt100::Callbacks for TerminalQueries {
    fn unhandled_csi(
        &mut self,
        screen: &mut vt100::Screen,
        i1: Option<u8>,
        _i2: Option<u8>,
        params: &[&[u16]],
        c: char,
    ) {
        match (i1, params, c) {
            // DSR "report cursor position" (ESC[6n) -> ESC[{row};{col}R
            (None, [[6]], 'n') => {
                let (row, col) = screen.cursor_position();
                let reply = format!("\x1b[{};{}R", row + 1, col + 1);
                self.pending_replies.extend_from_slice(reply.as_bytes());
            }
            // DSR "status report" (ESC[5n) -> ESC[0n ("terminal OK")
            (None, [[5]], 'n') => {
                self.pending_replies.extend_from_slice(b"\x1b[0n");
            }
            // DA1 "primary device attributes" (ESC[c / ESC[0c):
            // claim to be a VT220 with 256-color-ish feature set.
            (None, [[]] | [[0]], 'c') => {
                self.pending_replies
                    .extend_from_slice(b"\x1b[?62;4;6;22c");
            }
            // DECRQM would go here; vt100 handles the common modes itself.
            _ => {}
        }
    }

    fn unhandled_escape(
        &mut self,
        _screen: &mut vt100::Screen,
        i1: Option<u8>,
        _i2: Option<u8>,
        b: u8,
    ) {
        // ESC Z (DECID, ~"identify terminal") — answer like DA1.
        if i1.is_none() && b == b'Z' {
            self.pending_replies
                .extend_from_slice(b"\x1b[?62;4;6;22c");
        }
        // ESC c (RIS, full reset) — vt100 handles screen state; nothing
        // to answer.
    }
}

/// Interactive PTY-backed terminal embedded in the UI.
///
/// Replaces the original pipe-based implementation: the child process runs
/// attached to a real pseudo-terminal (via `portable-pty`), so REPLs
/// (python3, node), readline programs, full-screen TUIs (top, less, vim —
/// including Rustama itself) and ssh all work. Output is parsed into a
/// vt100 screen model which the UI renders cell-by-cell; keyboard input
/// (user or model via the terminal tools) is written to the PTY master.
pub struct TerminalState {
    pub core: Arc<Mutex<TerminalCore>>,
    writer: Arc<Mutex<Option<Box<dyn std::io::Write + Send>>>>,
    child: Arc<Mutex<Option<Box<dyn portable_pty::Child + Send>>>>,
    master: Arc<Mutex<Option<Box<dyn portable_pty::MasterPty + Send>>>>,
    reader: Option<JoinHandle<()>>,
    pub visible: bool,
    pub width_pct: u16,
    pub command: String,
    /// Last PTY size we resized to (cols, rows).
    size: (u16, u16),
}

impl TerminalState {
    pub fn new() -> Self {
        TerminalState {
            core: Arc::new(Mutex::new(TerminalCore::new(80, 24))),
            writer: Arc::new(Mutex::new(None)),
            child: Arc::new(Mutex::new(None)),
            master: Arc::new(Mutex::new(None)),
            reader: None,
            visible: false,
            width_pct: 40,
            command: String::new(),
            size: (80, 24),
        }
    }

    pub fn is_running(&self) -> bool {
        let mut child = self.child.lock().unwrap();
        if let Some(ref mut c) = *child {
            // `try_wait` returns Err while the child is alive on some
            // backends, Ok(Some) once reaped, Ok(None) if still running.
            match c.try_wait() {
                Ok(Some(_)) => return false,
                _ => return true,
            }
        }
        false
    }

    pub fn open(&mut self, command: &str) -> String {
        if self.is_running() {
            return format!("Terminal already running: {}", self.command);
        }

        let trimmed = command.trim_start();
        if trimmed.starts_with("sudo ") || trimmed == "sudo" {
            return "Error: sudo is not permitted.".to_string();
        }

        let pty_system = portable_pty::native_pty_system();
        let pair = match pty_system.openpty(portable_pty::PtySize {
            rows: self.size.1,
            cols: self.size.0,
            pixel_width: 0,
            pixel_height: 0,
        }) {
            Ok(p) => p,
            Err(e) => return format!("Error opening pty: {}", e),
        };

        // Always spawn bash — commands are sent as keystrokes afterwards.
        // This keeps one process, one screen, one monotonic cursor across
        // the entire session lifecycle.
        let mut cmd = portable_pty::CommandBuilder::new("bash");
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        // Keep prompt noise down: a simple, recognizable prompt.
        cmd.env("PS1", "$ ");

        match pair.slave.spawn_command(cmd) {
            Ok(child) => {
                let mut reader = match pair.master.try_clone_reader() {
                    Ok(r) => r,
                    Err(e) => return format!("Error cloning pty reader: {}", e),
                };
                let writer = match pair.master.take_writer() {
                    Ok(w) => w,
                    Err(e) => return format!("Error taking pty writer: {}", e),
                };

                let core = Arc::new(Mutex::new(TerminalCore::new(self.size.0, self.size.1)));
                let reader_core = core.clone();
                let reader = std::thread::spawn(move || {
                    let mut buf = [0u8; 8192];
                    loop {
                        match reader.read(&mut buf) {
                            Ok(0) => break,
                            Ok(n) => {
                                let text = String::from_utf8_lossy(&buf[..n]).to_string();
                                let mut c = reader_core.lock().unwrap();
                                c.parser.process(text.as_bytes());
                                c.raw.push(&text);
                            }
                            Err(_) => break,
                        }
                    }
                });

                self.core = core;
                *self.writer.lock().unwrap() = Some(writer);
                *self.child.lock().unwrap() = Some(child);
                *self.master.lock().unwrap() = Some(pair.master);
                self.reader = Some(reader);
                self.visible = true;

                if !command.trim().is_empty() {
                    self.command = command.to_string();
                    let _ = self.send_input(command);
                    format!("Terminal opened running: {}", command)
                } else {
                    self.command = "bash".to_string();
                    "Terminal opened: bash".to_string()
                }
            }
            Err(e) => format!("Error spawning shell: {}", e),
        }
    }

    /// Sends text to the PTY, appending a newline — like typing the text
    /// and pressing Enter. This is what the `terminal_send` tool uses.
    pub fn send_input(&mut self, input: &str) -> String {
        self.send_raw(&format!("{}\n", input))
            .map(|()| format!("Sent: {}", input))
            .unwrap_or_else(|e| e)
    }

    /// Writes raw bytes to the PTY master with no trailing newline —
    /// keystrokes, escape sequences, control characters.
    pub fn send_raw(&mut self, data: &str) -> Result<(), String> {
        let mut writer = self.writer.lock().unwrap();
        match writer.as_mut() {
            Some(w) => {
                use std::io::Write;
                w.write_all(data.as_bytes())
                    .and_then(|()| w.flush())
                    .map_err(|e| format!("Error writing to terminal: {}", e))
            }
            None => Err("Error: no terminal running".to_string()),
        }
    }

    /// Resizes the PTY and the screen emulator to match the panel size.
    pub fn resize(&mut self, cols: u16, rows: u16) {
        if cols == 0 || rows == 0 || (cols, rows) == self.size {
            return;
        }
        self.size = (cols, rows);
        if let Some(ref master) = *self.master.lock().unwrap() {
            let _ = master.resize(portable_pty::PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            });
        }
        self.core
            .lock()
            .unwrap()
            .parser
            .screen_mut()
            .set_size(rows, cols);
    }

    /// Writes any pending terminal-query replies (cursor position reports,
    /// device attributes — see [`TerminalQueries`]) back to the child
    /// through the PTY master. Called from the UI loop.
    pub fn flush_replies(&mut self) {
        let replies = {
            let mut core = self.core.lock().unwrap();
            if core.parser.callbacks().pending_replies.is_empty() {
                return;
            }
            std::mem::take(&mut core.parser.callbacks_mut().pending_replies)
        };
        let mut writer = self.writer.lock().unwrap();
        if let Some(w) = writer.as_mut() {
            use std::io::Write;
            let _ = w.write_all(&replies);
            let _ = w.flush();
        }
    }

    pub fn read_buffer_incremental(&self, cursor: Option<u64>) -> serde_json::Value {
        let c = self.core.lock().unwrap();
        let mut result = c.raw.read_incremental(cursor);
        // The rendered screen is far more useful than raw bytes for
        // understanding what a full-screen application is showing.
        result["screen"] = serde_json::json!(c.parser.screen().contents());
        let (row, col) = c.parser.screen().cursor_position();
        result["screen_cursor"] = serde_json::json!({"row": row, "col": col});
        result
    }

    pub fn close(&mut self) -> String {
        self.writer.lock().unwrap().take();
        if let Some(mut child) = self.child.lock().unwrap().take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.master.lock().unwrap().take();
        if let Some(handle) = self.reader.take() {
            let _ = handle.join();
        }
        self.visible = false;
        self.command.clear();
        *self.core.lock().unwrap() = TerminalCore::new(self.size.0, self.size.1);
        "Terminal closed".to_string()
    }
}

/// Maps a crossterm key event to the bytes a real terminal would send for
/// it, for forwarding to the PTY when the terminal panel has keyboard
/// focus. Returns `None` for keys that have no terminal encoding.
pub fn key_to_pty_bytes(key: &KeyEvent) -> Option<String> {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let alt = key.modifiers.contains(KeyModifiers::ALT);
    let shift = key.modifiers.contains(KeyModifiers::SHIFT);

    let base: String = match key.code {
        KeyCode::Char(c) => {
            if ctrl {
                // Ctrl+letter -> control byte (0x01-0x1a); also handle a
                // few common punctuation combos.
                let lc = c.to_ascii_lowercase();
                if lc.is_ascii_lowercase() {
                    ((lc as u8 - b'a' + 1) as char).to_string()
                } else {
                    match c {
                        '2' | '@' => "\x00".to_string(),
                        '3' | '[' => "\x1b".to_string(),
                        '4' | '\\' => "\x1c".to_string(),
                        '5' | ']' => "\x1d".to_string(),
                        '6' | '^' => "\x1e".to_string(),
                        '7' | '_' => "\x1f".to_string(),
                        '8' | '?' => "\x7f".to_string(),
                        _ => return None,
                    }
                }
            } else {
                c.to_string()
            }
        }
        KeyCode::Enter => "\r".to_string(),
        KeyCode::Tab => {
            if shift { "\x1b[Z".to_string() } else { "\t".to_string() }
        }
        KeyCode::BackTab => "\x1b[Z".to_string(),
        KeyCode::Backspace => "\x7f".to_string(),
        KeyCode::Esc => "\x1b".to_string(),
        KeyCode::Up => "\x1b[A".to_string(),
        KeyCode::Down => "\x1b[B".to_string(),
        KeyCode::Right => "\x1b[C".to_string(),
        KeyCode::Left => "\x1b[D".to_string(),
        KeyCode::Home => "\x1b[H".to_string(),
        KeyCode::End => "\x1b[F".to_string(),
        KeyCode::PageUp => "\x1b[5~".to_string(),
        KeyCode::PageDown => "\x1b[6~".to_string(),
        KeyCode::Delete => "\x1b[3~".to_string(),
        KeyCode::Insert => "\x1b[2~".to_string(),
        KeyCode::F(n) => match n {
            1 => "\x1bOP".to_string(),
            2 => "\x1bOQ".to_string(),
            3 => "\x1bOR".to_string(),
            4 => "\x1bOS".to_string(),
            5 => "\x1b[15~".to_string(),
            6 => "\x1b[17~".to_string(),
            7 => "\x1b[18~".to_string(),
            8 => "\x1b[19~".to_string(),
            9 => "\x1b[20~".to_string(),
            10 => "\x1b[21~".to_string(),
            11 => "\x1b[23~".to_string(),
            12 => "\x1b[24~".to_string(),
            _ => return None,
        },
        _ => return None,
    };

    // Alt prefixes the sequence with ESC (meta).
    if alt && !ctrl {
        Some(format!("\x1b{}", base))
    } else {
        Some(base)
    }
}

pub struct App {
    pub messages: Vec<ChatMessage>,
    pub textarea: TextArea<'static>,
    pub clipboard: Option<Clipboard>,
    pub scroll_offset: u16,
    pub auto_scroll: bool,
    pub scrollbar_dragging: bool,
    pub scrollbar_drag_start: Option<u16>,
    pub should_quit: bool,
    pub input_mode: InputMode,
    pub focus: Focus,
    pub main_menu: MainMenu,
    pub show_about: bool,
    pub about_message: String,
    pub show_quit_confirm: bool,
    pub quit_confirm_focus: crate::ui::ConfirmFocus,
    pub is_loading: bool,
    pub retrying: bool,
    pub status_message: String,
    pub ollama_url: String,
    pub model_name: String,
    pub save_path: String,
    pub response_rx: Option<mpsc::Receiver<StreamChunk>>,
    pub streaming_text: String,
    pub streaming_thinking: String,
    pub show_model_dialog: bool,
    pub available_models: Vec<String>,
    pub model_dialog_selection: usize,
    pub model_dialog_focus: ModelDialogFocus,
    pub models_rx: Option<mpsc::Receiver<Result<Vec<String>, String>>>,
    pub show_file_dialog: bool,
    pub file_dialog_path: PathBuf,
    pub file_dialog_entries: Vec<(String, bool)>,
    pub file_dialog_selection: usize,
    pub file_dialog_focus: FileDialogFocus,
    pub file_dialog_scroll: usize,
    pub file_dialog_mode: FileDialogMode,
    pub agentic_mode: bool,
    pub max_tool_rounds: usize,
    pub max_retries: u32,
    pub tool_round_count: usize,
    pub tool_call_count: usize,
    pub pending_tool_calls: Vec<serde_json::Value>,
    pub tool_call_log: Vec<(String, String, String)>,
    /// Effective generation parameters for the currently selected model.
    pub params: ModelParams,
    /// Per-model parameters for Ollama models (from `model_params.conf`).
    pub model_params_store: ModelParamsStore,
    pub show_save_dialog: bool,
    pub save_dialog_path: String,
    pub save_dialog_cursor: usize,
    pub save_dialog_focus: SaveDialogFocus,
    pub save_dialog_mode: SaveDialogMode,
    pub show_format_dropdown: bool,
    pub show_load_dialog: bool,
    pub load_dialog_path: String,
    pub load_dialog_cursor: usize,
    pub show_settings_dialog: bool,
    pub settings_focus: SettingsFocus,
    pub settings_proxy: String,
    pub settings_ollama_url: String,
    pub settings_temperature: String,
    pub settings_top_p: String,
    pub settings_top_k: String,
    pub settings_frequency_penalty: String,
    pub settings_presence_penalty: String,
    pub settings_max_tokens: String,
    pub settings_reasoning_effort: String,
    pub settings_max_tool_rounds: String,
    pub settings_max_retries: String,
    pub settings_cursor: usize,
    pub proxy: Option<String>,
    pub is_logging: bool,
    pub log_file: String,
    pub token_stats: TokenStats,
    pub cloud_models: Vec<CloudModel>,
    pub system_prompt: String,
    pub export_format: ExportFormat,
    pub theme: Theme,
    pub cached_output: Vec<Line<'static>>,
    pub cached_wrapped: Vec<Line<'static>>,
    pub cached_msg_count: usize,
    pub cached_streaming_len: usize,
    pub cached_streaming_thinking_len: usize,
    pub cached_width: u16,
    pub terminal_height: u16,
    pub output_width: u16,
    pub session_id: String,
    pub session_name: String,
    pub terminal_state: TerminalState,
    pub justify: bool,
    pub settings_justify: bool,
    pub selection_start: Option<usize>,
    pub selection_end: Option<usize>,
    pub selecting: bool,
    pub primary_selection: crate::primary_selection::PrimarySelection,
    pub show_retry_paused: bool,
    pub retry_paused_message: String,
    pub pending_continuation: bool,
    pub continuation_count: u32,
    /// Automatic "continue" nudges sent this turn because the model stopped
    /// with `finish_reason="stop"` right after an unfinished-looking text
    /// (see `needs_continuation`). Reset on fresh user input.
    pub auto_continue_count: u32,
    /// Token usage received mid-stream (via `StreamChunk::Stats`) that has not
    /// been committed to `token_stats` yet.
    last_stats: TokenStats,
}

impl App {
    pub fn new(cfg: Config) -> Self {
        let textarea = TextArea::default();
        let clipboard = Clipboard::new().ok();
        rotate_log_file(&cfg.logfile);
        let model_params_store = load_model_params();
        let cloud_models = load_cloud_models_with_base(&model_params_store.default);
        let params = params_for_model(&cfg.model, &cloud_models, &model_params_store);
        let mut app = App {
            messages: vec![ChatMessage::App(
                "Welcome to Rustama. Start typing your message.".to_string(),
            )],
            textarea,
            clipboard,
            scroll_offset: 0,
            auto_scroll: true,
            scrollbar_dragging: false,
            scrollbar_drag_start: None,
            should_quit: false,
            input_mode: InputMode::Input,
            focus: Focus::Input,
            main_menu: MainMenu::new(),
            show_about: false,
            about_message: format!(
                "Rustama v{}\n\nA terminal AI coding agent for Ollama LLMs.\nSupports markdown rendering, agentic tools, and saving.\n\nBuilt with ratatui + crossterm",
                env!("CARGO_PKG_VERSION")
            ),
            show_quit_confirm: false,
            quit_confirm_focus: crate::ui::ConfirmFocus::No,
            is_loading: false,
            retrying: false,
            status_message: String::new(),
            ollama_url: cfg.ollama_url.clone(),
            model_name: cfg.model,
            save_path: cfg.save_path.clone(),
            response_rx: None,
            streaming_text: String::new(),
            streaming_thinking: String::new(),
            show_model_dialog: false,
            available_models: Vec::new(),
            model_dialog_selection: 0,
            model_dialog_focus: ModelDialogFocus::List,
            models_rx: None,
            show_file_dialog: false,
            file_dialog_path: dirs_home(),
            file_dialog_entries: Vec::new(),
            file_dialog_selection: 0,
            file_dialog_focus: FileDialogFocus::List,
            file_dialog_scroll: 0,
            file_dialog_mode: FileDialogMode::AttachFile,
            agentic_mode: cfg.agentic,
            max_tool_rounds: cfg.max_tool_rounds,
            max_retries: cfg.max_retries,
            tool_round_count: 0,
            tool_call_count: 0,
            pending_tool_calls: Vec::new(),
            tool_call_log: Vec::new(),
            params: params.clone(),
            model_params_store,
            show_save_dialog: false,
            save_dialog_path: cfg.save_path,
            save_dialog_cursor: 0,
            save_dialog_focus: SaveDialogFocus::Path,
            save_dialog_mode: SaveDialogMode::ExportChat,
            show_format_dropdown: false,
            show_load_dialog: false,
            load_dialog_path: dirs_home().to_string_lossy().to_string(),
            load_dialog_cursor: 0,
            show_settings_dialog: false,
            settings_focus: SettingsFocus::Proxy,
            settings_proxy: cfg.proxy.clone().unwrap_or_default(),
            settings_ollama_url: cfg.ollama_url.clone(),
            settings_temperature: format!("{}", params.temperature),
            settings_top_p: format!("{}", params.top_p),
            settings_top_k: format!("{}", params.top_k),
            settings_frequency_penalty: format!("{}", params.frequency_penalty),
            settings_presence_penalty: format!("{}", params.presence_penalty),
            settings_max_tokens: params
                .max_output_tokens
                .map(|n| n.to_string())
                .unwrap_or_default(),
            settings_reasoning_effort: params.reasoning_effort.clone().unwrap_or_default(),
            settings_max_tool_rounds: cfg.max_tool_rounds.to_string(),
            settings_max_retries: cfg.max_retries.to_string(),
            settings_cursor: 0,
            proxy: cfg.proxy.clone(),
            is_logging: cfg.logging,
            log_file: cfg.logfile,
            token_stats: TokenStats::default(),
            cloud_models,
            system_prompt: cfg.system_prompt.clone(),
            export_format: ExportFormat::Markdown,
            theme: Theme::dark(),
            cached_output: Vec::new(),
            cached_wrapped: Vec::new(),
            cached_msg_count: 0,
            cached_streaming_len: 0,
            cached_streaming_thinking_len: 0,
            cached_width: 0,
            terminal_height: 24,
            output_width: 0,
            session_id: format!(
                "{:016x}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos()
                    & 0xffff_ffff_ffff_ffff
            ),
            session_name: String::new(),
            terminal_state: TerminalState {
                width_pct: cfg.terminal_width_pct,
                ..TerminalState::new()
            },
            justify: cfg.justify,
            settings_justify: cfg.justify,
            selection_start: None,
            selection_end: None,
            selecting: false,
            primary_selection: crate::primary_selection::PrimarySelection::new(),
            show_retry_paused: false,
            retry_paused_message: String::new(),
            pending_continuation: false,
            continuation_count: 0,
            auto_continue_count: 0,
            last_stats: TokenStats::default(),
        };
        app.session_name = app.session_id.clone();
        log_to_file(
            app.is_logging,
            &app.log_file,
            &app.session_id,
            "START",
            "Program started",
        );
        app.fetch_models_async();
        app
    }

    pub fn handle_global_key(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }

        if self.show_retry_paused {
            match key.code {
                KeyCode::Enter | KeyCode::Esc | KeyCode::Char(' ') => {
                    self.show_retry_paused = false;
                }
                _ => {}
            }
            return;
        }

        // Terminal focus mode: nearly every key goes straight to the PTY.
        // Ctrl+G is the escape hatch back to the chat UI.
        if self.focus == Focus::Terminal {
            if key.code == KeyCode::Char('g') || key.code == KeyCode::Char('G')
                && key.modifiers.contains(KeyModifiers::CONTROL)
            {
                self.focus = Focus::Input;
                self.input_mode = InputMode::Input;
                self.status_message = "Terminal focus released (back to input)".to_string();
                return;
            }
            if let Some(bytes) = key_to_pty_bytes(&key) {
                let _ = self.terminal_state.send_raw(&bytes);
            }
            return;
        }

        match key.code {
            KeyCode::F(9) => {
                self.open_menu();
                return;
            }
            KeyCode::F(10) => {
                self.open_quit_confirm();
                return;
            }
            KeyCode::F(6) => {
                // Grab terminal keyboard focus.
                if self.terminal_state.is_running() {
                    self.terminal_state.visible = true;
                    self.focus = Focus::Terminal;
                    self.status_message =
                        "Terminal focused — keystrokes go to the shell. Ctrl+G: release"
                            .to_string();
                } else {
                    let result = self.terminal_state.open("bash");
                    self.focus = Focus::Terminal;
                    self.status_message = result;
                }
                return;
            }
            KeyCode::Char('t' | 'T') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if self.terminal_state.is_running() {
                    self.terminal_state.visible = !self.terminal_state.visible;
                    if !self.terminal_state.visible && self.focus == Focus::Terminal {
                        self.focus = Focus::Input;
                        self.input_mode = InputMode::Input;
                    }
                } else {
                    let result = self.terminal_state.open("bash");
                    self.status_message = result;
                }
                return;
            }
            KeyCode::Left
                if key.modifiers.contains(KeyModifiers::CONTROL)
                    && key.modifiers.contains(KeyModifiers::SHIFT) =>
            {
                if !self.terminal_state.is_running() {
                    self.terminal_state.open("bash");
                }
                if self.terminal_state.width_pct > 20 {
                    self.terminal_state.width_pct -= 5;
                    self.status_message =
                        format!("Terminal width: {}%", self.terminal_state.width_pct);
                }
                return;
            }
            KeyCode::Right
                if key.modifiers.contains(KeyModifiers::CONTROL)
                    && key.modifiers.contains(KeyModifiers::SHIFT) =>
            {
                if !self.terminal_state.is_running() {
                    self.terminal_state.open("bash");
                }
                if self.terminal_state.width_pct < 80 {
                    self.terminal_state.width_pct += 5;
                    self.status_message =
                        format!("Terminal width: {}%", self.terminal_state.width_pct);
                }
                return;
            }
            _ => {}
        }

        if self.show_quit_confirm {
            self.handle_quit_confirm_key(key);
            return;
        }
        if self.show_about {
            self.handle_about_key(key);
            return;
        }
        if self.show_load_dialog {
            self.handle_load_dialog_key(key);
            return;
        }
        if self.show_save_dialog {
            self.handle_save_dialog_key(key);
            return;
        }
        if self.show_file_dialog {
            self.handle_file_dialog_key(key);
            return;
        }
        if self.show_model_dialog {
            self.handle_model_dialog_key(key);
            return;
        }
        if self.show_settings_dialog {
            self.handle_settings_dialog_key(key);
            return;
        }
        match self.input_mode {
            InputMode::Normal => self.handle_output_key(key),
            InputMode::Input => self.handle_input_key(key),
            InputMode::Menu => self.handle_menu_key(key),
        }
    }

    fn handle_output_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Tab => self.open_menu(),
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.open_export_dialog();
            }
            KeyCode::Up => self.scroll_up(),
            KeyCode::Down => self.scroll_down(),
            KeyCode::PageUp => self.page_up(),
            KeyCode::PageDown => self.page_down(),
            KeyCode::Home => self.home(),
            KeyCode::End => self.end(),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if let Some(text) = self.selected_text() {
                    if let Some(ref mut cb) = self.clipboard {
                        match cb.set_text(text.trim_end().to_string()) {
                            Ok(_) => {
                                self.status_message = "Copied selection to clipboard".to_string()
                            }
                            Err(e) => self.status_message = format!("Clipboard write: {}", e),
                        }
                    } else {
                        self.status_message = "No clipboard available".to_string();
                    }
                } else {
                    self.status_message = "Nothing selected".to_string();
                }
            }
            KeyCode::Char(c) => {
                self.input_mode = InputMode::Input;
                self.focus = Focus::Input;
                self.textarea.insert_char(c);
            }
            KeyCode::Enter => {
                self.input_mode = InputMode::Input;
                self.focus = Focus::Input;
            }
            _ => {}
        }
    }

    fn handle_input_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => {
                if key.modifiers == KeyModifiers::ALT {
                    self.textarea.input(key);
                } else if !self.textarea.lines().join("").trim().is_empty() && !self.is_loading {
                    self.send_to_ollama_async();
                }
            }
            KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.textarea.select_all();
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.textarea.copy();
                let yank = self.textarea.yank_text();
                if !yank.is_empty() {
                    if let Some(ref mut cb) = self.clipboard {
                        match cb.set_text(&yank) {
                            Ok(_) => self.status_message = "Copied to clipboard".to_string(),
                            Err(e) => self.status_message = format!("Clipboard write: {}", e),
                        }
                    } else {
                        self.status_message = "No clipboard available".to_string();
                    }
                } else {
                    self.status_message = "Nothing selected".to_string();
                }
            }
            KeyCode::Char('v') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if let Some(ref mut cb) = self.clipboard {
                    match cb.get_text() {
                        Ok(text) => {
                            let len = text.len();
                            for ch in text.chars() {
                                self.textarea.insert_char(ch);
                            }
                            self.status_message = format!("Pasted {} bytes", len);
                        }
                        Err(e) => self.status_message = format!("Clipboard read: {}", e),
                    }
                } else {
                    self.status_message = "No clipboard available".to_string();
                }
            }
            KeyCode::Tab => {
                self.tab_complete_model();
            }
            _ => {
                self.textarea.input(key);
            }
        }
    }

    fn tab_complete_model(&mut self) {
        let text = self.textarea.lines().join("");
        let lower = text.to_lowercase();

        let prefix = if lower.starts_with("/use ") {
            Some("/use ")
        } else if lower.starts_with("/model ") {
            Some("/model ")
        } else {
            None
        };

        let Some(cmd_prefix) = prefix else {
            return;
        };

        let partial = text[cmd_prefix.len()..].trim_start().to_string();
        if partial.is_empty() {
            return;
        }

        let mut matches: Vec<String> = self
            .available_models
            .iter()
            .filter(|m| m.to_lowercase().starts_with(&partial.to_lowercase()))
            .cloned()
            .collect();
        for m in &self.cloud_models {
            if m.name.to_lowercase().starts_with(&partial.to_lowercase())
                && !matches.contains(&m.name)
            {
                matches.push(m.name.clone());
            }
        }

        match matches.len() {
            0 => {
                self.status_message = format!("No models match '{}'", partial);
            }
            1 => {
                let completed = &matches[0];
                let new_text = format!("{}{}", cmd_prefix, completed);
                self.textarea = TextArea::default();
                for ch in new_text.chars() {
                    self.textarea.insert_char(ch);
                }
            }
            _ => {
                let common = common_prefix(&matches);
                if common.len() > partial.len() {
                    let new_text = format!("{}{}", cmd_prefix, common);
                    self.textarea = TextArea::default();
                    for ch in new_text.chars() {
                        self.textarea.insert_char(ch);
                    }
                }
                self.status_message = format!("Matches: {}", matches.join(", "));
            }
        }
    }

    fn handle_menu_key(&mut self, key: KeyEvent) {
        let action = self.main_menu.handle_key(key);
        self.handle_menu_action(action);
    }

    fn open_menu(&mut self) {
        self.main_menu.open(ActiveMenu::File);
        self.input_mode = InputMode::Menu;
    }

    fn handle_menu_action(&mut self, action: MenuAction) {
        match action {
            MenuAction::None => {}
            MenuAction::LoadSession => self.open_load_session_dialog(),
            MenuAction::SaveSession => self.open_save_dialog(),
            MenuAction::ExportChat => self.open_export_dialog(),
            MenuAction::Quit => self.open_quit_confirm(),
            MenuAction::OpenModelDialog => self.open_model_dialog(),
            MenuAction::ToggleAgenticMode(_) => {
                self.agentic_mode = !self.agentic_mode;
                self.status_message = if self.agentic_mode {
                    "Agentic mode: ON".to_string()
                } else {
                    "Agentic mode: OFF".to_string()
                };
            }
            MenuAction::ToggleTerminal => {
                if self.terminal_state.is_running() {
                    if self.terminal_state.visible {
                        self.terminal_state.visible = false;
                        self.status_message = "Terminal hidden".to_string();
                    } else {
                        self.terminal_state.visible = true;
                        self.status_message = "Terminal shown".to_string();
                    }
                } else {
                    let result = self.terminal_state.open("bash");
                    self.status_message = result;
                }
            }
            MenuAction::OpenSettingsDialog => self.open_settings_dialog(),
            MenuAction::ShowAbout => self.show_about = true,
        }
        if !matches!(action, MenuAction::None) {
            self.input_mode = InputMode::Normal;
        }
    }

    fn execute_terminal_tool(&mut self, name: &str, args_json: &str) -> Option<String> {
        match name {
            "terminal_open" => {
                let args: serde_json::Value =
                    serde_json::from_str(args_json).unwrap_or(serde_json::json!({}));
                let command = args["command"].as_str().unwrap_or("");
                let result = self.terminal_state.open(command);
                let cursor = self.terminal_state.core.lock().unwrap().raw.total_bytes_written;
                Some(
                    serde_json::json!({
                        "status": result,
                        "cursor": cursor,
                    })
                    .to_string(),
                )
            }
            "terminal_send" => {
                let args: serde_json::Value =
                    serde_json::from_str(args_json).unwrap_or(serde_json::json!({}));
                let input = args["input"].as_str().unwrap_or("");
                Some(self.terminal_state.send_input(input))
            }
            "terminal_read" => {
                let args: serde_json::Value =
                    serde_json::from_str(args_json).unwrap_or(serde_json::json!({}));
                let cursor = args["cursor"].as_u64();
                let result = self.terminal_state.read_buffer_incremental(cursor);
                Some(result.to_string())
            }
            "terminal_close" => Some(self.terminal_state.close()),
            _ => None,
        }
    }

    pub fn scroll_up(&mut self) {
        self.auto_scroll = false;
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    pub fn scroll_down(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_add(1);
    }

    pub fn page_up(&mut self) {
        self.auto_scroll = false;
        let page = self.terminal_height.saturating_sub(4);
        self.scroll_offset = self.scroll_offset.saturating_sub(page);
    }

    pub fn page_down(&mut self) {
        let page = self.terminal_height.saturating_sub(4);
        self.scroll_offset = self.scroll_offset.saturating_add(page);
    }

    pub fn home(&mut self) {
        self.auto_scroll = false;
        self.scroll_offset = 0;
    }

    pub fn end(&mut self) {
        self.auto_scroll = true;
    }

    pub fn set_auto_scroll(&mut self) {
        self.auto_scroll = true;
        self.scroll_offset = u16::MAX;
    }

    pub fn check_responses(&mut self) {
        if let Some(rx) = &self.response_rx {
            loop {
                match rx.try_recv() {
                    Ok(StreamChunk::Text(text)) => {
                        self.streaming_text.push_str(&text);
                        self.retrying = false;
                        self.auto_scroll = true;
                    }
                    Ok(StreamChunk::Thinking(text)) => {
                        self.streaming_thinking.push_str(&text);
                        self.retrying = false;
                        self.auto_scroll = true;
                    }
                    Ok(StreamChunk::StatusTick(msg)) => {
                        self.status_message = msg;
                        self.retrying = true;
                        self.auto_scroll = true;
                    }
                    Ok(StreamChunk::Stats(stats)) => {
                        // Mid-stream usage report; committed on the next
                        // Done/ToolCalls transition.
                        self.last_stats = stats;
                    }
                    Ok(StreamChunk::Truncated(msg)) => {
                        self.status_message = msg.clone();
                        self.retrying = false;
                        self.pending_continuation = true;
                        // Commit any partial thinking
                        if !self.streaming_thinking.is_empty() {
                            self.messages
                                .push(ChatMessage::Thinking(self.streaming_thinking.clone()));
                            self.streaming_thinking.clear();
                        }
                        // Commit partial assistant text (may be incomplete)
                        if !self.streaming_text.is_empty() {
                            self.messages
                                .push(ChatMessage::Assistant(self.streaming_text.clone()));
                            self.log_event("ASSISTANT", &self.streaming_text);
                            self.streaming_text.clear();
                        }
                        // Discard partial tool-call map (arguments JSON may be incomplete)
                        self.pending_tool_calls.clear();
                        // Push visible warning — Done handler will set_auto_scroll
                        self.messages.push(ChatMessage::App(format!(
                            "⚠ {} Increase max_output_tokens in cloud_models.conf or model_params.conf.",
                            msg
                        )));
                    }
                    Ok(StreamChunk::ToolCalls(tool_calls)) => {
                        if !self.streaming_thinking.is_empty() {
                            self.messages
                                .push(ChatMessage::Thinking(self.streaming_thinking.clone()));
                            self.streaming_thinking.clear();
                        }
                        if !self.streaming_text.is_empty() {
                            self.messages
                                .push(ChatMessage::Assistant(self.streaming_text.clone()));
                            self.log_event("ASSISTANT", &self.streaming_text);
                            self.streaming_text.clear();
                        }
                        let mut consecutive_unknown = 0;
                        for tc in &tool_calls {
                            let name = tc["function"]["name"]
                                .as_str()
                                .unwrap_or("unknown")
                                .to_string();
                            let args = if tc["function"]["arguments"].is_string() {
                                tc["function"]["arguments"].as_str().unwrap().to_string()
                            } else {
                                tc["function"]["arguments"].to_string()
                            };

                            if name == "unknown" || name == "call_unknown" {
                                consecutive_unknown += 1;
                                if consecutive_unknown >= 3 {
                                    break;
                                }
                            } else {
                                consecutive_unknown = 0;
                            }

                            let tool_call_id = Some(
                                tc["id"]
                                    .as_str()
                                    .map(|s| s.to_string())
                                    .unwrap_or_else(|| format!("call_{}", &name)),
                            );

                            self.messages.push(ChatMessage::ToolCall {
                                name: name.clone(),
                                arguments: args.clone(),
                                tool_call_id: tool_call_id.clone(),
                            });

                            let result = self
                                .execute_terminal_tool(&name, &args)
                                .unwrap_or_else(|| execute_tool_call(&name, &args, &self.proxy));
                            self.tool_call_count += 1;
                            self.tool_call_log
                                .push((name.clone(), args.clone(), result.clone()));
                            self.log_event(
                                "TOOL_CALL",
                                &format!("{}({}) -> {}", name, args, truncate(&result, 500)),
                            );

                            self.messages.push(ChatMessage::ToolResult {
                                name,
                                content: result,
                                tool_call_id,
                            });
                        }
                        self.pending_tool_calls = tool_calls;
                        self.is_loading = false;
                        // The finish chunk carried usage (sent as Stats): commit it.
                        // Token info is rendered in the status bar's own token slot,
                        // never in the message slot.
                        if self.last_stats.prompt_tokens > 0 || self.last_stats.response_tokens > 0
                        {
                            self.token_stats = self.last_stats.clone();
                        }
                        self.response_rx = None;
                        self.tool_round_count += 1;
                        if self.tool_round_count >= self.max_tool_rounds {
                            self.messages.push(ChatMessage::App(format!(
                                "Reached max tool rounds ({}/{} rounds, {} tool calls). Stopping.",
                                self.tool_round_count, self.max_tool_rounds, self.tool_call_count
                            )));
                            self.set_auto_scroll();
                            break;
                        }
                        self.send_tool_results_async();
                        break;
                    }
                    Ok(StreamChunk::Done(stats)) => {
                        self.retrying = false;
                        // A bare `[DONE]` sentinel or stream close reports default
                        // stats; keep the ones received earlier via Stats.
                        self.token_stats = if stats.prompt_tokens > 0 || stats.response_tokens > 0 {
                            stats
                        } else {
                            self.last_stats.clone()
                        };

                        // If Truncated already committed the partial text, handle continuation
                        if self.pending_continuation {
                            self.pending_continuation = false;
                            self.is_loading = false;
                            // Token info belongs to the status bar's token slot only.
                            self.status_message.clear();
                            self.response_rx = None;
                            if self.agentic_mode && self.continuation_count < 3 {
                                self.continuation_count += 1;
                                self.messages.push(ChatMessage::App(format!(
                                    "Auto-continuing (attempt {}/3)...",
                                    self.continuation_count
                                )));
                                self.set_auto_scroll();
                                self.send_to_ollama_async_with(Some(
                                    "Your previous response was truncated by the output token limit. Continue exactly from where you stopped.".to_string()
                                ));
                                break;
                            }
                            self.continuation_count = 0;
                            self.set_auto_scroll();
                            break;
                        }

                        if !self.streaming_thinking.is_empty() {
                            self.messages
                                .push(ChatMessage::Thinking(self.streaming_thinking.clone()));
                            self.streaming_thinking.clear();
                        }
                        if !self.streaming_text.is_empty() {
                            let text = self.streaming_text.clone();
                            let parsed_tool_calls = parse_text_tool_calls(&text);
                            if self.agentic_mode && !parsed_tool_calls.is_empty() {
                                for tc in &parsed_tool_calls {
                                    let name = tc["name"].as_str().unwrap_or("unknown").to_string();
                                    let args = tc["parameters"].to_string();
                                    let tc_id = format!("call_{}", &name);
                                    self.messages.push(ChatMessage::ToolCall {
                                        name: name.clone(),
                                        arguments: args.clone(),
                                        tool_call_id: Some(tc_id.clone()),
                                    });
                                    let result =
                                        self.execute_terminal_tool(&name, &args).unwrap_or_else(
                                            || execute_tool_call(&name, &args, &self.proxy),
                                        );
                                    self.tool_call_count += 1;
                                    self.tool_call_log.push((
                                        name.clone(),
                                        args.clone(),
                                        result.clone(),
                                    ));
                                    self.log_event(
                                        "TOOL_CALL",
                                        &format!(
                                            "{}({}) -> {}",
                                            name,
                                            args,
                                            truncate(&result, 500)
                                        ),
                                    );
                                    self.messages.push(ChatMessage::ToolResult {
                                        name,
                                        content: result,
                                        tool_call_id: Some(tc_id),
                                    });
                                }
                                self.streaming_thinking.clear();
                                self.streaming_text.clear();
                                self.is_loading = false;
                                // Token info belongs to the status bar's token slot only.
                                self.status_message.clear();
                                self.response_rx = None;
                                self.tool_round_count += 1;
                                if self.tool_round_count >= self.max_tool_rounds {
                                    self.messages.push(ChatMessage::App(
                                        format!("Reached max tool rounds ({}/{} rounds, {} tool calls). Stopping.", self.tool_round_count, self.max_tool_rounds, self.tool_call_count),
                                    ));
                                    self.set_auto_scroll();
                                    break;
                                }
                                self.send_tool_results_async();
                                break;
                            }
                            self.log_event("ASSISTANT", &text);
                            // Kimi-K3 sometimes sends finish_reason="stop" right after a
                            // transitional sentence ("Let me run the full suite:") instead
                            // of the tool call it announced. The loop only continues on
                            // tool calls, so the run stalled until the user typed
                            // "continue" — detect the unfinished-looking text and send it
                            // automatically.
                            let unfinished = self.agentic_mode && needs_continuation(&text);
                            let wants_more =
                                unfinished && self.auto_continue_count < MAX_AUTO_CONTINUES;
                            self.messages.push(ChatMessage::Assistant(text));
                            self.streaming_thinking.clear();
                            self.streaming_text.clear();
                            if wants_more {
                                self.auto_continue_count += 1;
                                self.log_event(
                                    "AUTO_CONTINUE",
                                    &format!(
                                        "attempt {}/{}",
                                        self.auto_continue_count, MAX_AUTO_CONTINUES
                                    ),
                                );
                                self.messages.push(ChatMessage::App(format!(
                                    "Model stopped mid-thought — auto-sending \"continue\" ({}/{})...",
                                    self.auto_continue_count, MAX_AUTO_CONTINUES
                                )));
                                self.set_auto_scroll();
                                self.send_to_ollama_async_with(Some("continue".to_string()));
                                break;
                            }
                            if unfinished {
                                self.messages.push(ChatMessage::App(format!(
                                    "Auto-continue limit ({}) reached — type \"continue\" to keep going.",
                                    MAX_AUTO_CONTINUES
                                )));
                            }
                        } else {
                            self.messages.push(ChatMessage::App(
                                "Error: Empty response from model".to_string(),
                            ));
                        }
                        self.is_loading = false;
                        // Token info belongs to the status bar's token slot only.
                        self.status_message.clear();
                        self.response_rx = None;
                        self.set_auto_scroll();
                        break;
                    }
                    Ok(StreamChunk::Error(msg)) => {
                        self.retrying = false;
                        let had_content =
                            !self.streaming_thinking.is_empty() || !self.streaming_text.is_empty();
                        if !self.streaming_thinking.is_empty() {
                            self.messages
                                .push(ChatMessage::Thinking(self.streaming_thinking.clone()));
                            self.streaming_thinking.clear();
                        }
                        if !self.streaming_text.is_empty() {
                            self.messages
                                .push(ChatMessage::Assistant(self.streaming_text.clone()));
                            self.streaming_text.clear();
                        }
                        if had_content {
                            self.messages.push(ChatMessage::App(
                                "⚠ Stream ended unexpectedly (partial response shown)".to_string(),
                            ));
                        } else {
                            self.messages.push(ChatMessage::App(msg));
                        }
                        self.is_loading = false;
                        self.status_message.clear();
                        self.response_rx = None;
                        self.set_auto_scroll();
                        break;
                    }
                    Ok(StreamChunk::RetryPaused(msg)) => {
                        self.retrying = false;
                        let had_content =
                            !self.streaming_thinking.is_empty() || !self.streaming_text.is_empty();
                        if !self.streaming_thinking.is_empty() {
                            self.messages
                                .push(ChatMessage::Thinking(self.streaming_thinking.clone()));
                            self.streaming_thinking.clear();
                        }
                        if !self.streaming_text.is_empty() {
                            self.messages
                                .push(ChatMessage::Assistant(self.streaming_text.clone()));
                            self.streaming_text.clear();
                        }
                        if had_content {
                            self.messages.push(ChatMessage::App(
                                "⚠ Stream ended unexpectedly (partial response shown)".to_string(),
                            ));
                        }
                        self.is_loading = false;
                        self.status_message.clear();
                        self.response_rx = None;
                        self.show_retry_paused = true;
                        self.retry_paused_message = msg;
                        self.set_auto_scroll();
                        break;
                    }
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        if !self.streaming_thinking.is_empty() {
                            self.messages
                                .push(ChatMessage::Thinking(self.streaming_thinking.clone()));
                            self.streaming_thinking.clear();
                        }
                        if !self.streaming_text.is_empty() {
                            self.messages
                                .push(ChatMessage::Assistant(self.streaming_text.clone()));
                            self.streaming_text.clear();
                        } else {
                            self.messages.push(ChatMessage::App(
                                "Error: Connection to background task lost.".to_string(),
                            ));
                        }
                        self.is_loading = false;
                        self.status_message.clear();
                        self.response_rx = None;
                        break;
                    }
                }
            }
        }
    }

    fn send_tool_results_async(&mut self) {
        self.is_loading = true;
        self.streaming_text.clear();
        self.streaming_thinking.clear();
        self.status_message = "Processing tool results...".to_string();

        let model = self.model_name.clone();
        let is_cloud = self.cloud_models.iter().any(|m| m.name == model);

        let mut api_messages: Vec<serde_json::Value> = Vec::new();

        let agentic_sys = "You are a coding assistant with access to tools. When the user asks you to do something, use the available tools to accomplish the task. Always use tools when needed - do not just describe what you would do. Execute the actual tool calls. After using a tool, continue working until the task is complete.";

        let sys_content = if !self.system_prompt.is_empty() {
            self.system_prompt.clone()
        } else {
            agentic_sys.to_string()
        };

        if !sys_content.is_empty() {
            api_messages.push(serde_json::json!({
                "role": "system",
                "content": sys_content,
            }));
        }

        for m in &self.messages {
            match m {
                ChatMessage::User(t) => {
                    api_messages.push(serde_json::json!({"role": "user", "content": t}));
                }
                ChatMessage::Assistant(t) => {
                    api_messages.push(serde_json::json!({"role": "assistant", "content": t}));
                }
                ChatMessage::System(_) => {}
                ChatMessage::App(_) => {}
                ChatMessage::Thinking(t) => {
                    api_messages.push(serde_json::json!({
                        "role": "system",
                        "content": format!("Your previous reasoning (interrupted): {}", t),
                    }));
                }
                ChatMessage::FileContent { name, content } => {
                    api_messages.push(serde_json::json!({
                        "role": "user",
                        "content": format!("Here is the content of `{}`:\n\n{}", name, content)
                    }));
                }
                ChatMessage::ToolCall {
                    name,
                    arguments,
                    tool_call_id,
                } => {
                    let args_value: serde_json::Value = if is_cloud {
                        serde_json::json!(arguments)
                    } else {
                        serde_json::from_str(arguments).unwrap_or(serde_json::json!({}))
                    };
                    let mut tc = serde_json::json!({
                        "function": {
                            "name": name,
                            "arguments": args_value
                        }
                    });
                    if is_cloud {
                        if let Some(id) = tool_call_id {
                            tc["id"] = serde_json::json!(id);
                        }
                        tc["type"] = serde_json::json!("function");
                    }
                    api_messages.push(serde_json::json!({
                        "role": "assistant",
                        "content": "",
                        "tool_calls": [tc],
                    }));
                }
                ChatMessage::ToolResult {
                    name,
                    content,
                    tool_call_id,
                } => {
                    let mut msg = serde_json::json!({
                        "role": "tool",
                        "content": content,
                    });
                    if is_cloud {
                        let id = tool_call_id.clone().unwrap_or_else(|| name.to_string());
                        msg["tool_call_id"] = serde_json::json!(id);
                    } else {
                        msg["name"] = serde_json::json!(name);
                    }
                    api_messages.push(msg);
                }
            }
        }

        let url = self.ollama_url.clone();
        let model = self.model_name.clone();
        let cloud_model = self.cloud_models.iter().find(|m| m.name == model).cloned();
        let is_logging = self.is_logging;
        let log_file = self.log_file.clone();
        let session_id = self.session_id.clone();
        let params = self.params.clone();
        let proxy = if cloud_model.is_some() {
            self.proxy.clone()
        } else {
            None
        };
        let max_retries = self.max_retries;
        let agentic = self.agentic_mode;

        self.log_event(
            "PROMPT",
            &serde_json::to_string_pretty(&api_messages).unwrap_or_default(),
        );

        let rx = stream_chat_request(
            api_messages,
            agentic,
            model,
            url,
            cloud_model,
            is_logging,
            log_file,
            session_id,
            params,
            proxy,
            max_retries,
        );
        self.response_rx = Some(rx);
    }

    fn send_to_ollama_async(&mut self) {
        self.send_to_ollama_async_with(None);
    }

    fn send_to_ollama_async_with(&mut self, override_prompt: Option<String>) {
        let prompt = if let Some(ref p) = override_prompt {
            p.clone()
        } else {
            let p = self.textarea.lines().join("\n").trim().to_string();
            self.textarea = TextArea::default();
            p
        };

        // Reset continuation tracking on fresh user input (not auto-continuations)
        if !prompt.is_empty() && override_prompt.is_none() {
            self.continuation_count = 0;
            self.pending_continuation = false;
            self.auto_continue_count = 0;
        }

        if let Some(result) = self.handle_slash_command(&prompt) {
            if !result.is_empty() {
                self.messages.push(ChatMessage::App(result));
            }
            self.set_auto_scroll();
            return;
        }

        if !prompt.is_empty() {
            self.messages.push(ChatMessage::User(prompt));
        }
        self.tool_round_count = 0;
        self.tool_call_count = 0;
        self.is_loading = true;
        self.retrying = false;
        self.streaming_text.clear();
        self.streaming_thinking.clear();
        self.status_message = "Streaming response...".to_string();
        self.set_auto_scroll();

        let model = self.model_name.clone();
        let is_cloud = self.cloud_models.iter().any(|m| m.name == model);

        let mut api_messages: Vec<serde_json::Value> = Vec::new();

        let agentic_sys = "You are a coding assistant with access to tools. When the user asks you to do something, use the available tools to accomplish the task. Always use tools when needed - do not just describe what you would do. Execute the actual tool calls. After using a tool, continue working until the task is complete.";

        let sys_content = if !self.system_prompt.is_empty() {
            self.system_prompt.clone()
        } else {
            agentic_sys.to_string()
        };

        if !sys_content.is_empty() {
            api_messages.push(serde_json::json!({
                "role": "system",
                "content": sys_content,
            }));
        }

        for m in &self.messages {
            match m {
                ChatMessage::User(t) => {
                    api_messages.push(serde_json::json!({"role": "user", "content": t}));
                }
                ChatMessage::Assistant(t) => {
                    api_messages.push(serde_json::json!({"role": "assistant", "content": t}));
                }
                ChatMessage::System(_) => {}
                ChatMessage::App(_) => {}
                ChatMessage::Thinking(t) => {
                    api_messages.push(serde_json::json!({
                        "role": "system",
                        "content": format!("Your previous reasoning (interrupted): {}", t),
                    }));
                }
                ChatMessage::FileContent { name, content } => {
                    api_messages.push(serde_json::json!({
                        "role": "user",
                        "content": format!("Here is the content of `{}`:\n\n{}", name, content)
                    }));
                }
                ChatMessage::ToolCall {
                    name,
                    arguments,
                    tool_call_id,
                } => {
                    let args_value: serde_json::Value = if is_cloud {
                        serde_json::json!(arguments)
                    } else {
                        serde_json::from_str(arguments).unwrap_or(serde_json::json!({}))
                    };
                    let mut tc = serde_json::json!({
                        "function": {
                            "name": name,
                            "arguments": args_value
                        }
                    });
                    if is_cloud {
                        if let Some(id) = tool_call_id {
                            tc["id"] = serde_json::json!(id);
                        }
                        tc["type"] = serde_json::json!("function");
                    }
                    api_messages.push(serde_json::json!({
                        "role": "assistant",
                        "content": "",
                        "tool_calls": [tc],
                    }));
                }
                ChatMessage::ToolResult {
                    name,
                    content,
                    tool_call_id,
                } => {
                    let mut msg = serde_json::json!({
                        "role": "tool",
                        "content": content,
                    });
                    if is_cloud {
                        let id = tool_call_id.clone().unwrap_or_else(|| name.to_string());
                        msg["tool_call_id"] = serde_json::json!(id);
                    } else {
                        msg["name"] = serde_json::json!(name);
                    }
                    api_messages.push(msg);
                }
            }
        }

        let url = self.ollama_url.clone();
        let model = self.model_name.clone();
        let agentic = self.agentic_mode;
        let cloud_model = self.cloud_models.iter().find(|m| m.name == model).cloned();
        let is_logging = self.is_logging;
        let log_file = self.log_file.clone();
        let session_id = self.session_id.clone();
        let params = self.params.clone();
        let proxy = if cloud_model.is_some() {
            self.proxy.clone()
        } else {
            None
        };
        let max_retries = self.max_retries;

        self.log_event(
            "PROMPT",
            &serde_json::to_string_pretty(&api_messages).unwrap_or_default(),
        );

        let rx = stream_chat_request(
            api_messages,
            agentic,
            model,
            url,
            cloud_model,
            is_logging,
            log_file,
            session_id,
            params,
            proxy,
            max_retries,
        );
        self.response_rx = Some(rx);
    }

    pub fn export_output(&mut self) {
        let is_md = self.export_format == ExportFormat::Markdown;
        let mut content = String::new();
        for msg in &self.messages {
            match msg {
                ChatMessage::User(t) => {
                    if is_md {
                        content.push_str(&format!("**User:** {}\n\n", t));
                    } else {
                        content.push_str(&format!("User: {}\n\n", t));
                    }
                }
                ChatMessage::Assistant(t) => {
                    if is_md {
                        content.push_str(&format!("**Assistant:** {}\n\n", t));
                    } else {
                        content.push_str(&format!("Assistant: {}\n\n", t));
                    }
                }
                ChatMessage::System(t) => {
                    if is_md {
                        content.push_str(&format!("_{}_\n\n", t));
                    } else {
                        content.push_str(&format!("System: {}\n\n", t));
                    }
                }
                ChatMessage::App(t) => {
                    content.push_str(&format!("{}\n\n", t));
                }
                ChatMessage::Thinking(t) => {
                    if is_md {
                        content.push_str(&format!("_Thinking:_ {}\n\n", t));
                    } else {
                        content.push_str(&format!("Thinking: {}\n\n", t));
                    }
                }
                ChatMessage::FileContent {
                    name,
                    content: file_content,
                } => {
                    if is_md {
                        content.push_str(&format!("**File:** `{}`\n\n{}\n\n", name, file_content));
                    } else {
                        content.push_str(&format!("File: {}\n{}\n\n", name, file_content));
                    }
                }
                ChatMessage::ToolCall {
                    name, arguments, ..
                } => {
                    if is_md {
                        content.push_str(&format!(
                            "**Tool Call:** `{}`\n```\n{}\n```\n\n",
                            name, arguments
                        ));
                    } else {
                        content.push_str(&format!("Tool Call: {}\n{}\n\n", name, arguments));
                    }
                }
                ChatMessage::ToolResult {
                    name,
                    content: result,
                    ..
                } => {
                    if is_md {
                        content.push_str(&format!(
                            "**Tool Result:** `{}`\n```\n{}\n```\n\n",
                            name, result
                        ));
                    } else {
                        content.push_str(&format!("Tool Result: {}\n{}\n\n", name, result));
                    }
                }
            }
        }
        if !self.streaming_text.is_empty() {
            if is_md {
                content.push_str(&format!(
                    "**Assistant:** {} _(streaming in progress)_\n\n",
                    self.streaming_text
                ));
            } else {
                content.push_str(&format!(
                    "Assistant: {} (streaming in progress)\n\n",
                    self.streaming_text
                ));
            }
        }
        match std::fs::write(&self.save_path, &content) {
            Ok(()) => self.status_message = format!("Exported to {}", self.save_path),
            Err(e) => self.status_message = format!("Export failed: {}", e),
        }
    }

    fn open_export_dialog(&mut self) {
        self.show_save_dialog = true;
        self.save_dialog_path = self.save_path.clone();
        self.save_dialog_cursor = self.save_dialog_path.chars().count();
        self.save_dialog_focus = SaveDialogFocus::Path;
        self.save_dialog_mode = SaveDialogMode::ExportChat;
    }

    fn open_save_dialog(&mut self) {
        let default_path = sessions_dir()
            .join(format!("{}.session.rustama", self.session_name))
            .to_string_lossy()
            .to_string();
        self.show_save_dialog = true;
        self.save_dialog_path = default_path;
        self.save_dialog_cursor = self.save_dialog_path.chars().count();
        self.save_dialog_focus = SaveDialogFocus::Path;
        self.save_dialog_mode = SaveDialogMode::SaveSession;
    }

    fn execute_export(&mut self) {
        let path = self.save_dialog_path.trim().to_string();
        if path.is_empty() {
            self.status_message = "Export path is empty".to_string();
            return;
        }
        match self.save_dialog_mode {
            SaveDialogMode::ExportChat => {
                self.save_path = path;
                self.export_output();
            }
            SaveDialogMode::SaveSession => {
                let data = serde_json::json!({
                    "session_id": self.session_id,
                    "model": self.model_name,
                    "messages": self.messages,
                });
                match serde_json::to_string_pretty(&data) {
                    Ok(json) => match std::fs::write(&path, &json) {
                        Ok(()) => self.status_message = format!("Session saved to {}", path),
                        Err(e) => self.status_message = format!("Save failed: {}", e),
                    },
                    Err(e) => self.status_message = format!("Save failed: {}", e),
                }
            }
        }
        self.show_save_dialog = false;
    }

    /// Saves the global config (proxy, urls, retries, ...) and the current
    /// model's generation parameters to their per-model config section
    /// (`cloud_models.conf` for cloud models, `model_params.conf` otherwise).
    pub fn save_config(&self) -> Result<(), String> {
        let cfg = Config {
            ollama_url: self.ollama_url.clone(),
            model: self.model_name.clone(),
            save_path: self.save_path.clone(),
            agentic: self.agentic_mode,
            logging: self.is_logging,
            logfile: self.log_file.clone(),
            system_prompt: self.system_prompt.clone(),
            proxy: self.proxy.clone(),
            max_tool_rounds: self.max_tool_rounds,
            max_retries: self.max_retries,
            terminal_width_pct: self.terminal_state.width_pct,
            ..Config::default()
        };
        cfg.save()?;
        let is_cloud = self.cloud_models.iter().any(|m| m.name == self.model_name);
        save_model_params(&self.model_name, &self.params, is_cloud)
    }

    pub fn save_session(&self) -> Result<(), String> {
        let sessions_dir = sessions_dir();
        std::fs::create_dir_all(&sessions_dir).map_err(|e| e.to_string())?;
        let path = sessions_dir.join(format!("{}.session.rustama", self.session_name));
        let data = serde_json::json!({
            "session_id": self.session_id,
            "model": self.model_name,
            "messages": self.messages,
        });
        let json = serde_json::to_string_pretty(&data).map_err(|e| e.to_string())?;
        std::fs::write(&path, &json).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn load_session(&mut self, sess_id: &str) -> Result<(), String> {
        let sessions_dir = sessions_dir();
        let path = sessions_dir.join(format!("{}.session.rustama", sess_id));
        let json =
            std::fs::read_to_string(&path).map_err(|e| format!("Session not found: {}", e))?;
        let data: serde_json::Value = serde_json::from_str(&json).map_err(|e| e.to_string())?;
        let mut msgs: Vec<ChatMessage> = serde_json::from_value(data["messages"].clone())
            .map_err(|e| format!("Invalid session data: {}", e))?;
        patch_tool_call_ids(&mut msgs);
        self.messages = msgs;
        self.streaming_text.clear();
        self.session_name = sess_id.to_string();
        if let Some(model) = data["model"].as_str() {
            self.model_name = model.to_string();
            self.apply_model_params();
        }
        self.status_message = format!("Loaded session {}", sess_id);
        Ok(())
    }

    fn open_load_session_dialog(&mut self) {
        self.file_dialog_mode = FileDialogMode::LoadSession;
        self.file_dialog_path = sessions_dir();
        self.file_dialog_entries.clear();
        self.file_dialog_selection = 0;
        self.file_dialog_scroll = 0;
        self.file_dialog_focus = FileDialogFocus::List;
        self.refresh_file_list();
        self.show_file_dialog = true;
    }

    fn execute_load_session(&mut self) {
        let path = self.load_dialog_path.trim().to_string();
        if path.is_empty() {
            self.status_message = "Session path is empty".to_string();
            return;
        }
        let json = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(e) => {
                self.status_message = format!("Failed to read session: {}", e);
                return;
            }
        };
        let data: serde_json::Value = match serde_json::from_str(&json) {
            Ok(v) => v,
            Err(e) => {
                self.status_message = format!("Invalid session file: {}", e);
                return;
            }
        };
        let mut msgs: Vec<ChatMessage> = match serde_json::from_value(data["messages"].clone()) {
            Ok(m) => m,
            Err(e) => {
                self.status_message = format!("Invalid session data: {}", e);
                return;
            }
        };
        patch_tool_call_ids(&mut msgs);
        self.messages = msgs;
        self.streaming_text.clear();
        if let Some(model) = data["model"].as_str() {
            self.model_name = model.to_string();
            self.apply_model_params();
        }
        if let Some(sid) = data["session_id"].as_str() {
            self.status_message = format!("Loaded session {}", sid);
        } else {
            self.status_message = "Session loaded".to_string();
        }
        self.show_load_dialog = false;
    }

    fn handle_load_dialog_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.show_load_dialog = false;
            }
            KeyCode::Enter => self.execute_load_session(),
            KeyCode::Left => {
                self.load_dialog_cursor = self.load_dialog_cursor.saturating_sub(1);
            }
            KeyCode::Right => {
                if self.load_dialog_cursor < self.load_dialog_path.chars().count() {
                    self.load_dialog_cursor += 1;
                }
            }
            KeyCode::Home => self.load_dialog_cursor = 0,
            KeyCode::End => self.load_dialog_cursor = self.load_dialog_path.chars().count(),
            KeyCode::Char(c) => {
                let mut chars: Vec<char> = self.load_dialog_path.chars().collect();
                let idx = self.load_dialog_cursor.min(chars.len());
                chars.insert(idx, c);
                self.load_dialog_path = chars.into_iter().collect();
                self.load_dialog_cursor += 1;
            }
            KeyCode::Backspace => {
                if self.load_dialog_cursor > 0 {
                    let mut chars: Vec<char> = self.load_dialog_path.chars().collect();
                    let idx = self
                        .load_dialog_cursor
                        .saturating_sub(1)
                        .min(chars.len().saturating_sub(1));
                    if idx < chars.len() {
                        chars.remove(idx);
                        self.load_dialog_path = chars.into_iter().collect();
                        self.load_dialog_cursor -= 1;
                    }
                }
            }
            KeyCode::Delete => {
                let mut chars: Vec<char> = self.load_dialog_path.chars().collect();
                if self.load_dialog_cursor < chars.len() {
                    chars.remove(self.load_dialog_cursor);
                    self.load_dialog_path = chars.into_iter().collect();
                }
            }
            _ => {}
        }
    }

    fn handle_save_dialog_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.show_format_dropdown = false;
                self.show_save_dialog = false;
            }
            KeyCode::Tab => {
                self.show_format_dropdown = false;
                self.save_dialog_focus = match self.save_dialog_focus {
                    SaveDialogFocus::Path => SaveDialogFocus::Format,
                    SaveDialogFocus::Format => SaveDialogFocus::Save,
                    SaveDialogFocus::Save => SaveDialogFocus::Cancel,
                    SaveDialogFocus::Cancel => SaveDialogFocus::Path,
                };
            }
            KeyCode::Left => {
                if self.save_dialog_focus == SaveDialogFocus::Path {
                    self.save_dialog_cursor = self.save_dialog_cursor.saturating_sub(1);
                } else if self.save_dialog_focus == SaveDialogFocus::Format {
                    if self.show_format_dropdown {
                        let idx = self.export_format as isize - 1;
                        if idx >= 0 {
                            self.export_format = match idx {
                                0 => ExportFormat::Markdown,
                                _ => ExportFormat::PlainText,
                            };
                        }
                    }
                } else if self.save_dialog_focus == SaveDialogFocus::Cancel {
                    self.save_dialog_focus = SaveDialogFocus::Save;
                }
            }
            KeyCode::Right => {
                if self.save_dialog_focus == SaveDialogFocus::Path {
                    if self.save_dialog_cursor < self.save_dialog_path.chars().count() {
                        self.save_dialog_cursor += 1;
                    }
                } else if self.save_dialog_focus == SaveDialogFocus::Format {
                    if self.show_format_dropdown {
                        let idx = self.export_format as isize + 1;
                        if idx < 2 {
                            self.export_format = match idx {
                                0 => ExportFormat::Markdown,
                                _ => ExportFormat::PlainText,
                            };
                        }
                    }
                } else if self.save_dialog_focus == SaveDialogFocus::Save {
                    self.save_dialog_focus = SaveDialogFocus::Cancel;
                }
            }
            KeyCode::Up => {
                if self.save_dialog_focus == SaveDialogFocus::Format && self.show_format_dropdown {
                    self.export_format = ExportFormat::Markdown;
                }
            }
            KeyCode::Down => {
                if self.save_dialog_focus == SaveDialogFocus::Format && self.show_format_dropdown {
                    self.export_format = ExportFormat::PlainText;
                }
            }
            KeyCode::Home => {
                if self.save_dialog_focus == SaveDialogFocus::Path {
                    self.save_dialog_cursor = 0;
                }
            }
            KeyCode::End => {
                if self.save_dialog_focus == SaveDialogFocus::Path {
                    self.save_dialog_cursor = self.save_dialog_path.chars().count();
                }
            }
            KeyCode::Enter | KeyCode::Char(' ') => match self.save_dialog_focus {
                SaveDialogFocus::Format => {
                    self.show_format_dropdown = !self.show_format_dropdown;
                }
                SaveDialogFocus::Path => self.save_dialog_focus = SaveDialogFocus::Format,
                SaveDialogFocus::Save => self.execute_export(),
                SaveDialogFocus::Cancel => {
                    self.show_format_dropdown = false;
                    self.show_save_dialog = false;
                }
            },
            KeyCode::Char(c) if self.save_dialog_focus == SaveDialogFocus::Path => {
                let byte_idx = self
                    .save_dialog_path
                    .char_indices()
                    .nth(self.save_dialog_cursor)
                    .map_or(self.save_dialog_path.len(), |(i, _)| i);
                self.save_dialog_path.insert(byte_idx, c);
                self.save_dialog_cursor += 1;
            }
            KeyCode::Backspace if self.save_dialog_focus == SaveDialogFocus::Path => {
                if self.save_dialog_cursor > 0 {
                    self.save_dialog_cursor -= 1;
                    let byte_idx = self
                        .save_dialog_path
                        .char_indices()
                        .nth(self.save_dialog_cursor)
                        .map_or(self.save_dialog_path.len(), |(i, _)| i);
                    self.save_dialog_path.remove(byte_idx);
                }
            }
            KeyCode::Delete if self.save_dialog_focus == SaveDialogFocus::Path => {
                if let Some((byte_idx, _)) = self
                    .save_dialog_path
                    .char_indices()
                    .nth(self.save_dialog_cursor)
                {
                    self.save_dialog_path.remove(byte_idx);
                }
            }
            _ => {}
        }
    }

    fn handle_save_dialog_click(&mut self, col: u16, row: u16, width: u16, height: u16) {
        let area = Rect {
            x: 0,
            y: 0,
            width,
            height,
        };

        let dlg = match self.save_dialog_mode {
            SaveDialogMode::SaveSession => {
                let mut d = FileActionDialog::new("Save Session");
                d.add_text_input(
                    "Session file:",
                    &self.save_dialog_path,
                    self.save_dialog_cursor,
                    self.save_dialog_focus == SaveDialogFocus::Path,
                );
                d.add_button("Cancel", self.save_dialog_focus == SaveDialogFocus::Cancel);
                d.add_button("Save", self.save_dialog_focus == SaveDialogFocus::Save);
                d
            }
            SaveDialogMode::ExportChat => {
                let mut d = FileActionDialog::new("Export As");
                d.add_text_input(
                    "File path:",
                    &self.save_dialog_path,
                    self.save_dialog_cursor,
                    self.save_dialog_focus == SaveDialogFocus::Path,
                );
                let fmt_state = DialogDropdownState {
                    focused: self.save_dialog_focus == SaveDialogFocus::Format,
                    expanded: self.show_format_dropdown,
                    selected: if self.export_format == ExportFormat::Markdown {
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
                d.add_button("Cancel", self.save_dialog_focus == SaveDialogFocus::Cancel);
                d.add_button("Export", self.save_dialog_focus == SaveDialogFocus::Save);
                d
            }
        };

        match dlg.hit_test(col, row, area) {
            DialogHit::Outside => {
                self.show_format_dropdown = false;
            }
            DialogHit::TextInput => {
                self.save_dialog_focus = SaveDialogFocus::Path;
            }
            DialogHit::DropdownItem(_idx, item_idx) => {
                self.save_dialog_focus = SaveDialogFocus::Format;
                if let Some(i) = item_idx {
                    self.export_format = match i {
                        0 => ExportFormat::Markdown,
                        _ => ExportFormat::PlainText,
                    };
                    self.show_format_dropdown = false;
                } else {
                    self.show_format_dropdown = !self.show_format_dropdown;
                }
            }
            DialogHit::Button(bi) => {
                let action_idx = 1;
                if bi == action_idx {
                    self.execute_export();
                } else {
                    self.show_format_dropdown = false;
                    self.show_save_dialog = false;
                }
            }
            DialogHit::FileListItem(_) | DialogHit::None => {}
        }
    }

    pub fn handle_click(&mut self, col: u16, row: u16, width: u16, height: u16) {
        if self.show_retry_paused {
            let area = Rect::new(0, 0, width, height);
            let mb = crate::ui::MessageBox::new("Retry Paused", &self.retry_paused_message);
            if mb.hit_test(col, row, area) {
                self.show_retry_paused = false;
            }
            return;
        }
        if self.show_save_dialog {
            self.handle_save_dialog_click(col, row, width, height);
            return;
        }
        if self.show_settings_dialog {
            self.handle_settings_dialog_click(col, row, width, height);
            return;
        }
        if self.show_file_dialog {
            self.handle_file_dialog_click(col, row, width, height);
            return;
        }
        if self.show_model_dialog {
            self.handle_model_dialog_click(col, row, width, height);
            return;
        }
        if self.show_quit_confirm {
            self.handle_quit_confirm_click(col, row, width, height);
            return;
        }
        if self.show_about {
            self.handle_about_dialog_click(col, row, width, height);
            return;
        }

        if row == 0 || self.main_menu.is_open() {
            let action = self.main_menu.handle_click(col, row);
            match action {
                MenuAction::None => {
                    if row == 0 || !self.main_menu.is_open() {
                        // Menu opened or closed, update input mode
                        if self.main_menu.is_open() {
                            self.input_mode = InputMode::Menu;
                        } else {
                            self.input_mode = InputMode::Normal;
                        }
                    }
                }
                _ => {
                    self.handle_menu_action(action);
                }
            }
            return;
        }

        let scrollbar_col = self.output_width.saturating_sub(1);
        let input_start = height.saturating_sub(6);
        let input_bottom = height.saturating_sub(2);

        // Clicks in the terminal panel grab terminal keyboard focus.
        let terminal_visible = self.terminal_state.visible && self.terminal_state.is_running();
        if terminal_visible {
            let term_start_col = width
                .saturating_mul(100 - self.terminal_state.width_pct)
                / 100;
            if col >= term_start_col && row > 0 {
                self.focus = Focus::Terminal;
                self.status_message =
                    "Terminal focused — keystrokes go to the shell. Ctrl+G: release".to_string();
                return;
            }
        }

        if row >= input_start && row <= input_bottom {
            if row == input_bottom && !self.textarea.lines().join("").trim().is_empty() {
                self.send_to_ollama_async();
                return;
            }
            self.input_mode = InputMode::Input;
            self.focus = Focus::Input;
        } else if row > 0 && row < input_start {
            if col == scrollbar_col {
                self.scrollbar_dragging = true;
                self.scrollbar_click_to(row, input_start);
            } else {
                self.focus = Focus::Output;
                self.input_mode = InputMode::Normal;
                self.auto_scroll = false;
                if let Some(idx) = self.output_line_at_row(row) {
                    self.selection_start = Some(idx);
                    self.selection_end = Some(idx);
                    self.selecting = true;
                }
            }
        }
    }

    fn output_line_at_row(&self, row: u16) -> Option<usize> {
        if row < 2 {
            return None;
        }
        let idx = self.scroll_offset as usize + (row - 2) as usize;
        if idx < self.cached_wrapped.len() {
            Some(idx)
        } else {
            None
        }
    }

    pub fn handle_mouse_drag(&mut self, row: u16, input_start: u16) {
        if self.selecting {
            self.auto_scroll = false;
            if let Some(idx) = self.output_line_at_row(row) {
                self.selection_end = Some(idx);
            }
        } else {
            self.scrollbar_drag_to(row, input_start);
        }
    }

    pub fn handle_mouse_up(&mut self) {
        if self.selecting {
            self.selecting = false;
            if let Some(text) = self.selected_text()
                && !text.trim().is_empty()
            {
                self.primary_selection.set_text(text.clone());
                let s = self.selection_start.unwrap_or(0);
                let e = self.selection_end.unwrap_or(0);
                let count = if s <= e { e - s + 1 } else { s - e + 1 };
                self.status_message = format!("Selected {} line(s) copied to X selection", count);
            }
        }
        self.scrollbar_drag_end();
    }

    fn selected_text(&self) -> Option<String> {
        let start = self.selection_start?;
        let end = self.selection_end?;
        let (start, end) = if start <= end {
            (start, end)
        } else {
            (end, start)
        };
        if start >= self.cached_wrapped.len() {
            return None;
        }
        let end = end.min(self.cached_wrapped.len() - 1);
        let mut out = String::new();
        for line in &self.cached_wrapped[start..=end] {
            let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
            out.push_str(text.trim_end());
            out.push('\n');
        }
        Some(out)
    }

    /// Maps a mouse row on the scrollbar track to a scroll offset. Thin
    /// wrapper over [`scrollbar_scroll_offset`] — see it for the geometry.
    fn scrollbar_click_to(&mut self, row: u16, input_start: u16) {
        self.auto_scroll = false;
        if let Some(offset) = scrollbar_scroll_offset(row, input_start, self.cached_wrapped.len()) {
            self.scroll_offset = offset;
        }
    }

    pub fn scrollbar_drag_to(&mut self, row: u16, input_start: u16) {
        if !self.scrollbar_dragging {
            return;
        }
        self.scrollbar_click_to(row, input_start);
    }

    pub fn scrollbar_drag_end(&mut self) {
        self.scrollbar_dragging = false;
        self.scrollbar_drag_start = None;
    }

    fn open_model_dialog(&mut self) {
        self.show_model_dialog = true;
        self.model_dialog_selection = 0;
        self.model_dialog_focus = ModelDialogFocus::List;
        self.available_models.clear();
        self.fetch_models_async();
    }

    fn fetch_models_async(&mut self) {
        let url = self.ollama_url.clone();
        let (tx, rx) = mpsc::channel();
        self.models_rx = Some(rx);

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            rt.block_on(async {
                let client = build_reqwest_client(&None);
                let result = match client
                    .get(format!("{}/api/tags", url))
                    .timeout(std::time::Duration::from_secs(10))
                    .send()
                    .await
                {
                    Ok(resp) => match resp.json::<serde_json::Value>().await {
                        Ok(json) => {
                            let models: Vec<String> = json["models"]
                                .as_array()
                                .map(|arr| {
                                    arr.iter()
                                        .filter_map(|m| m["name"].as_str().map(String::from))
                                        .collect()
                                })
                                .unwrap_or_default();
                            Ok(models)
                        }
                        Err(e) => Err(format!("Failed to parse models: {}", e)),
                    },
                    Err(e) => Err(format!("Failed to connect to Ollama: {}", e)),
                };
                let _ = tx.send(result);
            });
        });
    }

    pub fn check_model_responses(&mut self) {
        if let Some(rx) = &self.models_rx {
            match rx.try_recv() {
                Ok(Ok(models)) => {
                    self.available_models = models;
                    // Pre-select the current model if it's in the list
                    if let Some(idx) = self
                        .available_models
                        .iter()
                        .position(|m| m == &self.model_name)
                    {
                        self.model_dialog_selection = idx;
                    }
                    self.models_rx = None;
                }
                Ok(Err(e)) => {
                    self.available_models = vec![format!("Error: {}", e)];
                    self.models_rx = None;
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.available_models = vec!["Failed to fetch models".to_string()];
                    self.models_rx = None;
                }
            }
        }
    }

    fn handle_model_dialog_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.show_model_dialog = false;
            }
            KeyCode::Tab => {
                self.model_dialog_focus = match self.model_dialog_focus {
                    ModelDialogFocus::List => ModelDialogFocus::Confirm,
                    ModelDialogFocus::Confirm => ModelDialogFocus::Cancel,
                    ModelDialogFocus::Cancel => ModelDialogFocus::List,
                };
            }
            KeyCode::Left => {
                if self.model_dialog_focus == ModelDialogFocus::Cancel {
                    self.model_dialog_focus = ModelDialogFocus::Confirm;
                }
            }
            KeyCode::Right => {
                if self.model_dialog_focus == ModelDialogFocus::Confirm {
                    self.model_dialog_focus = ModelDialogFocus::Cancel;
                }
            }
            KeyCode::Up => {
                if self.model_dialog_focus == ModelDialogFocus::List
                    && !self.available_models.is_empty()
                {
                    self.model_dialog_selection = self.model_dialog_selection.saturating_sub(1);
                }
            }
            KeyCode::Down => {
                if self.model_dialog_focus == ModelDialogFocus::List {
                    let max = self.available_models.len();
                    if max > 0 && self.model_dialog_selection < max - 1 {
                        self.model_dialog_selection += 1;
                    }
                }
            }
            KeyCode::Enter => match self.model_dialog_focus {
                ModelDialogFocus::List => {
                    if !self.available_models.is_empty() {
                        self.model_dialog_focus = ModelDialogFocus::Confirm;
                    }
                }
                ModelDialogFocus::Confirm => self.confirm_model_selection(),
                ModelDialogFocus::Cancel => self.show_model_dialog = false,
            },
            _ => {}
        }
    }

    /// Reloads `self.params` from the per-model configuration after the
    /// current model changed (model dialog, /model, session load, ...).
    fn apply_model_params(&mut self) {
        self.params = params_for_model(&self.model_name, &self.cloud_models, &self.model_params_store);
    }

    fn confirm_model_selection(&mut self) {
        if let Some(model) = self.available_models.get(self.model_dialog_selection) {
            self.model_name = model.clone();
            self.apply_model_params();
            self.status_message = format!("Model set to: {}", self.model_name);
        }
        self.show_model_dialog = false;
    }

    fn handle_model_dialog_click(&mut self, col: u16, row: u16, width: u16, height: u16) {
        let dialog_w: u16 = 56;
        let model_count = self.available_models.len().min(12) as u16;
        let dialog_h = model_count + 7;
        let dialog_x = (width.saturating_sub(dialog_w)) / 2;
        let dialog_y = (height.saturating_sub(dialog_h)) / 2;
        let inner_x = dialog_x + 1;
        let inner_y = dialog_y + 1;

        if col < dialog_x
            || col >= dialog_x + dialog_w
            || row < dialog_y
            || row >= dialog_y + dialog_h
        {
            self.show_model_dialog = false;
            return;
        }

        if row >= inner_y && row < inner_y + model_count {
            let idx = (row - inner_y) as usize;
            if idx < self.available_models.len() {
                self.model_dialog_selection = idx;
                self.model_dialog_focus = ModelDialogFocus::List;
            }
            return;
        }

        let btn_y = inner_y + model_count + 1;
        let confirm_btn = Button::new(
            "Confirm",
            inner_x + 10,
            btn_y,
            self.model_dialog_focus == ModelDialogFocus::Confirm,
            Color::Green,
            Color::Green,
        );
        let cancel_btn = Button::new(
            "Cancel",
            inner_x + 22,
            btn_y,
            self.model_dialog_focus == ModelDialogFocus::Cancel,
            Color::Red,
            Color::Red,
        );

        if confirm_btn.is_clicked(col, row) {
            self.model_dialog_focus = ModelDialogFocus::Confirm;
            self.confirm_model_selection();
            return;
        }
        if cancel_btn.is_clicked(col, row) {
            self.show_model_dialog = false;
        }
    }

    fn open_settings_dialog(&mut self) {
        self.show_settings_dialog = true;
        self.settings_focus = SettingsFocus::Proxy;
        self.settings_cursor = 0;
        self.settings_proxy = self.proxy.clone().unwrap_or_default();
        self.settings_ollama_url = self.ollama_url.clone();
        self.settings_temperature = format!("{}", self.params.temperature);
        self.settings_top_p = format!("{}", self.params.top_p);
        self.settings_top_k = format!("{}", self.params.top_k);
        self.settings_frequency_penalty = format!("{}", self.params.frequency_penalty);
        self.settings_presence_penalty = format!("{}", self.params.presence_penalty);
        self.settings_max_tokens = self
            .params
            .max_output_tokens
            .map(|n| n.to_string())
            .unwrap_or_default();
        self.settings_reasoning_effort =
            self.params.reasoning_effort.clone().unwrap_or_default();
        self.settings_max_tool_rounds = self.max_tool_rounds.to_string();
        self.settings_justify = self.justify;
    }

    fn handle_settings_dialog_key(&mut self, key: KeyEvent) {
        let is_text_field = self.settings_focus.is_text_field();
        let is_toggle = self.settings_focus == SettingsFocus::Justify;

        match key.code {
            KeyCode::Esc => {
                self.show_settings_dialog = false;
            }
            KeyCode::Tab => {
                self.settings_cursor = 0;
                self.settings_focus = self.settings_focus.next();
            }
            KeyCode::BackTab => {
                self.settings_cursor = 0;
                self.settings_focus = self.settings_focus.prev();
            }
            KeyCode::Up => {
                self.settings_cursor = 0;
                self.settings_focus = self.settings_focus.prev();
            }
            KeyCode::Down => {
                self.settings_cursor = 0;
                self.settings_focus = self.settings_focus.next();
            }
            KeyCode::Left => {
                if is_text_field {
                    self.settings_cursor = self.settings_cursor.saturating_sub(1);
                } else if is_toggle {
                    self.settings_justify = !self.settings_justify;
                } else if self.settings_focus == SettingsFocus::Save {
                    self.settings_focus = SettingsFocus::Cancel;
                } else if self.settings_focus == SettingsFocus::Cancel {
                    self.settings_focus = SettingsFocus::Save;
                }
            }
            KeyCode::Right => {
                if is_text_field {
                    let field = self.settings_field_text();
                    if self.settings_cursor < field.chars().count() {
                        self.settings_cursor += 1;
                    }
                } else if is_toggle {
                    self.settings_justify = !self.settings_justify;
                } else if self.settings_focus == SettingsFocus::Save {
                    self.settings_focus = SettingsFocus::Cancel;
                } else if self.settings_focus == SettingsFocus::Cancel {
                    self.settings_focus = SettingsFocus::Save;
                }
            }
            KeyCode::Home => {
                if is_text_field {
                    self.settings_cursor = 0;
                }
            }
            KeyCode::End => {
                if is_text_field {
                    let field = self.settings_field_text();
                    self.settings_cursor = field.chars().count();
                }
            }
            KeyCode::Char(c) if is_text_field => {
                let cursor = self.settings_cursor;
                let field = self.settings_field_mut();
                let byte_idx = field
                    .char_indices()
                    .nth(cursor)
                    .map_or(field.len(), |(i, _)| i);
                field.insert(byte_idx, c);
                self.settings_cursor = cursor + 1;
            }
            KeyCode::Backspace if is_text_field => {
                if self.settings_cursor > 0 {
                    self.settings_cursor -= 1;
                    let cursor = self.settings_cursor;
                    let field = self.settings_field_mut();
                    let byte_idx = field
                        .char_indices()
                        .nth(cursor)
                        .map_or(field.len(), |(i, _)| i);
                    field.remove(byte_idx);
                }
            }
            KeyCode::Delete if is_text_field => {
                let cursor = self.settings_cursor;
                let field_text = self.settings_field_text();
                if cursor < field_text.chars().count() {
                    let field = self.settings_field_mut();
                    let byte_idx = field
                        .char_indices()
                        .nth(cursor)
                        .map_or(field.len(), |(i, _)| i);
                    field.remove(byte_idx);
                }
            }
            KeyCode::Enter | KeyCode::Char(' ') if !is_text_field => {
                if is_toggle {
                    self.settings_justify = !self.settings_justify;
                } else {
                    match self.settings_focus {
                        SettingsFocus::Save => self.confirm_settings(),
                        SettingsFocus::Cancel => self.show_settings_dialog = false,
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    fn settings_field_text(&self) -> &str {
        match self.settings_focus {
            SettingsFocus::Proxy => &self.settings_proxy,
            SettingsFocus::OllamaUrl => &self.settings_ollama_url,
            SettingsFocus::Temperature => &self.settings_temperature,
            SettingsFocus::TopP => &self.settings_top_p,
            SettingsFocus::TopK => &self.settings_top_k,
            SettingsFocus::FrequencyPenalty => &self.settings_frequency_penalty,
            SettingsFocus::PresencePenalty => &self.settings_presence_penalty,
            SettingsFocus::MaxTokens => &self.settings_max_tokens,
            SettingsFocus::ReasoningEffort => &self.settings_reasoning_effort,
            SettingsFocus::MaxToolRounds => &self.settings_max_tool_rounds,
            SettingsFocus::MaxRetries => &self.settings_max_retries,
            _ => "",
        }
    }

    fn settings_field_mut(&mut self) -> &mut String {
        match self.settings_focus {
            SettingsFocus::Proxy => &mut self.settings_proxy,
            SettingsFocus::OllamaUrl => &mut self.settings_ollama_url,
            SettingsFocus::Temperature => &mut self.settings_temperature,
            SettingsFocus::TopP => &mut self.settings_top_p,
            SettingsFocus::TopK => &mut self.settings_top_k,
            SettingsFocus::FrequencyPenalty => &mut self.settings_frequency_penalty,
            SettingsFocus::PresencePenalty => &mut self.settings_presence_penalty,
            SettingsFocus::MaxTokens => &mut self.settings_max_tokens,
            SettingsFocus::ReasoningEffort => &mut self.settings_reasoning_effort,
            SettingsFocus::MaxToolRounds => &mut self.settings_max_tool_rounds,
            SettingsFocus::MaxRetries => &mut self.settings_max_retries,
            _ => unreachable!(),
        }
    }

    fn confirm_settings(&mut self) {
        let proxy_val = self.settings_proxy.trim().to_string();
        self.proxy = if proxy_val.is_empty() || proxy_val.eq_ignore_ascii_case("off") {
            None
        } else {
            Some(proxy_val)
        };
        self.ollama_url = self.settings_ollama_url.trim().to_string();
        self.params
            .apply_key("temperature", self.settings_temperature.trim());
        self.params.apply_key("top_p", self.settings_top_p.trim());
        self.params.apply_key("top_k", self.settings_top_k.trim());
        self.params
            .apply_key("frequency_penalty", self.settings_frequency_penalty.trim());
        self.params
            .apply_key("presence_penalty", self.settings_presence_penalty.trim());
        self.params
            .apply_key("max_output_tokens", self.settings_max_tokens.trim());
        self.params
            .apply_key("reasoning_effort", self.settings_reasoning_effort.trim());
        if let Ok(v) = self.settings_max_tool_rounds.trim().parse::<usize>()
            && (1..=100).contains(&v)
        {
            self.max_tool_rounds = v;
        }
        if let Ok(v) = self.settings_max_retries.trim().parse::<u32>()
            && (1..=50).contains(&v)
        {
            self.max_retries = v;
        }
        self.justify = self.settings_justify;
        let _ = self.save_config();
        self.show_settings_dialog = false;
        self.status_message = "Settings saved".to_string();
    }

    fn handle_settings_dialog_click(&mut self, col: u16, row: u16, width: u16, height: u16) {
        let dialog_w: u16 = 60;
        let dialog_h: u16 = 28;
        let dialog_x = (width.saturating_sub(dialog_w)) / 2;
        let dialog_y = (height.saturating_sub(dialog_h)) / 2;
        let inner_x = dialog_x + 2;
        let inner_y = dialog_y + 1;
        let inner_w = dialog_w.saturating_sub(4);

        if col < dialog_x
            || col >= dialog_x + dialog_w
            || row < dialog_y
            || row >= dialog_y + dialog_h
        {
            self.show_settings_dialog = false;
            return;
        }

        let fields = [
            (SettingsFocus::Proxy, "Proxy URL:"),
            (SettingsFocus::OllamaUrl, "Ollama URL:"),
            (SettingsFocus::Temperature, "Temperature:"),
            (SettingsFocus::TopP, "Top-P:"),
            (SettingsFocus::TopK, "Top-K:"),
            (SettingsFocus::FrequencyPenalty, "Freq Penalty:"),
            (SettingsFocus::PresencePenalty, "Pres Penalty:"),
            (SettingsFocus::MaxTokens, "Max Tokens:"),
            (SettingsFocus::ReasoningEffort, "Effort:"),
            (SettingsFocus::MaxToolRounds, "Max Rounds:"),
            (SettingsFocus::MaxRetries, "Max Retries:"),
            (SettingsFocus::Justify, "Justify:"),
        ];

        for (i, (focus, _label)) in fields.iter().enumerate() {
            let field_y = inner_y + i as u16 * 2;
            let max_w = inner_w.saturating_sub(16);
            if row == field_y {
                if *focus == SettingsFocus::Justify {
                    self.settings_focus = SettingsFocus::Justify;
                    self.settings_justify = !self.settings_justify;
                    return;
                } else if col >= inner_x + 14 && col < inner_x + 14 + max_w + 2 {
                    self.settings_focus = *focus;
                    return;
                }
            }
        }

        let num_fields = 12u16;
        let btn_y = inner_y + num_fields * 2 + 1;
        let save_label = "Save";
        let cancel_label = "Cancel";
        let save_w = save_label.len() as u16 + 4;
        let cancel_w = cancel_label.len() as u16 + 4;
        let gap: u16 = 4;
        let total_btn_w = save_w + gap + cancel_w;
        let btn_start_x = inner_x + (inner_w.saturating_sub(total_btn_w)) / 2;

        if row == btn_y {
            if col >= btn_start_x && col < btn_start_x + save_w {
                self.settings_focus = SettingsFocus::Save;
                self.confirm_settings();
                return;
            }
            if col >= btn_start_x + save_w + gap && col < btn_start_x + total_btn_w {
                self.show_settings_dialog = false;
            }
        }
    }

    fn handle_about_dialog_click(&mut self, col: u16, row: u16, width: u16, height: u16) {
        let area = Rect::new(0, 0, width, height);
        let mb = crate::ui::MessageBox::new("About", &self.about_message);
        if mb.hit_test(col, row, area) {
            self.show_about = false;
        }
    }

    fn open_quit_confirm(&mut self) {
        self.show_quit_confirm = true;
        // Default to the safe choice.
        self.quit_confirm_focus = crate::ui::ConfirmFocus::No;
    }

    fn handle_quit_confirm_key(&mut self, key: KeyEvent) {
        use crate::ui::ConfirmFocus;
        match key.code {
            KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
                self.show_quit_confirm = false;
            }
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                self.terminal_state.close();
                self.should_quit = true;
            }
            KeyCode::Enter => match self.quit_confirm_focus {
                ConfirmFocus::Yes => {
                    self.terminal_state.close();
                    self.should_quit = true;
                }
                ConfirmFocus::No => {
                    self.show_quit_confirm = false;
                }
            },
            KeyCode::Left | KeyCode::Right | KeyCode::Tab | KeyCode::BackTab => {
                self.quit_confirm_focus = match self.quit_confirm_focus {
                    ConfirmFocus::Yes => ConfirmFocus::No,
                    ConfirmFocus::No => ConfirmFocus::Yes,
                };
            }
            _ => {}
        }
    }

    fn handle_about_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') | KeyCode::Char('Q') => {
                self.show_about = false;
            }
            _ => {}
        }
    }

    fn handle_quit_confirm_click(&mut self, col: u16, row: u16, width: u16, height: u16) {
        use crate::ui::ConfirmHit;
        let area = Rect::new(0, 0, width, height);
        let cb = crate::ui::ConfirmationBox::new("Confirm Quit", "Are you sure you want to quit?");
        match cb.hit_test(col, row, area) {
            ConfirmHit::Yes => {
                self.terminal_state.close();
                self.should_quit = true;
            }
            ConfirmHit::No | ConfirmHit::Outside => {
                self.show_quit_confirm = false;
            }
            ConfirmHit::None => {}
        }
    }

    fn refresh_file_list(&mut self) {
        self.file_dialog_entries.clear();
        self.file_dialog_selection = 0;
        self.file_dialog_scroll = 0;

        if let Ok(entries) = std::fs::read_dir(&self.file_dialog_path) {
            let mut dirs: Vec<(String, bool)> = Vec::new();
            let mut files: Vec<(String, bool)> = Vec::new();

            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with('.') {
                    continue;
                }
                let is_dir = entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
                if self.file_dialog_mode == FileDialogMode::LoadSession
                    && !is_dir
                    && !name.ends_with(".session.rustama")
                {
                    continue;
                }
                if is_dir {
                    dirs.push((name, true));
                } else {
                    files.push((name, false));
                }
            }

            dirs.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
            files.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));

            if self.file_dialog_path.parent().is_some() {
                self.file_dialog_entries.push(("..".to_string(), true));
            }

            self.file_dialog_entries.extend(dirs);
            self.file_dialog_entries.extend(files);
        }
    }

    fn navigate_into(&mut self, name: &str) {
        if name == ".." {
            if let Some(parent) = self.file_dialog_path.parent() {
                self.file_dialog_path = parent.to_path_buf();
            }
        } else {
            self.file_dialog_path.push(name);
        }
        self.refresh_file_list();
    }

    fn open_selected_file(&mut self) {
        if let Some((name, is_dir)) = self
            .file_dialog_entries
            .get(self.file_dialog_selection)
            .cloned()
        {
            if is_dir {
                self.navigate_into(&name);
                return;
            }

            let mut path = self.file_dialog_path.clone();
            path.push(&name);

            if self.file_dialog_mode == FileDialogMode::LoadSession {
                let json = match std::fs::read_to_string(&path) {
                    Ok(s) => s,
                    Err(e) => {
                        self.status_message = format!("Failed to read session: {}", e);
                        return;
                    }
                };
                let data: serde_json::Value = match serde_json::from_str(&json) {
                    Ok(v) => v,
                    Err(e) => {
                        self.status_message = format!("Invalid session file: {}", e);
                        return;
                    }
                };
                let mut msgs: Vec<ChatMessage> =
                    match serde_json::from_value(data["messages"].clone()) {
                        Ok(m) => m,
                        Err(e) => {
                            self.status_message = format!("Invalid session data: {}", e);
                            return;
                        }
                    };
                patch_tool_call_ids(&mut msgs);
                self.messages = msgs;
                self.streaming_text.clear();
                if let Some(model) = data["model"].as_str() {
                    self.model_name = model.to_string();
                    self.apply_model_params();
                }
                let display_name = name.strip_suffix(".session.rustama").unwrap_or(&name);
                self.session_name = display_name.to_string();
                if let Some(sid) = data["session_id"].as_str() {
                    self.status_message = format!("Loaded session {} ({})", sid, display_name);
                } else {
                    self.status_message = format!("Session loaded as {}", display_name);
                }
                self.file_dialog_mode = FileDialogMode::AttachFile;
                self.show_file_dialog = false;
                return;
            }

            match std::fs::read_to_string(&path) {
                Ok(content) => {
                    let display_path = path.display().to_string();
                    self.messages.push(ChatMessage::FileContent {
                        name: display_path,
                        content,
                    });
                    self.set_auto_scroll();
                    self.status_message = format!("Loaded: {}", name);
                }
                Err(e) => {
                    self.status_message = format!("Failed to load {}: {}", name, e);
                }
            }
        }
        self.show_file_dialog = false;
    }

    fn handle_file_dialog_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.show_file_dialog = false;
            }
            KeyCode::Tab => {
                self.file_dialog_focus = match self.file_dialog_focus {
                    FileDialogFocus::List => FileDialogFocus::Open,
                    FileDialogFocus::Open => FileDialogFocus::Cancel,
                    FileDialogFocus::Cancel => FileDialogFocus::List,
                };
            }
            KeyCode::Left => {
                if self.file_dialog_focus == FileDialogFocus::Open {
                    self.file_dialog_focus = FileDialogFocus::Cancel;
                }
            }
            KeyCode::Right => {
                if self.file_dialog_focus == FileDialogFocus::Cancel {
                    self.file_dialog_focus = FileDialogFocus::Open;
                }
            }
            KeyCode::Up => {
                if self.file_dialog_focus == FileDialogFocus::List
                    && !self.file_dialog_entries.is_empty()
                {
                    self.file_dialog_selection = self.file_dialog_selection.saturating_sub(1);
                    self.adjust_scroll();
                }
            }
            KeyCode::Down => {
                if self.file_dialog_focus == FileDialogFocus::List {
                    let max = self.file_dialog_entries.len();
                    if max > 0 && self.file_dialog_selection < max - 1 {
                        self.file_dialog_selection += 1;
                        self.adjust_scroll();
                    }
                }
            }
            KeyCode::Enter => match self.file_dialog_focus {
                FileDialogFocus::List => {
                    if !self.file_dialog_entries.is_empty() {
                        let is_dir = self
                            .file_dialog_entries
                            .get(self.file_dialog_selection)
                            .map(|(_, d)| *d)
                            .unwrap_or(false);
                        if is_dir {
                            let name = self
                                .file_dialog_entries
                                .get(self.file_dialog_selection)
                                .map(|(n, _)| n.clone())
                                .unwrap_or_default();
                            self.navigate_into(&name);
                        } else {
                            self.file_dialog_focus = FileDialogFocus::Open;
                        }
                    }
                }
                FileDialogFocus::Open => self.open_selected_file(),
                FileDialogFocus::Cancel => self.show_file_dialog = false,
            },
            _ => {}
        }
    }

    fn adjust_scroll(&mut self) {
        let visible = 12;
        if self.file_dialog_selection < self.file_dialog_scroll {
            self.file_dialog_scroll = self.file_dialog_selection;
        }
        if self.file_dialog_selection >= self.file_dialog_scroll + visible {
            self.file_dialog_scroll = self.file_dialog_selection - visible + 1;
        }
    }

    /// Builds the FileActionDialog for the current file dialog state.
    /// Shared by rendering (main.rs) and mouse hit testing so the two
    /// always describe the same layout.
    pub fn build_file_action_dialog(&self, area: Rect) -> FileActionDialog {
        let is_load_session = self.file_dialog_mode == FileDialogMode::LoadSession;
        let title = if is_load_session {
            "Load Session"
        } else {
            "Load File"
        };
        let mut d = FileActionDialog::new(title);

        // Show the current directory, truncated from the left if too long.
        let (dialog_w, _) = FileActionDialog::dialog_size(area);
        let max_path_chars = dialog_w.saturating_sub(2) as usize;
        let path_display = self.file_dialog_path.display().to_string();
        let path_len = path_display.chars().count();
        let path_str = if path_len > max_path_chars && max_path_chars > 3 {
            let tail: String = path_display
                .chars()
                .skip(path_len - (max_path_chars - 3))
                .collect();
            format!("...{}", tail)
        } else {
            path_display
        };
        d.add_label_colored(&path_str, self.theme.path_fg);

        d.add_file_list(
            self.file_dialog_entries.clone(),
            self.file_dialog_selection,
            self.file_dialog_scroll,
            self.file_dialog_focus == FileDialogFocus::List,
        );

        // Buttons render right-aligned in insertion order: [Cancel]  [Load/Open]
        d.add_button("Cancel", self.file_dialog_focus == FileDialogFocus::Cancel);
        let open_label = if is_load_session { "Load" } else { "Open" };
        d.add_button(open_label, self.file_dialog_focus == FileDialogFocus::Open);
        d
    }

    fn handle_file_dialog_click(&mut self, col: u16, row: u16, width: u16, height: u16) {
        let area = Rect::new(0, 0, width, height);
        let dlg = self.build_file_action_dialog(area);
        match dlg.hit_test(col, row, area) {
            DialogHit::Outside => {
                self.show_file_dialog = false;
            }
            DialogHit::FileListItem(idx) => {
                self.file_dialog_selection = idx;
                self.file_dialog_focus = FileDialogFocus::List;
            }
            DialogHit::Button(bi) => {
                if bi == 0 {
                    self.show_file_dialog = false;
                } else {
                    self.file_dialog_focus = FileDialogFocus::Open;
                    self.open_selected_file();
                }
            }
            _ => {}
        }
    }

    fn handle_slash_command(&mut self, input: &str) -> Option<String> {
        if !input.starts_with('/') {
            return None;
        }
        let parts: Vec<&str> = input[1..].splitn(2, ' ').collect();
        let cmd = parts[0];
        let arg = parts.get(1).map(|s| s.trim()).filter(|s| !s.is_empty());
        match cmd {
            "help" => Some(self.slash_help()),
            "model" | "use" => Some(self.slash_model(arg)),
            "list" => Some(self.slash_list()),
            "tools" => Some(self.slash_tools()),
            "config" => Some(self.slash_config()),
            "log" => Some(self.slash_log()),
            "maxrounds" => Some(match arg {
                Some(n) => match n.parse::<usize>() {
                    Ok(v) if (1..=100).contains(&v) => {
                        self.max_tool_rounds = v;
                        format!("Max tool rounds set to {}", v)
                    }
                    Ok(_) => "Value must be between 1 and 100".to_string(),
                    Err(_) => "Usage: /maxrounds <number>".to_string(),
                },
                None => format!(
                    "Current max tool rounds: {} (usage: /maxrounds <number>)",
                    self.max_tool_rounds
                ),
            }),
            "temp" => Some(match arg {
                Some(n) => match n.parse::<f64>() {
                    Ok(v) if (0.0..=2.0).contains(&v) => {
                        self.params.temperature = v;
                        format!("Temperature set to {}", v)
                    }
                    Ok(_) => "Value must be between 0.0 and 2.0".to_string(),
                    Err(_) => "Usage: /temp <number>".to_string(),
                },
                None => format!(
                    "Current temperature: {} (usage: /temp <number>)",
                    self.params.temperature
                ),
            }),
            "topp" => Some(match arg {
                Some(n) => match n.parse::<f64>() {
                    Ok(v) if (0.0..=1.0).contains(&v) => {
                        self.params.top_p = v;
                        format!("Top_p set to {}", v)
                    }
                    Ok(_) => "Value must be between 0.0 and 1.0".to_string(),
                    Err(_) => "Usage: /topp <number>".to_string(),
                },
                None => format!(
                    "Current top_p: {} (usage: /topp <number>)",
                    self.params.top_p
                ),
            }),
            "topk" => Some(match arg {
                Some(n) => match n.parse::<u32>() {
                    Ok(v) if (1..=100).contains(&v) => {
                        self.params.top_k = v;
                        format!("Top_k set to {}", v)
                    }
                    Ok(_) => "Value must be between 1 and 100".to_string(),
                    Err(_) => "Usage: /topk <number>".to_string(),
                },
                None => format!(
                    "Current top_k: {} (usage: /topk <number>)",
                    self.params.top_k
                ),
            }),
            "fpen" => Some(match arg {
                Some(n) => match n.parse::<f64>() {
                    Ok(v) if (-2.0..=2.0).contains(&v) => {
                        self.params.frequency_penalty = v;
                        format!("Frequency penalty set to {}", v)
                    }
                    Ok(_) => "Value must be between -2.0 and 2.0".to_string(),
                    Err(_) => "Usage: /fpen <number>".to_string(),
                },
                None => format!(
                    "Current frequency_penalty: {} (usage: /fpen <number>)",
                    self.params.frequency_penalty
                ),
            }),
            "ppen" => Some(match arg {
                Some(n) => match n.parse::<f64>() {
                    Ok(v) if (-2.0..=2.0).contains(&v) => {
                        self.params.presence_penalty = v;
                        format!("Presence penalty set to {}", v)
                    }
                    Ok(_) => "Value must be between -2.0 and 2.0".to_string(),
                    Err(_) => "Usage: /ppen <number>".to_string(),
                },
                None => format!(
                    "Current presence_penalty: {} (usage: /ppen <number>)",
                    self.params.presence_penalty
                ),
            }),
            "effort" => Some(match arg {
                Some(level) => {
                    let before = self.params.reasoning_effort.clone();
                    self.params.apply_key("reasoning_effort", level);
                    if self.params.reasoning_effort != before {
                        match &self.params.reasoning_effort {
                            Some(e) => format!("Reasoning effort set to {}", e),
                            None => "Reasoning effort cleared".to_string(),
                        }
                    } else {
                        format!(
                            "Invalid effort '{}'. Use low, medium, high, on, off or none.",
                            level
                        )
                    }
                }
                None => format!(
                    "Current reasoning_effort: {} (usage: /effort <low|medium|high|on|off|none>)",
                    self.params.reasoning_effort.as_deref().unwrap_or("(unset)")
                ),
            }),
            "maxtokens" => Some(match arg {
                Some(n) => {
                    let before = self.params.max_output_tokens;
                    self.params.apply_key("max_output_tokens", n);
                    if self.params.max_output_tokens != before {
                        match self.params.max_output_tokens {
                            Some(v) => format!("Max output tokens set to {}", v),
                            None => "Max output tokens cleared (provider default)".to_string(),
                        }
                    } else {
                        "Usage: /maxtokens <number> or /maxtokens off".to_string()
                    }
                }
                None => format!(
                    "Current max_output_tokens: {} (usage: /maxtokens <number> or /maxtokens off)",
                    self.params
                        .max_output_tokens
                        .map(|n| n.to_string())
                        .unwrap_or_else(|| "(unset)".to_string())
                ),
            }),
            "seed" => Some(match arg {
                Some(n) => {
                    let before = self.params.seed;
                    self.params.apply_key("seed", n);
                    if self.params.seed != before {
                        match self.params.seed {
                            Some(v) => format!("Seed set to {}", v),
                            None => "Seed cleared".to_string(),
                        }
                    } else {
                        "Usage: /seed <number> or /seed off".to_string()
                    }
                }
                None => format!(
                    "Current seed: {} (usage: /seed <number> or /seed off)",
                    self.params
                        .seed
                        .map(|n| n.to_string())
                        .unwrap_or_else(|| "(unset)".to_string())
                ),
            }),
            "proxy" => Some(match arg {
                Some("off") | Some("none") | Some("") => {
                    self.proxy = None;
                    let _ = self.save_config();
                    "Proxy disabled".to_string()
                }
                Some(url) => {
                    self.proxy = Some(url.to_string());
                    let _ = self.save_config();
                    format!("Proxy set to {}", url)
                }
                None => match &self.proxy {
                    Some(url) => {
                        format!("Current proxy: {} (usage: /proxy <url> or /proxy off)", url)
                    }
                    None => "No proxy set (usage: /proxy <url> or /proxy off)".to_string(),
                },
            }),
            "status" => Some(self.slash_status()),
            "system" => Some(self.slash_system()),
            "setsystem" => Some(self.slash_setsystem(arg)),
            "session" => Some(match arg {
                Some("save") => match self.save_session() {
                    Ok(()) => format!("Session {} saved", self.session_name),
                    Err(e) => format!("Save failed: {}", e),
                },
                Some(rename_arg) if rename_arg.starts_with("rename ") => {
                    let new_name = rename_arg[7..].trim();
                    if new_name.is_empty() {
                        "Usage: /session rename <name>".to_string()
                    } else if new_name.contains('/')
                        || new_name.contains('\\')
                        || new_name.contains("..")
                    {
                        "Invalid session name: no path separators or '..' allowed".to_string()
                    } else {
                        self.session_name = new_name.to_string();
                        format!("Session renamed to {}", new_name)
                    }
                }
                Some(load_arg) => match self.load_session(load_arg) {
                    Ok(()) => format!("Loaded session {}", load_arg),
                    Err(e) => format!("Load failed: {}", e),
                },
                _ => "Usage: /session save | /session rename <name> | /session load <name>"
                    .to_string(),
            }),
            "justify" => {
                self.justify = !self.justify;
                Some(
                    if self.justify {
                        "Justify: ON"
                    } else {
                        "Justify: OFF"
                    }
                    .to_string(),
                )
            }
            "quit" | "q" | "exit" => {
                self.open_quit_confirm();
                Some(String::new())
            }
            _ => Some(format!(
                "Unknown command: /{}. Type /help for available commands.",
                cmd
            )),
        }
    }

    fn slash_help(&self) -> String {
        let commands = [
            ("/help", "Show this help message"),
            ("/model <name>", "Set model (no arg opens dialog)"),
            ("/use <name>", "Alias for /model"),
            ("/list", "List available models"),
            ("/tools", "List agentic tools"),
            ("/config", "Show current configuration"),
            ("/system", "Show current system prompt"),
            ("/setsystem <text>", "Set system prompt (no arg clears)"),
            ("/log", "Toggle logging on/off"),
            ("/maxrounds <n>", "Max agentic tool rounds (default: 10)"),
            ("/temp <n>", "Temperature 0.0-2.0 (default: 1.0)"),
            ("/topp <n>", "Top_p 0.0-1.0 (default: 0.9)"),
            ("/topk <n>", "Top_k 1-100 (default: 40)"),
            ("/fpen <n>", "Frequency penalty -2.0-2.0 (default: 0.0)"),
            ("/ppen <n>", "Presence penalty -2.0-2.0 (default: 0.0)"),
            (
                "/effort <level>",
                "Reasoning effort low/medium/high/on/off",
            ),
            ("/maxtokens <n>", "Max output tokens (or /maxtokens off)"),
            ("/seed <n>", "Sampling seed (or /seed off)"),
            ("/proxy <url>", "Set HTTP proxy (e.g. http://proxy:8080)"),
            ("/proxy off", "Disable proxy"),
            ("/session save", "Save current session"),
            ("/session rename <name>", "Rename current session"),
            ("/session load <name>", "Load a session by name"),
            ("/justify", "Toggle paragraph justification"),
            ("/status", "Show app status"),
            ("/quit", "Exit the app"),
            ("/q", "Exit the app"),
            ("/exit", "Exit the app"),
        ];
        let mut out = String::from("Available commands:\n");
        for (name, desc) in &commands {
            out.push_str(&format!("  {:<18} {}\n", name, desc));
        }
        out
    }

    fn slash_model(&mut self, arg: Option<&str>) -> String {
        match arg {
            Some(name) => {
                if self.available_models.iter().any(|m| m == name) {
                    self.model_name = name.to_string();
                    self.apply_model_params();
                    format!("Model set to: {}", name)
                } else if self.cloud_models.iter().any(|m| m.name == name) {
                    self.model_name = name.to_string();
                    self.apply_model_params();
                    format!("Model set to: {} (cloud)", name)
                } else {
                    let mut all: Vec<&str> =
                        self.available_models.iter().map(|s| s.as_str()).collect();
                    for m in &self.cloud_models {
                        all.push(&m.name);
                    }
                    format!(
                        "Model '{}' not found. Available: {}",
                        name,
                        if all.is_empty() {
                            "none loaded yet".to_string()
                        } else {
                            all.join(", ")
                        }
                    )
                }
            }
            None => {
                let total = self.available_models.len() + self.cloud_models.len();
                if total == 0 {
                    "No models loaded yet. Wait a moment or check Ollama connection.".to_string()
                } else {
                    self.show_model_dialog = true;
                    self.model_dialog_focus = ModelDialogFocus::List;
                    self.model_dialog_selection = self
                        .available_models
                        .iter()
                        .position(|m| m == &self.model_name)
                        .or_else(|| {
                            self.cloud_models
                                .iter()
                                .position(|m| m.name == self.model_name)
                        })
                        .unwrap_or(0);
                    format!(
                        "Current model: {}. Select a new one in the dialog.",
                        self.model_name
                    )
                }
            }
        }
    }

    fn slash_list(&self) -> String {
        let ollama_count = self.available_models.len();
        let cloud_count = self.cloud_models.len();

        if ollama_count == 0 && cloud_count == 0 {
            "No models loaded yet. Wait a moment or check Ollama connection.".to_string()
        } else {
            let mut out = String::new();

            if ollama_count > 0 {
                out.push_str(&format!("Ollama models ({}):\n", ollama_count));
                for m in &self.available_models {
                    let marker = if *m == self.model_name { " *" } else { "" };
                    out.push_str(&format!("  {}{}\n", m, marker));
                }
            }

            if cloud_count > 0 {
                if ollama_count > 0 {
                    out.push('\n');
                }
                out.push_str(&format!("Cloud models ({}):\n", cloud_count));
                for m in &self.cloud_models {
                    let marker = if m.name == self.model_name { " *" } else { "" };
                    out.push_str(&format!("  {} [cloud]{}\n", m.name, marker));
                }
            }

            out.push_str("\n* = current model");
            out
        }
    }

    fn slash_tools(&self) -> String {
        let tools = get_tool_definitions();
        let mut out = format!("Agentic tools ({}):\n", tools.len());
        for t in &tools {
            let name = t["function"]["name"].as_str().unwrap_or("?");
            let desc = t["function"]["description"].as_str().unwrap_or("");
            out.push_str(&format!("  {:<16} {}\n", name, desc));
        }
        out
    }

    fn slash_config(&self) -> String {
        let home = std::env::var("HOME").unwrap_or_default();
        let rustama_dir = std::path::PathBuf::from(&home).join(".rustama");
        let conf_path = if rustama_dir.is_dir() {
            rustama_dir.join("rustama.conf")
        } else {
            std::path::PathBuf::from(&home)
                .join(".config")
                .join("rustama")
                .join("rustama.conf")
        };
        let p = &self.params;
        format!(
            "Config file: {}\n\n  ollama_url     = {}\n  model          = {}\n  save_path      = {}\n  agentic        = {}\n  max_rounds     = {}\n  timeout_secs   = {}\n  logging        = {}\n  logfile        = {}\n  justify        = {}\n\nModel parameters (per-model):\n  temperature    = {}\n  top_p          = {}\n  top_k          = {}\n  frequency_pen. = {}\n  presence_pen.  = {}\n  max_tokens     = {}\n  effort         = {}\n  seed           = {}",
            conf_path.display(),
            self.ollama_url,
            self.model_name,
            self.save_path,
            self.agentic_mode,
            self.max_tool_rounds,
            300,
            self.is_logging,
            self.log_file,
            self.justify,
            p.temperature,
            p.top_p,
            p.top_k,
            p.frequency_penalty,
            p.presence_penalty,
            p.max_output_tokens
                .map(|n| n.to_string())
                .unwrap_or_else(|| "(unset)".to_string()),
            p.reasoning_effort.as_deref().unwrap_or("(unset)"),
            p.seed
                .map(|n| n.to_string())
                .unwrap_or_else(|| "(unset)".to_string()),
        )
    }

    fn slash_log(&mut self) -> String {
        self.is_logging = !self.is_logging;
        if self.is_logging {
            format!("Logging enabled. Writing to: {}", self.log_file)
        } else {
            format!("Logging disabled. Last log file: {}", self.log_file)
        }
    }

    fn slash_status(&self) -> String {
        let model_count = self.available_models.len();
        let msg_count = self.messages.len();
        let tool_calls = self.tool_call_log.len();
        let p = &self.params;
        format!(
            "Status:\n  Session ID:    {}\n  Session Name:  {}\n  Model:         {}\n  Messages:      {}\n  Logging:       {}\n  Log file:      {}\n  Agentic mode:  {}\n  Available:     {} model(s)\n  Tool calls:    {}\n  System prompt: {}\n  Temperature:   {}\n  Top_p:         {}\n  Top_k:         {}\n  Freq penalty:  {}\n  Pres penalty:  {}\n  Max tokens:    {}\n  Effort:        {}\n  Seed:          {}\n  Justify:       {}",
            self.session_id,
            self.session_name,
            self.model_name,
            msg_count,
            if self.is_logging { "ON" } else { "OFF" },
            self.log_file,
            if self.agentic_mode { "ON" } else { "OFF" },
            model_count,
            tool_calls,
            if self.system_prompt.is_empty() {
                "(none)".to_string()
            } else {
                format!("{} chars", self.system_prompt.len())
            },
            p.temperature,
            p.top_p,
            p.top_k,
            p.frequency_penalty,
            p.presence_penalty,
            p.max_output_tokens
                .map(|n| n.to_string())
                .unwrap_or_else(|| "(unset)".to_string()),
            p.reasoning_effort.as_deref().unwrap_or("(unset)"),
            p.seed
                .map(|n| n.to_string())
                .unwrap_or_else(|| "(unset)".to_string()),
            if self.justify { "ON" } else { "OFF" },
        )
    }

    fn slash_system(&self) -> String {
        if self.system_prompt.is_empty() {
            "No system prompt set. Use /setsystem <prompt> to set one.".to_string()
        } else {
            format!("Current system prompt:\n{}", self.system_prompt)
        }
    }

    fn slash_setsystem(&mut self, arg: Option<&str>) -> String {
        match arg {
            Some(prompt) => {
                self.system_prompt = prompt.to_string();
                match Config::set_system_prompt(prompt) {
                    Ok(_) => format!("System prompt set and saved: {}", prompt),
                    Err(e) => format!("System prompt set (save failed: {}): {}", e, prompt),
                }
            }
            None => {
                self.system_prompt.clear();
                match Config::set_system_prompt("") {
                    Ok(_) => "System prompt cleared and saved.".to_string(),
                    Err(e) => format!("System prompt cleared (save failed: {}).", e),
                }
            }
        }
    }

    fn log_event(&self, kind: &str, content: &str) {
        log_to_file(
            self.is_logging,
            &self.log_file,
            &self.session_id,
            kind,
            content,
        );
    }

    pub fn format_token_stats(&self) -> String {
        let s = &self.token_stats;
        let mut parts = Vec::new();

        // Output tokens with max and percentage for cloud models
        let model = &self.model_name;
        let max_output = self
            .cloud_models
            .iter()
            .find(|m| m.name == *model)
            .and_then(|m| m.params.max_output_tokens);

        if let Some(max) = max_output {
            if max > 0 {
                let pct = s.response_tokens as f64 / max as f64 * 100.0;
                parts.push(format!("{}/{} ({:.0}%)", s.response_tokens, max, pct));
            } else {
                parts.push(format!("out:{}", s.response_tokens));
            }
        } else {
            // Ollama or unknown — just show count
            parts.push(format!("out:{}", s.response_tokens));
        }

        if s.prompt_tokens > 0 {
            parts.push(format!("prompt:{}", s.prompt_tokens));
        }
        if s.cached_tokens > 0 {
            parts.push(format!("cached:{}", s.cached_tokens));
        }
        if s.reasoning_tokens > 0 {
            parts.push(format!("reason:{}", s.reasoning_tokens));
        }
        if s.tokens_per_sec > 0.0 {
            parts.push(format!("{:.1} tok/s", s.tokens_per_sec));
        }

        if parts.is_empty() {
            return String::new();
        }
        format!("Tokens: {}", parts.join(" | "))
    }
}

fn rotate_log_file(log_file: &str) {
    use std::fs;
    let mut highest = 0;
    while std::path::Path::new(&format!("{}.{}", log_file, highest + 1)).exists() {
        highest += 1;
    }
    for i in (1..=highest).rev() {
        let from = format!("{}.{}", log_file, i);
        let to = format!("{}.{}", log_file, i + 1);
        let _ = fs::rename(&from, &to);
    }
    if std::path::Path::new(log_file).exists() {
        let _ = fs::rename(log_file, format!("{}.1", log_file));
    }
}

pub(crate) fn log_to_file(
    is_logging: bool,
    log_file: &str,
    session_id: &str,
    kind: &str,
    content: &str,
) {
    if !is_logging {
        return;
    }
    use std::fs::OpenOptions;
    use std::io::Write;
    let ts = chrono_now();
    let mut file = match OpenOptions::new().create(true).append(true).open(log_file) {
        Ok(f) => f,
        Err(_) => return,
    };
    let _ = writeln!(file, "[{} {}] {}: {}", ts, session_id, kind, content);
}

fn parse_token_stats(json: &serde_json::Value) -> TokenStats {
    let response_tokens = json["eval_count"].as_u64().unwrap_or(0);
    let eval_duration_ns = json["eval_duration"].as_u64().unwrap_or(0);
    let tokens_per_sec = if eval_duration_ns > 0 {
        response_tokens as f64 / (eval_duration_ns as f64 / 1_000_000_000.0)
    } else {
        0.0
    };
    TokenStats {
        prompt_tokens: json["prompt_eval_count"].as_u64().unwrap_or(0),
        response_tokens,
        cached_tokens: 0,
        reasoning_tokens: 0,
        total_duration_ms: json["total_duration"].as_u64().unwrap_or(0) / 1_000_000,
        tokens_per_sec,
    }
}

/// Locates the `usage` object in an OpenAI-compatible chunk.
///
/// Two layouts exist in the wild:
/// - top-level `usage` on a usage-only final chunk (OpenAI spec: `choices` is
///   empty and `usage` is a sibling of `choices`),
/// - `usage` nested inside `choices[i]` (e.g. Kimi K3 attaches it to the final
///   chunk that carries `finish_reason`).
fn usage_from_json(json: &serde_json::Value) -> Option<&serde_json::Value> {
    if !json["usage"].is_null() {
        return Some(&json["usage"]);
    }
    json["choices"].as_array().and_then(|choices| {
        choices.iter().find_map(|c| {
            if c["usage"].is_null() {
                None
            } else {
                Some(&c["usage"])
            }
        })
    })
}

fn parse_usage_stats(json: &serde_json::Value) -> TokenStats {
    let usage = match usage_from_json(json) {
        Some(u) => u,
        None => return TokenStats::default(),
    };
    TokenStats {
        prompt_tokens: usage["prompt_tokens"].as_u64().unwrap_or(0),
        response_tokens: usage["completion_tokens"].as_u64().unwrap_or(0),
        cached_tokens: usage["prompt_tokens_details"]["cached_tokens"]
            .as_u64()
            .unwrap_or(0),
        reasoning_tokens: usage["completion_tokens_details"]["reasoning_tokens"]
            .as_u64()
            .unwrap_or(0),
        total_duration_ms: 0,
        tokens_per_sec: 0.0,
    }
}

#[cfg(test)]
mod usage_tests {
    use super::*;

    #[test]
    fn usage_top_level_openai_style() {
        // OpenAI spec: usage-only final chunk, empty choices, top-level usage.
        let json = serde_json::json!({
            "id": "chatcmpl-1",
            "object": "chat.completion.chunk",
            "choices": [],
            "usage": {
                "prompt_tokens": 120,
                "completion_tokens": 42,
                "total_tokens": 162,
                "prompt_tokens_details": {"cached_tokens": 100},
                "completion_tokens_details": {"reasoning_tokens": 10}
            }
        });
        let stats = parse_usage_stats(&json);
        assert_eq!(stats.prompt_tokens, 120);
        assert_eq!(stats.response_tokens, 42);
        assert_eq!(stats.cached_tokens, 100);
        assert_eq!(stats.reasoning_tokens, 10);
    }

    #[test]
    fn usage_nested_in_choices_kimi_style() {
        // Kimi K3: usage nested inside choices[0] on the finish chunk.
        let json = serde_json::json!({
            "id": "chatcmpl-2",
            "object": "chat.completion.chunk",
            "choices": [{
                "index": 0,
                "delta": {},
                "finish_reason": "tool_calls",
                "usage": {
                    "prompt_tokens": 56525,
                    "completion_tokens": 338,
                    "total_tokens": 56863,
                    "cached_tokens": 55808,
                    "completion_tokens_details": {"reasoning_tokens": 218},
                    "prompt_tokens_details": {"cached_tokens": 55808}
                }
            }]
        });
        let stats = parse_usage_stats(&json);
        assert_eq!(stats.prompt_tokens, 56525);
        assert_eq!(stats.response_tokens, 338);
        assert_eq!(stats.cached_tokens, 55808);
        assert_eq!(stats.reasoning_tokens, 218);
    }

    #[test]
    fn usage_absent_yields_default() {
        let json = serde_json::json!({
            "choices": [{"index": 0, "delta": {"content": "hi"}, "finish_reason": null}]
        });
        assert!(usage_from_json(&json).is_none());
        let stats = parse_usage_stats(&json);
        assert_eq!(stats.prompt_tokens, 0);
        assert_eq!(stats.response_tokens, 0);
    }
}

#[cfg(test)]
mod scrollbar_tests {
    use super::scrollbar_scroll_offset;

    // Terminal 40 rows tall -> input_start = 34, output area rows 1..=33,
    // visible content height = 31. 131 total lines -> max_scroll = 100.
    const INPUT_START: u16 = 34;
    const TOTAL: usize = 131;
    const MAX: u16 = 100;

    #[test]
    fn top_row_reaches_absolute_top() {
        // Regression: the top row used to only nudge one line up.
        assert_eq!(scrollbar_scroll_offset(1, INPUT_START, TOTAL), Some(0));
    }

    #[test]
    fn bottom_row_reaches_absolute_bottom() {
        // Regression: the bottom row used to only nudge one line down.
        assert_eq!(scrollbar_scroll_offset(33, INPUT_START, TOTAL), Some(MAX));
    }

    #[test]
    fn dragging_past_edges_clamps() {
        assert_eq!(scrollbar_scroll_offset(0, INPUT_START, TOTAL), Some(0));
        assert_eq!(scrollbar_scroll_offset(39, INPUT_START, TOTAL), Some(MAX));
        assert_eq!(scrollbar_scroll_offset(u16::MAX, INPUT_START, TOTAL), Some(MAX));
    }

    #[test]
    fn middle_maps_linearly() {
        // Row 17 of track 1..=33 is exactly halfway -> half of max_scroll.
        assert_eq!(scrollbar_scroll_offset(17, INPUT_START, TOTAL), Some(50));
    }

    #[test]
    fn no_overflow_means_no_scroll() {
        // 20 lines fit into 31 visible rows — nothing to scroll.
        assert_eq!(scrollbar_scroll_offset(10, INPUT_START, 20), None);
    }

    #[test]
    fn degenerate_track_is_safe() {
        assert_eq!(scrollbar_scroll_offset(1, 2, TOTAL), None);
        assert_eq!(scrollbar_scroll_offset(1, 0, TOTAL), None);
    }
}

/// Maps a mouse row on the output scrollbar track to a scroll offset.
///
/// The geometry mirrors `render_output` (main.rs): the output area spans rows
/// `1 ..= input_start - 1`, so its height is `input_start - 1` and the visible
/// content height is that minus the 2 border rows. The track occupies the full
/// area height, so the click row maps linearly onto `0 ..= max_scroll`.
///
/// The row is clamped into the track range first, so dragging to (or past)
/// the very top/bottom row pins to the first/last position — the previous
/// implementation treated the edge rows as "nudge by one line" zones, which
/// made the ends of the history unreachable by dragging.
///
/// Returns `None` when scrolling is impossible (no overflow / no track).
fn scrollbar_scroll_offset(row: u16, input_start: u16, total_lines: usize) -> Option<u16> {
    let area_height = input_start.saturating_sub(1);
    let visible_height = area_height.saturating_sub(2) as usize;
    let max_scroll = total_lines.saturating_sub(visible_height);
    if area_height < 2 || max_scroll == 0 {
        return None;
    }

    let track_row = row.clamp(1, area_height);
    let ratio = (track_row - 1) as f64 / (area_height - 1) as f64;
    Some((ratio * max_scroll as f64).round() as u16)
}

fn chrono_now() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}... ({} bytes)", &s[..max], s.len())
    }
}

fn patch_tool_call_ids(msgs: &mut [ChatMessage]) {
    for msg in msgs.iter_mut() {
        match msg {
            ChatMessage::ToolCall {
                name, tool_call_id, ..
            }
            | ChatMessage::ToolResult {
                name, tool_call_id, ..
            } if tool_call_id.is_none() => {
                *tool_call_id = Some(format!("call_{}", name));
            }
            _ => {}
        }
    }
}

fn common_prefix(items: &[String]) -> String {
    if items.is_empty() {
        return String::new();
    }
    let first = &items[0];
    let mut end = first.len();
    for item in &items[1..] {
        end = end.min(item.len());
        for i in 0..end {
            if first.as_bytes()[i] != item.as_bytes()[i] {
                end = i;
                break;
            }
        }
    }
    first[..end].to_string()
}

fn dirs_home() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

/// Default directory for storing and loading session files.
///
/// Resolves to `<DOCUMENTS>/rustama` where `<DOCUMENTS>` comes from
/// `xdg-user-dir DOCUMENTS` (XDG user dirs). Falls back to `~/rustama`
/// when the tool is missing, fails, or returns an empty/non-absolute path.
fn sessions_dir() -> PathBuf {
    if let Some(docs) = xdg_documents_dir() {
        return docs.join("rustama");
    }
    dirs_home().join("rustama")
}

/// Runs `xdg-user-dir DOCUMENTS` and returns the parsed directory.
fn xdg_documents_dir() -> Option<PathBuf> {
    let output = std::process::Command::new("xdg-user-dir")
        .arg("DOCUMENTS")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if path.is_empty() {
        return None;
    }
    let path = PathBuf::from(path);
    // xdg-user-dir prints $HOME when the dir is not configured; an
    // absolute path is the only thing we can sanity-check here.
    if path.is_absolute() { Some(path) } else { None }
}

#[cfg(test)]
mod sessions_dir_tests {
    use super::*;

    #[test]
    fn sessions_dir_ends_with_rustama() {
        let dir = sessions_dir();
        assert_eq!(dir.file_name().unwrap().to_string_lossy(), "rustama");
    }

    #[test]
    fn sessions_dir_is_absolute() {
        assert!(sessions_dir().is_absolute());
    }

    #[test]
    fn xdg_documents_dir_returns_absolute_path_or_none() {
        if let Some(docs) = xdg_documents_dir() {
            assert!(docs.is_absolute());
        }
    }

    #[test]
    fn sessions_dir_prefers_documents_when_available() {
        // When xdg-user-dir works, sessions live under <DOCUMENTS>/rustama;
        // otherwise under ~/rustama.
        let dir = sessions_dir();
        if let Some(docs) = xdg_documents_dir() {
            assert_eq!(dir, docs.join("rustama"));
        } else {
            assert_eq!(dir, dirs_home().join("rustama"));
        }
    }
}

/// Resolves the effective generation parameters for a model: cloud models
/// carry their own (from `cloud_models.conf`); Ollama models look up
/// `model_params.conf` (named section, then `[default]`, then built-ins).
fn params_for_model(
    model: &str,
    cloud_models: &[CloudModel],
    store: &ModelParamsStore,
) -> ModelParams {
    if let Some(cm) = cloud_models.iter().find(|m| m.name == model) {
        return cm.params.clone();
    }
    store.params_for(model)
}

fn get_tool_definitions() -> Vec<serde_json::Value> {
    vec![
        serde_json::json!({
            "type": "function",
            "function": {
                "name": "read_file",
                "description": "Read the contents of a file",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "The file path to read"
                        }
                    },
                    "required": ["path"]
                }
            }
        }),
        serde_json::json!({
            "type": "function",
            "function": {
                "name": "write_file",
                "description": "Write content to a file, creating it if it doesn't exist",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "The file path to write"
                        },
                        "content": {
                            "type": "string",
                            "description": "The content to write"
                        }
                    },
                    "required": ["path", "content"]
                }
            }
        }),
        serde_json::json!({
            "type": "function",
            "function": {
                "name": "edit_file",
                "description": "Edit a file by replacing old_string with new_string",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "The file path to edit"
                        },
                        "old_string": {
                            "type": "string",
                            "description": "The string to find and replace"
                        },
                        "new_string": {
                            "type": "string",
                            "description": "The replacement string"
                        }
                    },
                    "required": ["path", "old_string", "new_string"]
                }
            }
        }),
        serde_json::json!({
            "type": "function",
            "function": {
                "name": "bash",
                "description": "Execute a bash command and return the output",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "command": {
                            "type": "string",
                            "description": "The bash command to execute"
                        }
                    },
                    "required": ["command"]
                }
            }
        }),
        serde_json::json!({
            "type": "function",
            "function": {
                "name": "list_files",
                "description": "List files and directories at a path",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "The directory path to list"
                        }
                    },
                    "required": ["path"]
                }
            }
        }),
        serde_json::json!({
            "type": "function",
            "function": {
                "name": "search_files",
                "description": "Search for files matching a glob pattern",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "pattern": {
                            "type": "string",
                            "description": "The glob pattern to match (e.g. '**/*.rs')"
                        },
                        "path": {
                            "type": "string",
                            "description": "The directory to search in (defaults to current directory)"
                        }
                    },
                    "required": ["pattern"]
                }
            }
        }),
        serde_json::json!({
            "type": "function",
            "function": {
                "name": "search_content",
                "description": "Search file contents for a pattern (regex supported)",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "pattern": {
                            "type": "string",
                            "description": "The regex pattern to search for"
                        },
                        "path": {
                            "type": "string",
                            "description": "The directory to search in"
                        },
                        "include": {
                            "type": "string",
                            "description": "File pattern to include (e.g. '*.rs')"
                        }
                    },
                    "required": ["pattern"]
                }
            }
        }),
        serde_json::json!({
            "type": "function",
            "function": {
                "name": "fetch_url",
                "description": "Fetch content from a URL via HTTP/HTTPS GET request",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "url": {
                            "type": "string",
                            "description": "The URL to fetch (http:// or https://)"
                        }
                    },
                    "required": ["url"]
                }
            }
        }),
        serde_json::json!({
            "type": "function",
            "function": {
                "name": "web_search",
                "description": "Search the web using DuckDuckGo and return results",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "The search query"
                        }
                    },
                    "required": ["query"]
                }
            }
        }),
        serde_json::json!({
            "type": "function",
            "function": {
                "name": "terminal_open",
                "description": "Open a terminal session to run a command in a real PTY — fully interactive programs work (REPLs, ssh, top, vim, even other TUI apps). The terminal panel becomes visible so you can observe the output. Returns JSON with 'status' (message) and 'cursor' (pass this to terminal_read for incremental reads). Use terminal_send to interact (supports escape sequences for special keys) and terminal_read to check output/screen. Use terminal_close when done.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "command": {
                            "type": "string",
                            "description": "The shell command to run (e.g. 'npm run dev', 'cargo build')"
                        }
                    },
                    "required": ["command"]
                }
            }
        }),
        serde_json::json!({
            "type": "function",
            "function": {
                "name": "terminal_send",
                "description": "Send input (keystrokes) to the running terminal session. The terminal is a real PTY, so interactive programs work: a newline is appended automatically (like pressing Enter). For special keys, embed the raw escape/control sequences in the string: Ctrl+C = \"\\u0003\", Ctrl+D = \"\\u0004\", arrows = \"\\u001b[A/B/C/D\" (up/down/right/left), Tab = \"\\t\", Escape = \"\\u001b\", F1 = \"\\u001bOP\", PgUp/PgDn = \"\\u001b[5~\"/\"\\u001b[6~\". Example: send \"\\u0003\" to interrupt a running program.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "input": {
                            "type": "string",
                            "description": "The text to send to the terminal (a newline is appended automatically). May contain escape/control sequences for special keys."
                        }
                    },
                    "required": ["input"]
                }
            }
        }),
        serde_json::json!({
            "type": "function",
            "function": {
                "name": "terminal_read",
                "description": "Read output from the terminal session. Returns JSON with 'output' (new raw output since 'cursor'), 'cursor' (pass on the next read for incremental output), 'gap' (true if output was lost to buffer overflow), 'screen' (the current rendered terminal screen as text — like a screenshot; this is what shows what full-screen/interactive programs are doing), and 'screen_cursor' (cursor row/col on screen). On the first read, omit 'cursor'. Use 'screen' to understand the current state of interactive/TUI programs.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "cursor": {
                            "type": "integer",
                            "description": "Optional cursor from a previous terminal_read response. When provided, only output written after that cursor is returned. Omit for the first read or to get the full tail window."
                        }
                    }
                }
            }
        }),
        serde_json::json!({
            "type": "function",
            "function": {
                "name": "terminal_close",
                "description": "Close the terminal session and hide the terminal panel. Kills the running process.",
                "parameters": {
                    "type": "object",
                    "properties": {}
                }
            }
        }),
    ]
}

fn build_reqwest_client(proxy: &Option<String>) -> reqwest::Client {
    let mut builder = reqwest::Client::builder();
    builder = builder.connect_timeout(std::time::Duration::from_secs(30));
    if let Some(proxy_url) = proxy
        && let Ok(proxy) = reqwest::Proxy::all(proxy_url)
    {
        builder = builder.proxy(proxy);
    }
    builder.build().unwrap_or_else(|_| reqwest::Client::new())
}

fn build_blocking_client(proxy: &Option<String>) -> reqwest::blocking::Client {
    let mut builder = reqwest::blocking::Client::builder();
    if let Some(proxy_url) = proxy
        && let Ok(proxy) = reqwest::Proxy::all(proxy_url)
    {
        builder = builder.proxy(proxy);
    }
    builder
        .build()
        .unwrap_or_else(|_| reqwest::blocking::Client::new())
}

#[allow(clippy::too_many_arguments)]
fn retry_countdown(
    label: &str,
    delay_secs: u64,
    retry_after: Option<u64>,
    attempt: u32,
    max_retries: u32,
    tx: &mpsc::Sender<StreamChunk>,
    is_logging: bool,
    log_file: &str,
    session_id: &str,
) {
    let ra_str = retry_after.map_or("none".to_string(), |v| format!("{}s", v));
    log_to_file(
        is_logging,
        log_file,
        session_id,
        "RETRY",
        &format!(
            "{}, retry-after: {}, waiting: {}s (attempt {}/{})",
            label,
            ra_str,
            delay_secs,
            attempt + 1,
            max_retries
        ),
    );
    for sec in 1..=delay_secs {
        let _ = tx.send(StreamChunk::StatusTick(format!(
            "⏳ {}/{}s (attempt {}/{})",
            sec,
            delay_secs,
            attempt + 1,
            max_retries
        )));
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}

#[allow(clippy::too_many_arguments)]
async fn send_with_retry(
    client: &reqwest::Client,
    method: reqwest::Method,
    url: &str,
    headers: Option<reqwest::header::HeaderMap>,
    body: &serde_json::Value,
    max_retries: u32,
    tx: &mpsc::Sender<StreamChunk>,
    is_logging: bool,
    log_file: &str,
    session_id: &str,
) -> Result<reqwest::Response, reqwest::Error> {
    let mut last_err = None;
    for attempt in 0..=max_retries {
        let mut req = client.request(method.clone(), url).json(body);
        if let Some(ref h) = headers {
            req = req.headers(h.clone());
        }
        match req.send().await {
            Ok(resp) if resp.status().as_u16() == 429 && attempt < max_retries => {
                let retry_after = resp
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| s.parse::<u64>().ok());
                let delay_secs = match attempt {
                    0 => 20u64,
                    1 => 40,
                    2 => 60,
                    _ => 60,
                };
                let delay_secs = retry_after.map_or(delay_secs, |ra| delay_secs.max(ra));
                let _ = resp.text().await;
                retry_countdown(
                    "Rate limited (429)",
                    delay_secs,
                    retry_after,
                    attempt,
                    max_retries,
                    tx,
                    is_logging,
                    log_file,
                    session_id,
                );
                continue;
            }
            Ok(resp) if resp.status().is_server_error() && attempt < max_retries => {
                let status = resp.status();
                let delay_secs = if attempt == 0 {
                    10
                } else {
                    2u64.pow(attempt) + 1
                };
                let _ = resp.text().await;
                retry_countdown(
                    &format!("Server error ({})", status),
                    delay_secs,
                    None,
                    attempt,
                    max_retries,
                    tx,
                    is_logging,
                    log_file,
                    session_id,
                );
                continue;
            }
            Ok(resp) => return Ok(resp),
            Err(e) => {
                last_err = Some(e);
                if attempt < max_retries {
                    let delay_secs = if attempt == 0 {
                        10
                    } else {
                        2u64.pow(attempt) + 1
                    };
                    retry_countdown(
                        "Request error",
                        delay_secs,
                        None,
                        attempt,
                        max_retries,
                        tx,
                        is_logging,
                        log_file,
                        session_id,
                    );
                    continue;
                }
            }
        }
    }
    Err(last_err.unwrap())
}

fn execute_tool_call(name: &str, args_json: &str, proxy: &Option<String>) -> String {
    let args: serde_json::Value = serde_json::from_str(args_json).unwrap_or(serde_json::json!({}));

    match name {
        "read_file" => {
            let path = args["path"].as_str().unwrap_or("");
            match std::fs::read_to_string(path) {
                Ok(content) => content,
                Err(e) => format!("Error reading file: {}", e),
            }
        }
        "write_file" => {
            let path = args["path"].as_str().unwrap_or("");
            let content = args["content"].as_str().unwrap_or("");
            match std::fs::write(path, content) {
                Ok(()) => format!("Successfully wrote {} bytes to {}", content.len(), path),
                Err(e) => format!("Error writing file: {}", e),
            }
        }
        "edit_file" => {
            let path = args["path"].as_str().unwrap_or("");
            let old = args["old_string"].as_str().unwrap_or("");
            let new = args["new_string"].as_str().unwrap_or("");
            match std::fs::read_to_string(path) {
                Ok(content) => {
                    if let Some(pos) = content.find(old) {
                        let mut new_content = content.clone();
                        new_content.replace_range(pos..pos + old.len(), new);
                        match std::fs::write(path, &new_content) {
                            Ok(()) => format!("Successfully edited {}", path),
                            Err(e) => format!("Error writing file: {}", e),
                        }
                    } else {
                        format!("Error: old_string not found in {}", path)
                    }
                }
                Err(e) => format!("Error reading file: {}", e),
            }
        }
        "bash" => {
            let command = args["command"].as_str().unwrap_or("");
            let trimmed = command.trim_start();
            if trimmed.starts_with("sudo ") || trimmed == "sudo" {
                return "Error: sudo is not permitted. You do not have elevated privileges and cannot run commands as root.".to_string();
            }
            use std::io::Read;
            use std::process::Stdio;
            match std::process::Command::new("sh")
                .arg("-c")
                .arg(command)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .stdin(Stdio::null())
                .spawn()
            {
                Ok(mut child) => {
                    let start = std::time::Instant::now();
                    let timeout = std::time::Duration::from_secs(30);
                    loop {
                        match child.try_wait() {
                            Ok(Some(status)) => {
                                let mut stdout = String::new();
                                let mut stderr = String::new();
                                if let Some(ref mut out) = child.stdout {
                                    let _ = out.read_to_string(&mut stdout);
                                }
                                if let Some(ref mut err) = child.stderr {
                                    let _ = err.read_to_string(&mut stderr);
                                }
                                let mut result = String::new();
                                if !stdout.is_empty() {
                                    result.push_str(&stdout);
                                }
                                if !stderr.is_empty() {
                                    if !result.is_empty() {
                                        result.push('\n');
                                    }
                                    result.push_str(&stderr);
                                }
                                if result.is_empty() {
                                    result = format!("(exit {})", status);
                                }
                                return result;
                            }
                            Ok(None) => {
                                if start.elapsed() > timeout {
                                    let _ = child.kill();
                                    let _ = child.wait();
                                    return "(command timed out after 30s)".to_string();
                                }
                                std::thread::sleep(std::time::Duration::from_millis(50));
                            }
                            Err(e) => return format!("Error: {}", e),
                        }
                    }
                }
                Err(e) => format!("Error executing command: {}", e),
            }
        }
        "list_files" => {
            let path = args["path"].as_str().unwrap_or(".");
            match std::fs::read_dir(path) {
                Ok(entries) => {
                    let mut files: Vec<String> = entries
                        .filter_map(|e| e.ok())
                        .map(|e| {
                            let name = e.file_name().to_string_lossy().to_string();
                            let is_dir = e.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
                            if is_dir { format!("{}/", name) } else { name }
                        })
                        .collect();
                    files.sort();
                    files.join("\n")
                }
                Err(e) => format!("Error listing directory: {}", e),
            }
        }
        "search_files" => {
            let pattern = args["pattern"].as_str().unwrap_or("*");
            let path = args["path"].as_str().unwrap_or(".");
            let full_pattern = format!("{}/{}", path, pattern);
            match glob::glob(&full_pattern) {
                Ok(paths) => {
                    let results: Vec<String> = paths
                        .filter_map(|p| p.ok())
                        .map(|p| p.display().to_string())
                        .collect();
                    if results.is_empty() {
                        "No files found".to_string()
                    } else {
                        results.join("\n")
                    }
                }
                Err(e) => format!("Error with glob pattern: {}", e),
            }
        }
        "search_content" => {
            let pattern = args["pattern"].as_str().unwrap_or("");
            let path = args["path"].as_str().unwrap_or(".");
            let include = args["include"].as_str().unwrap_or("*");

            match grep_regex(pattern, path, include) {
                Ok(results) => {
                    if results.is_empty() {
                        "No matches found".to_string()
                    } else {
                        results.join("\n")
                    }
                }
                Err(e) => format!("Error searching: {}", e),
            }
        }
        "fetch_url" => {
            let url = args["url"].as_str().unwrap_or("");
            if url.is_empty() {
                "Error: no URL provided".to_string()
            } else {
                let client = build_blocking_client(proxy);
                match client.get(url).send() {
                    Ok(resp) => {
                        let status = resp.status();
                        if !status.is_success() {
                            format!("HTTP error: {}", status)
                        } else {
                            match resp.text() {
                                Ok(text) => {
                                    if text.len() > 50000 {
                                        format!(
                                            "{}...[truncated, total {} bytes]",
                                            &text[..50000],
                                            text.len()
                                        )
                                    } else {
                                        text
                                    }
                                }
                                Err(e) => format!("Error reading response: {}", e),
                            }
                        }
                    }
                    Err(e) => format!("Error fetching URL: {}", e),
                }
            }
        }
        "web_search" => {
            let query = args["query"].as_str().unwrap_or("");
            if query.is_empty() {
                "Error: no search query provided".to_string()
            } else {
                let search_url = format!(
                    "https://html.duckduckgo.com/html/?q={}",
                    urlencoding::encode(query)
                );
                let client = build_blocking_client(proxy);
                match client.get(&*search_url).send() {
                    Ok(resp) => match resp.text() {
                        Ok(html) => {
                            let mut results = Vec::new();
                            for line in html.lines() {
                                if line.contains("result__snippet") || line.contains("result__a") {
                                    let cleaned = line
                                        .replace(
                                            "<a rel=\"nofollow\" class=\"result__a\" href=\"",
                                            "",
                                        )
                                        .replace("<a class=\"result__snippet\" href=\"", "")
                                        .replace("</a>", "")
                                        .replace("<span class=\"result__snippet\">", "")
                                        .replace("</span>", "")
                                        .replace("<b>", "")
                                        .replace("</b>", "")
                                        .trim()
                                        .to_string();
                                    if !cleaned.is_empty() && cleaned.len() > 5 {
                                        results.push(cleaned);
                                    }
                                }
                            }
                            if results.is_empty() {
                                "No results found".to_string()
                            } else {
                                results.join("\n")
                            }
                        }
                        Err(e) => format!("Error reading search results: {}", e),
                    },
                    Err(e) => format!("Error performing search: {}", e),
                }
            }
        }
        _ => {
            let tools: Vec<&str> = vec![
                "read_file",
                "write_file",
                "edit_file",
                "bash",
                "list_files",
                "search_files",
                "search_content",
                "fetch_url",
                "web_search",
                "terminal_open",
                "terminal_send",
                "terminal_read",
                "terminal_close",
            ];
            format!(
                "Unknown tool: '{}'. Available tools: {}",
                name,
                tools.join(", ")
            )
        }
    }
}

fn grep_regex(pattern: &str, path: &str, include: &str) -> Result<Vec<String>, String> {
    let re = regex::Regex::new(pattern).map_err(|e| format!("Invalid regex: {}", e))?;
    let glob_pattern = format!("{}/{}", path, include);

    let mut results = Vec::new();
    if let Ok(entries) = glob::glob(&glob_pattern) {
        for entry in entries.flatten() {
            if entry.is_file()
                && let Ok(content) = std::fs::read_to_string(&entry)
            {
                for (line_num, line) in content.lines().enumerate() {
                    if re.is_match(line) {
                        results.push(format!("{}:{}: {}", entry.display(), line_num + 1, line));
                    }
                }
            }
        }
    }
    Ok(results)
}

const TOOL_NAMES: &[&str] = &[
    "read_file",
    "write_file",
    "edit_file",
    "bash",
    "list_files",
    "search_files",
    "search_content",
    "fetch_url",
    "web_search",
    "terminal_open",
    "terminal_send",
    "terminal_read",
    "terminal_close",
];

fn parse_text_tool_calls(text: &str) -> Vec<serde_json::Value> {
    let mut tool_calls = Vec::new();

    let patterns = [
        (
            r#"\{"name"\s*:\s*"(\w+)"\s*,\s*"parameters"\s*:\s*(\{[^}]*\})\}"#,
            "name",
            "parameters",
        ),
        (
            r#"\{"name"\s*:\s*"(\w+)"\s*,\s*"arguments"\s*:\s*(\{[^}]*\})\}"#,
            "name",
            "arguments",
        ),
    ];

    for (regex_pattern, _name_key, _args_key) in &patterns {
        if let Ok(re) = regex::Regex::new(regex_pattern) {
            for cap in re.captures_iter(text) {
                if let (Some(name_match), Some(args_match)) = (cap.get(1), cap.get(2)) {
                    let name = name_match.as_str().to_string();
                    let args_str = args_match.as_str().to_string();

                    if TOOL_NAMES.contains(&name.as_str()) {
                        let mut tc = serde_json::json!({
                            "name": name,
                        });
                        tc["parameters"] =
                            serde_json::from_str(&args_str).unwrap_or(serde_json::json!({}));
                        tool_calls.push(tc);
                    }
                }
            }
        }
    }

    if !tool_calls.is_empty() {
        return tool_calls;
    }

    if let Ok(re) = regex::Regex::new(r#"(\w+):\s*(\{.*\})"#) {
        for cap in re.captures_iter(text) {
            if let (Some(name_match), Some(args_match)) = (cap.get(1), cap.get(2)) {
                let name = name_match.as_str().to_string();
                let args_str = args_match.as_str().to_string();

                if TOOL_NAMES.contains(&name.as_str())
                    && let Ok(args_json) = serde_json::from_str::<serde_json::Value>(&args_str)
                {
                    let tc = serde_json::json!({
                        "name": name,
                        "parameters": args_json,
                    });
                    tool_calls.push(tc);
                }
            }
        }
    }

    tool_calls
}

/// Upper bound on automatic "continue" nudges per user turn (see
/// [`needs_continuation`]). Bounds token waste when the model keeps
/// stopping without making progress; the user can always type "continue"
/// by hand to reset the budget.
const MAX_AUTO_CONTINUES: u32 = 10;

/// Heuristic: does this assistant text look like the model stopped right
/// before the tool call it announced?
///
/// Cloud models (Kimi-K3 in particular) occasionally emit
/// `finish_reason="stop"` immediately after a transitional preamble —
/// "All 3 tests pass. Let me run the full suite:" — with no `tool_calls`
/// delta. The agentic loop only continues on tool calls, so the run
/// stalled until the user typed "continue" by hand. When this returns
/// true we send that "continue" automatically.
///
/// Right-edge signals, checked after trimming whitespace and trailing
/// markdown emphasis/backticks:
///   * `:` or `,` — the classic pre-tool preamble ending ("...regressed:")
///   * an action phrase ("let me", "i'll", "going to", ...)
fn needs_continuation(text: &str) -> bool {
    let t = text.trim_end();
    if t.is_empty() {
        return false;
    }
    // Strip markdown decoration that often trails a preamble ("**X:**", "`X`:").
    let t = t.trim_end_matches(['*', '_', '`']).trim_end();
    if t.ends_with(':') || t.ends_with(',') {
        return true;
    }
    let lower = t.to_lowercase();
    const PHRASES: &[&str] = &[
        "let me",
        "let's",
        "i'll",
        "i will",
        "i need to",
        "i want to",
        "going to",
        "i should",
        "let me check",
        "checking",
        "i'm",
    ];
    PHRASES.iter().any(|p| lower.ends_with(p))
}

#[cfg(test)]
mod continuation_tests {
    use super::needs_continuation;

    #[test]
    fn trailing_colon_needs_continuation() {
        // The exact shapes observed in rustama.log.test before the user had
        // to type "continue" by hand.
        assert!(needs_continuation(
            "All 3 new tests pass. Let me run the full suite to make sure nothing regressed:"
        ));
        assert!(needs_continuation(
            "Let me see how the scrollbar is actually rendered, and how `handle_click`/`handle_mouse_drag` get called:"
        ));
        assert!(needs_continuation(
            "...worth confirming I haven't broken any shared assumption):"
        ));
        assert!(needs_continuation("Applying all four:"));
    }

    #[test]
    fn colon_with_trailing_decoration() {
        assert!(needs_continuation("Let me check X:  \n"));
        assert!(needs_continuation("**Let me check X:**"));
        assert!(needs_continuation("`cargo test`:"));
    }

    #[test]
    fn action_phrase_endings() {
        assert!(needs_continuation("Build clean. Let me"));
        assert!(needs_continuation("Now I'll"));
        assert!(needs_continuation("First I'm going to"));
        assert!(needs_continuation("First,"));
    }

    #[test]
    fn finished_answers_do_not_continue() {
        assert!(!needs_continuation("The drag bug is fixed."));
        assert!(!needs_continuation("All **45 tests pass** (39 + 6 new)."));
        assert!(!needs_continuation("Done — summary above."));
        assert!(!needs_continuation(""));
        assert!(!needs_continuation("   \n "));
        assert!(!needs_continuation("```rust\nfn main() {}\n```"));
    }
}

#[cfg(test)]
mod tests {
    use super::{StreamChunk, retry_countdown, send_with_retry};

    #[test]
    fn retry_countdown_sends_correct_messages() {
        use std::sync::mpsc;
        let (tx, rx) = mpsc::channel();
        retry_countdown("Test error", 3, Some(5), 0, 3, &tx, false, "", "");
        drop(tx);
        let msgs: Vec<String> = rx
            .iter()
            .filter_map(|m| match m {
                StreamChunk::StatusTick(s) => Some(s),
                _ => None,
            })
            .collect();
        assert_eq!(
            msgs.len(),
            3,
            "should have 3 ticks for 3s delay: {:?}",
            msgs
        );
        assert!(msgs[0].contains("1/3s (attempt 1/3)"), "got: {}", msgs[0]);
        assert!(msgs[1].contains("2/3s (attempt 1/3)"), "got: {}", msgs[1]);
        assert!(msgs[2].contains("3/3s (attempt 1/3)"), "got: {}", msgs[2]);
    }

    #[test]
    fn retry_countdown_no_detail() {
        use std::sync::mpsc;
        let (tx, rx) = mpsc::channel();
        retry_countdown("Rate limited (429)", 2, None, 1, 3, &tx, false, "", "");
        drop(tx);
        let msgs: Vec<String> = rx
            .iter()
            .filter_map(|m| match m {
                StreamChunk::StatusTick(s) => Some(s),
                _ => None,
            })
            .collect();
        assert!(
            msgs.iter().any(|m| m.contains("attempt 2/3")),
            "should show correct attempt: {:?}",
            msgs
        );
    }

    #[test]
    fn mock_server_429_flow() {
        use std::io::{Read, Write};
        use std::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        std::thread::spawn(move || {
            for i in 0..3 {
                let (mut stream, _) = listener.accept().unwrap();
                let mut buf = [0u8; 4096];
                let n = stream.read(&mut buf).unwrap();
                assert!(n > 0, "expected a request from the client");
                if i < 2 {
                    let body = r#"{"error": {"message": "quota exceeded"}}"#;
                    let resp = format!(
                        "HTTP/1.1 429 Too Many Requests\r\nContent-Type: application/json\r\nRetry-After: 1\r\nContent-Length: {}\r\n\r\n{}",
                        body.len(),
                        body
                    );
                    stream.write_all(resp.as_bytes()).unwrap();
                } else {
                    drop(stream);
                }
            }
        });

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(async {
            let client = reqwest::Client::new();
            let (tx, rx) = std::sync::mpsc::channel();
            let body = serde_json::json!({"model": "test"});
            let url = format!("http://{}/api/chat", addr);
            let result = send_with_retry(
                &client,
                reqwest::Method::POST,
                &url,
                None,
                &body,
                2,
                &tx,
                false,
                "",
                "test",
            )
            .await;
            drop(tx);
            let msgs: Vec<String> = rx
                .iter()
                .filter_map(|m| match m {
                    StreamChunk::StatusTick(s) => Some(s),
                    _ => None,
                })
                .collect();
            assert!(result.is_err(), "should fail after retries + server error");
            assert!(
                msgs.iter().any(|m| m.contains("attempt 1/2")),
                "should show attempt count: {:?}",
                msgs
            );
            assert!(
                msgs.iter().any(|m| m.contains("attempt 2/2")),
                "should show second attempt: {:?}",
                msgs
            );
        });
    }

    #[test]
    fn retry_countdown_actually_waits() {
        use std::sync::mpsc;
        use std::time::Instant;
        let (tx, rx) = mpsc::channel();
        let start = Instant::now();
        retry_countdown("Test", 3, None, 0, 3, &tx, false, "", "");
        let elapsed = start.elapsed().as_secs();
        drop(tx);
        let _: Vec<_> = rx.iter().collect();
        assert!(elapsed >= 2, "expected >=2s wait, got {}s", elapsed);
        assert!(elapsed <= 4, "expected <=4s wait, got {}s", elapsed);
    }

    // --- TerminalBuffer cursor arithmetic tests ---

    use super::TerminalBuffer;

    fn make_buffer(content: &str, total: u64, start_offset: u64) -> TerminalBuffer {
        TerminalBuffer {
            content: content.to_string(),
            total_bytes_written: total,
            buffer_start_offset: start_offset,
        }
    }

    #[test]
    fn cursor_empty_read_no_cursor() {
        let b = make_buffer("", 0, 0);
        let result = b.read_incremental(None);
        assert_eq!(result["output"].as_str().unwrap(), "");
        assert_eq!(result["cursor"].as_u64().unwrap(), 0);
        assert!(!result["gap"].as_bool().unwrap());
    }

    #[test]
    fn cursor_empty_read_with_cursor() {
        let b = make_buffer("", 0, 0);
        let result = b.read_incremental(Some(0));
        assert_eq!(result["output"].as_str().unwrap(), "");
        assert_eq!(result["cursor"].as_u64().unwrap(), 0);
    }

    #[test]
    fn cursor_read_no_cursor_returns_tail() {
        let b = make_buffer("hello world", 11, 0);
        let result = b.read_incremental(None);
        assert_eq!(result["output"].as_str().unwrap(), "hello world");
        assert_eq!(result["cursor"].as_u64().unwrap(), 11);
    }

    #[test]
    fn cursor_incremental_returns_only_new() {
        let b = make_buffer("abcxyz", 6, 0);
        let result = b.read_incremental(Some(3));
        assert_eq!(result["output"].as_str().unwrap(), "xyz");
        assert_eq!(result["cursor"].as_u64().unwrap(), 6);
        assert!(!result["gap"].as_bool().unwrap());
    }

    #[test]
    fn cursor_at_end_returns_empty() {
        let b = make_buffer("abc", 3, 0);
        let result = b.read_incremental(Some(3));
        assert_eq!(result["output"].as_str().unwrap(), "");
        assert_eq!(result["cursor"].as_u64().unwrap(), 3);
    }

    #[test]
    fn cursor_past_end_returns_error() {
        let b = make_buffer("abc", 3, 0);
        let result = b.read_incremental(Some(100));
        assert_eq!(result["output"].as_str().unwrap(), "");
        assert_eq!(result["cursor"].as_u64().unwrap(), 3);
        assert!(
            result["error"].as_str().is_some(),
            "should have error for out-of-range cursor"
        );
    }

    #[test]
    fn cursor_evicted_returns_gap() {
        // Buffer started at offset 1000, content is "xyz" (3 bytes), total written 1003
        // Asking for cursor=500 which is before buffer_start_offset=1000
        let b = make_buffer("xyz", 1003, 1000);
        let result = b.read_incremental(Some(500));
        assert_eq!(result["output"].as_str().unwrap(), "xyz");
        assert_eq!(result["cursor"].as_u64().unwrap(), 1003);
        assert!(result["gap"].as_bool().unwrap());
    }

    #[test]
    fn cursor_at_buffer_start_returns_all() {
        let b = make_buffer("abc", 1003, 1000);
        let result = b.read_incremental(Some(1000));
        assert_eq!(result["output"].as_str().unwrap(), "abc");
        assert!(!result["gap"].as_bool().unwrap());
    }

    #[test]
    fn cursor_exact_boundary_after_drain() {
        // Simulate: wrote 10000 bytes, drained 5000 from front
        // Buffer now holds bytes 5000..10000, content length = 5000
        let content = "x".repeat(5000);
        let b = make_buffer(&content, 10000, 5000);
        let result = b.read_incremental(Some(5000));
        assert_eq!(result["output"].as_str().unwrap().len(), 5000);
        assert_eq!(result["cursor"].as_u64().unwrap(), 10000);
        assert!(!result["gap"].as_bool().unwrap());
    }

    #[test]
    fn cursor_just_before_buffer_start_is_gap() {
        let content = "x".repeat(5000);
        let b = make_buffer(&content, 10000, 5000);
        let result = b.read_incremental(Some(4999));
        assert!(result["gap"].as_bool().unwrap());
        assert_eq!(result["cursor"].as_u64().unwrap(), 10000);
    }

    #[test]
    fn cursor_monotonic_across_multiple_reads() {
        let mut b = make_buffer("aaaa", 4, 0);
        let r1 = b.read_incremental(None);
        let c1 = r1["cursor"].as_u64().unwrap();

        b.content.push_str("bbbb");
        b.total_bytes_written = 8;
        let r2 = b.read_incremental(Some(c1));
        let c2 = r2["cursor"].as_u64().unwrap();

        assert_eq!(r2["output"].as_str().unwrap(), "bbbb");
        assert!(c2 > c1, "cursor must be monotonic: {} <= {}", c2, c1);
    }

    #[test]
    fn terminal_open_output_captured() {
        use super::TerminalState;
        let mut ts = TerminalState::new();
        let result = ts.open("echo CAPTURE_MARKER_42");
        assert!(
            result.contains("Terminal opened"),
            "open should succeed: {}",
            result
        );

        // Wait for the command to finish (echo is fast, but give it time)
        std::thread::sleep(std::time::Duration::from_millis(700));

        let r = ts.read_buffer_incremental(None);
        let output = r["output"].as_str().unwrap();
        assert!(
            output.contains("CAPTURE_MARKER_42"),
            "terminal_open command output should be in buffer, got: {}",
            output
        );
        assert!(
            r["cursor"].as_u64().unwrap() > 0,
            "cursor should have advanced past the output"
        );
        // The rendered screen must also carry the marker.
        assert!(
            r["screen"].as_str().unwrap().contains("CAPTURE_MARKER_42"),
            "screen should contain marker: {}",
            r["screen"].as_str().unwrap()
        );

        // Read again with cursor — should get empty (nothing new)
        let cursor = r["cursor"].as_u64().unwrap();
        let r2 = ts.read_buffer_incremental(Some(cursor));
        assert_eq!(r2["output"].as_str().unwrap(), "");

        ts.close();
    }

    #[test]
    fn terminal_cursor_continuous_across_send() {
        use super::TerminalState;
        let mut ts = TerminalState::new();
        let result = ts.open("echo BEFORE_SEND");
        assert!(result.contains("Terminal opened"), "open: {}", result);

        std::thread::sleep(std::time::Duration::from_millis(700));

        let r1 = ts.read_buffer_incremental(None);
        let cursor_before = r1["cursor"].as_u64().unwrap();
        assert!(cursor_before > 0, "should have output from open command");
        assert!(
            r1["output"].as_str().unwrap().contains("BEFORE_SEND"),
            "open command output: {}",
            r1["output"].as_str().unwrap()
        );

        // Send more input — same process, same buffer, cursor must remain valid
        let send_result = ts.send_input("echo AFTER_SEND");
        assert!(send_result.starts_with("Sent:"), "send: {}", send_result);

        std::thread::sleep(std::time::Duration::from_millis(700));

        // Incremental read from old cursor — must return only new bytes
        let r2 = ts.read_buffer_incremental(Some(cursor_before));
        let new_output = r2["output"].as_str().unwrap();
        assert!(
            new_output.contains("AFTER_SEND"),
            "incremental read after send should contain AFTER_SEND, got: {}",
            new_output
        );

        // Cursor must have advanced — never reset to 0
        let cursor_after = r2["cursor"].as_u64().unwrap();
        assert!(
            cursor_after > cursor_before,
            "cursor must advance: {} <= {}",
            cursor_after,
            cursor_before
        );

        // Full snapshot must contain both outputs
        let r3 = ts.read_buffer_incremental(None);
        let full = r3["output"].as_str().unwrap();
        assert!(
            full.contains("BEFORE_SEND"),
            "full snapshot missing open output: {}",
            full
        );
        assert!(
            full.contains("AFTER_SEND"),
            "full snapshot missing send output: {}",
            full
        );

        ts.close();
    }

    #[test]
    fn pty_interactive_readline_editing() {
        // A PTY honours readline control characters: type garbage, erase it
        // with backspace (0x7f), then run the real command. With pipes this
        // would send the literal bytes to the program.
        use super::TerminalState;
        let mut ts = TerminalState::new();
        let result = ts.open("");
        assert!(result.contains("Terminal opened"), "open: {}", result);
        std::thread::sleep(std::time::Duration::from_millis(500));

        ts.send_raw("echo GARBAGE\x7f\x7f\x7f\x7f\x7f\x7f\x7fPTY_OK")
            .expect("send_raw");
        std::thread::sleep(std::time::Duration::from_millis(700));

        let r = ts.read_buffer_incremental(None);
        let screen = r["screen"].as_str().unwrap();
        assert!(
            screen.contains("PTY_OK"),
            "screen should show the edited command result: {}",
            screen
        );
        ts.close();
    }

    #[test]
    fn pty_reports_tty_to_child() {
        // Programs check isatty() to decide on interactivity. Inside our PTY
        // the answer must be yes — this is what makes REPLs and TUIs work.
        use super::TerminalState;
        let mut ts = TerminalState::new();
        let result = ts.open("[ -t 0 ] && echo STDIN_IS_TTY || echo STDIN_NOT_TTY");
        assert!(result.contains("Terminal opened"), "open: {}", result);
        std::thread::sleep(std::time::Duration::from_millis(700));

        let r = ts.read_buffer_incremental(None);
        let screen = r["screen"].as_str().unwrap();
        assert!(
            screen.contains("STDIN_IS_TTY"),
            "child stdin should be a tty: {}",
            screen
        );
        ts.close();
    }

    #[test]
    fn pty_fullscreen_app_screen_model() {
        // Full-screen apps draw with cursor-positioning escapes; the vt100
        // screen model must resolve them into placed text (a linear log
        // cannot). `clear` + absolute cursor addressing is the minimal case.
        use super::TerminalState;
        let mut ts = TerminalState::new();
        let result = ts.open("clear; printf '\\033[3;10HTOP_LEFT_MARK'");
        assert!(result.contains("Terminal opened"), "open: {}", result);
        std::thread::sleep(std::time::Duration::from_millis(700));

        let r = ts.read_buffer_incremental(None);
        let screen = r["screen"].as_str().unwrap();
        assert!(
            screen.contains("TOP_LEFT_MARK"),
            "screen should contain positioned text: {:?}",
            screen
        );
        // Row 2 (0-based) must hold the mark starting at col 9 — proof the
        // cursor positioning was interpreted, not just logged.
        let line3 = screen.lines().nth(2).unwrap_or("");
        assert!(
            line3.contains("TOP_LEFT_MARK"),
            "mark must be on screen row 3: {:?}",
            line3
        );
        ts.close();
    }

    #[test]
    fn pty_resize_keeps_screen() {
        use super::TerminalState;
        let mut ts = TerminalState::new();
        let result = ts.open("echo RESIZE_MARKER");
        assert!(result.contains("Terminal opened"), "open: {}", result);
        std::thread::sleep(std::time::Duration::from_millis(700));

        ts.resize(60, 20);
        let r = ts.read_buffer_incremental(None);
        assert!(
            r["screen"].as_str().unwrap().contains("RESIZE_MARKER"),
            "screen must survive resize: {}",
            r["screen"].as_str().unwrap()
        );
        ts.close();
    }

    #[test]
    fn key_events_map_to_terminal_bytes() {
        use super::key_to_pty_bytes;
        use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let ev = |code, modifiers| KeyEvent::new(code, modifiers);
        assert_eq!(
            key_to_pty_bytes(&ev(KeyCode::Char('a'), KeyModifiers::NONE)).as_deref(),
            Some("a")
        );
        assert_eq!(
            key_to_pty_bytes(&ev(KeyCode::Char('c'), KeyModifiers::CONTROL)).as_deref(),
            Some("\x03"),
            "Ctrl+C -> ETX"
        );
        assert_eq!(
            key_to_pty_bytes(&ev(KeyCode::Char('d'), KeyModifiers::CONTROL)).as_deref(),
            Some("\x04"),
            "Ctrl+D -> EOT"
        );
        assert_eq!(
            key_to_pty_bytes(&ev(KeyCode::Enter, KeyModifiers::NONE)).as_deref(),
            Some("\r")
        );
        assert_eq!(
            key_to_pty_bytes(&ev(KeyCode::Backspace, KeyModifiers::NONE)).as_deref(),
            Some("\x7f")
        );
        assert_eq!(
            key_to_pty_bytes(&ev(KeyCode::Up, KeyModifiers::NONE)).as_deref(),
            Some("\x1b[A")
        );
        assert_eq!(
            key_to_pty_bytes(&ev(KeyCode::F(1), KeyModifiers::NONE)).as_deref(),
            Some("\x1bOP")
        );
        assert_eq!(
            key_to_pty_bytes(&ev(KeyCode::Delete, KeyModifiers::NONE)).as_deref(),
            Some("\x1b[3~")
        );
        assert_eq!(
            key_to_pty_bytes(&ev(KeyCode::Char('x'), KeyModifiers::ALT)).as_deref(),
            Some("\x1bx"),
            "Alt prefixes ESC"
        );
        assert_eq!(
            key_to_pty_bytes(&ev(KeyCode::Tab, KeyModifiers::SHIFT)).as_deref(),
            Some("\x1b[Z"),
            "Shift+Tab -> backtab"
        );
    }

    /// Full acid test: real interactive programs through the PTY — a REPL,
    /// readline history, a full-screen pager, and Rustama itself driven
    /// with raw keystrokes. Ignored by default (slow, spawns processes);
    /// run with: cargo test --release pty_acid -- --ignored --nocapture
    #[test]
    #[ignore]
    fn pty_acid_interactive_programs() {
        use super::TerminalState;
        let wait = |ms: u64| std::thread::sleep(std::time::Duration::from_millis(ms));
        let mut ts = TerminalState::new();
        ts.resize(100, 30);

        let r = ts.open("");
        assert!(r.contains("Terminal opened"), "{}", r);
        wait(500);

        // 1. python3 REPL — impossible with pipes
        ts.send_input("python3 -q");
        wait(900);
        ts.send_input("print('REPL_' + 'WORKS')");
        wait(900);
        let screen = ts.read_buffer_incremental(None);
        let screen = screen["screen"].as_str().unwrap().to_string();
        assert!(screen.contains("REPL_WORKS"), "python repl failed:\n{}", screen);

        // 2. Ctrl+C interrupt, then leave the REPL
        ts.send_raw("\x03").unwrap();
        wait(300);
        ts.send_input("exit()");
        wait(500);

        // 3. readline up-arrow history in bash
        ts.send_input("echo HISTORY_ONE");
        wait(400);
        ts.send_raw("\x1b[A").unwrap();
        wait(300);
        let screen = ts.read_buffer_incremental(None);
        let screen = screen["screen"].as_str().unwrap().to_string();
        assert!(
            screen.matches("HISTORY_ONE").count() >= 2,
            "up-arrow history failed:\n{}",
            screen
        );
        ts.send_raw("\x03").unwrap();
        wait(300);

        // 4. full-screen pager
        ts.send_input("seq 1 100 > /tmp/seq_rustama_test.txt; less /tmp/seq_rustama_test.txt");
        wait(800);
        let screen = ts.read_buffer_incremental(None);
        let screen = screen["screen"].as_str().unwrap().to_string();
        assert!(
            screen.contains("seq_rustama_test.txt"),
            "less didn't open:\n{}",
            screen
        );
        ts.send_raw("q").unwrap();
        wait(400);

        ts.close();
    }

    /// Rustama running inside its own embedded terminal: verifies the whole
    /// stack (PTY + vt100 + key encoding) can host a full-screen crossterm
    /// TUI and drive it with keystrokes. Needs a release binary; run with:
    ///   cargo build --release && cargo test --release pty_acid_rustama -- --ignored --nocapture
    #[test]
    #[ignore]
    fn pty_acid_rustama_in_rustama() {
        use super::TerminalState;
        let wait = |ms: u64| std::thread::sleep(std::time::Duration::from_millis(ms));
        let bin = format!("{}/target/release/rustama", env!("CARGO_MANIFEST_DIR"));
        assert!(
            std::path::Path::new(&bin).exists(),
            "build the release binary first: cargo build --release"
        );

        let mut ts = TerminalState::new();
        ts.resize(100, 30);
        let r = ts.open("");
        assert!(r.contains("Terminal opened"), "{}", r);
        wait(500);

        // Launch the inner Rustama.
        ts.send_input(&bin);
        // crossterm queries the cursor position at startup; the vt100
        // callbacks queue replies that must be flushed back to the PTY.
        for _ in 0..30 {
            wait(100);
            ts.flush_replies();
        }
        let screen = ts.read_buffer_incremental(None);
        let screen = screen["screen"].as_str().unwrap().to_string();
        assert!(
            screen.contains("F9") || screen.contains("Menu") || screen.contains("Rustama"),
            "inner rustama didn't render its UI:\n{}",
            screen
        );

        // F9 opens the inner menu.
        ts.send_raw("\x1b[20~").unwrap();
        wait(700);
        let screen = ts.read_buffer_incremental(None);
        let screen = screen["screen"].as_str().unwrap().to_string();
        assert!(
            screen.contains("Load") || screen.contains("Save") || screen.contains("Exit"),
            "F9 menu didn't open in inner rustama:\n{}",
            screen
        );

        // Esc closes it, F10 asks to quit, 'y' confirms.
        ts.send_raw("\x1b").unwrap();
        wait(400);
        ts.send_raw("\x1b[21~").unwrap();
        for _ in 0..7 {
            wait(100);
            ts.flush_replies();
        }
        let screen = ts.read_buffer_incremental(None);
        let screen = screen["screen"].as_str().unwrap().to_string();
        assert!(
            screen.contains("uit"),
            "F10 quit dialog didn't show:\n{}",
            screen
        );
        ts.send_raw("y").unwrap();
        wait(1000);
        let screen = ts.read_buffer_incremental(None);
        let screen = screen["screen"].as_str().unwrap().to_string();
        assert!(
            !screen.contains("Confirm Quit"),
            "inner rustama didn't exit:\n{}",
            screen
        );

        ts.close();
    }
}

/// Builds the JSON request body for a chat request, translating the
/// per-model [`ModelParams`] into the dialect the backend speaks:
/// OpenAI-style top-level fields for cloud models, Ollama `options` map +
/// top-level `think` for Ollama.
fn build_chat_body(
    model: &str,
    cloud_model: Option<&CloudModel>,
    agentic: bool,
    api_messages: &[serde_json::Value],
    params: &ModelParams,
) -> serde_json::Value {
    if let Some(cloud) = cloud_model {
        let mut body = serde_json::json!({
            "model": cloud.api_model,
            "messages": api_messages,
            "stream": true,
        });
        if let Some(max_tokens) = params.max_output_tokens {
            body["max_tokens"] = serde_json::json!(max_tokens);
        }
        body["tools"] = serde_json::json!(get_tool_definitions());
        body["temperature"] = serde_json::json!(params.temperature);
        body["top_p"] = serde_json::json!(params.top_p);
        if params.frequency_penalty != 0.0 {
            body["frequency_penalty"] = serde_json::json!(params.frequency_penalty);
        }
        if params.presence_penalty != 0.0 {
            body["presence_penalty"] = serde_json::json!(params.presence_penalty);
        }
        if let Some(seed) = params.seed {
            body["seed"] = serde_json::json!(seed);
        }
        // OpenAI o-series / reasoning-capable cloud APIs take levels only.
        if let Some(ref effort) = params.reasoning_effort
            && matches!(effort.as_str(), "low" | "medium" | "high")
        {
            body["reasoning_effort"] = serde_json::json!(effort);
        }
        body
    } else {
        let mut body = serde_json::json!({
            "model": model,
            "messages": api_messages,
            "stream": true,
        });
        if agentic {
            body["tools"] = serde_json::json!(get_tool_definitions());
        }
        let mut options = serde_json::json!({});
        options["temperature"] = serde_json::json!(params.temperature);
        options["top_p"] = serde_json::json!(params.top_p);
        if params.top_k > 0 {
            options["top_k"] = serde_json::json!(params.top_k);
        }
        if params.frequency_penalty != 0.0 {
            options["frequency_penalty"] = serde_json::json!(params.frequency_penalty);
        }
        if params.presence_penalty != 0.0 {
            options["presence_penalty"] = serde_json::json!(params.presence_penalty);
        }
        if let Some(seed) = params.seed {
            options["seed"] = serde_json::json!(seed);
        }
        if let Some(num_predict) = params.max_output_tokens {
            options["num_predict"] = serde_json::json!(num_predict);
        }
        body["options"] = options;
        // Ollama `think`: bool toggle, or a level string for models
        // that support graded thinking (e.g. gpt-oss).
        match params.reasoning_effort.as_deref() {
            Some("off" | "none" | "false") => {
                body["think"] = serde_json::json!(false);
            }
            Some("on" | "true") => {
                body["think"] = serde_json::json!(true);
            }
            Some(level @ ("low" | "medium" | "high")) => {
                body["think"] = serde_json::json!(level);
            }
            _ => {}
        }
        body
    }
}

/// Shared HTTP-send + SSE/Ollama-JSON stream-parse machinery.
///
/// Sends `api_messages` to the cloud or Ollama chat endpoint (with tools
/// attached when `agentic`), spawns a background thread with its own tokio
/// runtime, and forwards parsed `StreamChunk`s over the returned channel.
///
/// Both `send_to_ollama_async_with` (user prompt) and `send_tool_results_async`
/// (agentic tool round) build their message list and then delegate here — the
/// request/response plumbing was previously duplicated in both.
#[allow(clippy::too_many_arguments)]
fn stream_chat_request(
    api_messages: Vec<serde_json::Value>,
    agentic: bool,
    model: String,
    url: String,
    cloud_model: Option<CloudModel>,
    is_logging: bool,
    log_file: String,
    session_id: String,
    params: ModelParams,
    proxy: Option<String>,
    max_retries: u32,
) -> mpsc::Receiver<StreamChunk> {
    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(async {
            let client = build_reqwest_client(&proxy);

            let body = build_chat_body(&model, cloud_model.as_ref(), agentic, &api_messages, &params);
            let (api_url, headers) = if let Some(ref cloud) = cloud_model {
                let mut headers = reqwest::header::HeaderMap::new();
                headers.insert(
                    "Authorization",
                    reqwest::header::HeaderValue::from_str(&format!("Bearer {}", cloud.api_key)).unwrap(),
                );
                headers.insert(
                    "Content-Type",
                    reqwest::header::HeaderValue::from_static("application/json"),
                );
                (cloud.api_url.clone(), Some(headers))
            } else {
                (format!("{}/api/chat", url), None)
            };

            log_to_file(is_logging, &log_file, &session_id, "REQUEST", &format!("{} {}", api_url, serde_json::to_string(&body).unwrap_or_default()));

            let result = send_with_retry(&client, reqwest::Method::POST, &api_url, headers, &body, max_retries, &tx, is_logging, &log_file, &session_id).await;

            let is_cloud = cloud_model.is_some();
            match result {
                Ok(mut resp) => {
                    if !resp.status().is_success() {
                        let status = resp.status();
                        let body = resp.text().await.unwrap_or_default();
                        let msg = format!("HTTP {}: {}", status, truncate(&body, 200));
                        log_to_file(is_logging, &log_file, &session_id, "HTTP_ERROR", &msg);
                        if status.as_u16() == 429 {
                            let _ = tx.send(StreamChunk::RetryPaused(format!("Max retries ({}) reached. Request paused.", max_retries)));
                        } else {
                            let _ = tx.send(StreamChunk::Error(msg));
                        }
                        return;
                    }
                    let mut buffer = String::new();
                    let mut tool_call_map: std::collections::HashMap<u32, serde_json::Value> = std::collections::HashMap::new();
                    loop {
                        match resp.chunk().await {
                            Ok(Some(chunk)) => {
                                buffer.push_str(&String::from_utf8_lossy(&chunk));
                                while let Some(pos) = buffer.find('\n') {
                                    let raw = buffer[..pos].trim().to_string();
                                    buffer = buffer[pos + 1..].to_string();
                                    if raw.is_empty() {
                                        continue;
                                    }
                                    let line = if is_cloud {
                                        raw.strip_prefix("data: ").unwrap_or(&raw).trim().to_string()
                                    } else {
                                        raw
                                    };
                                    log_to_file(is_logging, &log_file, &session_id, "RESPONSE", &line);
                                    if line == "[DONE]" {
                                        log_to_file(is_logging, &log_file, &session_id, "STREAM_END", "DONE sentinel");
                                        let _ = tx.send(StreamChunk::Done(TokenStats::default()));
                                        return;
                                    }
                                    if line.is_empty() {
                                        continue;
                                    }
                                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line) {
                                        if is_cloud {
                                            // Usage can ride on any chunk (nested under
                                            // choices[i] or top-level); forward it whenever present.
                                            if usage_from_json(&json).is_some() {
                                                let _ = tx.send(StreamChunk::Stats(parse_usage_stats(&json)));
                                            }
                                            if let Some(reasoning) = json["choices"][0]["delta"]["reasoning_content"].as_str()
                                                && !reasoning.is_empty() {
                                                    let _ = tx.send(StreamChunk::Thinking(reasoning.to_string()));
                                                }
                                            if let Some(delta) = json["choices"][0]["delta"]["content"].as_str()
                                                && !delta.is_empty() {
                                                    let _ = tx.send(StreamChunk::Text(delta.to_string()));
                                                }
                                            if let Some(tc_array) = json["choices"][0]["delta"]["tool_calls"].as_array() {
                                                for tc in tc_array {
                                                    let idx = tc["index"].as_u64().unwrap_or(0) as u32;
                                                    let entry = tool_call_map.entry(idx).or_insert_with(|| {
                                                        let mut base = serde_json::json!({
                                                            "index": idx,
                                                            "type": "function",
                                                            "function": {"name": "", "arguments": ""}
                                                        });
                                                        if let Some(id) = tc["id"].as_str() {
                                                            base["id"] = serde_json::json!(id);
                                                        }
                                                        base
                                                    });
                                                    if let Some(id) = tc["id"].as_str() {
                                                        entry["id"] = serde_json::json!(id);
                                                    }
                                                    if let Some(name) = tc["function"]["name"].as_str()
                                                        && !name.is_empty() {
                                                            entry["function"]["name"] = serde_json::json!(name);
                                                        }
                                                    if let Some(args) = tc["function"]["arguments"].as_str() {
                                                        let existing = entry["function"]["arguments"].as_str().unwrap_or("").to_string();
                                                        entry["function"]["arguments"] = serde_json::json!(format!("{}{}", existing, args));
                                                    }
                                                }
                                            }
                                            let finish = json["choices"][0]["finish_reason"].as_str();
                                            if finish == Some("stop") || finish == Some("tool_calls") {
                                                 let stats = parse_usage_stats(&json);
                                                log_to_file(is_logging, &log_file, &session_id, "STREAM_END", &format!("{} finish_reason={}", api_url, finish.unwrap_or("?")));
                                                if !tool_call_map.is_empty() {
                                                    if stats.prompt_tokens > 0 || stats.response_tokens > 0 {
                                                        let _ = tx.send(StreamChunk::Stats(stats));
                                                    }
                                                    let mut calls: Vec<serde_json::Value> = tool_call_map.into_values().collect();
                                                    calls.sort_by_key(|tc| tc["index"].as_u64().unwrap_or(0));
                                                    let _ = tx.send(StreamChunk::ToolCalls(calls));
                                                } else {
                                                    let _ = tx.send(StreamChunk::Done(stats));
                                                }
                                                return;
                                            }
                                            if finish == Some("length") || finish == Some("max_tokens") {
                                                let stats = parse_usage_stats(&json);
                                                log_to_file(is_logging, &log_file, &session_id, "STREAM_TRUNCATED", &format!("{} finish_reason={}", api_url, finish.unwrap_or("?")));
                                                let _ = tx.send(StreamChunk::Truncated(format!(
                                                    "Response truncated (finish_reason={}). Output limit reached.", finish.unwrap_or("?")
                                                )));
                                                let _ = tx.send(StreamChunk::Done(stats));
                                                return;
                                            }
                                        } else {
                                            if let Some(thinking) = json["message"]["thinking"].as_str()
                                                && !thinking.is_empty() {
                                                    let _ = tx.send(StreamChunk::Thinking(thinking.to_string()));
                                                }
                                            if let Some(content) = json["message"]["content"].as_str()
                                                && !content.is_empty() {
                                                    let _ = tx.send(StreamChunk::Text(content.to_string()));
                                                }
                                            if let Some(tool_calls) = json["message"]["tool_calls"].as_array() {
                                                for tc in tool_calls {
                                                    let idx = tc["index"].as_u64().unwrap_or(0) as u32;
                                                    let entry = tool_call_map.entry(idx).or_insert_with(|| {
                                                        serde_json::json!({
                                                            "index": idx,
                                                            "type": "function",
                                                            "function": {"name": "", "arguments": ""}
                                                        })
                                                    });
                                                    if let Some(name) = tc["function"]["name"].as_str() {
                                                        entry["function"]["name"] = serde_json::json!(name);
                                                    }
                                                    if let Some(id) = tc["id"].as_str() {
                                                        entry["id"] = serde_json::json!(id);
                                                    }
                                                    if let Some(args) = tc["function"]["arguments"].as_str() {
                                                        entry["function"]["arguments"] = serde_json::json!(args);
                                                    } else {
                                                        entry["function"]["arguments"] = serde_json::json!(tc["function"]["arguments"].to_string());
                                                    }
                                                }
                                            }
                                            if json["done"].as_bool() == Some(true) {
                                                log_to_file(is_logging, &log_file, &session_id, "STREAM_END", &format!("{} done=true tokens={}", api_url, json["eval_count"].as_u64().unwrap_or(0)));
                                                if !tool_call_map.is_empty() {
                                                    let calls: Vec<serde_json::Value> = tool_call_map.into_values().collect();
                                                    let _ = tx.send(StreamChunk::ToolCalls(calls));
                                                } else {
                                                    let stats = parse_token_stats(&json);
                                                    let _ = tx.send(StreamChunk::Done(stats));
                                                }
                                                return;
                                            }
                                        }
                                    }
                                }
                            }
                            Ok(None) => {
                                log_to_file(is_logging, &log_file, &session_id, "STREAM_END", "stream closed");
                                let _ = tx.send(StreamChunk::Done(TokenStats::default()));
                                return;
                            }
                            Err(e) => {
                                log_to_file(is_logging, &log_file, &session_id, "STREAM_ERROR", &format!("{} | buffer: {:?}", e, buffer));
                                let _ = tx.send(StreamChunk::Error(format!("Stream error: {} | last bytes: {:?}", e, truncate(&buffer, 200))));
                                return;
                            }
                        }
                    }
                }
                Err(e) => {
                    log_to_file(is_logging, &log_file, &session_id, "CONNECT_ERROR", &e.to_string());
                    let _ = tx.send(StreamChunk::Error(format!("Failed to connect: {}", e)));
                }
            }
        });
    });

    rx
}

#[cfg(test)]
mod chat_body_tests {
    use super::*;

    fn msgs() -> Vec<serde_json::Value> {
        vec![serde_json::json!({"role": "user", "content": "hi"})]
    }

    fn cloud() -> CloudModel {
        CloudModel {
            name: "test-cloud".to_string(),
            api_url: "https://example.com/v1/chat/completions".to_string(),
            api_key: "sk-test".to_string(),
            api_model: "test-cloud".to_string(),
            params: ModelParams::default(),
        }
    }

    #[test]
    fn ollama_body_carries_all_params() {
        let params = ModelParams {
            temperature: 0.7,
            top_p: 0.8,
            top_k: 20,
            frequency_penalty: 0.5,
            presence_penalty: -0.5,
            max_output_tokens: Some(4096),
            reasoning_effort: Some("high".to_string()),
            seed: Some(42),
        };
        let body = build_chat_body("llama3.2:3b", None, true, &msgs(), &params);
        assert_eq!(body["model"], "llama3.2:3b");
        let o = &body["options"];
        assert_eq!(o["temperature"], 0.7);
        assert_eq!(o["top_p"], 0.8);
        assert_eq!(o["top_k"], 20);
        assert_eq!(o["frequency_penalty"], 0.5);
        assert_eq!(o["presence_penalty"], -0.5);
        assert_eq!(o["num_predict"], 4096);
        assert_eq!(o["seed"], 42);
        assert_eq!(body["think"], "high");
        assert!(body["tools"].is_array(), "agentic attaches tools");
    }

    #[test]
    fn ollama_body_defaults_send_only_basics() {
        let params = ModelParams::default();
        let body = build_chat_body("m", None, false, &msgs(), &params);
        let o = &body["options"];
        assert_eq!(o["temperature"], 1.0);
        assert_eq!(o["top_p"], 0.9);
        assert_eq!(o["top_k"], 40);
        assert!(o.get("frequency_penalty").is_none());
        assert!(o.get("presence_penalty").is_none());
        assert!(o.get("seed").is_none());
        assert!(o.get("num_predict").is_none());
        assert!(body.get("think").is_none());
        assert!(body.get("tools").is_none(), "non-agentic: no tools");
    }

    #[test]
    fn ollama_think_bool_mapping() {
        let mut params = ModelParams::default();
        params.reasoning_effort = Some("off".to_string());
        assert_eq!(
            build_chat_body("m", None, false, &msgs(), &params)["think"],
            false
        );
        params.reasoning_effort = Some("on".to_string());
        assert_eq!(
            build_chat_body("m", None, false, &msgs(), &params)["think"],
            true
        );
        params.reasoning_effort = Some("low".to_string());
        assert_eq!(
            build_chat_body("m", None, false, &msgs(), &params)["think"],
            "low"
        );
    }

    #[test]
    fn cloud_body_carries_openai_fields() {
        let cloud = cloud();
        let params = ModelParams {
            temperature: 0.6,
            top_p: 0.95,
            top_k: 10, // not an OpenAI concept — must not be sent
            frequency_penalty: 0.3,
            presence_penalty: 0.1,
            max_output_tokens: Some(8192),
            reasoning_effort: Some("medium".to_string()),
            seed: Some(7),
        };
        let body = build_chat_body("test-cloud", Some(&cloud), true, &msgs(), &params);
        assert_eq!(body["model"], "test-cloud");
        assert_eq!(body["temperature"], 0.6);
        assert_eq!(body["top_p"], 0.95);
        assert_eq!(body["frequency_penalty"], 0.3);
        assert_eq!(body["presence_penalty"], 0.1);
        assert_eq!(body["max_tokens"], 8192);
        assert_eq!(body["seed"], 7);
        assert_eq!(body["reasoning_effort"], "medium");
        assert!(body.get("top_k").is_none());
        assert!(body.get("options").is_none());
        assert!(body.get("think").is_none());
        assert!(body["tools"].is_array());
    }

    #[test]
    fn cloud_body_skips_unset_and_toggle_effort() {
        let cloud = cloud();
        let mut params = ModelParams::default();
        let body = build_chat_body("test-cloud", Some(&cloud), true, &msgs(), &params);
        assert!(body.get("max_tokens").is_none());
        assert!(body.get("frequency_penalty").is_none());
        assert!(body.get("presence_penalty").is_none());
        assert!(body.get("seed").is_none());
        assert!(body.get("reasoning_effort").is_none());

        // "off"/"on" are toggles, not levels — cloud APIs reject them.
        params.reasoning_effort = Some("off".to_string());
        let body = build_chat_body("test-cloud", Some(&cloud), true, &msgs(), &params);
        assert!(body.get("reasoning_effort").is_none());
    }
}
