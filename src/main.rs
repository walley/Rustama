use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, MouseButton, MouseEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap, Clear, Scrollbar, ScrollbarState, ScrollbarOrientation},
    Frame, Terminal,
};
use serde::{Deserialize, Serialize};
use std::io;
use std::fs;
use std::fs::OpenOptions;
use std::io::{Write, BufRead, BufReader};
use std::time::Duration;
use tokio::sync::mpsc;
use std::collections::HashMap;
use std::process;
use futures_util::stream::StreamExt;

// ============ CONFIG FILE ============

const CONFIG_FILE: &str = "config.keys";

fn load_api_keys() -> HashMap<String, String> {
    let mut keys = HashMap::new();
    
    if let Ok(file) = fs::File::open(CONFIG_FILE) {
        let reader = BufReader::new(file);
        for line in reader.lines() {
            if let Ok(line) = line {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                // Format: "cloud@model:key"
                if let Some((model, key)) = line.split_once(':') {
                    keys.insert(model.trim().to_string(), key.trim().to_string());
                }
            }
        }
    }
    
    keys
}

// ============ CLOUD MODELS CONFIGURATION ============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudModelConfig {
    pub name: String,
    pub display_name: String,
    pub api_type: CloudApiType,
    pub model_id: String,
    pub base_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CloudApiType {
    DeepSeek,
    Mistral,
}

impl CloudModelConfig {
    fn get_default_configs() -> Vec<Self> {
        vec![
            CloudModelConfig {
                name: "cloud@deepseek".to_string(),
                display_name: "DeepSeek Chat".to_string(),
                api_type: CloudApiType::DeepSeek,
                model_id: "deepseek-chat".to_string(),
                base_url: "https://api.deepseek.com".to_string(),
            },
            CloudModelConfig {
                name: "cloud@mistral".to_string(),
                display_name: "Mistral Medium".to_string(),
                api_type: CloudApiType::Mistral,
                model_id: "mistral-medium-3.5".to_string(),
                base_url: "https://api.mistral.ai".to_string(),
            },
            CloudModelConfig {
                name: "cloud@deepseek-coder".to_string(),
                display_name: "DeepSeek Coder".to_string(),
                api_type: CloudApiType::DeepSeek,
                model_id: "deepseek-coder".to_string(),
                base_url: "https://api.deepseek.com".to_string(),
            },
        ]
    }
}

// ============ TOOL DEFINITIONS ============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub parameters: ToolParameters,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameters {
    #[serde(rename = "type")]
    pub param_type: String,
    pub properties: serde_json::Value,
    pub required: Vec<String>,
}

impl Tool {
    fn list_directory() -> Self {
        Tool {
            name: "list_directory".to_string(),
            description: "List contents of a directory".to_string(),
            parameters: ToolParameters {
                param_type: "object".to_string(),
                properties: serde_json::json!({
                    "path": {
                        "type": "string",
                        "description": "Path to the directory to list"
                    },
                    "show_hidden": {
                        "type": "boolean",
                        "description": "Whether to show hidden files",
                        "default": false
                    }
                }),
                required: vec!["path".to_string()],
            },
        }
    }
    
    fn read_file() -> Self {
        Tool {
            name: "read_file".to_string(),
            description: "Read contents of a file".to_string(),
            parameters: ToolParameters {
                param_type: "object".to_string(),
                properties: serde_json::json!({
                    "path": {
                        "type": "string",
                        "description": "Path to the file to read"
                    }
                }),
                required: vec!["path".to_string()],
            },
        }
    }
}

// ============ TOOL EXECUTION ENGINE ==========

#[derive(Debug, Clone)]
pub struct ToolResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
}

struct ToolEngine;

impl ToolEngine {
    fn execute_tool(tool_name: &str, arguments: serde_json::Value) -> ToolResult {
        match tool_name {
            "list_directory" => Self::list_directory(arguments),
            "read_file" => Self::read_file(arguments),
            _ => ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Unknown tool: {}", tool_name)),
            }
        }
    }
    
    fn list_directory(args: serde_json::Value) -> ToolResult {
        let path = args.get("path")
            .and_then(|p| p.as_str())
            .unwrap_or(".");
        
        let show_hidden = args.get("show_hidden")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        
        match fs::read_dir(path) {
            Ok(entries) => {
                let mut output = String::new();
                let mut files = Vec::new();
                
                for entry in entries {
                    if let Ok(entry) = entry {
                        let name = entry.file_name().to_string_lossy().to_string();
                        if !show_hidden && name.starts_with('.') {
                            continue;
                        }
                        let is_dir = entry.path().is_dir();
                        let is_symlink = entry.path().is_symlink();
                        
                        let entry_type = if is_dir {
                            "📁"
                        } else if is_symlink {
                            "🔗"
                        } else {
                            "📄"
                        };
                        
                        files.push(format!("{} {}", entry_type, name));
                    }
                }
                
                files.sort();
                output.push_str(&format!("Directory: {}\n\n", path));
                output.push_str(&files.join("\n"));
                
                ToolResult {
                    success: true,
                    output,
                    error: None,
                }
            }
            Err(e) => ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Failed to read directory: {}", e)),
            }
        }
    }
    
    fn read_file(args: serde_json::Value) -> ToolResult {
        let path = args.get("path")
            .and_then(|p| p.as_str())
            .unwrap_or("");
        
        match fs::read_to_string(path) {
            Ok(content) => {
                let output = format!("File: {}\n\n{}", path, content);
                ToolResult {
                    success: true,
                    output,
                    error: None,
                }
            }
            Err(e) => ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Failed to read file: {}", e)),
            }
        }
    }
}

