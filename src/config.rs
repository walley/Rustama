use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Ollama per-model parameter file (renamed from `model_params.conf`).
pub const OLLAMA_MODELS_FILE: &str = "models_ollama.conf";
/// Cloud model definitions file (renamed from `cloud_models.conf`).
pub const CLOUD_MODELS_FILE: &str = "models_cloud.conf";
const LEGACY_OLLAMA_MODELS_FILE: &str = "model_params.conf";
const LEGACY_CLOUD_MODELS_FILE: &str = "cloud_models.conf";

#[derive(Debug, Clone)]
pub struct Config {
    pub ollama_url: String,
    pub model: String,
    pub save_path: String,
    pub agentic: bool,
    /// Startup workflow mode: "chat", "coder" (former agentic), or
    /// "assistant" (tool use, but not for coding). When absent, the
    /// legacy `agentic` flag decides: true → coder, false → chat.
    pub mode: String,
    pub timeout_secs: u64,
    pub logging: bool,
    pub logfile: String,
    /// System prompt for Coder mode (legacy key: `system_prompt`).
    pub system_prompt_coder: String,
    /// System prompt for Assistant mode.
    pub system_prompt_assistant: String,
    /// File name (relative to the config directory) that holds the
    /// Coder system prompt. When set, the `system_prompt_coder` conf
    /// key contains this file name and `system_prompt_coder` holds the
    /// prompt text loaded from the file.
    pub system_prompt_coder_file: Option<String>,
    /// Same as `system_prompt_coder_file`, for the Assistant prompt.
    pub system_prompt_assistant_file: Option<String>,
    pub proxy: Option<String>,
    pub max_tool_rounds: usize,
    /// Automatic "continue" nudges allowed per user turn; 0 disables them.
    pub max_auto_continues: u32,
    pub max_retries: u32,
    pub terminal_width_pct: u16,
    pub justify: bool,
    /// Show the hint bar (key hints / focus indicator) on the
    /// second-to-last terminal line. Default: off.
    pub hintbar: bool,
    /// Directory for session files. Default: <DOCUMENTS>/rustama or ~/rustama.
    pub session_dir: Option<String>,
    /// Directory for log files. Default: <session_dir>/logs.
    pub log_directory: String,
    /// Launch a rust-analyzer LSP server for compiler feedback
    /// (`lsp_diagnostics` tool). Default: off — it is a heavy process.
    pub lsp: bool,
    /// Path/name of the LSP server binary.
    pub lsp_server: String,
    /// LSP workspace root. Empty = directory of `save_path`, else cwd.
    pub lsp_workspace: String,
    /// After edit_file/write_file tool calls, automatically append fresh
    /// LSP diagnostics to the tool result.
    pub lsp_auto_diagnostics: bool,
}

/// Per-model sampling / generation parameters.
///
/// Used both for cloud models (parsed from the model's section in
/// `models_cloud.conf`) and for Ollama models (parsed from the model's
/// section in `models_ollama.conf`, falling back to that file's `[default]`
/// section). Values not recognized by a given backend are simply not sent.
#[derive(Debug, Clone)]
pub struct ModelParams {
    pub temperature: f64,
    pub top_p: f64,
    pub top_k: u32,
    pub frequency_penalty: f64,
    pub presence_penalty: f64,
    /// Cloud: `max_tokens`. Ollama: `num_predict`. `None` = provider default.
    pub max_output_tokens: Option<u32>,
    /// low/medium/high, or on/off to toggle thinking without a level.
    /// Cloud: sent as `reasoning_effort` (levels only). Ollama: maps to the
    /// top-level `think` field (bool or level string).
    pub reasoning_effort: Option<String>,
    /// Cloud only: `auto`, `none`, or `required`, sent as `tool_choice`.
    /// `None` falls back to `auto` when tools are attached.
    pub tool_choice: Option<String>,
    pub seed: Option<u32>,
    /// Ollama only: context window in tokens, sent as `options.num_ctx`.
    /// Overrides Ollama's default context window (2048) which silently
    /// forgets history beyond it. `None` = Ollama default (Rustama then
    /// falls back to the model's reported `context_length` when known).
    pub num_ctx: Option<u32>,
}

impl Default for ModelParams {
    fn default() -> Self {
        ModelParams {
            temperature: 1.0,
            top_p: 0.9,
            top_k: 40,
            frequency_penalty: 0.0,
            presence_penalty: 0.0,
            max_output_tokens: None,
            reasoning_effort: None,
            tool_choice: None,
            seed: None,
            num_ctx: None,
        }
    }
}

impl ModelParams {
    /// Applies one `key = value` config line. Returns true if the key was
    /// recognized (even if the value turned out invalid and was ignored).
    pub fn apply_key(&mut self, key: &str, value: &str) -> bool {
        match key {
            "temperature" => {
                if let Ok(n) = value.parse::<f64>()
                    && (0.0..=2.0).contains(&n)
                {
                    self.temperature = n;
                }
            }
            "top_p" => {
                if let Ok(n) = value.parse::<f64>()
                    && (0.0..=1.0).contains(&n)
                {
                    self.top_p = n;
                }
            }
            "top_k" => {
                if let Ok(n) = value.parse::<u32>()
                    && (1..=100).contains(&n)
                {
                    self.top_k = n;
                }
            }
            "frequency_penalty" => {
                if let Ok(n) = value.parse::<f64>()
                    && (-2.0..=2.0).contains(&n)
                {
                    self.frequency_penalty = n;
                }
            }
            "presence_penalty" => {
                if let Ok(n) = value.parse::<f64>()
                    && (-2.0..=2.0).contains(&n)
                {
                    self.presence_penalty = n;
                }
            }
            "max_output_tokens" | "max_tokens" => {
                let v = value.trim();
                if v.is_empty() || v.eq_ignore_ascii_case("off") || v.eq_ignore_ascii_case("none") {
                    self.max_output_tokens = None;
                } else if let Ok(n) = v.parse::<u32>()
                    && n > 0
                {
                    self.max_output_tokens = Some(n);
                }
            }
            "reasoning_effort" | "effort" => {
                let v = value.trim().to_lowercase();
                if v.is_empty() || v == "none" {
                    self.reasoning_effort = None;
                } else if matches!(
                    v.as_str(),
                    "low" | "medium" | "high" | "on" | "off" | "true" | "false"
                ) {
                    self.reasoning_effort = Some(v);
                }
            }
            "tool_choice" => {
                let v = value.trim().to_lowercase();
                if v.is_empty() {
                    self.tool_choice = None;
                } else if matches!(v.as_str(), "auto" | "none" | "required") {
                    self.tool_choice = Some(v);
                }
            }
            "seed" => {
                let v = value.trim();
                if v.is_empty() || v.eq_ignore_ascii_case("off") || v.eq_ignore_ascii_case("none") {
                    self.seed = None;
                } else if let Ok(n) = v.parse::<u32>() {
                    self.seed = Some(n);
                }
            }
            "num_ctx" | "n_ctx" | "context_length" => {
                let v = value.trim();
                if v.is_empty() || v.eq_ignore_ascii_case("off") || v.eq_ignore_ascii_case("none") {
                    self.num_ctx = None;
                } else if let Ok(n) = v.parse::<u32>()
                    && n > 0
                {
                    self.num_ctx = Some(n);
                }
            }
            _ => return false,
        }
        true
    }

    /// Key/value pairs as written back to config files.
    pub fn to_conf_entries(&self) -> Vec<(String, String)> {
        let mut out = vec![
            ("temperature".to_string(), format!("{}", self.temperature)),
            ("top_p".to_string(), format!("{}", self.top_p)),
            ("top_k".to_string(), format!("{}", self.top_k)),
            (
                "frequency_penalty".to_string(),
                format!("{}", self.frequency_penalty),
            ),
            (
                "presence_penalty".to_string(),
                format!("{}", self.presence_penalty),
            ),
        ];
        if let Some(n) = self.max_output_tokens {
            out.push(("max_output_tokens".to_string(), n.to_string()));
        }
        if let Some(ref e) = self.reasoning_effort {
            out.push(("reasoning_effort".to_string(), e.clone()));
        }
        if let Some(ref choice) = self.tool_choice {
            out.push(("tool_choice".to_string(), choice.clone()));
        }
        if let Some(n) = self.seed {
            out.push(("seed".to_string(), n.to_string()));
        }
        if let Some(n) = self.num_ctx {
            out.push(("num_ctx".to_string(), n.to_string()));
        }
        out
    }
}

/// Per-token pricing for a cloud model, in USD per 1M tokens.
///
/// All fields optional: models without pricing simply show token counts
/// without a cost estimate. `cache_write` covers providers that charge
/// extra for cache creation (Anthropic); most providers only discount
/// cached *reads*.
#[derive(Debug, Clone, Default)]
pub struct ModelPricing {
    pub input_per_mtok: Option<f64>,
    pub output_per_mtok: Option<f64>,
    pub cached_read_per_mtok: Option<f64>,
    pub cache_write_per_mtok: Option<f64>,
}

impl ModelPricing {
    pub fn is_configured(&self) -> bool {
        self.input_per_mtok.is_some() || self.output_per_mtok.is_some()
    }

