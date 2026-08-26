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
    pub terminal_width_pct: u16,
    pub justify: bool,
}

/// Per-model sampling / generation parameters.
///
/// Used both for cloud models (parsed from the model's section in
/// `cloud_models.conf`) and for Ollama models (parsed from the model's
/// section in `model_params.conf`, falling back to that file's `[default]`
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
    pub seed: Option<u32>,
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
            seed: None,
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
            "seed" => {
                let v = value.trim();
                if v.is_empty() || v.eq_ignore_ascii_case("off") || v.eq_ignore_ascii_case("none") {
                    self.seed = None;
                } else if let Ok(n) = v.parse::<u32>() {
                    self.seed = Some(n);
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
        if let Some(n) = self.seed {
            out.push(("seed".to_string(), n.to_string()));
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
                    (
                        "logging",
                        if dflt.logging {
                            "true".to_string()
                        } else {
                            "false".to_string()
                        },
                    ),
                    ("logfile", dflt.logfile.clone()),
                    ("system_prompt", dflt.system_prompt.clone()),
                    (
                        "justify",
                        if dflt.justify {
                            "true".to_string()
                        } else {
                            "false".to_string()
                        },
                    ),
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
             # Justify paragraphs in output (true/false, default: false)\n\
             justify = {}\n\n\
             # NOTE: model parameters (temperature, top_p, top_k,\n\
             # frequency_penalty, presence_penalty, max_output_tokens,\n\
             # reasoning_effort, seed) are per-model now — see\n\
             # model_params.conf (Ollama models) and cloud_models.conf.\n",
            self.ollama_url,
            self.model,
            self.save_path,
            self.agentic,
            self.timeout_secs,
            self.logging,
            self.logfile,
            self.system_prompt,
            self.max_tool_rounds,
            self.max_retries,
            self.justify,
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
        if let Some(v) = values.get("system_prompt") {
            cfg.system_prompt = v.clone();
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
            && (1..=100).contains(&n)
        {
            cfg.max_tool_rounds = n;
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
                if !lines.last().is_none_or(|l| l.is_empty()) {
                    lines.push(String::new());
                }
                lines.push(format!("system_prompt = {}", prompt));
            }
            let new_content = lines.join("\n");
            fs::write(&conf_path, new_content).map_err(|e| e.to_string())?;
        } else {
            let dir = conf_dir().ok_or("Cannot determine config directory")?;
            let conf_path = dir.join("rustama.conf");
            let cfg = Config {
                system_prompt: prompt.to_string(),
                ..Config::default()
            };
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
        lines.push(format!("terminal_width_pct = {}", self.terminal_width_pct));
        lines.push(format!("justify = {}", self.justify));
        fs::write(&conf_path, lines.join("\n")).map_err(|e| e.to_string())?;
        Ok(())
    }
}

/// Loads cloud models, using `base` (typically the `[default]` section of
/// `model_params.conf`) as the fallback for parameters a cloud section does
/// not set itself — so a global default like `top_p` applies to cloud
/// models too.
pub fn load_cloud_models_with_base(base: &ModelParams) -> Vec<CloudModel> {
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

    parse_cloud_models(&content, base)
}

fn default_cloud_conf() -> String {
    "# Cloud models configuration\n\
     # Each model has its own section [model_name]\n\
     # Required fields: api_url, api_key\n\
     # Optional: api_model (actual model name sent to API, defaults to section name)\n\
     # Optional: max_output_tokens (default: 16384)\n\
     # Optional model parameters: temperature (1.0), top_p (0.9), top_k (40),\n\
     #   frequency_penalty (0.0), presence_penalty (0.0),\n\
     #   reasoning_effort (low/medium/high/on/off), seed\n\
     # Optional pricing in USD per 1M tokens (enables cost estimate):\n\
     #   price_input, price_output, price_cached_read, price_cache_write\n\
     \n\
     [mistral-small]\n\
     api_url = https://api.mistral.ai/v1/chat/completions\n\
     api_key = YOUR_MISTRAL_API_KEY\n\
     \n\
     [kimi-k3]\n\
     api_url = https://api.moonshot.ai/v1/chat/completions\n\
     api_key = YOUR_MOONSHOT_API_KEY\n\
     \n\
     "
    .to_string()
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

fn parse_cloud_models(content: &str, base: &ModelParams) -> Vec<CloudModel> {
    let mut models = Vec::new();
    let mut current_name: Option<String> = None;
    let mut current_url = String::new();
    let mut current_key = String::new();
    let mut current_api_model = String::new();
    let mut current_params = cloud_params_default(base);
    let mut current_pricing = ModelPricing::default();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            if let Some(name) = current_name.take()
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
                });
            }
            current_name = Some(line[1..line.len() - 1].trim().to_string());
            current_url.clear();
            current_key.clear();
            current_api_model.clear();
            current_params = cloud_params_default(base);
            current_pricing = ModelPricing::default();
        } else if let Some((key, value)) = line.split_once('=') {
            let key = key.trim().to_lowercase();
            let value = value.trim().to_string();
            match key.as_str() {
                "api_url" => current_url = value,
                "api_key" => current_key = value,
                "api_model" => current_api_model = value,
                "price_input" | "input_price" => {
                    current_pricing.input_per_mtok = value.parse().ok();
                }
                "price_output" | "output_price" => {
                    current_pricing.output_per_mtok = value.parse().ok();
                }
                "price_cached_read" | "cached_read_price" | "price_cached" => {
                    current_pricing.cached_read_per_mtok = value.parse().ok();
                }
                "price_cache_write" | "cache_write_price" => {
                    current_pricing.cache_write_per_mtok = value.parse().ok();
                }
                _ => {
                    current_params.apply_key(&key, &value);
                }
            }
        }
    }

    if let Some(name) = current_name
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
        });
    }

    models
}

/// Per-model parameters for Ollama models, loaded from `model_params.conf`.
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
    let conf_path = dir.join("model_params.conf");

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

/// Builds the initial `model_params.conf`. Legacy global sampling keys
/// (temperature/top_p/top_k) left over in `rustama.conf` are migrated into
/// the `[default]` section so existing users keep their settings.
fn default_model_params_conf() -> String {
    let mut out = String::from(
        "# Per-model parameters for Ollama models\n\
         # [default] applies to every model; a named section overrides it.\n\
         # Keys: temperature, top_p, top_k, frequency_penalty, presence_penalty,\n\
         #   max_output_tokens, reasoning_effort (low/medium/high/on/off), seed\n\
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

/// Persists `params` into the section for `model` — in `cloud_models.conf`
/// for cloud models, otherwise in `model_params.conf`.
pub fn save_model_params(model: &str, params: &ModelParams, is_cloud: bool) -> Result<(), String> {
    let dir = conf_dir().ok_or("Cannot determine config directory")?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(if is_cloud {
        "cloud_models.conf"
    } else {
        "model_params.conf"
    });
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
    fn cloud_sections_inherit_default_base() {
        // The [default] section of model_params.conf is the fallback for
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
        assert!(p.apply_key("max_output_tokens", "off"));
        assert_eq!(p.max_output_tokens, None);
        assert!(!p.apply_key("not_a_param", "1"));
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