// ============ OLLAMA CLIENT WITH TOOL SUPPORT ============

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OllamaChatMessage {
    role: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<OllamaToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OllamaToolCall {
    function: OllamaFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OllamaFunction {
    name: String,
    arguments: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
struct OllamaChatResponse {
    message: OllamaResponseMessage,
}

#[derive(Debug, Clone, Deserialize)]
struct OllamaResponseMessage {
    role: String,
    content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<OllamaToolCall>>,
}

#[derive(Clone)]
struct OllamaClient {
    client: reqwest::Client,
    base_url: String,
}

impl OllamaClient {
    fn new(base_url: Option<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.unwrap_or_else(|| "http://localhost:11434".to_string()),
        }
    }

    async fn chat_with_context(
        &self,
        model: &str,
        messages: Vec<OllamaChatMessage>,
        tools: Option<Vec<Tool>>,
    ) -> Result<OllamaChatResponse, Box<dyn std::error::Error>> {
        let url = format!("{}/api/chat", self.base_url);
        
        let mut request_body = serde_json::json!({
            "model": model,
            "messages": messages,
            "stream": false,
        });

        if let Some(tools) = tools {
            let tool_definitions: Vec<serde_json::Value> = tools.iter().map(|tool| {
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": tool.name,
                        "description": tool.description,
                        "parameters": {
                            "type": tool.parameters.param_type,
                            "properties": tool.parameters.properties,
                            "required": tool.parameters.required,
                        }
                    }
                })
            }).collect();
            request_body["tools"] = serde_json::Value::Array(tool_definitions);
        }

        let response = self
            .client
            .post(&url)
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("Ollama API error: {}", error_text).into());
        }

        let chat_response: OllamaChatResponse = response.json().await?;
        Ok(chat_response)
    }

    async fn generate_streaming(
        &self,
        model: &str,
        messages: Vec<OllamaChatMessage>,
        tx: mpsc::UnboundedSender<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let url = format!("{}/api/chat", self.base_url);
        
        let request_body = serde_json::json!({
            "model": model,
            "messages": messages,
            "stream": true,
        });

        let response = self
            .client
            .post(&url)
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("Ollama API error: {}", error_text).into());
        }

        let bytes_stream = response.bytes_stream();
        let mut result = String::new();
        tokio::pin!(bytes_stream);

        while let Some(chunk) = bytes_stream.next().await {
            let chunk = chunk?;
            let chunk_str = String::from_utf8_lossy(&chunk);
            
            for line in chunk_str.lines() {
                if line.is_empty() {
                    continue;
                }
                if let Ok(resp) = serde_json::from_str::<serde_json::Value>(line) {
                    if let Some(response) = resp.get("message").and_then(|m| m.get("content")).and_then(|c| c.as_str()) {
                        result.push_str(response);
                        // Send only the accumulated response text
                        let _ = tx.send(result.clone());
                    }
                    if let Some(done) = resp.get("done").and_then(|d| d.as_bool()) {
                        if done {
                            return Ok(());
                        }
                    }
                }
            }
        }

        Ok(())
    }

    async fn list_models(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let url = format!("{}/api/tags", self.base_url);
        let response = self.client.get(&url).send().await?;
        
        if !response.status().is_success() {
            return Err(format!("HTTP error: {}", response.status()).into());
        }

        #[derive(Deserialize)]
        struct ModelList {
            models: Vec<ModelInfo>,
        }

        #[derive(Deserialize)]
        struct ModelInfo {
            name: String,
        }

        let model_list: ModelList = response.json().await?;
        Ok(model_list.models.into_iter().map(|m| m.name).collect())
    }
}

// ============ MARKDOWN RENDERER ============

struct MarkdownRenderer;

impl MarkdownRenderer {
    fn render(text: &str) -> Text<'static> {
        let mut result: Vec<Line<'static>> = Vec::new();
        let mut in_code_block = false;
        let mut code_lines: Vec<String> = Vec::new();
        let mut code_lang: Option<String> = None;
        let text = text.to_string();
        
        for line in text.lines() {
            let line = line.trim_end();
            
            if line.trim().starts_with("```") {
                if !in_code_block {
                    in_code_block = true;
                    code_lines.clear();
                    let lang = line.trim().strip_prefix("```").unwrap_or("").trim();
                    if !lang.is_empty() {
                        code_lang = Some(lang.to_string());
                    } else {
                        code_lang = None;
                    }
                } else {
                    in_code_block = false;
                    if let Some(ref lang) = code_lang {
                        let label = format!(" [{}] ", lang);
                        result.push(Line::styled(
                            label,
                            Style::default()
                                .fg(Color::Yellow)
                                .bg(Color::Rgb(0, 0, 139))
                                .add_modifier(Modifier::BOLD),
                        ));
                    }
                    for code_line in &code_lines {
                        result.push(Line::styled(
                            code_line.clone(),
                            Style::default()
                                .fg(Color::White)
                                .bg(Color::Rgb(0, 0, 139)),
                        ));
                    }
                    code_lines.clear();
                }
                continue;
            }
            
            if in_code_block {
                code_lines.push(line.to_string());
                continue;
            }
            
            if line.is_empty() {
                result.push(Line::from(""));
                continue;
            }
            
            if let Some(rest) = line.strip_prefix("# ") {
                if !result.is_empty() {
                    result.push(Line::from(""));
                }
                result.push(Line::styled(
                    rest.to_string(),
                    Style::default()
                        .fg(Color::Magenta)
                        .add_modifier(Modifier::BOLD),
                ));
                continue;
            }
            if let Some(rest) = line.strip_prefix("## ") {
                if !result.is_empty() {
                    result.push(Line::from(""));
                }
                result.push(Line::styled(
                    rest.to_string(),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ));
                continue;
            }
            if let Some(rest) = line.strip_prefix("### ") {
                if !result.is_empty() {
                    result.push(Line::from(""));
                }
                result.push(Line::styled(
                    rest.to_string(),
                    Style::default()
                        .fg(Color::Blue)
                        .add_modifier(Modifier::BOLD),
                ));
                continue;
            }
            
            if let Some(rest) = line.strip_prefix("- ") {
                result.push(Line::from(format!("• {}", rest)));
                continue;
            }
            if let Some(rest) = line.strip_prefix("* ") {
                result.push(Line::from(format!("• {}", rest)));
                continue;
            }
            
            if let Some(rest) = line.strip_prefix("1. ") {
                result.push(Line::from(format!("1. {}", rest)));
                continue;
            }
            
            if line.trim() == "---" || line.trim() == "***" || line.trim() == "___" {
                result.push(Line::from("─".repeat(50)));
                continue;
            }
            
            let mut spans: Vec<Span<'static>> = Vec::new();
            let mut current_text = String::new();
            let mut chars = line.chars().peekable();
            let mut in_bold = false;
            let mut in_italic = false;
            let mut in_code = false;
            
            while let Some(c) = chars.next() {
                if c == '*' && chars.peek() == Some(&'*') {
                    chars.next();
                    if !current_text.is_empty() {
                        let span_style = if in_bold || in_italic {
                            let mut s = Style::default();
                            if in_bold { s = s.add_modifier(Modifier::BOLD); }
                            if in_italic { s = s.add_modifier(Modifier::ITALIC); }
                            s
                        } else {
                            Style::default()
                        };
                        spans.push(Span::styled(std::mem::take(&mut current_text), span_style));
                    }
                    in_bold = !in_bold;
                } else if c == '*' {
                    if !current_text.is_empty() {
                        let span_style = if in_bold || in_italic {
                            let mut s = Style::default();
                            if in_bold { s = s.add_modifier(Modifier::BOLD); }
                            if in_italic { s = s.add_modifier(Modifier::ITALIC); }
                            s
                        } else {
                            Style::default()
                        };
                        spans.push(Span::styled(std::mem::take(&mut current_text), span_style));
                    }
                    in_italic = !in_italic;
                } else if c == '`' {
                    if !current_text.is_empty() {
                        let span_style = if in_bold || in_italic {
                            let mut s = Style::default();
                            if in_bold { s = s.add_modifier(Modifier::BOLD); }
                            if in_italic { s = s.add_modifier(Modifier::ITALIC); }
                            s
                        } else {
                            Style::default()
                        };
                        spans.push(Span::styled(std::mem::take(&mut current_text), span_style));
                    }
                    in_code = !in_code;
                } else {
                    current_text.push(c);
                }
            }
            
            if !current_text.is_empty() {
                let span_style = if in_bold || in_italic || in_code {
                    let mut s = Style::default();
                    if in_bold { s = s.add_modifier(Modifier::BOLD); }
                    if in_italic { s = s.add_modifier(Modifier::ITALIC); }
                    if in_code { 
                        s = s.fg(Color::Green).bg(Color::Rgb(30, 30, 30));
                    }
                    s
                } else {
                    Style::default()
                };
                spans.push(Span::styled(current_text, span_style));
            }
            
            if !spans.is_empty() {
                result.push(Line::from(spans));
            } else {
                result.push(Line::from(line.to_string()));
            }
        }
        