    /// Compact one-line summary for pickers/info rows, e.g.
    /// `$3/$15 per 1M` (in/out) or `$3/-$15 per 1M` when a rate is
    /// missing. `None` when no rates are configured at all.
    pub fn short_summary(&self) -> Option<String> {
        if !self.is_configured() {
            return None;
        }
        let fmt = |v: Option<f64>| {
            v.map(|r| {
                if r.fract() == 0.0 {
                    format!("${}", r)
                } else {
                    format!("${:.2}", r)
                }
            })
            .unwrap_or_else(|| "-".to_string())
        };
        Some(format!(
            "{}/{} per 1M",
            fmt(self.input_per_mtok),
            fmt(self.output_per_mtok)
        ))
    }

    /// Cost in USD for one request's usage. Cached-read tokens are billed
    /// at the cached rate when set, otherwise at the plain input rate
    /// (they are part of prompt_tokens already — never double-counted).
    pub fn cost(&self, input: u64, output: u64, cached_read: u64) -> Option<f64> {
        if !self.is_configured() {
            return None;
        }
        let per_m = |rate: Option<f64>, tokens: u64| rate.unwrap_or(0.0) * tokens as f64 / 1e6;
        let cached = cached_read.min(input);
        let fresh_input = input - cached;
        let cached_rate = self.cached_read_per_mtok.or(self.input_per_mtok);
        Some(
            per_m(self.input_per_mtok, fresh_input)
                + per_m(cached_rate, cached)
                + per_m(self.output_per_mtok, output),
        )
    }
}

#[derive(Debug, Clone)]
pub struct CloudModel {
    pub name: String,
    pub api_url: String,
    pub api_key: String,
    pub api_model: String,
    pub params: ModelParams,
    pub pricing: ModelPricing,
    /// Whether this model accepts the `reasoning_effort` parameter.
    /// Set to `false` for models like Mistral that reject unknown parameters.
    pub supports_reasoning_effort: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            ollama_url: "http://localhost:11434".to_string(),
            model: "llama3.2:3b".to_string(),
            save_path: "output.md".to_string(),
            agentic: true,
            mode: "coder".to_string(),
            timeout_secs: 300,
            logging: true,
            logfile: "rustama.log".to_string(),
            system_prompt_coder: "You are a coding assistant with access to tools. When the user asks you to do something, use the available tools to accomplish the task. Always use tools when needed - do not just describe what you would do. Execute the actual tool calls. After using a tool, continue working until the task is complete.".to_string(),
            system_prompt_assistant: "You are a helpful assistant".to_string(),
            system_prompt_coder_file: None,
            system_prompt_assistant_file: None,
            proxy: None,
            max_tool_rounds: 100,
            max_auto_continues: 10,
            max_retries: 10,
            terminal_width_pct: 40,
            justify: false,
            hintbar: false,
            lsp: false,
            lsp_server: "rust-analyzer".to_string(),
            lsp_workspace: String::new(),
            lsp_auto_diagnostics: true,
            session_dir: None,
            log_directory: "logs".to_string(),
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let dir = match conf_dir() {
            Some(d) => d,
            None => return Config::default(),
        };

        if let Some(conf_path) = find_conf_file() {
            if let Ok(content) = fs::read_to_string(&conf_path) {
                let mut cfg = Self::parse(&content);
                // The system_prompt_* keys may hold a file name (relative
                // to the config directory) instead of the prompt text;
                // load the actual prompt from that file.
                if let Some(dir) = conf_path.parent() {
                    let (text, file) = resolve_prompt_file(&cfg.system_prompt_coder, dir);
                    cfg.system_prompt_coder = text;
                    cfg.system_prompt_coder_file = file;
                    let (text, file) = resolve_prompt_file(&cfg.system_prompt_assistant, dir);
                    cfg.system_prompt_assistant = text;
                    cfg.system_prompt_assistant_file = file;
                }
                let dflt = Config::default();
                // Migrate the legacy single `system_prompt` key: its value
                // becomes the coder prompt, not the hardcoded default.
                let coder_default = cfg.system_prompt_coder.clone();
                let missing: Vec<(String, String)> = vec![
                    (
                        "logging".to_string(),
                        if dflt.logging {
                            "true".to_string()
                        } else {
                            "false".to_string()
                        },
                    ),
                    ("logfile".to_string(), dflt.logfile.clone()),
                    (
                        "max_auto_continues".to_string(),
                        dflt.max_auto_continues.to_string(),
                    ),
                    ("mode".to_string(), dflt.mode.clone()),
                    (
                        "system_prompt_coder".to_string(),
                        coder_default,
                    ),
                    (
                        "system_prompt_assistant".to_string(),
                        dflt.system_prompt_assistant.clone(),
                    ),
                    (
                        "justify".to_string(),
                        if dflt.justify {
                            "true".to_string()
                        } else {
                            "false".to_string()
                        },
                    ),
                    (
                        "hintbar".to_string(),
                        if dflt.hintbar {
                            "on".to_string()
                        } else {
                            "off".to_string()
                        },
                    ),
                    ("lsp".to_string(), if dflt.lsp { "on" } else { "off" }.to_string()),
                    ("lsp_server".to_string(), dflt.lsp_server.clone()),
                    ("lsp_workspace".to_string(), dflt.lsp_workspace.clone()),
                    (
                        "lsp_auto_diagnostics".to_string(),
                        if dflt.lsp_auto_diagnostics { "on" } else { "off" }.to_string(),
                    ),
                    ("session_dir".to_string(), dflt.session_dir.clone().unwrap_or_default()),
                    ("log_directory".to_string(), dflt.log_directory.clone()),
                ];

                let mut updated = content.clone();
                for (key, value) in &missing {
                    // Match a real `key = ...` line, not a substring of a
                    // longer key (e.g. "mode" inside "model").
                    let has_key = content.lines().any(|l| {
                        let l = l.trim();
                        !l.starts_with('#')
                            && !l.starts_with(';')
                            && l.split_once('=')
                                .is_some_and(|(k, _)| k.trim() == key)
                    });
                    if !has_key {
                        if !updated.ends_with('\n') {
                            updated.push('\n');
                        }
                        updated.push_str(&format!("\n{} = {}\n", key, value));
                    }
                }
                if updated != content {
                    let _ = fs::write(&conf_path, &updated);
                }
                return cfg;
            }
            return Config::default();
        }

