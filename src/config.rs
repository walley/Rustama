use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    pub ollama_url: String,
    pub model: String,
    pub save_path: String,
    pub agentic: bool,
    pub timeout_secs: u64,
    pub logging: bool,
    pub logfile: String,
    pub system_prompt: String,
    pub proxy: Option<String>,
    pub max_tool_rounds: usize,
    pub max_retries: u32,
    pub temperature: f64,
    pub top_p: f64,
    pub top_k: u32,
    pub terminal_width_pct: u16,
    pub justify: bool,
}

#[derive(Debug, Clone)]
pub struct CloudModel {
    pub name: String,
    pub api_url: String,
    pub api_key: String,
    pub api_model: String,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            ollama_url: "http://localhost:11434".to_string(),
            model: "llama3.2:3b".to_string(),
            save_path: "output.md".to_string(),
            agentic: true,
            timeout_secs: 300,
            logging: true,
            logfile: "rustama.log".to_string(),
            system_prompt: "You are a coding assistant with access to tools. When the user asks you to do something, use the available tools to accomplish the task. Always use tools when needed - do not just describe what you would do. Execute the actual tool calls. After using a tool, continue working until the task is complete.".to_string(),
            proxy: None,
            max_tool_rounds: 10,
            max_retries: 10,
            temperature: 1.0,
            top_p: 0.9,
            top_k: 40,
            terminal_width_pct: 40,
            justify: false,
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
                let cfg = Self::parse(&content);
                let dflt = Config::default();
                let missing: Vec<(&str, String)> = vec![
                    ("logging", if dflt.logging { "true".to_string() } else { "false".to_string() }),
                    ("logfile", dflt.logfile.clone()),
                    ("system_prompt", dflt.system_prompt.clone()),
                    ("justify", if dflt.justify { "true".to_string() } else { "false".to_string() }),
                ];