        Text::from(result)
    }
}

// ============ MENU SYSTEM ============

#[derive(Debug, Clone, PartialEq)]
enum MenuItem {
    File(Vec<FileSubMenuItem>),
    Options(Vec<OptionsSubMenuItem>),
    Help(HelpSubMenu),
}

#[derive(Debug, Clone, PartialEq)]
enum FileSubMenuItem {
    Save,
    Load,
    Exit,
}

#[derive(Debug, Clone, PartialEq)]
enum OptionsSubMenuItem {
    ChatMode,
    AgentMode,
}

#[derive(Debug, Clone, PartialEq)]
enum HelpSubMenu {
    About,
}

#[derive(Debug, Clone, PartialEq)]
enum MenuAction {
    Save,
    Load,
    Exit,
    ChatMode,
    AgentMode,
    About,
    None,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    Chat,
    Agent,
}

#[derive(Debug, Clone)]
struct MenuState {
    active: bool,
    selected_menu: usize,
    selected_subitem: usize,
    menus: Vec<MenuItem>,
    menu_positions: Vec<(u16, u16)>,
    menu_bar_rect: Rect,
}

impl MenuState {
    fn new() -> Self {
        Self {
            active: false,
            selected_menu: 0,
            selected_subitem: 0,
            menus: vec![
                MenuItem::File(vec![
                    FileSubMenuItem::Save,
                    FileSubMenuItem::Load,
                    FileSubMenuItem::Exit,
                ]),
                MenuItem::Options(vec![
                    OptionsSubMenuItem::ChatMode,
                    OptionsSubMenuItem::AgentMode,
                ]),
                MenuItem::Help(HelpSubMenu::About),
            ],
            menu_positions: Vec::new(),
            menu_bar_rect: Rect::default(),
        }
    }

    fn get_submenu_items(&self, menu_index: usize) -> Vec<String> {
        match self.menus.get(menu_index) {
            Some(MenuItem::File(items)) => {
                items.iter().map(|item| match item {
                    FileSubMenuItem::Save => "Save".to_string(),
                    FileSubMenuItem::Load => "Load".to_string(),
                    FileSubMenuItem::Exit => "Exit".to_string(),
                }).collect()
            }
            Some(MenuItem::Options(items)) => {
                items.iter().map(|item| match item {
                    OptionsSubMenuItem::ChatMode => "Chat Mode".to_string(),
                    OptionsSubMenuItem::AgentMode => "Agent Mode".to_string(),
                }).collect()
            }
            Some(MenuItem::Help(HelpSubMenu::About)) => vec!["About".to_string()],
            None => vec![],
        }
    }

    fn select_subitem(&mut self, menu_index: usize, sub_index: usize) -> MenuAction {
        match self.menus.get(menu_index) {
            Some(MenuItem::File(items)) => {
                if let Some(item) = items.get(sub_index) {
                    match item {
                        FileSubMenuItem::Save => return MenuAction::Save,
                        FileSubMenuItem::Load => return MenuAction::Load,
                        FileSubMenuItem::Exit => return MenuAction::Exit,
                    }
                }
            }
            Some(MenuItem::Options(items)) => {
                if let Some(item) = items.get(sub_index) {
                    match item {
                        OptionsSubMenuItem::ChatMode => return MenuAction::ChatMode,
                        OptionsSubMenuItem::AgentMode => return MenuAction::AgentMode,
                    }
                }
            }
            Some(MenuItem::Help(HelpSubMenu::About)) => return MenuAction::About,
            None => {}
        }
        MenuAction::None
    }

    fn navigate(&mut self, direction: i32) {
        if !self.active {
            return;
        }
        
        let menu_count = self.menus.len();
        if menu_count == 0 {
            return;
        }

        if self.selected_subitem == 0 {
            let new_index = (self.selected_menu as i32 + direction).rem_euclid(menu_count as i32) as usize;
            self.selected_menu = new_index;
            self.selected_subitem = 0;
        } else {
            let sub_items = self.get_submenu_items(self.selected_menu);
            let sub_count = sub_items.len();
            if sub_count > 0 {
                let new_index = (self.selected_subitem as i32 + direction - 1).rem_euclid(sub_count as i32) as usize;
                self.selected_subitem = new_index + 1;
            }
        }
    }

    fn activate(&mut self) {
        self.active = true;
        self.selected_subitem = 1;
    }

    fn deactivate(&mut self) {
        self.active = false;
        self.selected_subitem = 0;
    }

    fn toggle(&mut self) {
        if self.active {
            self.deactivate();
        } else {
            self.activate();
        }
    }
}

// ============ TOOL SYSTEM ============

#[derive(Debug, Clone, Serialize)]
struct ToolCall {
    name: String,
    arguments: serde_json::Value,
    timestamp: String,
}

struct ToolSystem;

impl ToolSystem {
    fn log_tool_call(name: &str, arguments: serde_json::Value) {
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let tool_call = ToolCall {
            name: name.to_string(),
            arguments,
            timestamp,
        };
        
        let log_entry = serde_json::to_string_pretty(&tool_call).unwrap_or_else(|_| format!("{:?}", tool_call));
        
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open("tools.log")
            .unwrap_or_else(|_| {
                fs::File::create("tools.log").unwrap()
            });
        
        writeln!(file, "{}\n---\n", log_entry).unwrap_or(());
    }
}

// ============ CLOUD API CLIENTS ============

// DeepSeek Client
#[derive(Clone)]
struct DeepSeekClient {
    client: reqwest::Client,
    api_key: String,
    base_url: String,
    model_id: String,
}

impl DeepSeekClient {
    fn new(config: &CloudModelConfig, api_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            base_url: config.base_url.clone(),
            model_id: config.model_id.clone(),
        }
    }

    async fn chat_with_context(
        &self,
        messages: Vec<OllamaChatMessage>,
        tools: Vec<Tool>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let url = format!("{}/v1/chat/completions", self.base_url);
        
        let tool_definitions: Vec<serde_json::Value> = tools.iter().map(|tool| {
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": tool.name,
                    "description": tool.description,
                    "parameters": {
                        "type": tool.parameters.param_type,
                        "properties": tool.parameters.properties,
                        "required": tool.parameters.required,
                    }
                }
            })
        }).collect();
        
        // Convert our messages to OpenAI/DeepSeek format
        let deepseek_messages: Vec<serde_json::Value> = messages.iter().map(|msg| {
            serde_json::json!({
                "role": msg.role,
                "content": msg.content
            })
        }).collect();
        
        let request_body = serde_json::json!({
            "model": self.model_id,
            "messages": deepseek_messages,
            "tools": tool_definitions,
            "tool_choice": "auto",
            "stream": false,
        });

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("DeepSeek API error: {}", error_text).into());
        }

        let json_response: serde_json::Value = response.json().await?;
        
        if let Some(tool_calls) = json_response["choices"][0]["message"]["tool_calls"].as_array() {
            if !tool_calls.is_empty() {
                let mut tool_results = Vec::new();
                for tool_call in tool_calls {
                    let tool_name = tool_call["function"]["name"].as_str().unwrap_or("");
                    let tool_args_str = tool_call["function"]["arguments"].as_str().unwrap_or("{}");
                    let tool_args: serde_json::Value = serde_json::from_str(tool_args_str).unwrap_or(serde_json::json!({}));
                    
                    let result = ToolEngine::execute_tool(tool_name, tool_args.clone());
                    tool_results.push(format!(
                        "Tool: {}\nArguments: {}\nResult: {}\n",
                        tool_name,
                        serde_json::to_string_pretty(&tool_args).unwrap_or_default(),
                        if result.success { result.output } else { result.error.unwrap_or_default() }
                    ));
                }
                return Ok(format!("🔧 Tool Execution Results:\n\n{}", tool_results.join("\n---\n")));
            }
        }
        
        let content = json_response["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("No response from DeepSeek")
            .to_string();
        
        Ok(content)
    }
}