        let _ = fs::create_dir_all(&dir);
        let mut cfg = Config::default();
        // Store the default system prompts in separate files — they are
        // too large to keep inline in the config file.
        let coder_file = "coder.prompt";
        let assistant_file = "assistant.prompt";
        let _ = fs::write(dir.join(coder_file), &cfg.system_prompt_coder);
        let _ = fs::write(dir.join(assistant_file), &cfg.system_prompt_assistant);
        cfg.system_prompt_coder_file = Some(coder_file.to_string());
        cfg.system_prompt_assistant_file = Some(assistant_file.to_string());
        let conf_path = dir.join("rustama.conf");
        let _ = fs::write(&conf_path, cfg.default_conf());
        cfg
    }

    fn default_conf(&self) -> String {
        // When the prompts live in separate files, the conf keys hold
        // the file names rather than the (large) prompt texts.
        let coder_prompt = self
            .system_prompt_coder_file
            .clone()
            .unwrap_or_else(|| self.system_prompt_coder.clone());
        let assistant_prompt = self
            .system_prompt_assistant_file
            .clone()
            .unwrap_or_else(|| self.system_prompt_assistant.clone());
        format!(
            "# Rustama configuration\n\n\
             # Ollama API URL\n\
             ollama_url = {}\n\n\
             # Default model\n\
             model = {}\n\n\
             # Default save path\n\
             save_path = {}\n\n\
             # Enable agentic mode on startup (true/false) — legacy,\n\
             # superseded by `mode` below\n\
             agentic = {}\n\n\
             # Startup workflow mode: chat, coder, or assistant\n\
             mode = {}\n\n\
             # HTTP request timeout in seconds\n\
             timeout_secs = {}\n\n\
             # Enable logging on startup (true/yes/on or false/no/off)\n\
             logging = {}\n\n\
             # Log file path\n\
             logfile = {}\n\n\
             # System prompt for Coder mode (set via /setsystem command)\n\
             system_prompt_coder = {}\n\n\
             # System prompt for Assistant mode\n\
             system_prompt_assistant = {}\n\n\
             # HTTP proxy URL (optional, e.g. http://proxy:8080)\n\
             # proxy = http://proxy:8080\n\n\
             # Max agentic tool rounds per request (default: 100)\n\
             max_tool_rounds = {}\n\n\
             # Max automatic continue nudges per user turn (0 disables, default: 10)\n\
             max_auto_continues = {}\n\n\
             # Max API retries on 429/rate-limit errors (1-50, default: 10)\n\
             max_retries = {}\n\n\
             # Justify paragraphs in output (true/false, default: false)\n\
             justify = {}\n\n\
             # Show the hint bar above the keybar (on/off, default: off)\n\
             hintbar = {}\n\n\
             # Launch rust-analyzer LSP for compiler feedback (on/off,\n\
             # default: off — it is a heavy process)\n\
             lsp = {}\n\n\
             # LSP server binary (default: rust-analyzer)\n\
             lsp_server = {}\n\n\
             # LSP workspace root (default: dir of save_path, else cwd)\n\
             lsp_workspace = {}\n\n\
             # Append LSP diagnostics to edit_file/write_file results\n\
             # (on/off, default: on)\n\
             lsp_auto_diagnostics = {}\n\n\
             # Directory for session files (default: <DOCUMENTS>/rustama)\n\
             session_dir = {}\n\n\
             # Directory for log files (default: <session_dir>/logs)\n\
             log_directory = {}\n\n\
             # NOTE: model parameters (temperature, top_p, top_k,\n\
             # frequency_penalty, presence_penalty, max_output_tokens,\n\
             # reasoning_effort, seed) are per-model now — see\n\
             # models_ollama.conf (Ollama models) and models_cloud.conf.\n",
            self.ollama_url,
            self.model,
            self.save_path,
            self.agentic,
            self.mode,
            self.timeout_secs,
            self.logging,
            self.logfile,
            coder_prompt,
            assistant_prompt,
            self.max_tool_rounds,
            self.max_auto_continues,
            self.max_retries,
            self.justify,
            if self.hintbar { "on" } else { "off" },
            if self.lsp { "on" } else { "off" },
            self.lsp_server,
            self.lsp_workspace,
            if self.lsp_auto_diagnostics { "on" } else { "off" },
            self.session_dir.clone().unwrap_or_default(),
            self.log_directory.clone(),
        )
    }

    fn parse(content: &str) -> Self {
        let values = parse_ini(content);
        let mut cfg = Config::default();

        if let Some(v) = values.get("ollama_url") {
            cfg.ollama_url = v.clone();
        }
        if let Some(v) = values.get("model") {
            cfg.model = v.clone();
        }
        if let Some(v) = values.get("save_path") {
            cfg.save_path = v.clone();
        }
        if let Some(v) = values.get("agentic") {
            cfg.agentic = parse_bool(v);
        }
        if let Some(v) = values.get("mode") {
            let v = v.trim().to_lowercase();
            if matches!(v.as_str(), "chat" | "coder" | "assistant") {
                cfg.mode = v;
            }
        } else {
            // Legacy configs only have the `agentic` flag.
            cfg.mode = if cfg.agentic {
                "coder".to_string()
            } else {
                "chat".to_string()
            };
        }
        if let Some(v) = values.get("timeout_secs")
            && let Ok(n) = v.parse::<u64>()
        {
            cfg.timeout_secs = n;
        }
        if let Some(v) = values.get("logging") {
            cfg.logging = parse_bool(v);
        }
        if let Some(v) = values.get("logfile") {
            cfg.logfile = v.clone();
        }
        if let Some(v) = values.get("system_prompt_coder") {
            cfg.system_prompt_coder = v.clone();
        } else if let Some(v) = values.get("system_prompt") {
            // Legacy key: the old single system prompt was the coder one.
            cfg.system_prompt_coder = v.clone();
        }
        if let Some(v) = values.get("system_prompt_assistant") {
            cfg.system_prompt_assistant = v.clone();
        }
        if let Some(v) = values.get("proxy") {
            let v = v.trim();
            if v.is_empty()
                || v.eq_ignore_ascii_case("off")
                || v.eq_ignore_ascii_case("none")
                || v.eq_ignore_ascii_case("false")
            {
                cfg.proxy = None;
            } else {
                cfg.proxy = Some(v.to_string());
            }
        }
        if let Some(v) = values.get("max_tool_rounds")
            && let Ok(n) = v.parse::<usize>()
            && n > 0
        {
            cfg.max_tool_rounds = n;
        }
        if let Some(v) = values.get("max_auto_continues")
            && let Ok(n) = v.parse::<u32>()
        {
            cfg.max_auto_continues = n;
        }
        if let Some(v) = values.get("max_retries")
            && let Ok(n) = v.parse::<u32>()
            && (1..=50).contains(&n)
        {
            cfg.max_retries = n;
        }
        if let Some(v) = values.get("terminal_width_pct")
            && let Ok(n) = v.parse::<u16>()
            && (20..=80).contains(&n)
        {
            cfg.terminal_width_pct = n;
        }
        if let Some(v) = values.get("justify") {
            cfg.justify = parse_bool(v);
        }
        if let Some(v) = values.get("hintbar") {
            cfg.hintbar = parse_bool(v);
        }
        if let Some(v) = values.get("lsp") {
            cfg.lsp = parse_bool(v);
        }
        if let Some(v) = values.get("lsp_server") {
            cfg.lsp_server = v.clone();
        }
        if let Some(v) = values.get("lsp_workspace") {
            cfg.lsp_workspace = v.clone();
        }
        if let Some(v) = values.get("lsp_auto_diagnostics") {
            cfg.lsp_auto_diagnostics = parse_bool(v);
        }
        if let Some(v) = values.get("session_dir") {
            let v = v.trim();
            if !v.is_empty() {
                cfg.session_dir = Some(v.to_string());
            }
        }
        if let Some(v) = values.get("log_directory") {
            let v = v.trim();
            if !v.is_empty() {
                cfg.log_directory = v.to_string();
            }
        }

        cfg
    }

    /// Persist a system prompt under `key` (`system_prompt_coder` or
    /// `system_prompt_assistant`), updating the existing key in place or
    /// appending it.
    pub fn set_system_prompt_key(key: &str, prompt: &str) -> Result<(), String> {
        if let Some(conf_path) = find_conf_file() {
            let content = fs::read_to_string(&conf_path).map_err(|e| e.to_string())?;
            // If the conf key currently references a prompt file, write
            // the new prompt into that file and leave the conf untouched.
            let values = parse_ini(&content);
            if let (Some(name), Some(dir)) = (values.get(key), conf_path.parent()) {
                let name = name.trim();
                let path = dir.join(name);
                if !name.is_empty() && !name.contains('\n') && path.is_file() {
                    fs::write(&path, prompt).map_err(|e| e.to_string())?;
                    return Ok(());
                }
            }
            let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
            let mut found = false;
            for line in &mut lines {
                let trimmed = line.trim();
                if !trimmed.starts_with('#')
                    && !trimmed.starts_with(';')
                    && trimmed
                        .split_once('=')
                        .is_some_and(|(k, _)| k.trim() == key)
                {
                    *line = format!("{} = {}", key, prompt);
                    found = true;
                    break;
                }
            }
            if !found {
                if !lines.last().is_none_or(|l| l.is_empty()) {
                    lines.push(String::new());
                }
                lines.push(format!("{} = {}", key, prompt));
            }
            let new_content = lines.join("\n");
            fs::write(&conf_path, new_content).map_err(|e| e.to_string())?;
        } else {
            let dir = conf_dir().ok_or("Cannot determine config directory")?;
            let conf_path = dir.join("rustama.conf");
            let mut cfg = Config::default();
            match key {
                "system_prompt_assistant" => cfg.system_prompt_assistant = prompt.to_string(),
                _ => cfg.system_prompt_coder = prompt.to_string(),
            }
            fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            fs::write(&conf_path, cfg.default_conf()).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// Sets (or adds) a single `key = value` line in rustama.conf,
    /// updating an existing key in place. Used to persist the selected
    /// theme name (`theme = <name>`).
    pub fn set_conf_value(key: &str, value: &str) -> Result<(), String> {
        let Some(conf_path) = find_conf_file() else {
            return Err("Cannot determine config directory".to_string());
        };
        let content = fs::read_to_string(&conf_path).map_err(|e| e.to_string())?;
        let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
        let mut found = false;
        for line in &mut lines {
            let trimmed = line.trim();
            if !trimmed.starts_with('#')
                && !trimmed.starts_with(';')
                && trimmed
                    .split_once('=')
                    .is_some_and(|(k, _)| k.trim() == key)
            {
                *line = format!("{} = {}", key, value);
                found = true;
                break;
            }
        }
        if !found {
            if !lines.last().is_none_or(|l| l.is_empty()) {
                lines.push(String::new());
            }
            lines.push(format!("{} = {}", key, value));
        }
        fs::write(&conf_path, lines.join("\n")).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn save(&self) -> Result<(), String> {
        let dir = conf_dir().ok_or("Cannot determine config directory")?;
        let conf_path = dir.join("rustama.conf");
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let mut lines: Vec<String> = Vec::new();
        lines.push("# Rustama configuration".to_string());
        lines.push(String::new());
        lines.push(format!("ollama_url = {}", self.ollama_url));
        lines.push(format!("model = {}", self.model));
        lines.push(format!("save_path = {}", self.save_path));
        lines.push(format!("agentic = {}", self.agentic));
        lines.push(format!("mode = {}", self.mode));
        lines.push(format!("timeout_secs = {}", self.timeout_secs));
        lines.push(format!("logging = {}", self.logging));
        lines.push(format!("logfile = {}", self.logfile));
        // Prompts living in separate files: the conf keeps the file
        // name, the prompt text goes to the file.
        match &self.system_prompt_coder_file {
            Some(name) => {
                let _ = fs::write(dir.join(name), &self.system_prompt_coder);
                lines.push(format!("system_prompt_coder = {}", name));
            }
            None => lines.push(format!("system_prompt_coder = {}", self.system_prompt_coder)),
        }
        match &self.system_prompt_assistant_file {
            Some(name) => {
                let _ = fs::write(dir.join(name), &self.system_prompt_assistant);
                lines.push(format!("system_prompt_assistant = {}", name));
            }
            None => lines.push(format!(
                "system_prompt_assistant = {}",
                self.system_prompt_assistant
            )),
        }
        lines.push(String::new());
        if let Some(ref proxy) = self.proxy {
            lines.push(format!("proxy = {}", proxy));
        } else {
            lines.push("# proxy = off".to_string());
        }
        lines.push(format!("max_tool_rounds = {}", self.max_tool_rounds));
        lines.push(format!("max_auto_continues = {}", self.max_auto_continues));
        lines.push(format!("max_retries = {}", self.max_retries));
        lines.push(format!("terminal_width_pct = {}", self.terminal_width_pct));
        lines.push(format!("justify = {}", self.justify));
        lines.push(format!(
            "hintbar = {}",
            if self.hintbar { "on" } else { "off" }
        ));
        lines.push(format!("lsp = {}", if self.lsp { "on" } else { "off" }));
        lines.push(format!("lsp_server = {}", self.lsp_server));
        lines.push(format!("lsp_workspace = {}", self.lsp_workspace));
        lines.push(format!(
            "lsp_auto_diagnostics = {}",
            if self.lsp_auto_diagnostics { "on" } else { "off" }
        ));
        lines.push(format!("session_dir = {}", self.session_dir.clone().unwrap_or_default()));
        lines.push(format!("log_directory = {}", self.log_directory));
        fs::write(&conf_path, lines.join("\n")).map_err(|e| e.to_string())?;
        Ok(())
    }
}

/// Loads cloud models, using `base` (typically the `[default]` section of
/// `models_ollama.conf`) as the fallback for parameters a cloud section does
/// not set itself — so a global default like `top_p` applies to cloud
/// models too.
pub fn load_cloud_models_with_base(base: &ModelParams) -> Vec<CloudModel> {
    let dir = match conf_dir() {
        Some(d) => d,
        None => return Vec::new(),
    };
    migrate_legacy_conf(&dir, LEGACY_CLOUD_MODELS_FILE, CLOUD_MODELS_FILE);
    let conf_path = dir.join(CLOUD_MODELS_FILE);

    if !conf_path.exists() {
        let _ = fs::create_dir_all(&dir);
        let _ = fs::write(&conf_path, default_cloud_conf());
        return Vec::new();
    }

    let content = match fs::read_to_string(&conf_path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    // Existing configs predate the [default] section; add it once so new
    // cloud-level defaults actually take effect.
    let original = content.clone();
    let content = ensure_cloud_default_section(content);
    if content != original {
        let _ = fs::write(&conf_path, &content);
    }

    parse_cloud_models(&content, base)
}

const CLOUD_DEFAULT_TOOL_CHOICE: &str = "auto";
const CLOUD_DEFAULT_REASONING_EFFORT: &str = "high";

/// Entries shipped in the models_cloud.conf `[default]` section. Sampling
/// defaults mirror [`ModelParams::default`] so the generated file documents
/// the effective values instead of inheriting them invisibly.
fn cloud_default_entries() -> Vec<(&'static str, String)> {
    let params = ModelParams::default();
    vec![
        ("tool_choice", CLOUD_DEFAULT_TOOL_CHOICE.to_string()),
        (
            "reasoning_effort",
            CLOUD_DEFAULT_REASONING_EFFORT.to_string(),
        ),
        ("temperature", params.temperature.to_string()),
        ("top_p", params.top_p.to_string()),
        ("top_k", params.top_k.to_string()),
        ("frequency_penalty", params.frequency_penalty.to_string()),
        ("presence_penalty", params.presence_penalty.to_string()),
    ]
}

fn default_cloud_conf() -> String {
    let mut out = String::from(
        "# Cloud models configuration\n\
         # Each model has its own section [model_name]\n\
         # Required fields: api_url, api_key\n\
         # Optional: api_model (actual model name sent to API, defaults to section name)\n\
         # Optional: max_output_tokens (default: 16384), seed\n\
         # Optional pricing in USD per 1M tokens (enables cost estimate):\n\
         #   price_input, price_output, price_cached_read, price_cache_write\n\
         \n\
         # [default] applies to every cloud model; named sections override it.\n\
         # tool_choice: auto, none, or required\n\
         # reasoning_effort: low, medium, high, or on/off\n\
         [default]\n",
    );
    for (key, value) in cloud_default_entries() {
        out.push_str(&format!("{} = {}\n", key, value));
    }
    out.push_str(
        "\n\
         [mistral-small]\n\
         api_url = https://api.mistral.ai/v1/chat/completions\n\
         api_key = YOUR_MISTRAL_API_KEY\n\
         \n\
         [kimi-k3]\n\
         api_url = https://api.moonshot.ai/v1/chat/completions\n\
         api_key = YOUR_MOONSHOT_API_KEY\n\
         \n\
         ",
    );
    out
}

/// `[section]`'s defined keys, lowercased (header matched case-insensitively).
fn section_keys(content: &str, section: &str) -> Vec<String> {
    let mut keys = Vec::new();
    let mut inside = false;
    for line in content.lines() {
        let line = line.trim();
        if let Some(inner) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            if inside {
                break;
            }
            inside = inner.trim().eq_ignore_ascii_case(section);
            continue;
        }
        if inside
            && !line.is_empty()
            && !line.starts_with('#')
            && !line.starts_with(';')
            && let Some((key, _)) = line.split_once('=')
        {
            keys.push(key.trim().to_lowercase());
        }
    }
    keys
}

/// Adds the cloud-wide `[default]` section when an older config lacks it, and
/// backfills any missing default keys into an existing one. Values the user
/// already set are never overwritten.
fn ensure_cloud_default_section(content: String) -> String {
    let entries = cloud_default_entries();
    if let Some(section) = find_section_name(&content, "default") {
        let defined = section_keys(&content, &section);
        let missing: Vec<(String, String)> = entries
            .into_iter()
            .filter(|(key, _)| !defined.iter().any(|k| k == key))
            .map(|(key, value)| (key.to_string(), value))
            .collect();
        if missing.is_empty() {
            return content;
        }
        return append_section_entries(&content, &section, &missing);
    }
    let mut out = String::from(
        "# [default] applies to every cloud model; named sections override it.\n\
         [default]\n",
    );
    for (key, value) in entries {
        out.push_str(&format!("{} = {}\n", key, value));
    }
    out.push('\n');
    out.push_str(&content);
    out
}

/// First section header matching `name` (case-insensitive), original spelling.
fn find_section_name(content: &str, name: &str) -> Option<String> {
    for line in content.lines() {
        let line = line.trim();
        if let Some(inner) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            let inner = inner.trim();
            if inner.eq_ignore_ascii_case(name) {
                return Some(inner.to_string());
            }
        }
    }
    None
}

/// Inserts `entries` after the last `key = value` line of `section`, keeping
/// the block grouped instead of landing after any trailing comments. Falls
/// back to directly after the header for an empty section.
fn append_section_entries(content: &str, section: &str, entries: &[(String, String)]) -> String {
    let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
    let mut inside = false;
    let mut anchor: Option<usize> = None;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if let Some(inner) = trimmed.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            if inside {
                break;
            }
            inside = inner.trim().eq_ignore_ascii_case(section);
            if inside {
                anchor = Some(i);
            }
            continue;
        }
        if inside
            && !trimmed.is_empty()
            && !trimmed.starts_with('#')
            && !trimmed.starts_with(';')
            && trimmed.contains('=')
        {
            anchor = Some(i);
        }
    }

    let Some(at) = anchor else {
        return content.to_string();
    };
    let tail = lines.split_off(at + 1);
    lines.extend(entries.iter().map(|(k, v)| format!("{} = {}", k, v)));
    lines.extend(tail);

    let mut out = lines.join("\n");
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

