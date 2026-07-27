use crate::ui::{Button, DialogDropdownState, DialogHit, FileActionDialog, Theme};
use ratatui::layout::Rect;
use serde::{Deserialize, Serialize};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::style::Color;
use ratatui::text::Line;
use ratatui_textarea::TextArea;
use arboard::Clipboard;
use std::path::PathBuf;
use std::sync::mpsc;

use crate::config::{Config, CloudModel, load_cloud_models};

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
pub enum ActiveMenu {
    None,
    File,
    Options,
    Help,
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
    pub total_duration_ms: u64,
    pub tokens_per_sec: f64,
}

#[derive(Debug, Clone)]
pub enum StreamChunk {
    Text(String),
    Thinking(String),
    Done(TokenStats),
    Error(String),
    ToolCalls(Vec<serde_json::Value>),
}

pub struct App {
    pub messages: Vec<ChatMessage>,
    pub textarea: TextArea<'static>,
    pub clipboard: Option<Clipboard>,
    pub scroll_offset: u16,
    pub auto_scroll: bool,
    pub should_quit: bool,
    pub input_mode: InputMode,
    pub focus: Focus,
    pub active_menu: ActiveMenu,
    pub menu_selection: usize,
    pub show_about: bool,
    pub show_quit_confirm: bool,
    pub is_loading: bool,
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
    pub tool_round_count: usize,
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
    pub session_id: String,
    pub session_name: String,
}

impl App {
    pub fn new(cfg: Config) -> Self {
        let mut textarea = TextArea::default();
        textarea.set_line_number_style(ratatui::style::Style::default());
        let clipboard = Clipboard::new().ok();
        let mut app = App {
            messages: vec![ChatMessage::App(
                "Welcome to Rustama. Start typing your message.".to_string(),
            )],
            textarea,
            clipboard,
            scroll_offset: 0,
            auto_scroll: true,
            should_quit: false,
            input_mode: InputMode::Input,
            focus: Focus::Input,
            active_menu: ActiveMenu::None,
            menu_selection: 0,
            show_about: false,
            show_quit_confirm: false,
            is_loading: false,
            status_message: String::new(),
            ollama_url: cfg.ollama_url,
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
            max_tool_rounds: 10,
            tool_round_count: 0,
            pending_tool_calls: Vec::new(),
            tool_call_log: Vec::new(),
            temperature: 1.0,
            top_p: 0.9,
            top_k: 40,
            show_save_dialog: false,
            save_dialog_path: cfg.save_path,
            save_dialog_cursor: 0,
            save_dialog_focus: SaveDialogFocus::Path,
            save_dialog_mode: SaveDialogMode::ExportChat,
            show_format_dropdown: false,
            show_load_dialog: false,
            load_dialog_path: dirs_home().to_string_lossy().to_string(),
            load_dialog_cursor: 0,
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
            session_id: format!("{:016x}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() & 0xffff_ffff_ffff_ffff),
            session_name: String::new(),
        };
        app.session_name = app.session_id.clone();
        app.fetch_models_async();
        app
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }

        match key.code {
            KeyCode::F(9) => {
                self.open_menu();
                return;
            }
            KeyCode::F(10) => {
                self.show_quit_confirm = true;
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
        match self.input_mode {
            InputMode::Normal => self.handle_normal_key(key),
            InputMode::Input => self.handle_input_key(key),
            InputMode::Menu => self.handle_menu_key(key),
        }
    }

    fn handle_normal_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Tab => self.open_menu(),
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.open_export_dialog();
            }
            KeyCode::Up | KeyCode::Char('k') => self.scroll_up(),
            KeyCode::Down | KeyCode::Char('j') => self.scroll_down(),
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
                if key.modifiers == KeyModifiers::ALT
                {
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
            if m.name.to_lowercase().starts_with(&partial.to_lowercase()) && !matches.contains(&m.name) {
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
                self.textarea.set_line_number_style(ratatui::style::Style::default());
                for ch in new_text.chars() {
                    self.textarea.insert_char(ch);
                }
            }
            _ => {
                let common = common_prefix(&matches);
                if common.len() > partial.len() {
                    let new_text = format!("{}{}", cmd_prefix, common);
                    self.textarea = TextArea::default();
                    self.textarea.set_line_number_style(ratatui::style::Style::default());
                    for ch in new_text.chars() {
                        self.textarea.insert_char(ch);
                    }
                }
                self.status_message = format!("Matches: {}", matches.join(", "));
            }
        }
    }