// Mistral Client
#[derive(Clone)]
struct MistralClient {
    client: reqwest::Client,
    api_key: String,
    base_url: String,
    model_id: String,
}

impl MistralClient {
    fn new(config: &CloudModelConfig, api_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            base_url: config.base_url.clone(),
            model_id: config.model_id.clone(),
        }
    }

    async fn chat_with_context(
        &self,
        messages: Vec<OllamaChatMessage>,
        tools: Vec<Tool>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let url = format!("{}/v1/chat/completions", self.base_url);
        
        let tool_definitions: Vec<serde_json::Value> = tools.iter().map(|tool| {
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": tool.name,
                    "description": tool.description,
                    "parameters": {
                        "type": tool.parameters.param_type,
                        "properties": tool.parameters.properties,
                        "required": tool.parameters.required,
                    }
                }
            })
        }).collect();
        
        let mistral_messages: Vec<serde_json::Value> = messages.iter().map(|msg| {
            serde_json::json!({
                "role": msg.role,
                "content": msg.content
            })
        }).collect();
        
        let request_body = serde_json::json!({
            "model": self.model_id,
            "messages": mistral_messages,
            "tools": tool_definitions,
            "tool_choice": "auto",
            "stream": false,
        });

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("Mistral API error: {}", error_text).into());
        }

        let json_response: serde_json::Value = response.json().await?;
        
        if let Some(tool_calls) = json_response["choices"][0]["message"]["tool_calls"].as_array() {
            if !tool_calls.is_empty() {
                let mut tool_results = Vec::new();
                for tool_call in tool_calls {
                    let tool_name = tool_call["function"]["name"].as_str().unwrap_or("");
                    let tool_args_str = tool_call["function"]["arguments"].as_str().unwrap_or("{}");
                    let tool_args: serde_json::Value = serde_json::from_str(tool_args_str).unwrap_or(serde_json::json!({}));
                    
                    let result = ToolEngine::execute_tool(tool_name, tool_args.clone());
                    tool_results.push(format!(
                        "Tool: {}\nArguments: {}\nResult: {}\n",
                        tool_name,
                        serde_json::to_string_pretty(&tool_args).unwrap_or_default(),
                        if result.success { result.output } else { result.error.unwrap_or_default() }
                    ));
                }
                return Ok(format!("🔧 Tool Execution Results:\n\n{}", tool_results.join("\n---\n")));
            }
        }
        
        let content = json_response["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("No response from Mistral")
            .to_string();
        
        Ok(content)
    }
}

// ============ APP ============

#[derive(Clone)]
struct App {
    input: String,
    output: String,
    is_loading: bool,
    model: String,
    models: Vec<String>,
    error: Option<String>,
    menu: MenuState,
    show_about: bool,
    status_message: Option<String>,
    history: Vec<String>,
    history_index: usize,
    mode: AppMode,
    scroll_offset: usize,
    max_scroll: usize,
    auto_scroll: bool,
    ollama_client: OllamaClient,
    cloud_configs: Vec<CloudModelConfig>,
    api_keys: HashMap<String, String>,
    deepseek_clients: Vec<DeepSeekClient>,
    mistral_clients: Vec<MistralClient>,
    streaming_buffer: String,
    has_model_line: bool,
    conversation: Vec<OllamaChatMessage>,
}

impl App {
    fn new(ollama_client: OllamaClient, cloud_configs: Vec<CloudModelConfig>) -> Self {
        let api_keys = load_api_keys();
        let mut deepseek_clients = Vec::new();
        let mut mistral_clients = Vec::new();
        
        for config in &cloud_configs {
            if let Some(api_key) = api_keys.get(&config.name) {
                match config.api_type {
                    CloudApiType::DeepSeek => {
                        deepseek_clients.push(DeepSeekClient::new(config, api_key.clone()));
                    }
                    CloudApiType::Mistral => {
                        mistral_clients.push(MistralClient::new(config, api_key.clone()));
                    }
                }
            }
        }
        
        Self {
            input: String::new(),
            output: String::new(),
            is_loading: false,
            model: "llama2".to_string(),
            models: Vec::new(),
            error: None,
            menu: MenuState::new(),
            show_about: false,
            status_message: None,
            history: Vec::new(),
            history_index: 0,
            mode: AppMode::Chat,
            scroll_offset: 0,
            max_scroll: 0,
            auto_scroll: true,
            ollama_client,
            cloud_configs,
            api_keys,
            deepseek_clients,
            mistral_clients,
            streaming_buffer: String::new(),
            has_model_line: false,
            conversation: Vec::new(),
        }
    }

    fn get_cloud_models(&self) -> Vec<String> {
        let mut cloud_models = Vec::new();
        for config in &self.cloud_configs {
            if self.api_keys.contains_key(&config.name) {
                cloud_models.push(config.name.clone());
            }
        }
        cloud_models
    }

    fn is_cloud_model(&self, model: &str) -> bool {
        model.starts_with("cloud@")
    }

    fn get_cloud_config(&self, model: &str) -> Option<&CloudModelConfig> {
        self.cloud_configs.iter().find(|c| c.name == model)
    }

    fn get_model_display(&self) -> String {
        if self.model.starts_with("cloud@") {
            if let Some(config) = self.get_cloud_config(&self.model) {
                config.display_name.clone()
            } else {
                self.model.clone()
            }
        } else {
            self.model.clone()
        }
    }

    fn handle_menu_action(&mut self, action: MenuAction) {
        match action {
            MenuAction::Save => {
                self.save_output();
            }
            MenuAction::Load => {
                self.status_message = Some("📂 Load action (not implemented)".to_string());
            }
            MenuAction::Exit => {
                self.cleanup_and_exit();
            }
            MenuAction::ChatMode => {
                self.mode = AppMode::Chat;
                self.status_message = Some("💬 Switched to Chat Mode".to_string());
            }
            MenuAction::AgentMode => {
                self.mode = AppMode::Agent;
                self.status_message = Some("🤖 Switched to Agent Mode - Tools are available".to_string());
            }
            MenuAction::About => {
                self.show_about = true;
                self.status_message = Some("ℹ️ About".to_string());
            }
            MenuAction::None => {}
        }
        self.menu.deactivate();
    }

    fn cleanup_and_exit(&self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        process::exit(0);
    }

    fn save_output(&mut self) {
        if self.output.is_empty() {
            self.status_message = Some("⚠️ No output to save".to_string());
            return;
        }

        match fs::write("output.txt", &self.output) {
            Ok(_) => {
                self.status_message = Some("✅ Output saved to output.txt".to_string());
            }
            Err(e) => {
                self.status_message = Some(format!("❌ Failed to save: {}", e));
            }
        }
    }

