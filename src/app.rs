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
use std::process::{Child, Stdio};
use std::sync::{Arc, Mutex, mpsc};
use std::thread::JoinHandle;

use crate::config::{CloudModel, Config, load_cloud_models};

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

#[derive(Debug, Clone, PartialEq)]
pub enum SettingsFocus {
    Proxy,
    OllamaUrl,
    Temperature,
    TopP,
    TopK,
    MaxToolRounds,
    MaxRetries,
    Justify,
    Save,
    Cancel,
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

impl ExportFormat {
    pub fn name(&self) -> &'static str {
        match self {
            ExportFormat::Markdown => "MD",
            ExportFormat::PlainText => "PLAIN TEXT",
        }
    }
    pub fn next(&self) -> ExportFormat {
        match self {
            ExportFormat::Markdown => ExportFormat::PlainText,
            ExportFormat::PlainText => ExportFormat::Markdown,
        }
    }
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
    Status(String),
    StatusTick(String),
    ToolCalls(Vec<serde_json::Value>),
    Truncated(String),
    /// Token usage mid-stream (e.g. attached to the finish chunk before
    /// tool calls are executed). `check_responses` folds it into the stats
    /// that the next `Done`/`ToolCalls` transition reports.
    Stats(TokenStats),
}

const TERMINAL_BUFFER_MAX: usize = 102400;

pub struct TerminalBuffer {
    pub content: String,
    pub total_bytes_written: u64,
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