    fn handle_menu_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.active_menu = ActiveMenu::None;
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Left => {
                self.active_menu = match self.active_menu {
                    ActiveMenu::File => ActiveMenu::Help,
                    ActiveMenu::Options => ActiveMenu::File,
                    ActiveMenu::Help => ActiveMenu::Options,
                    ActiveMenu::None => ActiveMenu::File,
                };
                self.menu_selection = 0;
            }
            KeyCode::Right => {
                self.active_menu = match self.active_menu {
                    ActiveMenu::File => ActiveMenu::Options,
                    ActiveMenu::Options => ActiveMenu::Help,
                    ActiveMenu::Help => ActiveMenu::File,
                    ActiveMenu::None => ActiveMenu::File,
                };
                self.menu_selection = 0;
            }
            KeyCode::Up => {
                self.menu_selection = self.menu_selection.saturating_sub(1);
            }
            KeyCode::Down => {
                let max = self.max_menu_items();
                if max > 0 && self.menu_selection < max - 1 {
                    self.menu_selection += 1;
                }
            }
            KeyCode::Enter => self.execute_menu_selection(),
            _ => {}
        }
    }

    fn open_menu(&mut self) {
        self.active_menu = ActiveMenu::File;
        self.menu_selection = 0;
        self.input_mode = InputMode::Menu;
    }

    fn max_menu_items(&self) -> usize {
        match self.active_menu {
            ActiveMenu::File => 4,
            ActiveMenu::Options => 2,
            ActiveMenu::Help => 1,
            ActiveMenu::None => 0,
        }
    }

    fn execute_menu_selection(&mut self) {
        match (&self.active_menu, self.menu_selection) {
            (ActiveMenu::File, 0) => self.open_load_session_dialog(),
            (ActiveMenu::File, 1) => self.open_save_dialog(),
            (ActiveMenu::File, 2) => self.open_export_dialog(),
            (ActiveMenu::File, 3) => self.show_quit_confirm = true,
            (ActiveMenu::Help, 0) => self.show_about = true,
            (ActiveMenu::Options, 0) => self.open_model_dialog(),
            (ActiveMenu::Options, 1) => {
                self.agentic_mode = !self.agentic_mode;
                self.status_message = if self.agentic_mode {
                    "Agentic mode: ON".to_string()
                } else {
                    "Agentic mode: OFF".to_string()
                };
            }
            _ => {}
        }
        self.active_menu = ActiveMenu::None;
        self.input_mode = InputMode::Normal;
    }

    pub fn scroll_up(&mut self) {
        self.auto_scroll = false;
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    pub fn scroll_down(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_add(1);
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
                    }
                    Ok(StreamChunk::Thinking(text)) => {
                        self.streaming_thinking.push_str(&text);
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

                            let tool_call_id = tc["id"].as_str().map(|s| s.to_string());

                            self.messages.push(ChatMessage::ToolCall {
                                name: name.clone(),
                                arguments: args.clone(),
                                tool_call_id: tool_call_id.clone(),
                            });

                            let result = execute_tool_call(&name, &args);
                            self.tool_call_log
                                .push((name.clone(), args.clone(), result.clone()));
                            self.log_event("TOOL_CALL", &format!("{}({}) -> {}", name, args, truncate(&result, 500)));

                            self.messages.push(ChatMessage::ToolResult {
                                name,
                                content: result,
                                tool_call_id,
                            });
                        }
                        self.pending_tool_calls = tool_calls;
                        self.is_loading = false;
                        self.status_message.clear();
                        self.response_rx = None;
                        self.tool_round_count += 1;
                        if self.tool_round_count >= self.max_tool_rounds {
                            self.messages.push(ChatMessage::App(
                                format!("Reached max tool rounds ({}). Stopping.", self.max_tool_rounds),
                            ));
                            self.set_auto_scroll();
                            break;
                        }
                        self.send_tool_results_async();
                        break;
                    }
                    Ok(StreamChunk::Done(stats)) => {
                        self.token_stats = stats;
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
                                    self.messages.push(ChatMessage::ToolCall {
                                        name: name.clone(),
                                        arguments: args.clone(),
                                        tool_call_id: None,
                                    });
                                    let result = execute_tool_call(&name, &args);
                                    self.tool_call_log.push((name.clone(), args.clone(), result.clone()));
                                    self.log_event("TOOL_CALL", &format!("{}({}) -> {}", name, args, truncate(&result, 500)));
                                    self.messages.push(ChatMessage::ToolResult {
                                        name,
                                        content: result,
                                        tool_call_id: None,
                                    });
                                }
                                self.streaming_thinking.clear();
                                self.streaming_text.clear();
                                self.is_loading = false;
                                self.status_message.clear();
                                self.response_rx = None;
                                self.tool_round_count += 1;
                                if self.tool_round_count >= self.max_tool_rounds {
                                    self.messages.push(ChatMessage::App(
                                        format!("Reached max tool rounds ({}). Stopping.", self.max_tool_rounds),
                                    ));
                                    self.set_auto_scroll();
                                    break;
                                }
                                self.send_tool_results_async();
                                break;
                            }
                            self.log_event("ASSISTANT", &text);
                            self.messages
                                .push(ChatMessage::Assistant(text));
                            self.streaming_thinking.clear();
                            self.streaming_text.clear();
                        }
                        self.is_loading = false;
                        self.status_message.clear();
                        self.response_rx = None;
                        self.set_auto_scroll();
                        break;
                    }
                    Ok(StreamChunk::Error(msg)) => {
                        self.messages.push(ChatMessage::App(msg));
                        self.is_loading = false;
                        self.status_message.clear();
                        self.response_rx = None;
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
                ChatMessage::Thinking(_) => {}
                ChatMessage::FileContent { name, content } => {
                    api_messages.push(serde_json::json!({
                        "role": "user",
                        "content": format!("Here is the content of `{}`:\n\n{}", name, content)
                    }));
                }
                ChatMessage::ToolCall { name, arguments, tool_call_id } => {
                    let mut tc = serde_json::json!({
                        "function": {
                            "name": name,
                            "arguments": serde_json::from_str::<serde_json::Value>(arguments).unwrap_or(serde_json::json!({}))
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
                ChatMessage::ToolResult { name, content, tool_call_id } => {
                    let mut msg = serde_json::json!({
                        "role": "tool",
                        "content": content,
                    });
                    if is_cloud {
                        if let Some(id) = tool_call_id {
                            msg["tool_call_id"] = serde_json::json!(id);
                        } else {
                            msg["name"] = serde_json::json!(name);
                        }
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

        self.log_event("PROMPT", &serde_json::to_string_pretty(&api_messages).unwrap_or_default());

        let (tx, rx) = mpsc::channel();
        self.response_rx = Some(rx);

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            rt.block_on(async {
                let client = reqwest::Client::new();

                let (api_url, headers, body) = if let Some(ref cloud) = cloud_model {
                    let mut body = serde_json::json!({
                        "model": cloud.api_model,
                        "messages": api_messages,
                        "stream": true,
                    });
                    body["tools"] = serde_json::json!(get_tool_definitions());
                    body["temperature"] = serde_json::json!(temperature);
                    body["top_p"] = serde_json::json!(top_p);
                    if top_k > 0 {
                        body["top_k"] = serde_json::json!(top_k);
                    }
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
                        "tools": get_tool_definitions(),
                    });
                    let mut options = serde_json::json!({});
                    options["temperature"] = serde_json::json!(temperature);
                    options["top_p"] = serde_json::json!(top_p);
                    if top_k > 0 {
                        options["top_k"] = serde_json::json!(top_k);
                    }
                    body["options"] = options;
                    (format!("{}/api/chat", url), None, body)
                };

                let mut req = client
                    .post(&api_url)
                    .json(&body)
                    .timeout(std::time::Duration::from_secs(300));

                if let Some(h) = headers {
                    req = req.headers(h);
                }

                log_to_file(is_logging, &log_file, &session_id, "REQUEST", &format!("{} {}", api_url, serde_json::to_string(&body).unwrap_or_default()));

                let is_cloud = cloud_model.is_some();
                stream_with_retry(req, tx, is_cloud, is_logging, &log_file, &session_id, &api_url).await;
            });
        });
    }

    fn send_to_ollama_async(&mut self) {
        let prompt = self.textarea.lines().join("\n").trim().to_string();
        self.textarea = TextArea::default();
        self.textarea.set_line_number_style(ratatui::style::Style::default());

        if let Some(result) = self.handle_slash_command(&prompt) {
            self.messages.push(ChatMessage::App(result));
            self.set_auto_scroll();
            return;
        }

        self.messages.push(ChatMessage::User(prompt));
        self.tool_round_count = 0;
        self.is_loading = true;
        self.streaming_text.clear();
        self.streaming_thinking.clear();
        self.status_message = "Streaming response...".to_string();

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
                ChatMessage::Thinking(_) => {}
                ChatMessage::FileContent { name, content } => {
                    api_messages.push(serde_json::json!({
                        "role": "user",
                        "content": format!("Here is the content of `{}`:\n\n{}", name, content)
                    }));
                }
                ChatMessage::ToolCall { name, arguments, tool_call_id } => {
                    let mut tc = serde_json::json!({
                        "function": {
                            "name": name,
                            "arguments": serde_json::from_str::<serde_json::Value>(arguments).unwrap_or(serde_json::json!({}))
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
                ChatMessage::ToolResult { name, content, tool_call_id } => {
                    let mut msg = serde_json::json!({
                        "role": "tool",
                        "content": content,
                    });
                    if is_cloud {
                        if let Some(id) = tool_call_id {
                            msg["tool_call_id"] = serde_json::json!(id);
                        } else {
                            msg["name"] = serde_json::json!(name);
                        }
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

        self.log_event("PROMPT", &serde_json::to_string_pretty(&api_messages).unwrap_or_default());

        let (tx, rx) = mpsc::channel();
        self.response_rx = Some(rx);

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            rt.block_on(async {
                let client = reqwest::Client::new();

                let (api_url, headers, body) = if let Some(ref cloud) = cloud_model {
                    let mut body = serde_json::json!({
                        "model": cloud.api_model,
                        "messages": api_messages,
                        "stream": true,
                    });
                    body["tools"] = serde_json::json!(get_tool_definitions());
                    body["temperature"] = serde_json::json!(temperature);
                    body["top_p"] = serde_json::json!(top_p);
                    if top_k > 0 {
                        body["top_k"] = serde_json::json!(top_k);
                    }
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
                    if is_cloud {
                        body["temperature"] = serde_json::json!(temperature);
                        body["top_p"] = serde_json::json!(top_p);
                        if top_k > 0 {
                            body["top_k"] = serde_json::json!(top_k);
                        }
                    } else {
                        let mut options = serde_json::json!({});
                        options["temperature"] = serde_json::json!(temperature);
                        options["top_p"] = serde_json::json!(top_p);
                        if top_k > 0 {
                            options["top_k"] = serde_json::json!(top_k);
                        }
                        body["options"] = options;
                    }
                    (format!("{}/api/chat", url), None, body)
                };

                let mut req = client
                    .post(&api_url)
                    .json(&body)
                    .timeout(std::time::Duration::from_secs(300));

                if let Some(h) = headers {
                    req = req.headers(h);
                }

                log_to_file(is_logging, &log_file, &session_id, "REQUEST", &format!("{} {}", api_url, serde_json::to_string(&body).unwrap_or_default()));

                let is_cloud = cloud_model.is_some();
                stream_with_retry(req, tx, is_cloud, is_logging, &log_file, &session_id, &api_url).await;
            });
        });
    }

    pub fn export_output(&mut self) {
        let is_md = self.export_format == ExportFormat::Markdown;
        let mut content = String::new();
        for msg in &self.messages {
            match msg {
                ChatMessage::User(t) => {
                    if is_md { content.push_str(&format!("**User:** {}\n\n", t)); }
                    else { content.push_str(&format!("User: {}\n\n", t)); }
                }
                ChatMessage::Assistant(t) => {
                    if is_md { content.push_str(&format!("**Assistant:** {}\n\n", t)); }
                    else { content.push_str(&format!("Assistant: {}\n\n", t)); }
                }
                ChatMessage::System(t) => {
                    if is_md { content.push_str(&format!("_{}_\n\n", t)); }
                    else { content.push_str(&format!("System: {}\n\n", t)); }
                }
                ChatMessage::App(t) => { content.push_str(&format!("{}\n\n", t)); }
                ChatMessage::Thinking(t) => {
                    if is_md { content.push_str(&format!("_Thinking:_ {}\n\n", t)); }
                    else { content.push_str(&format!("Thinking: {}\n\n", t)); }
                }
                ChatMessage::FileContent { name, content: file_content } => {
                    if is_md { content.push_str(&format!("**File:** `{}`\n\n{}\n\n", name, file_content)); }
                    else { content.push_str(&format!("File: {}\n{}\n\n", name, file_content)); }
                }
                ChatMessage::ToolCall { name, arguments, .. } => {
                    if is_md { content.push_str(&format!("**Tool Call:** `{}`\n```\n{}\n```\n\n", name, arguments)); }
                    else { content.push_str(&format!("Tool Call: {}\n{}\n\n", name, arguments)); }
                }
                ChatMessage::ToolResult { name, content: result, .. } => {
                    if is_md { content.push_str(&format!("**Tool Result:** `{}`\n```\n{}\n```\n\n", name, result)); }
                    else { content.push_str(&format!("Tool Result: {}\n{}\n\n", name, result)); }
                }
            }
        }
        if !self.streaming_text.is_empty() {
            if is_md { content.push_str(&format!("**Assistant:** {} _(streaming in progress)_\n\n", self.streaming_text)); }
            else { content.push_str(&format!("Assistant: {} (streaming in progress)\n\n", self.streaming_text)); }
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

    pub fn save_session(&self) -> Result<(), String> {
        let sessions_dir = dirs_home().join(".config/rustama");
        std::fs::create_dir_all(&sessions_dir).map_err(|e| e.to_string())?;
        let path = sessions_dir.join(format!("{}.session.rustama", self.session_name));
        let data = serde_json::json!({
            "session_id": self.session_id,
            "messages": self.messages,
        });
        let json = serde_json::to_string_pretty(&data).map_err(|e| e.to_string())?;
        std::fs::write(&path, &json).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn load_session(&mut self, sess_id: &str) -> Result<(), String> {
        let sessions_dir = dirs_home().join(".config/rustama");
        let path = sessions_dir.join(format!("{}.session.rustama", sess_id));
        let json = std::fs::read_to_string(&path).map_err(|e| format!("Session not found: {}", e))?;
        let data: serde_json::Value = serde_json::from_str(&json).map_err(|e| e.to_string())?;
        let msgs: Vec<ChatMessage> = serde_json::from_value(data["messages"].clone())
            .map_err(|e| format!("Invalid session data: {}", e))?;
        self.messages = msgs;
        self.streaming_text.clear();
        self.session_name = sess_id.to_string();
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
        let msgs: Vec<ChatMessage> = match serde_json::from_value(data["messages"].clone()) {
            Ok(m) => m,
            Err(e) => {
                self.status_message = format!("Invalid session data: {}", e);
                return;
            }
        };
        self.messages = msgs;
        self.streaming_text.clear();
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
                let idx = self
                    .load_dialog_cursor
                    .min(chars.len());
                chars.insert(idx, c);
                self.load_dialog_path = chars.into_iter().collect();
                self.load_dialog_cursor += 1;
            }
            KeyCode::Backspace => {
                if self.load_dialog_cursor > 0 {
                    let mut chars: Vec<char> = self.load_dialog_path.chars().collect();
                    let idx = self.load_dialog_cursor.saturating_sub(1).min(chars.len().saturating_sub(1));
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
        let cancel_btn = Button::new(
            "Cancel",
            22,
            btn_y,
            true,
            Color::Red,
            Color::Red,
        );

        if load_btn.is_clicked(rel_x, rel_y) {
            self.execute_load_session();
            return;
        }
        if cancel_btn.is_clicked(rel_x, rel_y) {
            self.show_load_dialog = false;
            return;
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
        let area = Rect { x: 0, y: 0, width, height };

        let dlg = match self.save_dialog_mode {
            SaveDialogMode::SaveSession => {
                let mut d = FileActionDialog::new("Save Session");
                d.add_text_input("Session file:", &self.save_dialog_path, self.save_dialog_cursor, self.save_dialog_focus == SaveDialogFocus::Path);
                d.add_button("Cancel", self.save_dialog_focus == SaveDialogFocus::Cancel);
                d.add_button("Save", self.save_dialog_focus == SaveDialogFocus::Save);
                d
            }
            SaveDialogMode::ExportChat => {
                let mut d = FileActionDialog::new("Export As");
                d.add_text_input("File path:", &self.save_dialog_path, self.save_dialog_cursor, self.save_dialog_focus == SaveDialogFocus::Path);
                let fmt_state = DialogDropdownState {
                    focused: self.save_dialog_focus == SaveDialogFocus::Format,
                    expanded: self.show_format_dropdown,
                    selected: if self.export_format == ExportFormat::Markdown { 0 } else { 1 },
                };
                d.add_dropdown("Format", vec!["Markdown".to_string(), "Plain Text".to_string()], fmt_state);
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

    pub fn menu_item_names(&self) -> Vec<&'static str> {
        match self.active_menu {
            ActiveMenu::File => vec!["Load", "Save", "Export...", "Exit"],
            ActiveMenu::Options => vec!["Set Model", "Agentic Mode"],
            ActiveMenu::Help => vec!["About"],
            ActiveMenu::None => vec![],
        }
    }

    pub fn handle_click(&mut self, col: u16, row: u16, width: u16, height: u16) {
        if self.show_save_dialog {
            self.handle_save_dialog_click(col, row, width, height);
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

        if row == 0 {
            let new_menu = match col {
                0..=6 => ActiveMenu::File,
                7..=16 => ActiveMenu::Options,
                _ => ActiveMenu::Help,
            };
            if self.active_menu == new_menu {
                self.active_menu = ActiveMenu::None;
                self.input_mode = InputMode::Normal;
            } else {
                self.active_menu = new_menu;
                self.menu_selection = 0;
                self.input_mode = InputMode::Menu;
            }
            return;
        }

        if self.active_menu != ActiveMenu::None {
            let x_offset = match self.active_menu {
                ActiveMenu::File => 0,
                ActiveMenu::Options => 7,
                ActiveMenu::Help => 17,
                _ => 0,
            };
            let item_count = self.max_menu_items() as u16;
            if row >= 2 && row < 2 + item_count {
                if col >= x_offset && col < x_offset + 20 {
                    self.menu_selection = (row - 2) as usize;
                    self.execute_menu_selection();
                    return;
                }
            }
            self.active_menu = ActiveMenu::None;
            self.input_mode = InputMode::Normal;
            return;
        }

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
            self.focus = Focus::Output;
            self.input_mode = InputMode::Normal;
        }
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
                let client = reqwest::Client::new();
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
                    if let Some(idx) = self.available_models.iter().position(|m| m == &self.model_name)
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
                    self.model_dialog_selection =
                        self.model_dialog_selection.saturating_sub(1);
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
            return;
        }
    }

    fn handle_about_dialog_click(&mut self, col: u16, row: u16, width: u16, height: u16) {
        let dialog_w: u16 = 50;
        let dialog_h: u16 = 10;
        let dialog_x = (width.saturating_sub(dialog_w)) / 2;
        let dialog_y = (height.saturating_sub(dialog_h)) / 2;
        let inner_x = dialog_x + 1;
        let inner_y = dialog_y + 1;
        let inner_h = dialog_h - 2;

        if col < dialog_x
            || col >= dialog_x + dialog_w
            || row < dialog_y
            || row >= dialog_y + dialog_h
        {
            self.show_about = false;
            return;
        }

        let btn_y = inner_y + inner_h - 1;
        let ok_btn = Button::new(
            "OK",
            inner_x + 20,
            btn_y,
            true,
            Color::Cyan,
            Color::Cyan,
        );

        if ok_btn.is_clicked(col, row) {
            self.show_about = false;
            return;
        }
    }

    fn handle_quit_confirm_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
                self.show_quit_confirm = false;
            }
            KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
                self.should_quit = true;
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
        let dialog_w: u16 = 40;
        let dialog_h: u16 = 8;
        let dialog_x = (width.saturating_sub(dialog_w)) / 2;
        let dialog_y = (height.saturating_sub(dialog_h)) / 2;
        let inner_x = dialog_x + 1;
        let inner_y = dialog_y + 1;
        let inner_h = dialog_h - 2;

        if col < dialog_x
            || col >= dialog_x + dialog_w
            || row < dialog_y
            || row >= dialog_y + dialog_h
        {
            self.show_quit_confirm = false;
            return;
        }

        let btn_y = inner_y + inner_h - 1;
        let yes_btn = Button::new("Yes", inner_x + 9, btn_y, true, Color::Cyan, Color::Cyan);
        let no_btn = Button::new("No", inner_x + 16, btn_y, true, Color::Cyan, Color::Cyan);

        if yes_btn.is_clicked(col, row) {
            self.should_quit = true;
            return;
        }
        if no_btn.is_clicked(col, row) {
            self.show_quit_confirm = false;
            return;
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
                if self.file_dialog_mode == FileDialogMode::LoadSession && !is_dir && !name.ends_with(".session.rustama") {
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
        if let Some((name, is_dir)) = self.file_dialog_entries.get(self.file_dialog_selection).cloned() {
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
                let msgs: Vec<ChatMessage> = match serde_json::from_value(data["messages"].clone()) {
                    Ok(m) => m,
                    Err(e) => {
                        self.status_message = format!("Invalid session data: {}", e);
                        return;
                    }
                };
                self.messages = msgs;
                self.streaming_text.clear();
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
                if self.file_dialog_focus == FileDialogFocus::Cancel {
                    self.file_dialog_focus = FileDialogFocus::Open;
                }
            }
            KeyCode::Right => {
                if self.file_dialog_focus == FileDialogFocus::Open {
                    self.file_dialog_focus = FileDialogFocus::Cancel;
                }
            }
            KeyCode::Up => {
                if self.file_dialog_focus == FileDialogFocus::List
                    && !self.file_dialog_entries.is_empty()
                {
                    self.file_dialog_selection =
                        self.file_dialog_selection.saturating_sub(1);
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
                        let is_dir = self.file_dialog_entries
                            .get(self.file_dialog_selection)
                            .map(|(_, d)| *d)
                            .unwrap_or(false);
                        if is_dir {
                            let name = self.file_dialog_entries
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

    fn handle_file_dialog_click(&mut self, col: u16, row: u16, width: u16, height: u16) {
        let dialog_w: u16 = 60;
        let entry_count = self.file_dialog_entries.len().min(12) as u16;
        let dialog_h = entry_count + 6;
        let dialog_x = (width.saturating_sub(dialog_w)) / 2;
        let dialog_y = (height.saturating_sub(dialog_h)) / 2;
        let inner_x = dialog_x + 1;
        let inner_y = dialog_y + 1;

        if col < dialog_x
            || col >= dialog_x + dialog_w
            || row < dialog_y
            || row >= dialog_y + dialog_h
        {
            self.show_file_dialog = false;
            return;
        }

        let list_h = self.file_dialog_entries.len().min(12) as u16;
        if row >= inner_y + 1 && row < inner_y + 1 + list_h {
            let idx = (row - inner_y - 1 + self.file_dialog_scroll as u16) as usize;
            if idx < self.file_dialog_entries.len() {
                self.file_dialog_selection = idx;
                self.file_dialog_focus = FileDialogFocus::List;
            }
            return;
        }

        let btn_y = inner_y + list_h + 1;
        let btn_label = if self.file_dialog_mode == FileDialogMode::LoadSession { "Load" } else { "Open" };
        let open_btn = Button::new(
            btn_label,
            inner_x + 8,
            btn_y,
            self.file_dialog_focus == FileDialogFocus::Open,
            Color::Green,
            Color::Green,
        );
        let cancel_btn = Button::new(
            "Cancel",
            inner_x + 19,
            btn_y,
            self.file_dialog_focus == FileDialogFocus::Cancel,
            Color::Red,
            Color::Red,
        );

        if open_btn.is_clicked(col, row) {
            self.file_dialog_focus = FileDialogFocus::Open;
            self.open_selected_file();
            return;
        }
        if cancel_btn.is_clicked(col, row) {
            self.show_file_dialog = false;
            return;
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
                    Ok(v) if v >= 1 && v <= 100 => {
                        self.max_tool_rounds = v;
                        format!("Max tool rounds set to {}", v)
                    }
                    Ok(_) => "Value must be between 1 and 100".to_string(),
                    Err(_) => "Usage: /maxrounds <number>".to_string(),
                },
                None => format!("Current max tool rounds: {} (usage: /maxrounds <number>)", self.max_tool_rounds),
            }),
            "temp" => Some(match arg {
                Some(n) => match n.parse::<f64>() {
                    Ok(v) if v >= 0.0 && v <= 2.0 => {
                        self.temperature = v;
                        format!("Temperature set to {}", v)
                    }
                    Ok(_) => "Value must be between 0.0 and 2.0".to_string(),
                    Err(_) => "Usage: /temp <number>".to_string(),
                },
                None => format!("Current temperature: {} (usage: /temp <number>)", self.temperature),
            }),
            "topp" => Some(match arg {
                Some(n) => match n.parse::<f64>() {
                    Ok(v) if v >= 0.0 && v <= 1.0 => {
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
                    Ok(v) if v >= 1 && v <= 100 => {
                        self.top_k = v;
                        format!("Top_k set to {}", v)
                    }
                    Ok(_) => "Value must be between 1 and 100".to_string(),
                    Err(_) => "Usage: /topk <number>".to_string(),
                },
                None => format!("Current top_k: {} (usage: /topk <number>)", self.top_k),
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
                    } else if new_name.contains('/') || new_name.contains('\\') || new_name.contains("..") {
                        "Invalid session name: no path separators or '..' allowed".to_string()
                    } else {
                        self.session_name = new_name.to_string();
                        format!("Session renamed to {}", new_name)
                    }
                }
                Some(load_arg) => {
                    match self.load_session(load_arg) {
                        Ok(()) => format!("Loaded session {}", load_arg),
                        Err(e) => format!("Load failed: {}", e),
                    }
                }
                _ => "Usage: /session save | /session rename <name> | /session load <name>".to_string(),
            }),
            "quit" | "q" | "exit" => {
                self.show_quit_confirm = true;
                None
            }
            _ => Some(format!("Unknown command: /{}. Type /help for available commands.", cmd)),
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
            ("/session save", "Save current session"),
            ("/session rename <name>", "Rename current session"),
            ("/session load <name>", "Load a session by name"),
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
                    let mut all: Vec<&str> = self.available_models.iter().map(|s| s.as_str()).collect();
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
                        .or_else(|| self.cloud_models.iter().position(|m| m.name == self.model_name))
                        .unwrap_or(0);
                    format!("Current model: {}. Select a new one in the dialog.", self.model_name)
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
            "Config file: {}\n\n  ollama_url     = {}\n  model          = {}\n  save_path      = {}\n  agentic        = {}\n  max_rounds     = {}\n  timeout_secs   = {}\n  logging        = {}\n  logfile        = {}\n  temperature    = {}\n  top_p          = {}\n  top_k          = {}",
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
            "Status:\n  Session ID:    {}\n  Session Name:  {}\n  Model:         {}\n  Messages:      {}\n  Logging:       {}\n  Log file:      {}\n  Agentic mode:  {}\n  Available:     {} model(s)\n  Tool calls:    {}\n  System prompt: {}\n  Temperature:   {}\n  Top_p:         {}\n  Top_k:         {}",
            self.session_id,
            self.session_name,
            self.model_name,
            msg_count,
            if self.is_logging { "ON" } else { "OFF" },
            self.log_file,
            if self.agentic_mode { "ON" } else { "OFF" },
            model_count,
            tool_calls,
            if self.system_prompt.is_empty() { "(none)".to_string() } else { format!("{} chars", self.system_prompt.len()) },
            self.temperature,
            self.top_p,
            self.top_k,
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
        log_to_file(self.is_logging, &self.log_file, &self.session_id, kind, content);
    }
}

async fn stream_with_retry(
    req: reqwest::RequestBuilder,
    tx: mpsc::Sender<StreamChunk>,
    is_cloud: bool,
    is_logging: bool,
    log_file: &str,
    session_id: &str,
    api_url: &str,
) {
    let max_retries = 3u32;
    let mut resp: Option<reqwest::Response> = None;
    for attempt in 1..=max_retries {
        let result: Result<reqwest::Response, reqwest::Error> = req.try_clone().unwrap().send().await;
        match result {
            Ok(r) if r.status().as_u16() == 429 => {
                let retry_after: u64 = r.headers()
                    .get("retry-after")
                    .and_then(|v: &reqwest::header::HeaderValue| v.to_str().ok())
                    .and_then(|s: &str| s.parse::<u64>().ok())
                    .unwrap_or(2);
                let msg = format!("⚠ Rate limited (429), retrying in {}s (attempt {}/{})", retry_after, attempt, max_retries);
                log_to_file(is_logging, log_file, session_id, "RATE_LIMIT", &msg);
                let _ = tx.send(StreamChunk::Text(format!("\n{}\n", msg)));
                tokio::time::sleep(std::time::Duration::from_secs(retry_after)).await;
                continue;
            }
            Ok(r) => {
                resp = Some(r);
                break;
            }
            Err(e) => {
                log_to_file(is_logging, log_file, session_id, "CONNECT_ERROR", &e.to_string());
                let _ = tx.send(StreamChunk::Error(format!("Failed to connect: {}", e)));
                return;
            }
        }
    }
    let Some(mut resp) = resp else {
        let msg = format!("Rate limited (429) after {} retries, giving up", max_retries);
        log_to_file(is_logging, log_file, session_id, "RATE_LIMIT", &msg);
        let _ = tx.send(StreamChunk::Error(msg));
        return;
    };

    let mut buffer = String::new();
    let mut collected_tool_calls: Vec<serde_json::Value> = Vec::new();
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
                    log_to_file(is_logging, log_file, session_id, "RESPONSE", &line);
                    if line == "[DONE]" {
                        log_to_file(is_logging, log_file, session_id, "STREAM_END", "DONE sentinel");
                        let _ = tx.send(StreamChunk::Done(TokenStats::default()));
                        return;
                    }
                    if line.is_empty() {
                        continue;
                    }
                    match serde_json::from_str::<serde_json::Value>(&line) {
                        Ok(json) => {
                            if is_cloud {
                                if let Some(delta) = json["choices"][0]["delta"]["content"].as_str() {
                                    if !delta.is_empty() {
                                        let _ = tx.send(StreamChunk::Text(delta.to_string()));
                                    }
                                }
                                if let Some(tc_array) = json["choices"][0]["delta"]["tool_calls"].as_array() {
                                    for tc in tc_array {
                                        collected_tool_calls.push(tc.clone());
                                    }
                                }
                                let finish = json["choices"][0]["finish_reason"].as_str();
                                if finish == Some("stop") || finish == Some("tool_calls") {
                                    let stats = parse_usage_stats(&json);
                                    log_to_file(is_logging, log_file, session_id, "STREAM_END", &format!("{} finish_reason={}", api_url, finish.unwrap_or("?")));
                                    if !collected_tool_calls.is_empty() {
                                        let _ = tx.send(StreamChunk::ToolCalls(collected_tool_calls));
                                    } else {
                                        let _ = tx.send(StreamChunk::Done(stats));
                                    }
                                    return;
                                }
                            } else {
                                if let Some(thinking) = json["message"]["thinking"].as_str() {
                                    if !thinking.is_empty() {
                                        let _ = tx.send(StreamChunk::Thinking(thinking.to_string()));
                                    }
                                }
                                if let Some(content) = json["message"]["content"].as_str() {
                                    if !content.is_empty() {
                                        let _ = tx.send(StreamChunk::Text(content.to_string()));
                                    }
                                }
                                if let Some(tool_calls) = json["message"]["tool_calls"].as_array() {
                                    for tc in tool_calls {
                                        collected_tool_calls.push(tc.clone());
                                    }
                                }
                                if json["done"].as_bool() == Some(true) {
                                    log_to_file(is_logging, log_file, session_id, "STREAM_END", &format!("{} done=true tokens={}", api_url, json["eval_count"].as_u64().unwrap_or(0)));
                                    if !collected_tool_calls.is_empty() {
                                        let _ = tx.send(StreamChunk::ToolCalls(collected_tool_calls));
                                    } else {
                                        let stats = parse_token_stats(&json);
                                        let _ = tx.send(StreamChunk::Done(stats));
                                    }
                                    return;
                                }
                            }
                        }
                        Err(_) => {}
                    }
                }
            }
            Ok(None) => {
                log_to_file(is_logging, log_file, session_id, "STREAM_END", "stream closed");
                let _ = tx.send(StreamChunk::Done(TokenStats::default()));
                return;
            }
            Err(e) => {
                log_to_file(is_logging, log_file, session_id, "STREAM_ERROR", &e.to_string());
                let _ = tx.send(StreamChunk::Error(format!("Stream error: {}", e)));
                return;
            }
        }
    }
}

fn log_to_file(is_logging: bool, log_file: &str, session_id: &str, kind: &str, content: &str) {
    if !is_logging {
        return;
    }
    use std::fs::OpenOptions;
    use std::io::Write;
    let ts = chrono_now();
    let mut file = match OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file)
    {
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
        total_duration_ms: json["total_duration"].as_u64().unwrap_or(0) / 1_000_000,
        tokens_per_sec,
    }
}

fn parse_usage_stats(json: &serde_json::Value) -> TokenStats {
    TokenStats {
        prompt_tokens: json["usage"]["prompt_tokens"].as_u64().unwrap_or(0),
        response_tokens: json["usage"]["completion_tokens"].as_u64().unwrap_or(0),
        total_duration_ms: 0,
        tokens_per_sec: 0.0,
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
    ]
}

fn execute_tool_call(name: &str, args_json: &str) -> String {
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
                                if let Some(ref mut out) = child.stdout { let _ = out.read_to_string(&mut stdout); }
                                if let Some(ref mut err) = child.stderr { let _ = err.read_to_string(&mut stderr); }
                                let mut result = String::new();
                                if !stdout.is_empty() { result.push_str(&stdout); }
                                if !stderr.is_empty() {
                                    if !result.is_empty() { result.push('\n'); }
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
                            if is_dir {
                                format!("{}/", name)
                            } else {
                                name
                            }
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
        _ => format!("Unknown tool: {}", name),
    }
}

fn grep_regex(pattern: &str, path: &str, include: &str) -> Result<Vec<String>, String> {
    let re = regex::Regex::new(pattern).map_err(|e| format!("Invalid regex: {}", e))?;
    let glob_pattern = format!("{}/{}", path, include);

    let mut results = Vec::new();
    if let Ok(entries) = glob::glob(&glob_pattern) {
        for entry in entries.flatten() {
            if entry.is_file() {
                if let Ok(content) = std::fs::read_to_string(&entry) {
                    for (line_num, line) in content.lines().enumerate() {
                        if re.is_match(line) {
                            results.push(format!(
                                "{}:{}: {}",
                                entry.display(),
                                line_num + 1,
                                line
                            ));
                        }
                    }
                }
            }
        }
    }
    Ok(results)
}

const TOOL_NAMES: &[&str] = &["read_file", "write_file", "edit_file", "bash", "list_files", "search_files", "search_content"];

fn parse_text_tool_calls(text: &str) -> Vec<serde_json::Value> {
    let mut tool_calls = Vec::new();

    let patterns = [
        (r#"\{"name"\s*:\s*"(\w+)"\s*,\s*"parameters"\s*:\s*(\{[^}]*\})\}"#, "name", "parameters"),
        (r#"\{"name"\s*:\s*"(\w+)"\s*,\s*"arguments"\s*:\s*(\{[^}]*\})\}"#, "name", "arguments"),
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
                        tc["parameters"] = serde_json::from_str(&args_str).unwrap_or(serde_json::json!({}));
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

                if TOOL_NAMES.contains(&name.as_str()) {
                    if let Ok(args_json) = serde_json::from_str::<serde_json::Value>(&args_str) {
                        let tc = serde_json::json!({
                            "name": name,
                            "parameters": args_json,
                        });
                        tool_calls.push(tc);
                    }
                }
            }
        }
    }

    tool_calls
}