    fn previous_history(&mut self) -> Option<String> {
        if self.history_index > 0 {
            self.history_index -= 1;
            Some(self.history[self.history_index].clone())
        } else {
            None
        }
    }
    
    fn next_history(&mut self) -> Option<String> {
        if self.history_index < self.history.len() - 1 {
            self.history_index += 1;
            Some(self.history[self.history_index].clone())
        } else if self.history_index == self.history.len() - 1 {
            self.history_index = self.history.len();
            Some(String::new())
        } else {
            None
        }
    }

    fn add_to_history(&mut self, prompt: String) {
        self.history.push(prompt);
        self.history_index = self.history.len();
    }

    fn scroll_to_bottom(&mut self) {
        self.scroll_offset = self.max_scroll;
        self.auto_scroll = true;
    }

    fn update_max_scroll(&mut self, total_lines: usize, viewport_lines: usize) {
        if total_lines > viewport_lines {
            self.max_scroll = total_lines - viewport_lines;
        } else {
            self.max_scroll = 0;
        }
        
        if self.auto_scroll {
            self.scroll_offset = self.max_scroll;
        }
    }

    fn append_to_conversation(&mut self, role: &str, content: &str) {
        self.conversation.push(OllamaChatMessage {
            role: role.to_string(),
            content: content.to_string(),
            tool_calls: None,
            tool_name: None,
        });
        // Limit conversation length to prevent memory issues
        if self.conversation.len() > 20 {
            self.conversation.drain(0..2);
        }
    }

    async fn process_chat_request(&mut self, prompt: String, tx: mpsc::UnboundedSender<String>) -> String {
        // Add user message to conversation
        self.append_to_conversation("user", &prompt);

        // Check if this is a cloud model
        if self.is_cloud_model(&self.model) {
            let config = match self.get_cloud_config(&self.model) {
                Some(c) => c,
                None => return format!("❌ Cloud model not found: {}", self.model),
            };

            let tools = vec![
                Tool::list_directory(),
                Tool::read_file(),
            ];

            let response = match config.api_type {
                CloudApiType::DeepSeek => {
                    for client in &self.deepseek_clients {
                        match client.chat_with_context(self.conversation.clone(), tools.clone()).await {
                            Ok(response) => {
                                self.append_to_conversation("assistant", &response);
                                return response;
                            }
                            Err(e) => return format!("❌ DeepSeek error: {}", e),
                        }
                    }
                    "❌ No DeepSeek client configured".to_string()
                }
                CloudApiType::Mistral => {
                    for client in &self.mistral_clients {
                        match client.chat_with_context(self.conversation.clone(), tools.clone()).await {
                            Ok(response) => {
                                self.append_to_conversation("assistant", &response);
                                return response;
                            }
                            Err(e) => return format!("❌ Mistral error: {}", e),
                        }
                    }
                    "❌ No Mistral client configured".to_string()
                }
            };
            return response;
        }

        // For local Ollama models
        if self.mode == AppMode::Agent {
            return self.process_agent_request(prompt).await;
        }

        // Chat mode - use streaming with context
        let model_display = self.get_model_display();
        
        // Send the model line first
        let _ = tx.send(format!("MODEL_LINE:{}", model_display));
        
        // Now stream the response
        match self.ollama_client.generate_streaming(
            &self.model,
            self.conversation.clone(),
            tx.clone(),
        ).await {
            Ok(_) => {
                // Get the final response from the streaming buffer
                let response = self.streaming_buffer.clone();
                if !response.is_empty() {
                    self.append_to_conversation("assistant", &response);
                }
                return response;
            }
            Err(e) => {
                let error_msg = format!("❌ Error: {}", e);
                self.append_to_conversation("assistant", &error_msg);
                return error_msg;
            }
        }
    }

    async fn process_agent_request(&mut self, prompt: String) -> String {
        // Add user message to conversation
        self.append_to_conversation("user", &prompt);

        let tools = vec![
            Tool::list_directory(),
            Tool::read_file(),
        ];

        let max_iterations = 5;
        let mut iteration = 0;

        loop {
            iteration += 1;
            if iteration > max_iterations {
                return "⚠️ Max tool iterations reached".to_string();
            }

            let response = match self.ollama_client.chat_with_context(
                &self.model,
                self.conversation.clone(),
                Some(tools.clone()),
            ).await {
                Ok(r) => r,
                Err(e) => return format!("❌ Error: {}", e),
            };

            let response_message = response.message;

            if let Some(tool_calls) = response_message.tool_calls {
                if !tool_calls.is_empty() {
                    // Add assistant message with tool calls
                    self.conversation.push(OllamaChatMessage {
                        role: "assistant".to_string(),
                        content: String::new(),
                        tool_calls: Some(tool_calls.clone()),
                        tool_name: None,
                    });

                    let mut tool_results = Vec::new();
                    
                    for tool_call in &tool_calls {
                        let tool_name = &tool_call.function.name;
                        let args = tool_call.function.arguments.clone();
                        
                        ToolSystem::log_tool_call(tool_name, args.clone());
                        
                        let result = ToolEngine::execute_tool(tool_name, args);
                        tool_results.push((tool_name.clone(), result));
                    }

                    for (tool_name, result) in tool_results {
                        let content = if result.success {
                            result.output
                        } else {
                            format!("Error: {}", result.error.unwrap_or_default())
                        };
                        
                        self.conversation.push(OllamaChatMessage {
                            role: "tool".to_string(),
                            content,
                            tool_calls: None,
                            tool_name: Some(tool_name),
                        });
                    }

                    continue;
                }
            }

            // No tool calls, we have the final answer
            if let Some(content) = response_message.content {
                self.append_to_conversation("assistant", &content);
                return content;
            } else {
                return "No response from model".to_string();
            }
        }
    }
}

// ============ COMMAND HANDLER ============