/// A key left at its shipped placeholder (`YOUR_...`) counts as "not configured".
fn is_placeholder_key(key: &str) -> bool {
    key.starts_with("YOUR_")
}

#[cfg(test)]
mod pricing_tests {
    use super::*;

    fn priced() -> ModelPricing {
        ModelPricing {
            input_per_mtok: Some(3.0),
            output_per_mtok: Some(15.0),
            cached_read_per_mtok: Some(0.30),
            cache_write_per_mtok: None,
        }
    }

    #[test]
    fn cost_splits_cached_from_fresh_input() {
        // 1M input of which 500k cached, 100k output:
        // 500k * $3 + 500k * $0.30 + 100k * $15 = 1.50 + 0.15 + 1.50
        let c = priced().cost(1_000_000, 100_000, 500_000).unwrap();
        assert!((c - 3.15).abs() < 1e-9, "got {}", c);
    }

    #[test]
    fn cost_without_cached_rate_bills_input_rate() {
        let mut p = priced();
        p.cached_read_per_mtok = None;
        // cached tokens fall back to the plain input rate.
        let c = p.cost(1_000_000, 0, 400_000).unwrap();
        assert!((c - 3.0).abs() < 1e-9, "got {}", c);
    }

    #[test]
    fn short_summary_shows_in_out_rates() {
        assert_eq!(priced().short_summary().unwrap(), "$3/$15 per 1M");
    }