                let mut updated = content.clone();
                for (key, value) in &missing {
                    if !content.contains(key) {
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
        let cfg = Config::default();
        let conf_path = dir.join("rustama.conf");
        let _ = fs::write(&conf_path, cfg.default_conf());
        cfg
    }

    fn default_conf(&self) -> String {
        format!(
            "# Rustama configuration\n\n\
             # Ollama API URL\n\
             ollama_url = {}\n\n\
             # Default model\n\
             model = {}\n\n\
             # Default save path\n\
             save_path = {}\n\n\
             # Enable agentic mode on startup (true/false)\n\
             agentic = {}\n\n\
             # HTTP request timeout in seconds\n\
             timeout_secs = {}\n\n\
             # Enable logging on startup (true/yes/on or false/no/off)\n\
             logging = {}\n\n\
             # Log file path\n\
             logfile = {}\n\n\
             # System prompt (set via /setsystem command)\n\
             system_prompt = {}\n\n\
             # HTTP proxy URL (optional, e.g. http://proxy:8080)\n\
             # proxy = http://proxy:8080\n\n\
             # Max agentic tool rounds per request (1-100, default: 10)\n\
             max_tool_rounds = {}\n\n\
             # Max API retries on 429/rate-limit errors (1-50, default: 10)\n\
             max_retries = {}\n\n\
             # Sampling temperature (0.0-2.0, default: 1.0)\n\
             temperature = {}\n\n\
             # Top-p sampling (0.0-1.0, default: 0.9)\n\
             top_p = {}\n\n\
              # Top-k sampling (1-100, default: 40)\n\
              top_k = {}\n\n\
              # Justify paragraphs in output (true/false, default: false)\n\
              justify = {}\n",
            self.ollama_url, self.model, self.save_path, self.agentic, self.timeout_secs,
            self.logging, self.logfile, self.system_prompt, self.max_tool_rounds, self.max_retries,
            self.temperature, self.top_p, self.top_k, self.justify,
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
        if let Some(v) = values.get("timeout_secs") {
            if let Ok(n) = v.parse::<u64>() {
                cfg.timeout_secs = n;
            }
        }
        if let Some(v) = values.get("logging") {
            cfg.logging = parse_bool(v);
        }
        if let Some(v) = values.get("logfile") {
            cfg.logfile = v.clone();
        }
        if let Some(v) = values.get("system_prompt") {
            cfg.system_prompt = v.clone();
        }
        if let Some(v) = values.get("proxy") {
            let v = v.trim();
            if v.is_empty() || v.eq_ignore_ascii_case("off") || v.eq_ignore_ascii_case("none") || v.eq_ignore_ascii_case("false") {
                cfg.proxy = None;
            } else {
                cfg.proxy = Some(v.to_string());
            }
        }
        if let Some(v) = values.get("max_tool_rounds") {
            if let Ok(n) = v.parse::<usize>() {
                if n >= 1 && n <= 100 {
                    cfg.max_tool_rounds = n;
                }
            }
        }
        if let Some(v) = values.get("max_retries") {
            if let Ok(n) = v.parse::<u32>() {
                if n >= 1 && n <= 50 {
                    cfg.max_retries = n;
                }
            }
        }
        if let Some(v) = values.get("temperature") {
            if let Ok(n) = v.parse::<f64>() {
                if n >= 0.0 && n <= 2.0 {
                    cfg.temperature = n;
                }
            }
        }
        if let Some(v) = values.get("top_p") {
            if let Ok(n) = v.parse::<f64>() {
                if n >= 0.0 && n <= 1.0 {
                    cfg.top_p = n;
                }
            }
        }
        if let Some(v) = values.get("top_k") {
            if let Ok(n) = v.parse::<u32>() {
                if n >= 1 && n <= 100 {
                    cfg.top_k = n;
                }
            }
        }
        if let Some(v) = values.get("terminal_width_pct") {
            if let Ok(n) = v.parse::<u16>() {
                if n >= 20 && n <= 80 {
                    cfg.terminal_width_pct = n;
                }
            }
        }
        if let Some(v) = values.get("justify") {
            cfg.justify = parse_bool(v);
        }

        cfg
    }

    pub fn set_system_prompt(prompt: &str) -> Result<(), String> {
        if let Some(conf_path) = find_conf_file() {
            let content = fs::read_to_string(&conf_path).map_err(|e| e.to_string())?;
            let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
            let mut found = false;
            for line in &mut lines {
                if line.starts_with("system_prompt") {
                    *line = format!("system_prompt = {}", prompt);
                    found = true;
                    break;
                }
            }
            if !found {
                if !lines.last().map_or(true, |l| l.is_empty()) {
                    lines.push(String::new());
                }
                lines.push(format!("system_prompt = {}", prompt));
            }
            let new_content = lines.join("\n");
            fs::write(&conf_path, new_content).map_err(|e| e.to_string())?;
        } else {
            let dir = conf_dir().ok_or("Cannot determine config directory")?;
            let conf_path = dir.join("rustama.conf");
            let mut cfg = Config::default();
            cfg.system_prompt = prompt.to_string();
            fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            fs::write(&conf_path, cfg.default_conf()).map_err(|e| e.to_string())?;
        }
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
        lines.push(format!("timeout_secs = {}", self.timeout_secs));
        lines.push(format!("logging = {}", self.logging));
        lines.push(format!("logfile = {}", self.logfile));
        lines.push(format!("system_prompt = {}", self.system_prompt));
        lines.push(String::new());
        if let Some(ref proxy) = self.proxy {
            lines.push(format!("proxy = {}", proxy));
        } else {
            lines.push("# proxy = off".to_string());
        }
        lines.push(format!("max_tool_rounds = {}", self.max_tool_rounds));
        lines.push(format!("max_retries = {}", self.max_retries));
        lines.push(format!("temperature = {}", self.temperature));
        lines.push(format!("top_p = {}", self.top_p));
        lines.push(format!("top_k = {}", self.top_k));
        lines.push(format!("terminal_width_pct = {}", self.terminal_width_pct));
        lines.push(format!("justify = {}", self.justify));
        fs::write(&conf_path, lines.join("\n")).map_err(|e| e.to_string())?;
        Ok(())
    }
}

pub fn load_cloud_models() -> Vec<CloudModel> {
    let dir = match conf_dir() {
        Some(d) => d,
        None => return Vec::new(),
    };
    let conf_path = dir.join("cloud_models.conf");

    if !conf_path.exists() {
        let _ = fs::create_dir_all(&dir);
        let _ = fs::write(&conf_path, default_cloud_conf());
        return Vec::new();
    }

    let content = match fs::read_to_string(&conf_path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    parse_cloud_models(&content)
}

fn default_cloud_conf() -> String {
    "# Cloud models configuration\n\
     # Each model has its own section [model_name]\n\
     # Required fields: api_url, api_key\n\
     # Optional: api_model (actual model name sent to API, defaults to section name)\n\
     \n\
     [mistral-small]\n\
     api_url = https://api.mistral.ai/v1/chat/completions\n\
     api_key = XJUStDrci7RWGaYXPxKWJ0urj4vlYqKA\n\
     \n\
     [kimi-k3]\n\
     api_url = https://api.moonshot.ai/v1/chat/completions\n\
     api_key = YOUR_MOONSHOT_API_KEY\n\
     \n\
     "
    .to_string()
}

fn parse_cloud_models(content: &str) -> Vec<CloudModel> {
    let mut models = Vec::new();
    let mut current_name: Option<String> = None;
    let mut current_url = String::new();
    let mut current_key = String::new();
    let mut current_api_model = String::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            if let Some(name) = current_name.take() {
                if !current_url.is_empty() && !current_key.is_empty() {
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
                    });
                }
            }
            current_name = Some(line[1..line.len() - 1].trim().to_string());
            current_url.clear();
            current_key.clear();
            current_api_model.clear();
        } else if let Some((key, value)) = line.split_once('=') {
            let key = key.trim().to_lowercase();
            let value = value.trim().to_string();
            match key.as_str() {
                "api_url" => current_url = value,
                "api_key" => current_key = value,
                "api_model" => current_api_model = value,
                _ => {}
            }
        }
    }

    if let Some(name) = current_name {
        if !current_url.is_empty() && !current_key.is_empty() {
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
            });
        }
    }

    models
}

fn conf_dir() -> Option<PathBuf> {
    let home = dirs_home()?;
    Some(home.join(".config").join("rustama"))
}

fn find_conf_file() -> Option<PathBuf> {
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
    std::env::var("HOME")
        .map(PathBuf::from)
        .ok()
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