    pub fn read_incremental(&self, cursor: Option<u64>) -> serde_json::Value {
        let total = self.total_bytes_written;
        let start = self.buffer_start_offset;

        match cursor {
            None => {
                let output = if self.content.len() > 4000 {
                    self.content[self.content.len() - 4000..].to_string()
                } else {
                    self.content.clone()
                };
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
                        "error": format!("Invalid cursor {}: beyond total bytes written ({})", c, total),
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
                    let output = if offset < self.content.len() {
                        self.content[offset..].to_string()
                    } else {
                        String::new()
                    };
                    serde_json::json!({
                        "output": output,
                        "cursor": total,
                        "gap": false,
                    })
                } else {
                    let output = if self.content.len() > 4000 {
                        self.content[self.content.len() - 4000..].to_string()
                    } else {
                        self.content.clone()
                    };
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

pub struct TerminalState {
    pub buffer: Arc<Mutex<TerminalBuffer>>,
    pub stdin_tx: Option<std::sync::mpsc::Sender<String>>,
    pub child: Arc<Mutex<Option<Child>>>,
    pub reader_stdout: Option<JoinHandle<()>>,
    pub reader_stderr: Option<JoinHandle<()>>,
    pub stdin_writer: Option<JoinHandle<()>>,
    pub visible: bool,
    pub width_pct: u16,
    pub command: String,
}

/// Shell reserved words / builtins that must not be prefixed with `stdbuf`.
const SHELL_KEYWORDS: &[&str] = &[
    "if", "then", "elif", "else", "fi", "for", "while", "until", "do", "done", "case", "esac",
    "in", "function", "select", "time", "coproc", "!", "{", "}", "(", ")", "[[", "]]", "cd",
    "echo", "export", "pwd", "set", "unset", "shift", "read", "printf", "return", "exit", "eval",
    "exec", "source", "alias", "unalias", "declare", "typeset", "local", "readonly", "trap",
    "wait", "jobs", "bg", "fg", "kill", "history", "let", "pushd", "popd", "dirs", "umask",
    "ulimit", "test", "true", "false", "break", "continue",
];

/// Line-buffers the spawned command's output so long-running scripts (builds,
/// python/node -c, ...) show output promptly even without a pty.
///
/// Verified: `PYTHONUNBUFFERED=1` fixes python; `stdbuf -oL -eL` fixes any
/// program (line buffering via LD_PRELOAD). Together they handle script and
/// progress output.
///
/// FIXME: interactive REPLs (python3 -q, node, ...) cannot work through pipes
/// at all — they buffer piped stdin until EOF regardless of stdbuf/-u. Full
/// PTY support (e.g. the `portable-pty` crate) is required for that, and for
/// readline programs, top, less, ssh, ... . Should eventually replace this
/// pipe-based TerminalState implementation.
fn terminal_command(command: &str) -> String {
    let first = command.split_whitespace().next().unwrap_or("");
    let simple = !first.is_empty()
        && !SHELL_KEYWORDS.contains(&first)
        && !command.contains([';', '&', '|', '<', '>', '$', '`', '\'', '"', '\n'])
        && !command.starts_with(['{', '(', '!']);
    if simple {
        format!("stdbuf -oL -eL {}", command)
    } else {
        command.to_string()
    }
}

impl TerminalState {
    pub fn new() -> Self {
        TerminalState {
            buffer: Arc::new(Mutex::new(TerminalBuffer::new())),
            stdin_tx: None,
            child: Arc::new(Mutex::new(None)),
            reader_stdout: None,
            reader_stderr: None,
            stdin_writer: None,
            visible: false,
            width_pct: 40,
            command: String::new(),
        }
    }

    pub fn is_running(&self) -> bool {
        if self.stdin_tx.is_none() {
            return false;
        }
        // Check if child process has exited
        let mut child = self.child.lock().unwrap();
        if let Some(ref mut c) = *child
            && let Ok(Some(_)) = c.try_wait()
        {
            return false;
        }
        true
    }

    pub fn open(&mut self, command: &str) -> String {
        if self.is_running() {
            return format!("Terminal already running: {}", self.command);
        }

        let trimmed = command.trim_start();
        if trimmed.starts_with("sudo ") || trimmed == "sudo" {
            return "Error: sudo is not permitted.".to_string();
        }

        let (stdin_tx, stdin_rx) = std::sync::mpsc::channel::<String>();
        let buffer = Arc::new(Mutex::new(TerminalBuffer::new()));

        // Always spawn bash — commands are sent through stdin.
        // This keeps one process, one buffer, one monotonic cursor
        // across the entire session lifecycle.
        match std::process::Command::new("bash")
            .env("PYTHONUNBUFFERED", "1")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::piped())
            .spawn()
        {
            Ok(mut child) => {
                let mut stdin = child.stdin.take().expect("stdin was piped");
                let stdout = child.stdout.take().expect("stdout was piped");
                let stderr = child.stderr.take().expect("stderr was piped");

                let stdin_writer = std::thread::spawn(move || {
                    use std::io::Write;
                    while let Ok(msg) = stdin_rx.recv() {
                        if stdin.write_all(msg.as_bytes()).is_err() {
                            break;
                        }
                        let _ = stdin.flush();
                    }
                });

                let reader_stdout = {
                    let buffer = buffer.clone();
                    std::thread::spawn(move || {
                        let mut buf = [0u8; 4096];
                        let mut out = stdout;
                        loop {
                            match out.read(&mut buf) {
                                Ok(0) => break,
                                Ok(n) => {
                                    if let Ok(text) = String::from_utf8(buf[..n].to_vec()) {
                                        let mut b = buffer.lock().unwrap();
                                        b.total_bytes_written += text.len() as u64;
                                        b.content.push_str(&text);
                                        let len = b.content.len();
                                        if len > TERMINAL_BUFFER_MAX {
                                            let drain_to = len - TERMINAL_BUFFER_MAX / 2;
                                            b.buffer_start_offset += drain_to as u64;
                                            b.content = b.content.split_off(drain_to);
                                        }
                                    }
                                }
                                Err(_) => break,
                            }
                        }
                    })
                };

                let reader_stderr = {
                    let buffer = buffer.clone();
                    std::thread::spawn(move || {
                        let mut buf = [0u8; 4096];
                        let mut err = stderr;
                        loop {
                            match err.read(&mut buf) {
                                Ok(0) => break,
                                Ok(n) => {
                                    if let Ok(text) = String::from_utf8(buf[..n].to_vec()) {
                                        let mut b = buffer.lock().unwrap();
                                        b.total_bytes_written += text.len() as u64;
                                        b.content.push_str(&text);
                                        let len = b.content.len();
                                        if len > TERMINAL_BUFFER_MAX {
                                            let drain_to = len - TERMINAL_BUFFER_MAX / 2;
                                            b.buffer_start_offset += drain_to as u64;
                                            b.content = b.content.split_off(drain_to);
                                        }
                                    }
                                }
                                Err(_) => break,
                            }
                        }
                    })
                };

                self.buffer = buffer;
                self.stdin_tx = Some(stdin_tx);
                *self.child.lock().unwrap() = Some(child);
                self.reader_stdout = Some(reader_stdout);
                self.reader_stderr = Some(reader_stderr);
                self.stdin_writer = Some(stdin_writer);
                self.visible = true;

                // If a command was provided, send it to the shell
                if !command.trim().is_empty() {
                    self.command = command.to_string();
                    let _ = self.send_input(command);
                    format!("Terminal opened running: {}", command)
                } else {
                    self.command = "bash".to_string();
                    "Terminal opened: bash".to_string()
                }
            }
            Err(e) => format!("Error opening terminal: {}", e),
        }
    }

    pub fn send_input(&mut self, input: &str) -> String {
        if let Some(ref tx) = self.stdin_tx {
            match tx.send(format!("{}\n", input)) {
                Ok(()) => format!("Sent: {}", input),
                Err(_) => "Error: terminal process has exited".to_string(),
            }
        } else {
            "Error: no terminal running".to_string()
        }
    }

    pub fn read_buffer_incremental(&self, cursor: Option<u64>) -> serde_json::Value {
        let b = self.buffer.lock().unwrap();
        b.read_incremental(cursor)
    }

    pub fn close(&mut self) -> String {
        self.stdin_tx.take();
        if let Some(mut child) = self.child.lock().unwrap().take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        if let Some(handle) = self.stdin_writer.take() {
            let _ = handle.join();
        }
        if let Some(handle) = self.reader_stdout.take() {
            let _ = handle.join();
        }
        if let Some(handle) = self.reader_stderr.take() {
            let _ = handle.join();
        }
        self.visible = false;
        self.command.clear();
        *self.buffer.lock().unwrap() = TerminalBuffer::new();
        "Terminal closed".to_string()
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
    pub temperature: f64,
    pub top_p: f64,
    pub top_k: u32,
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
    /// Token usage received mid-stream (via `StreamChunk::Stats`) that has not
    /// been committed to `token_stats` yet.
    last_stats: TokenStats,
}

impl App {
    pub fn new(cfg: Config) -> Self {
        let textarea = TextArea::default();
        let clipboard = Clipboard::new().ok();
        rotate_log_file(&cfg.logfile);
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
            temperature: cfg.temperature,
            top_p: cfg.top_p,
            top_k: cfg.top_k,
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
            settings_temperature: "1.0".to_string(),
            settings_top_p: "0.9".to_string(),
            settings_top_k: "40".to_string(),
            settings_max_tool_rounds: cfg.max_tool_rounds.to_string(),
            settings_max_retries: cfg.max_retries.to_string(),
            settings_cursor: 0,
            proxy: cfg.proxy.clone(),
            is_logging: cfg.logging,
            log_file: cfg.logfile,
            token_stats: TokenStats::default(),
            cloud_models: load_cloud_models(),
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

        match key.code {
            KeyCode::F(9) => {
                self.open_menu();
                return;
            }
            KeyCode::F(10) => {
                self.open_quit_confirm();
                return;
            }
            KeyCode::Char('t' | 'T') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if self.terminal_state.is_running() {
                    self.terminal_state.visible = !self.terminal_state.visible;
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
                } else if key.modifiers == KeyModifiers::CONTROL {
                    if !self.textarea.lines().join("").trim().is_empty() && !self.is_loading {
                        self.send_to_ollama_async();
                    }
                } else {
                    if !self.textarea.lines().join("").trim().is_empty() && !self.is_loading {
                        self.send_to_ollama_async();
                    }
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
                let cursor = self
                    .terminal_state
                    .buffer
                    .lock()
                    .unwrap()
                    .total_bytes_written;
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
                    Ok(StreamChunk::Status(msg)) => {
                        self.status_message = msg;
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
                            "⚠ {} Increase max_output_tokens in cloud_models.conf.",
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
                            self.messages.push(ChatMessage::Assistant(text));
                            self.streaming_thinking.clear();
                            self.streaming_text.clear();
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
        let temperature = self.temperature;
        let top_p = self.top_p;
        let top_k = self.top_k;
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
            temperature,
            top_p,
            top_k,
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
        let temperature = self.temperature;
        let top_p = self.top_p;
        let top_k = self.top_k;
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
            temperature,
            top_p,
            top_k,
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
        let default_path = dirs_home()
            .join(".config/rustama")
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

    pub fn save_proxy_to_config(&self) -> Result<(), String> {
        let mut cfg = Config::default();
        cfg.ollama_url = self.ollama_url.clone();
        cfg.model = self.model_name.clone();
        cfg.save_path = self.save_path.clone();
        cfg.agentic = self.agentic_mode;
        cfg.logging = self.is_logging;
        cfg.logfile = self.log_file.clone();
        cfg.system_prompt = self.system_prompt.clone();
        cfg.proxy = self.proxy.clone();
        cfg.max_tool_rounds = self.max_tool_rounds;
        cfg.max_retries = self.max_retries;
        cfg.temperature = self.temperature;
        cfg.top_p = self.top_p;
        cfg.top_k = self.top_k;
        cfg.terminal_width_pct = self.terminal_state.width_pct;
        cfg.save()
    }

    pub fn save_session(&self) -> Result<(), String> {
        let sessions_dir = dirs_home().join(".config/rustama");
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
        let sessions_dir = dirs_home().join(".config/rustama");
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
        }
        self.status_message = format!("Loaded session {}", sess_id);
        Ok(())
    }

    fn open_load_session_dialog(&mut self) {
        self.file_dialog_mode = FileDialogMode::LoadSession;
        self.file_dialog_path = dirs_home().join(".config/rustama");
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

    fn handle_load_dialog_click(&mut self, col: u16, row: u16, width: u16, height: u16) {
        let dialog_w: u16 = 60;
        let dialog_h: u16 = 6;
        let dialog_x = (width.saturating_sub(dialog_w)) / 2;
        let dialog_y = (height.saturating_sub(dialog_h)) / 2;

        if col < dialog_x
            || col >= dialog_x + dialog_w
            || row < dialog_y
            || row >= dialog_y + dialog_h
        {
            self.show_load_dialog = false;
            return;
        }

        let rel_x = col - dialog_x;
        let rel_y = row - dialog_y;

        if rel_y == 3 {
            self.load_dialog_path.clear();
        }

        let btn_y = dialog_h - 2;
        let load_btn = Button::new("Load", 10, btn_y, true, Color::Green, Color::Green);
        let cancel_btn = Button::new("Cancel", 22, btn_y, true, Color::Red, Color::Red);

        if load_btn.is_clicked(rel_x, rel_y) {
            self.execute_load_session();
            return;
        }
        if cancel_btn.is_clicked(rel_x, rel_y) {
            self.show_load_dialog = false;
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
            DialogHit::TextInput(_) => {
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
            DialogHit::Button(_, bi) => {
                let action_idx = 1;
                if bi == action_idx {
                    self.execute_export();
                } else {
                    self.show_format_dropdown = false;
                    self.show_save_dialog = false;
                }
            }
            DialogHit::FileListItem(_, _) | DialogHit::None => {}
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

    fn scrollbar_click_to(&mut self, row: u16, input_start: u16) {
        self.auto_scroll = false;
        let output_height = input_start.saturating_sub(1) as f64;
        let total_lines = self.cached_wrapped.len() as f64;
        let visible_height = self.terminal_height.saturating_sub(4) as f64;
        let max_scroll = (total_lines - visible_height).max(0.0);
        if output_height <= 0.0 || max_scroll <= 0.0 {
            return;
        }
        if row <= 1 {
            self.scroll_offset = self.scroll_offset.saturating_sub(1);
        } else if row as f64 >= output_height {
            self.scroll_offset = (self.scroll_offset + 1).min(max_scroll as u16);
        } else {
            let ratio = (row as f64 - 1.0) / output_height;
            self.scroll_offset = (ratio * max_scroll).round() as u16;
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

    fn confirm_model_selection(&mut self) {
        if let Some(model) = self.available_models.get(self.model_dialog_selection) {
            self.model_name = model.clone();
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
        self.settings_temperature = format!("{}", self.temperature);
        self.settings_top_p = format!("{}", self.top_p);
        self.settings_top_k = format!("{}", self.top_k);
        self.settings_max_tool_rounds = self.max_tool_rounds.to_string();
        self.settings_justify = self.justify;
    }

    fn handle_settings_dialog_key(&mut self, key: KeyEvent) {
        let is_text_field = matches!(
            self.settings_focus,
            SettingsFocus::Proxy
                | SettingsFocus::OllamaUrl
                | SettingsFocus::Temperature
                | SettingsFocus::TopP
                | SettingsFocus::TopK
                | SettingsFocus::MaxToolRounds
                | SettingsFocus::MaxRetries
        );
        let is_toggle = self.settings_focus == SettingsFocus::Justify;

        match key.code {
            KeyCode::Esc => {
                self.show_settings_dialog = false;
            }
            KeyCode::Tab => {
                self.settings_cursor = 0;
                self.settings_focus = match self.settings_focus {
                    SettingsFocus::Proxy => SettingsFocus::OllamaUrl,
                    SettingsFocus::OllamaUrl => SettingsFocus::Temperature,
                    SettingsFocus::Temperature => SettingsFocus::TopP,
                    SettingsFocus::TopP => SettingsFocus::TopK,
                    SettingsFocus::TopK => SettingsFocus::MaxToolRounds,
                    SettingsFocus::MaxToolRounds => SettingsFocus::MaxRetries,
                    SettingsFocus::MaxRetries => SettingsFocus::Justify,
                    SettingsFocus::Justify => SettingsFocus::Save,
                    SettingsFocus::Save => SettingsFocus::Cancel,
                    SettingsFocus::Cancel => SettingsFocus::Proxy,
                };
            }
            KeyCode::BackTab => {
                self.settings_cursor = 0;
                self.settings_focus = match self.settings_focus {
                    SettingsFocus::Proxy => SettingsFocus::Cancel,
                    SettingsFocus::OllamaUrl => SettingsFocus::Proxy,
                    SettingsFocus::Temperature => SettingsFocus::OllamaUrl,
                    SettingsFocus::TopP => SettingsFocus::Temperature,
                    SettingsFocus::TopK => SettingsFocus::TopP,
                    SettingsFocus::MaxToolRounds => SettingsFocus::TopK,
                    SettingsFocus::MaxRetries => SettingsFocus::MaxToolRounds,
                    SettingsFocus::Justify => SettingsFocus::MaxRetries,
                    SettingsFocus::Save => SettingsFocus::Justify,
                    SettingsFocus::Cancel => SettingsFocus::Save,
                };
            }
            KeyCode::Up => {
                self.settings_cursor = 0;
                self.settings_focus = match self.settings_focus {
                    SettingsFocus::Proxy => SettingsFocus::Cancel,
                    SettingsFocus::OllamaUrl => SettingsFocus::Proxy,
                    SettingsFocus::Temperature => SettingsFocus::OllamaUrl,
                    SettingsFocus::TopP => SettingsFocus::Temperature,
                    SettingsFocus::TopK => SettingsFocus::TopP,
                    SettingsFocus::MaxToolRounds => SettingsFocus::TopK,
                    SettingsFocus::MaxRetries => SettingsFocus::MaxToolRounds,
                    SettingsFocus::Justify => SettingsFocus::MaxRetries,
                    SettingsFocus::Save => SettingsFocus::Justify,
                    SettingsFocus::Cancel => SettingsFocus::Save,
                };
            }
            KeyCode::Down => {
                self.settings_cursor = 0;
                self.settings_focus = match self.settings_focus {
                    SettingsFocus::Proxy => SettingsFocus::OllamaUrl,
                    SettingsFocus::OllamaUrl => SettingsFocus::Temperature,
                    SettingsFocus::Temperature => SettingsFocus::TopP,
                    SettingsFocus::TopP => SettingsFocus::TopK,
                    SettingsFocus::TopK => SettingsFocus::MaxToolRounds,
                    SettingsFocus::MaxToolRounds => SettingsFocus::MaxRetries,
                    SettingsFocus::MaxRetries => SettingsFocus::Justify,
                    SettingsFocus::Justify => SettingsFocus::Save,
                    SettingsFocus::Save => SettingsFocus::Cancel,
                    SettingsFocus::Cancel => SettingsFocus::Proxy,
                };
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
        if let Ok(v) = self.settings_temperature.trim().parse::<f64>()
            && (0.0..=2.0).contains(&v)
        {
            self.temperature = v;
        }
        if let Ok(v) = self.settings_top_p.trim().parse::<f64>()
            && (0.0..=1.0).contains(&v)
        {
            self.top_p = v;
        }
        if let Ok(v) = self.settings_top_k.trim().parse::<u32>()
            && (1..=100).contains(&v)
        {
            self.top_k = v;
        }
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
        let _ = self.save_proxy_to_config();
        self.show_settings_dialog = false;
        self.status_message = "Settings saved".to_string();
    }

    fn handle_settings_dialog_click(&mut self, col: u16, row: u16, width: u16, height: u16) {
        let dialog_w: u16 = 60;
        let dialog_h: u16 = 22;
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
                    self.settings_focus = focus.clone();
                    return;
                }
            }
        }

        let num_fields = 8u16;
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

    fn open_file_dialog(&mut self) {
        self.show_file_dialog = true;
        self.file_dialog_selection = 0;
        self.file_dialog_scroll = 0;
        self.file_dialog_focus = FileDialogFocus::List;
        self.refresh_file_list();
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
            DialogHit::FileListItem(_, idx) => {
                self.file_dialog_selection = idx;
                self.file_dialog_focus = FileDialogFocus::List;
            }
            DialogHit::Button(_, bi) => {
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
                        self.temperature = v;
                        format!("Temperature set to {}", v)
                    }
                    Ok(_) => "Value must be between 0.0 and 2.0".to_string(),
                    Err(_) => "Usage: /temp <number>".to_string(),
                },
                None => format!(
                    "Current temperature: {} (usage: /temp <number>)",
                    self.temperature
                ),
            }),
            "topp" => Some(match arg {
                Some(n) => match n.parse::<f64>() {
                    Ok(v) if (0.0..=1.0).contains(&v) => {
                        self.top_p = v;
                        format!("Top_p set to {}", v)
                    }
                    Ok(_) => "Value must be between 0.0 and 1.0".to_string(),
                    Err(_) => "Usage: /topp <number>".to_string(),
                },
                None => format!("Current top_p: {} (usage: /topp <number>)", self.top_p),
            }),
            "topk" => Some(match arg {
                Some(n) => match n.parse::<u32>() {
                    Ok(v) if (1..=100).contains(&v) => {
                        self.top_k = v;
                        format!("Top_k set to {}", v)
                    }
                    Ok(_) => "Value must be between 1 and 100".to_string(),
                    Err(_) => "Usage: /topk <number>".to_string(),
                },
                None => format!("Current top_k: {} (usage: /topk <number>)", self.top_k),
            }),
            "proxy" => Some(match arg {
                Some("off") | Some("none") | Some("") => {
                    self.proxy = None;
                    let _ = self.save_proxy_to_config();
                    "Proxy disabled".to_string()
                }
                Some(url) => {
                    self.proxy = Some(url.to_string());
                    let _ = self.save_proxy_to_config();
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
                    format!("Model set to: {}", name)
                } else if self.cloud_models.iter().any(|m| m.name == name) {
                    self.model_name = name.to_string();
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
        format!(
            "Config file: {}\n\n  ollama_url     = {}\n  model          = {}\n  save_path      = {}\n  agentic        = {}\n  max_rounds     = {}\n  timeout_secs   = {}\n  logging        = {}\n  logfile        = {}\n  temperature    = {}\n  top_p          = {}\n  top_k          = {}\n  justify        = {}",
            conf_path.display(),
            self.ollama_url,
            self.model_name,
            self.save_path,
            self.agentic_mode,
            self.max_tool_rounds,
            300,
            self.is_logging,
            self.log_file,
            self.temperature,
            self.top_p,
            self.top_k,
            self.justify,
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
        format!(
            "Status:\n  Session ID:    {}\n  Session Name:  {}\n  Model:         {}\n  Messages:      {}\n  Logging:       {}\n  Log file:      {}\n  Agentic mode:  {}\n  Available:     {} model(s)\n  Tool calls:    {}\n  System prompt: {}\n  Temperature:   {}\n  Top_p:         {}\n  Top_k:         {}\n  Justify:       {}",
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
            self.temperature,
            self.top_p,
            self.top_k,
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
            .map(|m| m.max_output_tokens);

        if let Some(max) = max_output {
            if max > 0 {
                let pct = s.response_tokens as f64 / max as f64 * 100.0;
                parts.push(format!("{}/{} ({:.0}%)", s.response_tokens, max, pct));
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

fn patch_tool_call_ids(msgs: &mut Vec<ChatMessage>) {
    for i in 0..msgs.len() {
        if let ChatMessage::ToolCall {
            name, tool_call_id, ..
        } = &msgs[i]
            && tool_call_id.is_none()
        {
            let id = format!("call_{}", name);
            msgs[i] = match msgs[i].clone() {
                ChatMessage::ToolCall {
                    name, arguments, ..
                } => ChatMessage::ToolCall {
                    name,
                    arguments,
                    tool_call_id: Some(id),
                },
                other => other,
            };
        }
        if let ChatMessage::ToolResult {
            name, tool_call_id, ..
        } = &msgs[i]
            && tool_call_id.is_none()
        {
            let id = format!("call_{}", name);
            msgs[i] = match msgs[i].clone() {
                ChatMessage::ToolResult { name, content, .. } => ChatMessage::ToolResult {
                    name,
                    content,
                    tool_call_id: Some(id),
                },
                other => other,
            };
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
                "description": "Open a terminal session to run a command. The terminal panel becomes visible so you can observe the output. Returns JSON with 'status' (message) and 'cursor' (pass this to terminal_read for incremental reads). Use terminal_send to interact and terminal_read to check output. Use terminal_close when done.",
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
                "description": "Send input (keystrokes) to the running terminal session. Use this to interact with programs, answer prompts, press Enter, etc.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "input": {
                            "type": "string",
                            "description": "The text to send to the terminal (a newline is appended automatically)"
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
                "description": "Read output from the terminal session. Returns JSON with 'output' (the text), 'cursor' (a position to pass on the next read for incremental output), and 'gap' (true if some output was lost due to buffer overflow). On the first read, omit 'cursor'. On subsequent reads, pass the 'cursor' value from the previous response to get only new output.",
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
#[cfg(test)]
mod tests {
    use super::{StreamChunk, retry_countdown, send_with_retry, terminal_command};

    #[test]
    fn wraps_simple_command() {
        assert_eq!(terminal_command("python3 -q"), "stdbuf -oL -eL python3 -q");
        assert_eq!(terminal_command("ls -la"), "stdbuf -oL -eL ls -la");
        assert_eq!(
            terminal_command("cmake --build ."),
            "stdbuf -oL -eL cmake --build ."
        );
    }

    #[test]
    fn leaves_shell_composites_alone() {
        assert_eq!(terminal_command("cd /tmp && make"), "cd /tmp && make");
        assert_eq!(terminal_command("echo hi"), "echo hi");
        assert_eq!(
            terminal_command("for i in 1 2; do echo $i; done"),
            "for i in 1 2; do echo $i; done"
        );
        assert_eq!(terminal_command("ls | grep foo"), "ls | grep foo");
        assert_eq!(terminal_command("cat < file"), "cat < file");
        assert_eq!(terminal_command("FOO=1 bar"), "stdbuf -oL -eL FOO=1 bar");
        assert_eq!(terminal_command(""), "");
    }

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
                stream.read(&mut buf).unwrap();
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
                    StreamChunk::Status(s) | StreamChunk::StatusTick(s) => Some(s),
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
        assert_eq!(result["gap"].as_bool().unwrap(), false);
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
        assert_eq!(result["gap"].as_bool().unwrap(), false);
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
        assert_eq!(result["gap"].as_bool().unwrap(), true);
    }

    #[test]
    fn cursor_at_buffer_start_returns_all() {
        let b = make_buffer("abc", 1003, 1000);
        let result = b.read_incremental(Some(1000));
        assert_eq!(result["output"].as_str().unwrap(), "abc");
        assert_eq!(result["gap"].as_bool().unwrap(), false);
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
        assert_eq!(result["gap"].as_bool().unwrap(), false);
    }

    #[test]
    fn cursor_just_before_buffer_start_is_gap() {
        let content = "x".repeat(5000);
        let b = make_buffer(&content, 10000, 5000);
        let result = b.read_incremental(Some(4999));
        assert_eq!(result["gap"].as_bool().unwrap(), true);
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
        std::thread::sleep(std::time::Duration::from_millis(500));

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

        std::thread::sleep(std::time::Duration::from_millis(500));

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

        std::thread::sleep(std::time::Duration::from_millis(500));

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
    temperature: f64,
    top_p: f64,
    top_k: u32,
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

            let (api_url, headers, body) = if let Some(ref cloud) = cloud_model {
                let mut body = serde_json::json!({
                    "model": cloud.api_model,
                    "messages": api_messages,
                    "stream": true,
                    "max_tokens": cloud.max_output_tokens,
                });
                body["tools"] = serde_json::json!(get_tool_definitions());
                body["temperature"] = serde_json::json!(temperature);
                body["top_p"] = serde_json::json!(top_p);
                let mut headers = reqwest::header::HeaderMap::new();
                headers.insert(
                    "Authorization",
                    reqwest::header::HeaderValue::from_str(&format!("Bearer {}", cloud.api_key)).unwrap(),
                );
                headers.insert(
                    "Content-Type",
                    reqwest::header::HeaderValue::from_static("application/json"),
                );
                (cloud.api_url.clone(), Some(headers), body)
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
                options["temperature"] = serde_json::json!(temperature);
                options["top_p"] = serde_json::json!(top_p);
                if top_k > 0 {
                    options["top_k"] = serde_json::json!(top_k);
                }
                body["options"] = options;
                (format!("{}/api/chat", url), None, body)
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