    #[test]
    fn short_summary_none_without_rates() {
        let p = ModelPricing {
            input_per_mtok: None,
            output_per_mtok: None,
            cached_read_per_mtok: Some(0.30),
            cache_write_per_mtok: None,
        };
        assert!(p.short_summary().is_none());
    }

    #[test]
    fn cost_never_double_counts_cached() {
        // cached > input must not produce negative fresh input.
        let c = priced().cost(100, 0, 500).unwrap();
        let cached_only = 0.30 * 100.0 / 1e6;
        assert!((c - cached_only).abs() < 1e-12, "got {}", c);
    }

    #[test]
    fn unconfigured_pricing_yields_none() {
        assert!(ModelPricing::default().cost(1, 1, 1).is_none());
        assert!(!ModelPricing::default().is_configured());
    }

    #[test]
    fn parses_pricing_keys() {
        let conf = "[m]\n\
                    api_url = https://x\n\
                    api_key = sk-real\n\
                    price_input = 3.0\n\
                    price_output = 15\n\
                    price_cached_read = 0.3\n\
                    price_cache_write = 3.75\n";
        let models = parse_cloud_models(conf, &ModelParams::default());
        assert_eq!(models.len(), 1);
        let p = &models[0].pricing;
        assert_eq!(p.input_per_mtok, Some(3.0));
        assert_eq!(p.output_per_mtok, Some(15.0));
        assert_eq!(p.cached_read_per_mtok, Some(0.3));
        assert_eq!(p.cache_write_per_mtok, Some(3.75));
        assert!(p.is_configured());
    }

    #[test]
    fn pricing_optional_and_defaulted() {
        let conf = "[m]\napi_url = https://x\napi_key = sk-real\n";
        let models = parse_cloud_models(conf, &ModelParams::default());
        assert_eq!(models.len(), 1);
        assert!(!models[0].pricing.is_configured());
        assert_eq!(models[0].cost_check(), None);
    }

    impl CloudModel {
        fn cost_check(&self) -> Option<f64> {
            self.pricing.cost(1, 1, 1)
        }
    }
}

/// Default max_tokens for cloud models when the section does not say
/// otherwise (preserves the historical 16384).
const CLOUD_DEFAULT_MAX_OUTPUT_TOKENS: u32 = 16384;

fn cloud_params_default(base: &ModelParams) -> ModelParams {
    ModelParams {
        max_output_tokens: Some(CLOUD_DEFAULT_MAX_OUTPUT_TOKENS),
        ..base.clone()
    }
}

fn apply_cloud_setting(
    key: &str,
    value: &str,
    params: &mut ModelParams,
    pricing: &mut ModelPricing,
) {
    match key {
        "price_input" | "input_price" => {
            pricing.input_per_mtok = value.parse().ok();
        }
        "price_output" | "output_price" => {
            pricing.output_per_mtok = value.parse().ok();
        }
        "price_cached_read" | "cached_read_price" | "price_cached" => {
            pricing.cached_read_per_mtok = value.parse().ok();
        }
        "price_cache_write" | "cache_write_price" => {
            pricing.cache_write_per_mtok = value.parse().ok();
        }
        _ => {
            params.apply_key(key, value);
        }
    }
}

/// Parses the models_cloud.conf `[default]` section (model params + pricing)
/// that every named cloud model inherits.
fn parse_cloud_defaults(content: &str, base: &ModelParams) -> (ModelParams, ModelPricing) {
    let mut params = cloud_params_default(base);
    let mut pricing = ModelPricing::default();
    let mut in_default = false;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            in_default = line[1..line.len() - 1]
                .trim()
                .eq_ignore_ascii_case("default");
            continue;
        }
        if in_default
            && let Some((key, value)) = line.split_once('=')
        {
            apply_cloud_setting(
                &key.trim().to_lowercase(),
                value.trim(),
                &mut params,
                &mut pricing,
            );
        }
    }
    (params, pricing)
}

fn parse_cloud_models(content: &str, base: &ModelParams) -> Vec<CloudModel> {
    let mut models = Vec::new();
    let (default_params, default_pricing) = parse_cloud_defaults(content, base);
    let mut current_name: Option<String> = None;
    let mut current_url = String::new();
    let mut current_key = String::new();
    let mut current_api_model = String::new();
    let mut current_params = default_params.clone();
    let mut current_pricing = default_pricing.clone();
    let mut current_supports_reasoning_effort = true;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            if let Some(name) = current_name.take()
                && !name.eq_ignore_ascii_case("default")
                && !current_url.is_empty()
                && !current_key.is_empty()
                && !is_placeholder_key(&current_key)
            {
                let api_model = if current_api_model.is_empty() {
                    name.clone()
                } else {
                    current_api_model.clone()
                };
                models.push(CloudModel {
                    name,
                    api_url: current_url.clone(),
                    api_key: current_key.clone(),
                    api_model,
                    params: current_params.clone(),
                    pricing: current_pricing.clone(),
                    supports_reasoning_effort: current_supports_reasoning_effort,
                });
            }
            current_name = Some(line[1..line.len() - 1].trim().to_string());
            current_url.clear();
            current_key.clear();
            current_api_model.clear();
            current_params = default_params.clone();
            current_pricing = default_pricing.clone();
            current_supports_reasoning_effort = true;
        } else if let Some((key, value)) = line.split_once('=') {
            let key = key.trim().to_lowercase();
            let value = value.trim().to_string();
            match key.as_str() {
                "api_url" => current_url = value,
                "api_key" => current_key = value,
                "api_model" => current_api_model = value,
                "reasoning_effort_supported" => {
                    current_supports_reasoning_effort =
                        value.parse::<bool>().unwrap_or(true);
                }
                _ => apply_cloud_setting(&key, &value, &mut current_params, &mut current_pricing),
            }
        }
    }

    if let Some(name) = current_name
        && !name.eq_ignore_ascii_case("default")
        && !current_url.is_empty()
        && !current_key.is_empty()
        && !is_placeholder_key(&current_key)
    {
        let api_model = if current_api_model.is_empty() {
            name.clone()
        } else {
            current_api_model
        };
        models.push(CloudModel {
            name,
            api_url: current_url,
            api_key: current_key,
            api_model,
            params: current_params,
            pricing: current_pricing,
            supports_reasoning_effort: current_supports_reasoning_effort,
        });
    }

    models
}

/// Per-model parameters for Ollama models, loaded from `models_ollama.conf`.
///
/// The optional `[default]` section provides the base for every model; a
/// named section starts from those defaults and overrides what it sets.
#[derive(Debug, Clone)]
pub struct ModelParamsStore {
    pub default: ModelParams,
    pub per_model: HashMap<String, ModelParams>,
}

impl ModelParamsStore {
    pub fn params_for(&self, model: &str) -> ModelParams {
        self.per_model
            .get(model)
            .cloned()
            .unwrap_or_else(|| self.default.clone())
    }
}

pub fn load_model_params() -> ModelParamsStore {
    let Some(dir) = conf_dir() else {
        return ModelParamsStore {
            default: ModelParams::default(),
            per_model: HashMap::new(),
        };
    };
    migrate_legacy_conf(&dir, LEGACY_OLLAMA_MODELS_FILE, OLLAMA_MODELS_FILE);
    let conf_path = dir.join(OLLAMA_MODELS_FILE);

    if !conf_path.exists() {
        let _ = fs::create_dir_all(&dir);
        let content = default_model_params_conf();
        let _ = fs::write(&conf_path, &content);
        return parse_model_params(&content);
    }

    match fs::read_to_string(&conf_path) {
        Ok(content) => parse_model_params(&content),
        Err(_) => ModelParamsStore {
            default: ModelParams::default(),
            per_model: HashMap::new(),
        },
    }
}

/// Builds the initial `models_ollama.conf`. Legacy global sampling keys
/// (temperature/top_p/top_k) left over in `rustama.conf` are migrated into
/// the `[default]` section so existing users keep their settings.
fn default_model_params_conf() -> String {
    let mut out = String::from(
        "# Per-model parameters for Ollama models\n\
         # [default] applies to every model; a named section overrides it.\n\
         # Keys: temperature, top_p, top_k, frequency_penalty, presence_penalty,\n\
         #   max_output_tokens, reasoning_effort (low/medium/high/on/off), seed,\n\
         #   num_ctx (context window in tokens, Ollama only)\n\
         \n\
         [default]\n",
    );
    let mut params = ModelParams::default();
    if let Some(conf_path) = find_conf_file()
        && let Ok(content) = fs::read_to_string(&conf_path)
    {
        let values = parse_ini(&content);
        for key in ["temperature", "top_p", "top_k"] {
            if let Some(v) = values.get(key) {
                params.apply_key(key, v);
            }
        }
    }
    for (k, v) in params.to_conf_entries() {
        out.push_str(&format!("{} = {}\n", k, v));
    }
    out.push('\n');
    out
}

fn parse_model_params(content: &str) -> ModelParamsStore {
    // Collect sections in order, preserving raw key/value pairs.
    let mut sections: Vec<(String, Vec<(String, String)>)> = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            sections.push((line[1..line.len() - 1].trim().to_string(), Vec::new()));
        } else if let Some((key, value)) = line.split_once('=')
            && let Some((_, kvs)) = sections.last_mut()
        {
            kvs.push((key.trim().to_lowercase(), value.trim().to_string()));
        }
    }

    let mut default = ModelParams::default();
    if let Some((_, kvs)) = sections.iter().find(|(n, _)| n == "default") {
        for (k, v) in kvs {
            default.apply_key(k, v);
        }
    }

    let mut per_model = HashMap::new();
    for (name, kvs) in &sections {
        if name == "default" {
            continue;
        }
        let mut params = default.clone();
        for (k, v) in kvs {
            params.apply_key(k, v);
        }
        per_model.insert(name.clone(), params);
    }

    ModelParamsStore { default, per_model }
}