async fn handle_command(
    prompt: &str,
    app: &mut App,
    ollama_client: &OllamaClient,
    tx: &mpsc::UnboundedSender<String>,
) -> bool {
    match prompt {
        "/help" => {
            let response = "📋 Available commands:\n\n/list - List available models\n/use <model> - Switch to a specific model\n/context - Show current conversation context\n/quit or /q or /exit - Quit the application\n/help - Show this help message".to_string();
            let _ = tx.send(response);
            true
        }
        "/quit" | "/q" | "/exit" => {
            app.cleanup_and_exit();
            true
        }
        "/list" => {
            app.status_message = Some("📋 Fetching model list...".to_string());
            app.auto_scroll = true;
            let client = ollama_client.clone();
            let tx = tx.clone();
            let cloud_models = app.get_cloud_models();
            tokio::spawn(async move {
                match client.list_models().await {
                    Ok(models) => {
                        let mut response = "📋 Available models:\n\n".to_string();
                        for model in &models {
                            response.push_str(&format!("   {}\n", model));
                        }
                        for model in &cloud_models {
                            response.push_str(&format!("☁️  {}\n", model));
                        }
                        response.push_str("\n☁️  = Cloud model (requires API key in config.keys)");
                        let _ = tx.send(response);
                    }
                    Err(e) => {
                        let _ = tx.send(format!("❌ Error listing models: {}", e));
                    }
                }
            });
            true
        }
        "/context" => {
            if app.conversation.is_empty() {
                let response = "📭 No conversation context yet. Start chatting to build context!".to_string();
                let _ = tx.send(response);
            } else {
                let mut response = "📚 Current Conversation Context:\n\n".to_string();
                for (i, msg) in app.conversation.iter().enumerate() {
                    let role_emoji = match msg.role.as_str() {
                        "user" => "👤",
                        "assistant" => "🤖",
                        "tool" => "🔧",
                        _ => "📝",
                    };
                    response.push_str(&format!("{}. {} {}: \n   {}\n\n", 
                        i + 1, 
                        role_emoji, 
                        msg.role.to_uppercase(),
                        msg.content
                    ));
                }
                response.push_str(&format!("📊 Total messages: {}", app.conversation.len()));
                let _ = tx.send(response);
            }
            true
        }
        cmd if cmd.starts_with("/use ") => {
            let model = cmd.strip_prefix("/use ").unwrap().trim();
            if !model.is_empty() {
                let model_name = model.to_string();
                
                let is_cloud = app.is_cloud_model(&model_name);
                let cloud_config = if is_cloud {
                    app.get_cloud_config(&model_name).cloned()
                } else {
                    None
                };
                
                let found_model = app.models.iter().find(|m| {
                    **m == model_name ||
                    m.starts_with(&format!("{}:", &model_name)) ||
                    m.starts_with(&model_name)
                });
                
                if let Some(config) = cloud_config {
                    if app.api_keys.contains_key(&model_name) {
                        app.model = model_name.clone();
                        let display_name = config.display_name.clone();
                        app.status_message = Some(format!("🔄 Switched to cloud model: {}", display_name));
                        let response = format!("🔄 Switched to cloud model: {}\n", display_name);
                        let _ = tx.send(response);
                    } else {
                        let msg = format!("❌ No API key found for {} in {}\nAdd a line: {}:your_key_here", 
                            config.display_name, CONFIG_FILE, model_name);
                        app.status_message = Some(msg.clone());
                        let _ = tx.send(msg);
                    }
                } else if let Some(found) = found_model {
                    app.model = found.clone();
                    let found_clone = found.clone();
                    app.status_message = Some(format!("🔄 Switched to model: {}", found_clone));
                    let response = format!("🔄 Switched to model: {}\n", found_clone);
                    let _ = tx.send(response);
                } else {
                    let models_list = app.models.join(", ");
                    let msg = format!("❌ Model not found: {}. Available: {}", model_name, models_list);
                    app.status_message = Some(msg.clone());
                    let _ = tx.send(msg);
                }
            }
            true
        }
        _ => false,
    }
}

// ============ UI RENDERING ============

fn render_menu_bar(f: &mut Frame, app: &mut App, area: Rect) {
    app.menu.menu_bar_rect = area;
    
    let menu_labels = vec!["File", "Options", "Help"];
    let mut x_pos = area.x;
    
    app.menu.menu_positions.clear();
    
    for (i, label) in menu_labels.iter().enumerate() {
        let is_selected = app.menu.active && app.menu.selected_menu == i && app.menu.selected_subitem == 0;
        
        let display = if is_selected {
            format!("[{}] ", label)
        } else {
            format!(" {}  ", label)
        };
        
        let width = display.len() as u16;
        app.menu.menu_positions.push((x_pos, area.y));
        x_pos += width;
        
        let style = if is_selected {
            Style::default()
                .fg(Color::Black)
                .bg(Color::White)
                .add_modifier(Modifier::BOLD)
        } else if app.menu.active && app.menu.selected_menu == i {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        };
        
        let paragraph = Paragraph::new(display)
            .style(style);
        f.render_widget(paragraph, Rect {
            x: x_pos - width,
            y: area.y,
            width,
            height: 1,
        });
    }
}

fn render_menu_popup(f: &mut Frame, app: &App) {
    if !app.menu.active {
        return;
    }

    let area = f.area();
    let menu_index = app.menu.selected_menu;
    
    let (menu_x, menu_y) = if menu_index < app.menu.menu_positions.len() {
        app.menu.menu_positions[menu_index]
    } else {
        (area.x + 1, area.y + 1)
    };
    
    let sub_items = app.menu.get_submenu_items(menu_index);
    if sub_items.is_empty() {
        return;
    }
    
    let menu_width = 20;
    let menu_height = sub_items.len() as u16 + 2;
    
    let popup_area = Rect {
        x: menu_x,
        y: menu_y + 1,
        width: menu_width,
        height: menu_height,
    };

    f.render_widget(Clear, popup_area);

    let mut content = Vec::new();
    for (i, item) in sub_items.iter().enumerate() {
        let is_selected = app.menu.selected_subitem == i + 1;
        let display_item = if is_selected {
            format!("▶ {}", item)
        } else {
            format!("  {}", item)
        };
        content.push(display_item);
    }

    let menu_text = Text::from(content.join("\n"));
    let paragraph = Paragraph::new(menu_text)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)))
        .style(Style::default().fg(Color::White));
    
    f.render_widget(paragraph, popup_area);
}

