# Rustama 🦀

A Rust-based TUI (Terminal User Interface) client for Ollama and cloud AI models with agentic capabilities.

## Overview

Rustama is a modern, terminal-based interface for interacting with [Ollama](https://ollama.ai/) models and cloud AI providers. Built with Rust, it provides a responsive TUI experience with support for agentic interactions, allowing you to leverage local and remote language models directly from your terminal.

## Features

- 🎨 **Modern TUI Interface** - Built with [ratatui](https://github.com/ratatui-org/ratatui), with mouse support, a menu bar (File / Edit / View / Help), and dialog-based workflows
- ⚡ **Async/Await Support** - Powered by [tokio](https://tokio.rs/) for responsive interactions
- 🤖 **Agentic Mode** - Tool-based agent interactions with configurable tool rounds
- ☁️ **Cloud Model Support** - Connect to OpenAI-compatible APIs (Mistral, Moonshot, etc.)
- 📡 **Streaming Responses** - Real-time streaming of model outputs, including thinking/reasoning traces
- 📝 **Markdown Rendering** - Full markdown support with syntax highlighting
- 🎛️ **Per-Model Parameters** - Temperature, top-p, top-k, penalties, seed, reasoning effort and more, configured per model
- 💾 **Session Management** - Save, load, and manage chat sessions
- 🖥️ **Embedded Terminal** - Interactive terminal panel the agent (and you) can drive
- 📋 **Clipboard Integration** - Copy/paste support, including X11 primary selection for mouse-selected text
- 📊 **Token Usage Stats** - Prompt/response token counts displayed per message
- 🔧 **Highly Configurable** - INI-style configuration with sane defaults

## Requirements

- Rust 2024 edition or later
- Ollama service running locally or remotely (for local models)
- Terminal with support for modern ANSI escape codes

## Installation

1. Clone the repository:
```bash
git clone https://github.com/walley/Rustama.git
cd Rustama
```

2. Build the project:
```bash
cargo build --release
```

3. Run Rustama:
```bash
cargo run --release
```

## Configuration

Rustama uses INI-style configuration files stored in `~/.config/rustama/`. On first run, default configuration files are created automatically.

| File | Purpose |
|------|---------|
| `rustama.conf` | Main application settings |
| `model_params.conf` | Per-model sampling parameters for Ollama models |
| `cloud_models.conf` | Cloud model definitions + their parameters |

Session files live separately in `$DOCUMENTS/rustama/` (see [Session Files](#session-files)).

### Main Configuration: `~/.config/rustama/rustama.conf`

This is the primary configuration file. All options are optional — defaults are used for any missing values.

```ini
# Ollama API URL
ollama_url = http://localhost:11434

# Default model to use on startup
model = llama3.2:3b

# Default file path for saving output
save_path = output.md

# Enable agentic mode on startup (true/false)
agentic = true

# HTTP request timeout in seconds
timeout_secs = 300

# Enable logging on startup (true/yes/on or false/no/off)
logging = true

# Log file path
logfile = rustama.log

# System prompt sent with every conversation
system_prompt = You are a coding assistant with access to tools.

# HTTP proxy URL (optional — set to "off" or leave commented to disable)
# proxy = http://proxy:8080

# Max agentic tool rounds per request (1–100, default: 10)
max_tool_rounds = 10

# Max API retries on 429/rate-limit errors (1–50, default: 10)
max_retries = 10

# Terminal output width percentage (20–80, default: 40)
terminal_width_pct = 40

# Justify paragraphs in output (true/false, default: false)
justify = false
```

> **Note:** Sampling parameters (`temperature`, `top_p`, `top_k`, etc.) are **per-model** now — see `model_params.conf` and `cloud_models.conf` below. Legacy global values left in `rustama.conf` are migrated into the `[default]` section of `model_params.conf` automatically.

#### Configuration Options Reference

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `ollama_url` | string | `http://localhost:11434` | URL of the Ollama API endpoint |
| `model` | string | `llama3.2:3b` | Default model to use on startup |
| `save_path` | string | `output.md` | Default file path for saving chat output |
| `agentic` | bool | `true` | Enable agentic mode on startup |
| `timeout_secs` | integer | `300` | HTTP request timeout in seconds |
| `logging` | bool | `true` | Enable session logging on startup |
| `logfile` | string | `rustama.log` | Path to the log file |
| `system_prompt` | string | (coding assistant prompt) | System prompt sent with every conversation |
| `proxy` | string | _(none)_ | HTTP proxy URL; set to `off`, `none`, or `false` to disable |
| `max_tool_rounds` | integer | `10` | Maximum agentic tool call rounds per request (1–100) |
| `max_retries` | integer | `10` | Maximum API retries on rate-limit errors (1–50) |
| `terminal_width_pct` | integer | `40` | Percentage of terminal width for the output panel (20–80) |
| `justify` | bool | `false` | Justify text paragraphs in the output pane |

Boolean values accept: `true`, `yes`, `on`, `1` (and `false`, `no`, `off` for false).

### Per-Model Parameters: `~/.config/rustama/model_params.conf`

Sampling/generation parameters for Ollama models. The `[default]` section applies to every model; a named section starts from those defaults and overrides what it sets.

```ini
# Per-model parameters for Ollama models
# [default] applies to every model; a named section overrides it.

[default]
temperature = 1.0
top_p = 0.9
top_k = 40
frequency_penalty = 0.0
presence_penalty = 0.0

[llama3.2:3b]
temperature = 0.7
reasoning_effort = medium

[qwen3:8b]
seed = 42
max_output_tokens = 8192
```

#### Model Parameter Keys

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `temperature` | float | `1.0` | Sampling temperature (0.0–2.0) |
| `top_p` | float | `0.9` | Top-p (nucleus) sampling (0.0–1.0) |
| `top_k` | integer | `40` | Top-k sampling (1–100) |
| `frequency_penalty` | float | `0.0` | Frequency penalty (−2.0–2.0) |
| `presence_penalty` | float | `0.0` | Presence penalty (−2.0–2.0) |
| `max_output_tokens` | integer | _(provider default)_ | Max output tokens; `off`/`none` to unset |
| `reasoning_effort` | string | _(unset)_ | `low`, `medium`, `high`, or `on`/`off` to toggle thinking |
| `seed` | integer | _(unset)_ | Sampling seed; `off`/`none` to unset |

Keys not recognized by a given backend are simply not sent.

### Cloud Models Configuration: `~/.config/rustama/cloud_models.conf`

Define cloud-hosted models (OpenAI-compatible APIs) in a separate configuration file. Each model gets its own `[section]`, and supports the same model parameter keys listed above. Parameters not set in a section fall back to the `[default]` section of `model_params.conf` (except `max_output_tokens`, which defaults to `16384` for cloud models).

```ini
# Cloud models configuration
# Required fields per section: api_url, api_key
# Optional: api_model, plus any model parameter key

[mistral-small]
api_url = https://api.mistral.ai/v1/chat/completions
api_key = YOUR_MISTRAL_API_KEY

[kimi-k3]
api_url = https://api.moonshot.ai/v1/chat/completions
api_key = YOUR_MOONSHOT_API_KEY
api_model = moonshot-v1-8k
max_output_tokens = 16384
temperature = 0.6
reasoning_effort = high
```

#### Cloud Model Options

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `api_url` | yes | — | The chat completions endpoint URL |
| `api_key` | yes | — | API authentication key (sections with `YOUR_...` placeholders are ignored) |
| `api_model` | no | section name | Actual model name sent to the API |
| `max_output_tokens` | no | `16384` | Maximum output tokens per response |
| _(any model parameter key)_ | no | `[default]` section | Per-model overrides (temperature, penalties, seed, effort, …) |

Cloud models appear in the model picker alongside local Ollama models. Use `/model` or `/list` to browse and switch between them.

### Runtime Configuration (Slash Commands)

Many settings can be viewed or changed at runtime using slash commands:

| Command | Description |
|---------|-------------|
| `/help` | Show all available commands |
| `/config` | Show current configuration |
| `/status` | Show session status |
| `/model <name>` | Switch to a different model (no arg opens picker dialog) |
| `/use <name>` | Alias for `/model` |
| `/list` | List available models |
| `/tools` | List available agentic tools |
| `/system` | Show current system prompt |
| `/setsystem <text>` | Set system prompt (persisted to config; no arg clears) |
| `/temp <n>` | Set temperature (0.0–2.0) |
| `/topp <n>` | Set top-p (0.0–1.0) |
| `/topk <n>` | Set top-k (1–100) |
| `/fpen <n>` | Set frequency penalty (−2.0–2.0) |
| `/ppen <n>` | Set presence penalty (−2.0–2.0) |
| `/effort <level>` | Set reasoning effort (`low`/`medium`/`high`/`on`/`off`/`none`) |
| `/maxtokens <n>` | Set max output tokens (or `/maxtokens off`) |
| `/seed <n>` | Set sampling seed (or `/seed off`) |
| `/maxrounds <n>` | Set max agentic tool rounds (1–100) |
| `/proxy <url>` | Set HTTP proxy (`/proxy off` disables) |
| `/log` | Toggle logging on/off |
| `/justify` | Toggle paragraph justification |
| `/session save` | Save current session |
| `/session rename <name>` | Rename current session |
| `/session load <name>` | Load a saved session |
| `/quit` | Exit Rustama (aliases: `/q`, `/exit`) |

Parameter changes made at runtime apply to the currently selected model and are persisted to its section in `model_params.conf` / `cloud_models.conf`.

### Menu Bar & Settings Dialog

Press **F9** (or click the menu bar) to open the menu:

- **File** — Load, Save, Export…, Exit
- **Edit** — Set Model, Agentic Mode, Settings…
- **View** — Terminal
- **Help** — About

The **Settings** dialog provides a GUI-based editor for proxy, Ollama URL, temperature, top-p, top-k, frequency/presence penalties, max output tokens, reasoning effort, max tool rounds, max retries, and text justification. Changes are saved to the appropriate config files when you press **Save**.

## Usage

Once running, Rustama provides a terminal interface for:
- Sending prompts to Ollama or cloud models
- Receiving and streaming responses in real-time with markdown rendering (including thinking/reasoning traces for models that support it)
- Managing conversation history and sessions
- Using agentic modes with tool-based interactions
- Switching between local and cloud models on the fly
- Selecting output text with the mouse (copied to the X11 primary selection) and copying with `Ctrl+C`

### Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `F9` | Open menu |
| `F10` | Quit |
| `Tab` | Open menu (output pane) / autocomplete model name (input) |
| `Ctrl+S` | Save / export session |
| `Ctrl+T` | Toggle embedded terminal panel (starts a shell if not running) |
| `Ctrl+Shift+←` / `Ctrl+Shift+→` | Shrink / grow terminal panel width |
| `Enter` | Send message (input) / enter input mode (output pane) |
| `Alt+Enter` | Insert newline in input |
| `Ctrl+A` | Select all input text |
| `Ctrl+C` | Copy selection (output) / copy input text |
| `Ctrl+V` | Paste from clipboard into input |
| `↑` `↓` `PgUp` `PgDn` `Home` `End` | Scroll output |
| any printable key | Focus input and start typing |
| Mouse drag | Select output text (copied to X primary selection) |
| Mouse scroll | Scroll output |

## Agentic Tools

When agentic mode is enabled, the model can use these tools:

### File & Shell Tools

| Tool | Description |
|------|-------------|
| `read_file` | Read contents of a file |
| `write_file` | Create or overwrite a file |
| `edit_file` | Search-and-replace edit in a file |
| `bash` | Execute a shell command |
| `list_files` | List directory contents |
| `search_files` | Find files by glob pattern |
| `search_content` | Search file contents with regex |

### Web Tools

| Tool | Description |
|------|-------------|
| `fetch_url` | Fetch content from a URL via HTTP/HTTPS GET request |
| `web_search` | Search the web using DuckDuckGo |

### Terminal Handling Tools

These tools manage an interactive terminal session inside Rustama. The terminal panel becomes visible in the UI so you can observe the running process in real time.

| Tool | Description |
|------|-------------|
| `terminal_open` | Open a terminal session and run a command (e.g. `npm run dev`, `cargo build`). Returns a `cursor` for incremental reads |
| `terminal_send` | Send input (keystrokes) to the running session — interact with prompts, press Enter, etc. |
| `terminal_read` | Read output from the session. Pass the `cursor` from a previous response to get only new output |
| `terminal_close` | Close the session, hide the panel, and kill the running process |

Typical workflow: `terminal_open` → `terminal_read` (poll output) → `terminal_send` (if interaction needed) → `terminal_close`.

The agentic loop continues until the model stops making tool calls or `max_tool_rounds` is reached.

## Session Files

Sessions are stored as JSON in `$DOCUMENTS/rustama/<session_id>.session.rustama`, where `$DOCUMENTS` is resolved via `xdg-user-dir DOCUMENTS` (falling back to `~/rustama` when XDG user dirs are unavailable). Use the save/load dialogs, the File menu, or `/session` commands to manage them.

## License

This project is licensed under the **GNU Affero General Public License v3.0** (AGPL-3.0).

See the [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please feel free to open issues or submit pull requests.

## Resources

- [Ollama Documentation](https://ollama.ai/)
- [ratatui Documentation](https://docs.rs/ratatui/)
- [Tokio Documentation](https://tokio.rs/tokio/tutorial)

---

Built with ❤️ in Rust YAY!