/// Persists `params` into the section for `model` — in `models_cloud.conf`
/// for cloud models, otherwise in `models_ollama.conf`.
pub fn save_model_params(model: &str, params: &ModelParams, is_cloud: bool) -> Result<(), String> {
    let dir = conf_dir().ok_or("Cannot determine config directory")?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let (file, legacy) = if is_cloud {
        (CLOUD_MODELS_FILE, LEGACY_CLOUD_MODELS_FILE)
    } else {
        (OLLAMA_MODELS_FILE, LEGACY_OLLAMA_MODELS_FILE)
    };
    migrate_legacy_conf(&dir, legacy, file);
    let path = dir.join(file);
    let content = fs::read_to_string(&path).unwrap_or_else(|_| {
        if is_cloud {
            default_cloud_conf()
        } else {
            default_model_params_conf()
        }
    });
    let updated = update_ini_section(&content, model, &params.to_conf_entries());
    fs::write(&path, updated).map_err(|e| e.to_string())
}

/// Replaces/adds `entries` inside the `[section]` of an INI-style document,
/// preserving everything else (comments, other sections, key order).
/// Existing keys are updated in place; new keys are appended at the end of
/// the section; a missing section is appended at the end of the file.
fn update_ini_section(content: &str, section: &str, entries: &[(String, String)]) -> String {
    let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
    let mut remaining: Vec<(String, String)> = entries.to_vec();

    let mut in_section = false;
    let mut section_found = false;
    let mut insert_at: Option<usize> = None;
    // Index of the last non-blank line seen inside the target section
    // (the section header counts, so empty sections insert right after it).
    let mut last_content: Option<usize> = None;

    for (i, line) in lines.iter_mut().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            if in_section && insert_at.is_none() {
                // The target section ended right before this header; insert
                // after its last content line, keeping trailing blank lines
                // attached to the gap between sections.
                insert_at = Some(last_content.map(|i| i + 1).unwrap_or(i));
            }
            in_section = trimmed[1..trimmed.len() - 1].trim() == section;
            section_found = section_found || in_section;
            if in_section {
                last_content = Some(i);
            }
            continue;
        }
        if in_section && !trimmed.is_empty() {
            last_content = Some(i);
            if !trimmed.starts_with('#')
                && !trimmed.starts_with(';')
                && let Some((key, _)) = trimmed.split_once('=')
            {
                let key = key.trim().to_lowercase();
                if let Some(pos) = remaining.iter().position(|(k, _)| k == &key) {
                    let (_, value) = remaining.remove(pos);
                    *line = format!("{} = {}", key, value);
                }
            }
        }
    }

    if in_section && insert_at.is_none() {
        insert_at = Some(last_content.map(|i| i + 1).unwrap_or(lines.len()));
    }

    if !section_found {
        if !lines.last().is_none_or(|l| l.trim().is_empty()) {
            lines.push(String::new());
        }
        lines.push(format!("[{}]", section));
        for (k, v) in remaining {
            lines.push(format!("{} = {}", k, v));
        }
    } else if !remaining.is_empty() {
        let at = insert_at.unwrap_or(lines.len());
        let new_lines: Vec<String> = remaining
            .iter()
            .map(|(k, v)| format!("{} = {}", k, v))
            .collect();
        let tail: Vec<String> = lines.split_off(at);
        lines.extend(new_lines);
        lines.extend(tail);
    }

    let mut out = lines.join("\n");
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

/// Renames a config file from its pre-rename name on first load. The new
/// file always wins when both exist, so a stale legacy file is ignored.
fn migrate_legacy_conf(dir: &Path, legacy: &str, current: &str) {
    let old_path = dir.join(legacy);
    let new_path = dir.join(current);
    if new_path.exists() || !old_path.exists() {
        return;
    }
    if fs::rename(&old_path, &new_path).is_err() {
        let _ = fs::copy(&old_path, &new_path);
    }
}

pub fn conf_dir() -> Option<PathBuf> {
    let home = dirs_home()?;
    Some(home.join(".config").join("rustama"))
}

/// Read-only snapshot of the per-model config files (`models_ollama.conf`
/// and `models_cloud.conf`) for display commands like `/config`. Unlike the
/// `load_*` functions this never creates, migrates, or rewrites files;
/// missing or unreadable files yield `None` fields.
#[derive(Debug, Clone)]
pub struct ModelsConfFiles {
    pub ollama_path: PathBuf,
    /// Parsed `models_ollama.conf` (`[default]` + per-model sections).
    pub ollama: Option<ModelParamsStore>,
    pub cloud_path: PathBuf,
    /// Named cloud models from `models_cloud.conf` (placeholder api_keys
    /// filtered out, like [`load_cloud_models_with_base`]).
    pub cloud_models: Option<Vec<CloudModel>>,
    /// The `[default]` section of `models_cloud.conf` (params + pricing)
    /// that every named cloud model inherits.
    pub cloud_default: Option<(ModelParams, ModelPricing)>,
}

/// Reads both model-parameter files from the config dir without any
/// write-back side effects. Returns `None` when the config dir cannot be
/// determined (no HOME).
pub fn read_models_conf_files() -> Option<ModelsConfFiles> {
    Some(read_models_conf_files_from(conf_dir()?))
}

fn read_models_conf_files_from(dir: PathBuf) -> ModelsConfFiles {
    let ollama_path = dir.join(OLLAMA_MODELS_FILE);
    let cloud_path = dir.join(CLOUD_MODELS_FILE);
    let ollama = fs::read_to_string(&ollama_path)
        .ok()
        .map(|c| parse_model_params(&c));
    // Cloud sections fall back to the Ollama [default] for unset params,
    // exactly like load_cloud_models_with_base.
    let base = ollama
        .as_ref()
        .map(|s| s.default.clone())
        .unwrap_or_default();
    let cloud_content = fs::read_to_string(&cloud_path).ok();
    let (cloud_models, cloud_default) = match cloud_content {
        Some(ref c) => (
            Some(parse_cloud_models(c, &base)),
            Some(parse_cloud_defaults(c, &base)),
        ),
        None => (None, None),
    };
    ModelsConfFiles {
        ollama_path,
        ollama,
        cloud_path,
        cloud_models,
        cloud_default,
    }
}

/// Resolve a `system_prompt_*` config value: if it names an existing
/// file relative to the config directory, return the file's contents
/// (trimmed) together with the file name; otherwise treat the value as
/// an inline prompt and return it unchanged.
fn resolve_prompt_file(value: &str, dir: &std::path::Path) -> (String, Option<String>) {
    let name = value.trim();
    // A file name is a single short line; real prompts are long and may
    // contain spaces — but a non-existent file must not swallow a prompt,
    // so only accept names that actually resolve to a file.
    if !name.is_empty() && !name.contains('\n') && name.len() < 256 {
        let path = dir.join(name);
        if path.is_file()
            && let Ok(content) = fs::read_to_string(&path)
        {
            return (content.trim().to_string(), Some(name.to_string()));
        }
    }
    (value.to_string(), None)
}