fn render_about_dialog(f: &mut Frame, app: &App) {
    if !app.show_about {
        return;
    }

    let area = f.area();
    
    let dialog_width = (area.width * 2 / 5).min(50);
    let dialog_height = 13;
    
    let dialog_x = (area.width - dialog_width) / 2;
    let dialog_y = (area.height - dialog_height) / 2;
    
    let dialog_area = Rect {
        x: dialog_x,
        y: dialog_y,
        width: dialog_width,
        height: dialog_height,
    };

    f.render_widget(Clear, dialog_area);

    let about_content = vec![
        Line::from(Span::styled(
            "Ollama TUI Client",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("Version: 0.9.0"),
        Line::from(""),
        Line::from("A terminal-based client for Ollama with"),
        Line::from("cloud model support and tool calling."),
        Line::from(""),
        Line::from("Features:"),
        Line::from("  • Chat & Agent modes"),
        Line::from("  • Cloud models: DeepSeek, Mistral"),
        Line::from("  • Tool calling (list_directory, read_file)"),
        Line::from("  • Agent loop with multi-turn tool execution"),
        Line::from("  • Full conversation context"),
        Line::from("  • Streaming responses"),
        Line::from("  • Markdown rendering"),
        Line::from("  • Mouse support"),
        Line::from("  • Prompt history"),
        Line::from(""),
        Line::from("Cloud API Keys:"),
        Line::from("  • Add keys to config.keys file"),
        Line::from("  • Format: cloud@model:your_api_key"),
        Line::from(""),
        Line::from(Span::styled(
            "Press Enter or ESC to close",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let about_text = Text::from(about_content);
    let paragraph = Paragraph::new(about_text)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(" ℹ️ About "))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });
    
    f.render_widget(paragraph, dialog_area);
}

fn ui(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(1),  // Menu bar
            Constraint::Min(1),     // Output
            Constraint::Length(3),  // Input
            Constraint::Length(1),  // Status bar
        ].as_ref())
        .split(f.area());

    render_menu_bar(f, app, chunks[0]);

    let mode_indicator = match app.mode {
        AppMode::Chat => "💬 CHAT MODE",
        AppMode::Agent => "🤖 AGENT MODE (Tools available)",
    };
    
    let model_display = app.get_model_display();
    let model_display_with_cloud = if app.model.starts_with("cloud@") {
        format!("☁️ {}", model_display)
    } else {
        model_display.clone()
    };
    
    let output_area = chunks[1];
    let output_block = Block::default()
        .borders(Borders::ALL)
        .title(format!("📝 Response [{}] - {}", mode_indicator, model_display_with_cloud));
    
    if let Some(err) = &app.error {
        let error_text = Paragraph::new(format!("❌ Error: {}", err))
            .style(Style::default().fg(Color::Red))
            .block(output_block)
            .wrap(Wrap { trim: true });
        f.render_widget(error_text, output_area);
    } else {
        let content_height = if !app.output.is_empty() {
            let text = MarkdownRenderer::render(&app.output);
            text.height()
        } else {
            0
        };
        
        let viewport_height = output_area.height.saturating_sub(2) as usize;
        
        app.update_max_scroll(content_height, viewport_height);
        
        let visible_text = if !app.output.is_empty() {
            let full_text = MarkdownRenderer::render(&app.output);
            let start = app.scroll_offset.min(content_height.saturating_sub(1));
            let end = (start + viewport_height).min(content_height);
            
            let mut visible_lines = Vec::new();
            for i in start..end {
                if let Some(line) = full_text.lines.get(i) {
                    visible_lines.push(line.clone());
                }
            }
            Text::from(visible_lines)
        } else {
            Text::default()
        };
        
        let paragraph = Paragraph::new(visible_text)
            .block(output_block)
            .wrap(Wrap { trim: true });
        f.render_widget(paragraph, output_area);
        
        if content_height > viewport_height {
            let scrollbar_area = Rect {
                x: output_area.x + output_area.width - 2,
                y: output_area.y + 1,
                width: 1,
                height: output_area.height.saturating_sub(2),
            };
            
            let mut scrollbar_state = ScrollbarState::default()
                .content_length(content_height)
                .position(app.scroll_offset);
            
            f.render_stateful_widget(
                Scrollbar::default()
                    .orientation(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(Some("↑"))
                    .end_symbol(Some("↓"))
                    .track_symbol(Some("│"))
                    .thumb_symbol("█"),
                scrollbar_area,
                &mut scrollbar_state,
            );
        }
    }

    let input_block = Block::default()
        .borders(Borders::ALL)
        .title("✏️ Input");
    let input_text = app.input.clone();
    let input = Paragraph::new(input_text)
        .style(Style::default().fg(Color::White))
        .block(input_block)
        .wrap(Wrap { trim: true });
    f.render_widget(input, chunks[2]);

    let status_text = if let Some(msg) = &app.status_message {
        msg.clone()
    } else if let Some(err) = &app.error {
        format!("❌ {}", err)
    } else if app.is_loading {
        "⏳ Processing request...".to_string()
    } else {
        let cloud_indicator = if app.model.starts_with("cloud@") { "☁️ " } else { "" };
        let tools_indicator = if app.mode == AppMode::Agent { " 🔧" } else { "" };
        let context_indicator = if !app.conversation.is_empty() { " 📚" } else { "" };
        format!("🤖 {}{}{}{} | Ctrl+S to save | F9 for menu | /help for commands | Scroll: PgUp/PgDn/Mouse", 
            cloud_indicator,
            app.get_model_display(), 
            tools_indicator,
            context_indicator
        )
    };
    let status = Paragraph::new(status_text)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    f.render_widget(status, chunks[3]);

    render_menu_popup(f, app);
    render_about_dialog(f, app);

    if !app.menu.active && !app.show_about {
        let cursor_x = chunks[2].x + 1 + app.input.len() as u16;
        let cursor_y = chunks[2].y + 1;
        f.set_cursor_position((cursor_x, cursor_y));
    }
}

// ============ MAIN ============

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    
    let ollama_client = OllamaClient::new(None);
    let cloud_configs = CloudModelConfig::get_default_configs();
    
    let mut app = App::new(ollama_client.clone(), cloud_configs);
    
    match ollama_client.list_models().await {
        Ok(models) => {
            let mut all_models = models.clone();
            let cloud_models = app.get_cloud_models();
            all_models.extend(cloud_models);
            
            if !all_models.is_empty() {
                app.models = all_models;
                app.model = models[0].clone();
                let cloud_count = app.get_cloud_models().len();
                app.status_message = Some(format!(
                    "✅ Connected - {} local models, {} cloud models available", 
                    models.len() - cloud_count,
                    cloud_count
                ));
            }
        }
        Err(e) => {
            app.error = Some(format!("Failed to connect to Ollama: {}", e));
        }
    }
    
    let res = run_app(&mut terminal, &mut app, &mut rx, ollama_client, tx).await;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}


async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    rx: &mut mpsc::UnboundedReceiver<String>,
    ollama_client: OllamaClient,
    tx: mpsc::UnboundedSender<String>,
) -> io::Result<()> {
    let tx_clone = tx.clone();
    
    loop {
        terminal.draw(|f| ui(f, app))?;

        while let Ok(chunk) = rx.try_recv() {
            if chunk.is_empty() {
                if !app.streaming_buffer.is_empty() {
                    let response = app.streaming_buffer.clone();
                    app.append_to_conversation("assistant", &response);
                }
                app.streaming_buffer.clear();
                app.is_loading = false;
                app.status_message = None;
                app.auto_scroll = true;
                app.has_model_line = false;
                continue;
            }

            if chunk.starts_with("MODEL_LINE:") {
                let model_name = chunk.strip_prefix("MODEL_LINE:").unwrap_or("").to_string();
                let new_line = format!("{}: ", model_name);
                if !app.output.is_empty() && !app.output.ends_with('\n') {
                    app.output.push('\n');
                }
                app.output.push_str(&new_line);
                app.has_model_line = true;
                app.auto_scroll = true;
                continue;
            }

            if app.has_model_line {
                app.streaming_buffer = chunk;
                let model_display = app.get_model_display();
                let model_prefix = format!("{}:", model_display);
                
                let lines: Vec<&str> = app.output.lines().collect();
                let mut model_line_index = None;
                for (i, line) in lines.iter().enumerate().rev() {
                    if line.starts_with(&model_prefix) {
                        model_line_index = Some(i);
                        break;
                    }
                }
                
                if let Some(idx) = model_line_index {
                    let mut new_output = lines[..idx].join("\n");
                    if !new_output.is_empty() {
                        new_output.push('\n');
                    }
                    new_output.push_str(&format!("{}: {}", model_display, app.streaming_buffer));
                    app.output = new_output;
                } else {
                    if !app.output.is_empty() && !app.output.ends_with('\n') {
                        app.output.push('\n');
                    }
                    app.output.push_str(&format!("{}: {}", model_display, app.streaming_buffer));
                }
                app.auto_scroll = true;
                app.status_message = None;
            } else {
                if !app.output.is_empty() && !app.output.ends_with('\n') {
                    app.output.push('\n');
                }
                app.output.push_str(&chunk);
                app.auto_scroll = true;
                app.status_message = None;
            }
        }

        if event::poll(Duration::from_millis(10))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') if key.modifiers == crossterm::event::KeyModifiers::CONTROL => {
                        app.cleanup_and_exit();
                    }
                    KeyCode::Char('s') if key.modifiers == crossterm::event::KeyModifiers::CONTROL => {
                        if !app.is_loading {
                            app.save_output();
                        }
                    }
                    KeyCode::F(9) => {
                        app.menu.toggle();
                        if app.menu.active {
                            app.show_about = false;
                        }
                    }
                    KeyCode::Esc => {
                        if app.menu.active {
                            app.menu.deactivate();
                        } else if app.show_about {
                            app.show_about = false;
                        }
                    }
                    KeyCode::PageUp => {
                        if app.scroll_offset > 0 {
                            app.scroll_offset = app.scroll_offset.saturating_sub(10);
                            app.auto_scroll = false;
                        }
                    }
                    KeyCode::PageDown => {
                        if app.scroll_offset < app.max_scroll {
                            app.scroll_offset = (app.scroll_offset + 10).min(app.max_scroll);
                            app.auto_scroll = false;
                        }
                    }
                    KeyCode::Home => {
                        app.scroll_offset = 0;
                        app.auto_scroll = false;
                    }
                    KeyCode::End => {
                        app.scroll_offset = app.max_scroll;
                        app.auto_scroll = true;
                    }
                    KeyCode::Enter => {
                        if app.show_about {
                            app.show_about = false;
                        } else if app.menu.active {
                            if app.menu.selected_subitem > 0 {
                                let action = app.menu.select_subitem(
                                    app.menu.selected_menu,
                                    app.menu.selected_subitem - 1
                                );
                                app.handle_menu_action(action);
                            }
                        } else if !app.input.is_empty() && !app.is_loading {
                            let prompt = app.input.clone();
                            app.input.clear();
                            
                            if prompt.starts_with('/') {
                                let handled = handle_command(&prompt, app, &ollama_client, &tx_clone).await;
                                if handled {
                                    continue;
                                }
                            }
                            
                            if !app.output.is_empty() && !app.output.ends_with('\n') {
                                app.output.push('\n');
                            }
                            app.output.push_str(&format!("USER: {}", prompt));
                            
                            app.add_to_history(prompt.clone());
                            app.is_loading = true;
                            app.status_message = Some("⏳ Processing request...".to_string());
                            app.auto_scroll = true;
                            app.has_model_line = false;
                            
                            let mut app_clone = app.clone();
                            let tx = tx_clone.clone();
                            let prompt_clone = prompt.clone();
                            
                            tokio::spawn(async move {
                                let response = app_clone.process_chat_request(prompt_clone, tx.clone()).await;
                                let _ = tx.send(response);
                            });
                        }
                    }
                    KeyCode::Char(c) => {
                        if !app.menu.active && !app.show_about {
                            app.input.push(c);
                        }
                    }
                    KeyCode::Backspace => {
                        if !app.menu.active && !app.show_about {
                            app.input.pop();
                        }
                    }
                    KeyCode::Up => {
                        if app.menu.active {
                            app.menu.navigate(-1);
                        } else if !app.show_about && !app.is_loading {
                            if let Some(prev) = app.previous_history() {
                                app.input = prev;
                            }
                        }
                    }
                    KeyCode::Down => {
                        if app.menu.active {
                            if app.menu.selected_subitem == 0 {
                                app.menu.selected_subitem = 1;
                            } else {
                                app.menu.navigate(1);
                            }
                        } else if !app.show_about && !app.is_loading {
                            if let Some(next) = app.next_history() {
                                app.input = next;
                            }
                        }
                    }
                    KeyCode::Left => {
                        if app.menu.active {
                            app.menu.selected_subitem = 0;
                            app.menu.navigate(-1);
                        }
                    }
                    KeyCode::Right => {
                        if app.menu.active {
                            app.menu.selected_subitem = 0;
                            app.menu.navigate(1);
                        }
                    }
                    _ => {}
                }
            } else if let Event::Mouse(mouse) = event::read()? {
                match mouse.kind {
                    MouseEventKind::ScrollUp => {
                        if app.menu.active {
                            app.menu.navigate(-1);
                        } else if !app.show_about && app.scroll_offset > 0 {
                            app.scroll_offset = app.scroll_offset.saturating_sub(3);
                            app.auto_scroll = false;
                        }
                    }
                    MouseEventKind::ScrollDown => {
                        if app.menu.active {
                            app.menu.navigate(1);
                        } else if !app.show_about && app.scroll_offset < app.max_scroll {
                            app.scroll_offset = (app.scroll_offset + 3).min(app.max_scroll);
                            app.auto_scroll = false;
                        }
                    }
                    MouseEventKind::Down(MouseButton::Left) => {
                        if app.show_about {
                            // Use the terminal's current frame area
                            let area = terminal.get_frame().area();
                            let dialog_width = (area.width * 2 / 5).min(50);
                            let dialog_height = 13;
                            let dialog_x = (area.width - dialog_width) / 2;
                            let dialog_y = (area.height - dialog_height) / 2;
                            
                            let dialog_area = Rect {
                                x: dialog_x,
                                y: dialog_y,
                                width: dialog_width,
                                height: dialog_height,
                            };
                            
                            if mouse.column < dialog_area.x || mouse.column > dialog_area.x + dialog_area.width
                                || mouse.row < dialog_area.y || mouse.row > dialog_area.y + dialog_area.height {
                                app.show_about = false;
                            }
                            continue;
                        }
                        
                        let menu_bar = app.menu.menu_bar_rect;
                        
                        if mouse.row == menu_bar.y && mouse.column >= menu_bar.x && mouse.column < menu_bar.x + menu_bar.width {
                            let click_x = mouse.column - menu_bar.x;
                            
                            let menu_idx = if click_x < 8 {
                                0
                            } else if click_x < 18 {
                                1
                            } else {
                                2
                            };
                            
                            if !app.menu.active || app.menu.selected_menu != menu_idx {
                                app.menu.activate();
                                app.menu.selected_menu = menu_idx;
                                app.menu.selected_subitem = 1;
                            } else {
                                app.menu.deactivate();
                            }
                            continue;
                        }
                        
                        if app.menu.active {
                            let menu_index = app.menu.selected_menu;
                            let sub_items = app.menu.get_submenu_items(menu_index);
                            if !sub_items.is_empty() {
                                let (menu_x, menu_y) = if menu_index < app.menu.menu_positions.len() {
                                    app.menu.menu_positions[menu_index]
                                } else {
                                    (2, 2)
                                };
                                
                                let submenu_area = Rect {
                                    x: menu_x,
                                    y: menu_y + 1,
                                    width: 20,
                                    height: sub_items.len() as u16 + 2,
                                };
                                
                                if mouse.column >= submenu_area.x && mouse.column <= submenu_area.x + submenu_area.width
                                    && mouse.row >= submenu_area.y && mouse.row <= submenu_area.y + submenu_area.height {
                                    let sub_index = (mouse.row - submenu_area.y - 1) as usize;
                                    if sub_index < sub_items.len() {
                                        let action = app.menu.select_subitem(
                                            menu_index,
                                            sub_index
                                        );
                                        app.handle_menu_action(action);
                                    }
                                } else {
                                    app.menu.deactivate();
                                }
                            } else {
                                app.menu.deactivate();
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}