/// Load the application-global AGENTS.md (~/.config/rustama/AGENTS.md).
/// Returns `None` if the file does not exist or is empty after trimming.
/// The content is appended to the system prompt on every request so agents
/// always see these standing instructions.
pub fn load_agents_md() -> Option<String> {
    let path = conf_dir()?.join("AGENTS.md");
    let content = fs::read_to_string(path).ok()?;
    let trimmed = content.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

pub fn find_conf_file() -> Option<PathBuf> {
    let home = dirs_home()?;
    let preferred = home.join(".config").join("rustama").join("rustama.conf");
    if preferred.exists() {
        return Some(preferred);
    }
    let fallback = home.join("rustama.conf");
    if fallback.exists() {
        return Some(fallback);
    }
    None
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var("HOME").map(PathBuf::from).ok()
}

fn parse_bool(s: &str) -> bool {
    matches!(
        s.to_lowercase().as_str(),
        "true" | "yes" | "on" | "igen" | "1"
    )
}

fn parse_ini(content: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            map.insert(key.trim().to_string(), value.trim().to_string());
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lsp_keys_parse_and_default() {
        // Defaults: off, rust-analyzer, empty workspace, auto-diagnostics on.
        let cfg = Config::parse("model = x\n");
        assert!(!cfg.lsp);
        assert_eq!(cfg.lsp_server, "rust-analyzer");
        assert_eq!(cfg.lsp_workspace, "");
        assert!(cfg.lsp_auto_diagnostics);

        let cfg = Config::parse(
            "lsp = igen\n\
             lsp_server = /opt/ra/rust-analyzer\n\
             lsp_workspace = /home/user/project\n\
             lsp_auto_diagnostics = off\n",
        );
        assert!(cfg.lsp);
        assert_eq!(cfg.lsp_server, "/opt/ra/rust-analyzer");
        assert_eq!(cfg.lsp_workspace, "/home/user/project");
        assert!(!cfg.lsp_auto_diagnostics);
    }

    #[test]
    fn lsp_keys_roundtrip_through_default_conf() {
        // default_conf() (fresh installs) must carry parseable lsp_* keys.
        let cfg = Config::default();
        let text = cfg.default_conf();
        let back = Config::parse(&text);
        assert_eq!(back.lsp, cfg.lsp);
        assert_eq!(back.lsp_server, cfg.lsp_server);
        assert_eq!(back.lsp_workspace, cfg.lsp_workspace);
        assert_eq!(back.lsp_auto_diagnostics, cfg.lsp_auto_diagnostics);
    }

    #[test]
    fn auto_continue_limit_parses_and_roundtrips() {
        assert_eq!(Config::parse("model = x\n").max_auto_continues, 10);
        assert_eq!(Config::parse("max_auto_continues = 37\n").max_auto_continues, 37);
        assert_eq!(Config::parse("max_auto_continues = 0\n").max_auto_continues, 0);

        let mut cfg = Config::default();
        cfg.max_auto_continues = 23;
        let back = Config::parse(&cfg.default_conf());
        assert_eq!(back.max_auto_continues, 23);
    }

    #[test]
    fn placeholder_keys_are_skipped() {
        let conf = "[mistral-small]\n\
                    api_url = https://api.mistral.ai/v1/chat/completions\n\
                    api_key = YOUR_MISTRAL_API_KEY\n\
                    \n\
                    [kimi-k3]\n\
                    api_url = https://api.moonshot.ai/v1/chat/completions\n\
                    api_key = YOUR_MOONSHOT_API_KEY\n";
        assert!(parse_cloud_models(conf, &ModelParams::default()).is_empty());
    }

    #[test]
    fn real_keys_are_registered() {
        let conf = "[mistral-small]\n\
                    api_url = https://api.mistral.ai/v1/chat/completions\n\
                    api_key = abc123realkey\n";
        let models = parse_cloud_models(conf, &ModelParams::default());
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].name, "mistral-small");
        assert_eq!(models[0].api_key, "abc123realkey");
        assert_eq!(models[0].api_model, "mistral-small"); // defaults to section name
    }

    #[test]
    fn mixed_placeholder_and_real() {
        let conf = "[placeholder]\n\
                    api_url = https://example.com/v1/chat/completions\n\
                    api_key = YOUR_KEY_HERE\n\
                    \n\
                    [real]\n\
                    api_url = https://example.com/v1/chat/completions\n\
                    api_key = sk-live-123\n";
        let models = parse_cloud_models(conf, &ModelParams::default());
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].name, "real");
    }

    #[test]
    fn missing_key_is_skipped() {
        let conf = "[nokey]\napi_url = https://example.com\n";
        assert!(parse_cloud_models(conf, &ModelParams::default()).is_empty());
    }

    #[test]
    fn shipped_default_conf_has_no_real_keys() {
        // Regression: the default config must never embed a live credential.
        assert!(parse_cloud_models(&default_cloud_conf(), &ModelParams::default()).is_empty());
    }

    #[test]
    fn cloud_section_parses_model_params() {
        let conf = "[kimi-k3]\n\
                    api_url = https://api.moonshot.ai/v1/chat/completions\n\
                    api_key = sk-real\n\
                    temperature = 0.6\n\
                    frequency_penalty = 0.5\n\
                    reasoning_effort = high\n\
                    max_output_tokens = 8192\n\
                    seed = 42\n";
        let models = parse_cloud_models(conf, &ModelParams::default());
        assert_eq!(models.len(), 1);
        let p = &models[0].params;
        assert_eq!(p.temperature, 0.6);
        assert_eq!(p.frequency_penalty, 0.5);
        assert_eq!(p.presence_penalty, 0.0);
        assert_eq!(p.reasoning_effort.as_deref(), Some("high"));
        assert_eq!(p.max_output_tokens, Some(8192));
        assert_eq!(p.seed, Some(42));
        // untouched keys keep cloud defaults
        assert_eq!(p.top_p, 0.9);
        assert_eq!(p.top_k, 40);
    }

    #[test]
    fn cloud_section_defaults_max_output_tokens() {
        let conf = "[m]\napi_url = https://x\napi_key = sk-real\n";
        let models = parse_cloud_models(conf, &ModelParams::default());
        assert_eq!(models[0].params.max_output_tokens, Some(16384));
    }

    #[test]
    fn cloud_default_section_applies_to_every_model() {
        let conf = "[default]\n\
                    tool_choice = auto\n\
                    reasoning_effort = high\n\
                    temperature = 0.7\n\
                    price_input = 2.5\n\
                    \n\
                    [a]\napi_url = https://x\napi_key = sk-real\n\
                    \n\
                    [b]\napi_url = https://y\napi_key = sk-real\n\
                    reasoning_effort = low\n\
                    tool_choice = required\n";
        let models = parse_cloud_models(conf, &ModelParams::default());
        assert_eq!(models.len(), 2); // [default] is not a model
        assert_eq!(models[0].params.tool_choice.as_deref(), Some("auto"));
        assert_eq!(models[0].params.reasoning_effort.as_deref(), Some("high"));
        assert_eq!(models[0].params.temperature, 0.7);
        assert_eq!(models[0].pricing.input_per_mtok, Some(2.5));
        assert_eq!(models[1].params.tool_choice.as_deref(), Some("required"));
        assert_eq!(models[1].params.reasoning_effort.as_deref(), Some("low"));
    }

    #[test]
    fn shipped_cloud_defaults_include_tool_choice_and_reasoning() {
        let (params, _) = parse_cloud_defaults(&default_cloud_conf(), &ModelParams::default());
        assert_eq!(params.tool_choice.as_deref(), Some("auto"));
        assert_eq!(params.reasoning_effort.as_deref(), Some("high"));
        assert_eq!(params.temperature, 1.0);
        assert_eq!(params.top_p, 0.9);
        assert_eq!(params.top_k, 40);
        assert_eq!(params.frequency_penalty, 0.0);
        assert_eq!(params.presence_penalty, 0.0);
    }

    #[test]
    fn missing_cloud_default_section_is_backfilled_once() {
        let old = "[m]\napi_url = https://x\napi_key = sk-real\n";
        let updated = ensure_cloud_default_section(old.to_string());
        assert!(updated.contains("[default]\n"));
        let models = parse_cloud_models(&updated, &ModelParams::default());
        assert_eq!(models[0].params.tool_choice.as_deref(), Some("auto"));
        assert_eq!(models[0].params.reasoning_effort.as_deref(), Some("high"));
        assert_eq!(models[0].params.temperature, 1.0);
        assert_eq!(models[0].params.top_p, 0.9);
        // Idempotent.
        assert_eq!(ensure_cloud_default_section(updated.clone()), updated);

        // Backfills missing keys but never overwrites existing ones.
        let partial =
            "[default]\ntool_choice = none\n[m]\napi_url = https://x\napi_key = sk-real\n";
        let filled = ensure_cloud_default_section(partial.to_string());
        // Keys join the existing block, right below the last key line.
        assert!(filled.starts_with(
            "[default]\ntool_choice = none\nreasoning_effort = high\ntemperature = 1\n"
        ));
        let models = parse_cloud_models(&filled, &ModelParams::default());
        assert_eq!(models[0].params.tool_choice.as_deref(), Some("none"));
        assert_eq!(models[0].params.reasoning_effort.as_deref(), Some("high"));
        assert_eq!(models[0].params.temperature, 1.0);
        assert_eq!(models[0].params.top_p, 0.9);
    }

    #[test]
    fn legacy_conf_files_are_renamed_on_load() {
        let dir = std::env::temp_dir().join(format!("rustama-conf-migrate-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(LEGACY_OLLAMA_MODELS_FILE), "x = 1\n").unwrap();
        fs::write(dir.join(LEGACY_CLOUD_MODELS_FILE), "y = 2\n").unwrap();

        migrate_legacy_conf(&dir, LEGACY_OLLAMA_MODELS_FILE, OLLAMA_MODELS_FILE);
        migrate_legacy_conf(&dir, LEGACY_CLOUD_MODELS_FILE, CLOUD_MODELS_FILE);

        assert!(!dir.join(LEGACY_OLLAMA_MODELS_FILE).exists());
        assert!(!dir.join(LEGACY_CLOUD_MODELS_FILE).exists());
        assert_eq!(
            fs::read_to_string(dir.join(OLLAMA_MODELS_FILE)).unwrap(),
            "x = 1\n"
        );
        assert_eq!(
            fs::read_to_string(dir.join(CLOUD_MODELS_FILE)).unwrap(),
            "y = 2\n"
        );

        // The renamed file wins: a stale legacy file is left untouched.
        fs::write(dir.join(LEGACY_OLLAMA_MODELS_FILE), "stale = 1\n").unwrap();
        migrate_legacy_conf(&dir, LEGACY_OLLAMA_MODELS_FILE, OLLAMA_MODELS_FILE);
        assert_eq!(
            fs::read_to_string(dir.join(OLLAMA_MODELS_FILE)).unwrap(),
            "x = 1\n"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn example_config_files_parse() {
        let main = Config::parse(include_str!("../config.example/rustama.conf"));
        assert_eq!(main.mode, "coder");
        assert_eq!(main.max_auto_continues, 10);
        assert_eq!(main.max_tool_rounds, 100);
        assert_eq!(main.terminal_width_pct, 40);
        assert!(!main.lsp);
        assert!(!main.hintbar);

        let cloud = include_str!("../config.example/models_cloud.conf");
        // Placeholder keys (YOUR_...) mean no model is registered yet.
        assert!(parse_cloud_models(cloud, &ModelParams::default()).is_empty());
        let (defaults, _) = parse_cloud_defaults(cloud, &ModelParams::default());
        assert_eq!(defaults.tool_choice.as_deref(), Some("auto"));
        assert_eq!(defaults.top_p, 0.9);

        let ollama = include_str!("../config.example/models_ollama.conf");
        let store = parse_model_params(ollama);
        assert_eq!(store.default.top_k, 40);
        assert_eq!(store.params_for("llama3.2:3b").temperature, 0.7);
        assert_eq!(store.params_for("qwen3:8b").num_ctx, Some(32768));
    }

    #[test]
    fn read_models_conf_files_reads_both_files_without_writing() {
        let dir = std::env::temp_dir().join(format!(
            "rustama-test-{}-models-conf-snap",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join(OLLAMA_MODELS_FILE),
            "[default]\ntemperature = 0.7\n\n[qwen3:8b]\nseed = 42\n",
        )
        .unwrap();
        fs::write(
            dir.join(CLOUD_MODELS_FILE),
            "[default]\ntool_choice = auto\nreasoning_effort = high\n\n\
             [kimi]\napi_url = https://x\napi_key = sk-real\nprice_input = 2.0\n",
        )
        .unwrap();

        let snap = read_models_conf_files_from(dir.clone());
        assert_eq!(snap.ollama_path, dir.join(OLLAMA_MODELS_FILE));
        assert_eq!(snap.cloud_path, dir.join(CLOUD_MODELS_FILE));

        let ollama = snap.ollama.as_ref().unwrap();
        assert_eq!(ollama.default.temperature, 0.7);
        assert_eq!(ollama.per_model["qwen3:8b"].seed, Some(42));

        let models = snap.cloud_models.as_ref().unwrap();
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].name, "kimi");
        assert_eq!(models[0].pricing.input_per_mtok, Some(2.0));

        let (cdefault, _) = snap.cloud_default.as_ref().unwrap();
        assert_eq!(cdefault.tool_choice.as_deref(), Some("auto"));
        assert_eq!(cdefault.reasoning_effort.as_deref(), Some("high"));

        // Read-only: nothing created or rewritten (no legacy files appear).
        let entries: Vec<_> = fs::read_dir(&dir).unwrap().collect();
        assert_eq!(entries.len(), 2);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn read_models_conf_files_tolerates_missing_files() {
        let dir = std::env::temp_dir().join(format!(
            "rustama-test-{}-models-conf-empty",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let snap = read_models_conf_files_from(dir.clone());
        assert!(snap.ollama.is_none());
        assert!(snap.cloud_models.is_none());
        assert!(snap.cloud_default.is_none());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn cloud_sections_inherit_default_base() {
        // The [default] section of models_ollama.conf is the fallback for
        // cloud models too; section keys still win; max_output_tokens keeps
        // its own cloud default.
        let mut base = ModelParams::default();
        base.top_p = 0.95;
        base.frequency_penalty = 0.4;
        let conf = "[a]\napi_url = https://x\napi_key = sk-real\n\n\
                    [b]\napi_url = https://y\napi_key = sk-real\ntop_p = 0.5\n";
        let models = parse_cloud_models(conf, &base);
        assert_eq!(models[0].params.top_p, 0.95); // inherited from base
        assert_eq!(models[0].params.frequency_penalty, 0.4);
        assert_eq!(models[0].params.max_output_tokens, Some(16384));
        assert_eq!(models[1].params.top_p, 0.5); // section overrides base
        assert_eq!(models[1].params.frequency_penalty, 0.4);
    }

    #[test]
    fn model_params_default_section_applies_to_all() {
        let conf = "[default]\n\
                    temperature = 0.7\n\
                    frequency_penalty = 0.3\n\
                    \n\
                    [llama3.2:3b]\n\
                    temperature = 0.2\n\
                    reasoning_effort = medium\n";
        let store = parse_model_params(conf);
        assert_eq!(store.default.temperature, 0.7);
        assert_eq!(store.default.frequency_penalty, 0.3);
        // named section inherits from [default] and overrides
        let p = store.params_for("llama3.2:3b");
        assert_eq!(p.temperature, 0.2);
        assert_eq!(p.frequency_penalty, 0.3);
        assert_eq!(p.reasoning_effort.as_deref(), Some("medium"));
        // unknown model falls back to [default]
        let p = store.params_for("no-such-model");
        assert_eq!(p.temperature, 0.7);
    }

    #[test]
    fn apply_key_validates_ranges_and_values() {
        let mut p = ModelParams::default();
        assert!(p.apply_key("temperature", "9.9")); // out of range -> ignored
        assert_eq!(p.temperature, 1.0);
        assert!(p.apply_key("frequency_penalty", "-1.5"));
        assert_eq!(p.frequency_penalty, -1.5);
        assert!(p.apply_key("reasoning_effort", "HIGH"));
        assert_eq!(p.reasoning_effort.as_deref(), Some("high"));
        assert!(p.apply_key("reasoning_effort", "bogus")); // ignored
        assert_eq!(p.reasoning_effort.as_deref(), Some("high"));
        assert!(p.apply_key("tool_choice", "REQUIRED"));
        assert_eq!(p.tool_choice.as_deref(), Some("required"));
        assert!(p.apply_key("tool_choice", "bogus")); // ignored
        assert_eq!(p.tool_choice.as_deref(), Some("required"));
        assert!(p.apply_key("max_output_tokens", "off"));
        assert_eq!(p.max_output_tokens, None);
        assert!(!p.apply_key("not_a_param", "1"));
    }

    #[test]
    fn apply_key_handles_num_ctx() {
        let mut p = ModelParams::default();
        assert_eq!(p.num_ctx, None);
        assert!(p.apply_key("num_ctx", "202752"));
        assert_eq!(p.num_ctx, Some(202752));
        // aliases
        let mut q = ModelParams::default();
        assert!(q.apply_key("n_ctx", "8192"));
        assert_eq!(q.num_ctx, Some(8192));
        let mut r = ModelParams::default();
        assert!(r.apply_key("context_length", "4096"));
        assert_eq!(r.num_ctx, Some(4096));
        // off / invalid
        let mut s = ModelParams::default();
        s.num_ctx = Some(123);
        assert!(s.apply_key("num_ctx", "off"));
        assert_eq!(s.num_ctx, None);
        assert!(p.apply_key("num_ctx", "abc")); // ignored
        assert_eq!(p.num_ctx, Some(202752));
    }

    #[test]
    fn num_ctx_round_trips_through_conf_entries() {
        let mut p = ModelParams::default();
        p.num_ctx = Some(202752);
        let entries = p.to_conf_entries();
        assert!(entries.iter().any(|(k, v)| k == "num_ctx" && v == "202752"));
        // parses back
        let mut q = ModelParams::default();
        for (k, v) in &entries {
            q.apply_key(k, v);
        }
        assert_eq!(q.num_ctx, Some(202752));
    }

    #[test]
    fn update_ini_section_updates_in_place() {
        let content = "# header\n\
                       [a]\n\
                       temperature = 1.0\n\
                       # comment\n\
                       api_key = secret\n\
                       \n\
                       [b]\n\
                       temperature = 2.0\n";
        let entries = vec![("temperature".to_string(), "0.5".to_string())];
        let out = update_ini_section(content, "a", &entries);
        assert!(out.contains("[a]\ntemperature = 0.5\n# comment\napi_key = secret"));
        // other section untouched
        assert!(out.contains("[b]\ntemperature = 2.0"));
    }

    #[test]
    fn update_ini_section_appends_missing_keys_at_section_end() {
        let content = "[a]\ntemperature = 1.0\n\n[b]\nx = 1\n";
        let entries = vec![
            ("temperature".to_string(), "0.5".to_string()),
            ("seed".to_string(), "7".to_string()),
        ];
        let out = update_ini_section(content, "a", &entries);
        assert!(out.contains("[a]\ntemperature = 0.5\nseed = 7\n\n[b]\nx = 1\n"));
    }

    #[test]
    fn update_ini_section_creates_missing_section() {
        let content = "[a]\nx = 1\n";
        let entries = vec![("temperature".to_string(), "0.5".to_string())];
        let out = update_ini_section(content, "new-model", &entries);
        assert!(out.contains("[a]\nx = 1\n"));
        assert!(out.contains("[new-model]\ntemperature = 0.5\n"));
    }

    #[test]
    fn update_ini_section_at_file_end() {
        // Target section is the last one; new keys append at EOF.
        let content = "[a]\nx = 1\n\n[b]\ny = 2\n";
        let entries = vec![("seed".to_string(), "3".to_string())];
        let out = update_ini_section(content, "b", &entries);
        assert!(out.contains("[b]\ny = 2\nseed = 3\n"));
    }
}
