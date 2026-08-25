# AGENTS.md — Rustama Codebase Reference

## Overview

Rustama is a terminal UI (TUI) client for Ollama and cloud AI models, written in Rust (edition 2024, v0.1.0). It provides a chat interface with streaming responses, markdown rendering, syntax highlighting, and agentic tool capabilities.

## Project Structure

```
Rustama/
├── Cargo.toml          # Dependencies and project metadata
├── src/
│   ├── main.rs         # Entry point, event loop, all rendering (1599 lines)
│   ├── app.rs          # Application state, logic, API calls, tools (3062 lines)
│   ├── config.rs       # Configuration loading and parsing (316 lines)
│   └── ui.rs           # Reusable UI widgets: Theme, Button, Dialogs (620 lines)
```

## Build & Run

```bash
cargo build --release
cargo run
```

No test suite, CI, or benchmarks exist yet.

## Architecture

### Event Loop (`main.rs`)

- Uses `crossterm` for terminal manipulation (raw mode, alternate screen, mouse capture).
- `run_app()` polls for keyboard/mouse events every 50ms.
- Dispatches input to `app.handle_key()` / `app.handle_click()`.
- Calls `app.check_responses()` / `app.check_model_responses()` to receive streaming data from background threads via `mpsc::channel`.
- All rendering happens on the main thread using `ratatui`.

### Application Logic (`app.rs`)

This is the largest module. Key components:

- **Data Types**: `App` (main state), `ChatMessage` (8 variants: User, Assistant, System, App, Thinking, FileContent, ToolCall, ToolResult), `InputMode` (Normal/Input/Menu), `Focus` (Output/Input), `StreamChunk` (Text/Thinking/Done/Error/ToolCalls).
- **Input Handling**: Three modes — Normal (vim-like navigation), Input (text entry with Ctrl+C/V, Tab for model completion), Menu (arrow key navigation). Dialog-specific handlers for model selector, file browser, save/load.
- **Slash Commands**: ~25 commands (`/help`, `/model`, `/use`, `/list`, `/tools`, `/config`, `/setsystem`, `/temp`, `/topp`, `/topk`, `/fpen`, `/ppen`, `/effort`, `/maxtokens`, `/seed`, `/maxrounds`, `/session save|rename|load`, `/status`, `/log`, `/quit`). Slash-command parameter changes are session-only; the Settings dialog (F9 → Settings) persists them to the current model's config section via `save_config()`.
- **AI Communication**: `send_to_ollama_async()` and `send_tool_results_async()` spawn background OS threads with their own single-threaded tokio runtime. Streams are parsed line-by-line (Ollama JSON or OpenAI SSE format) and chunks are sent back via `mpsc::channel`.
- **Agentic Tool System**: 13 tools — file ops (`read_file`, `write_file`, `edit_file`), `bash`, `list_files`, `search_files`, `search_content`, `fetch_url`, `web_search`, and 4 terminal tools (`terminal_open/send/read/close`). Tool call loop with configurable `max_tool_rounds` (default: 10). Two paths: native function calling and text-based JSON parsing fallback. **Auto-continue heuristic**: some cloud models (Kimi-K3) emit `finish_reason="stop"` right after a transitional sentence ("Let me run X:") instead of the announced tool call; `needs_continuation()` detects the unfinished-looking text and the `Done` handler auto-sends "continue" (cap `MAX_AUTO_CONTINUES = 10` per turn, reset on fresh user input).
- **Session Management**: JSON files at `~/.config/rustama/<session_id>.session.rustama`.
- **File Dialog**: Built-in file browser for navigating directories and attaching files to conversations.

### Configuration (`config.rs`)

- INI-style parser (hand-rolled, no external config crate).
- `~/.config/rustama/rustama.conf` — main config (`ollama_url`, `model`, `save_path`, `agentic`, `timeout_secs`, `logging`, `logfile`, `system_prompt`).
- `~/.config/rustama/cloud_models.conf` — cloud model definitions with `api_url`, `api_key`, `api_model`. Ships with Mistral Small preconfigured.
- Boolean parsing supports English (true/yes/on/1) and Hungarian (igen).
- **Per-model parameters**: `ModelParams` (temperature, top_p, top_k, frequency_penalty, presence_penalty, max_output_tokens, reasoning_effort, seed) is configured per model, not globally:
  - Cloud models: parsed from the model's section in `cloud_models.conf` (max_output_tokens defaults to 16384 there).
  - Ollama models: `~/.config/rustama/model_params.conf` with a `[default]` section (base for all models) and per-model `[model-name]` override sections. Legacy global temperature/top_p/top_k keys from `rustama.conf` are migrated into `[default]` on first run.
  - `save_model_params()` rewrites a section in place via `update_ini_section()` (preserves comments/other sections).
  - Request translation: cloud bodies get OpenAI-style fields (`max_tokens`, `reasoning_effort` levels only); Ollama bodies get `options.*` (`num_predict`, `seed`, penalties) plus top-level `think` (bool or low/medium/high level) — see `build_chat_body()` in app.rs.

### UI Widgets (`ui.rs`)

- `Theme` — color theme with `default()` and `dark()` presets.
- `Button` — clickable button with hotkey support and hit testing.
- `Dropdown` — expandable dropdown selector with hit testing.
- `FileActionDialog` — composable dialog builder (labels, inputs, dropdowns, file lists, buttons) with `render()` and `hit_test()`.
- `dialog_block()` — styled, shadowed dialog block.

### Rendering Details (`main.rs`)

- `render_output()` — chat history with multi-layer caching (`cached_output`, `cached_wrapped`, `cached_msg_count`, `cached_streaming_len`, `cached_width`) to avoid recomputation.
- `render_markdown()` — full markdown parser using `pulldown-cmark` converting to styled ratatui `Line` objects. Supports headings, bold, italic, code blocks, tables (Unicode box-drawing), lists, horizontal rules, inline code.
- `highlight_code()` — `syntect` with "base16-ocean.dark" theme, lazy-initialized via `OnceLock`.
- `wrap_and_justify_lines()` — text layout engine with word wrapping and paragraph justification.

## Dependencies

| Crate | Purpose |
|-------|---------|
| `ratatui` 0.30 | TUI framework |
| `crossterm` 0.29 | Terminal abstraction |
| `ratatui-textarea` 0.9 | Text area widget |
| `tokio` 1.37 (full) | Async runtime (background threads) |
| `reqwest` 0.12 (rustls-tls) | HTTP client for API calls |
| `serde_json` / `serde` | JSON serialization |
| `pulldown-cmark` 0.10 | Markdown parsing |
| `syntect` 5 | Syntax highlighting |
| `arboard` 3 | Clipboard access |
| `glob` 0.3 | File pattern matching |
| `regex` 1 | Regex for search and tool call parsing |
| `chrono` 0.4 | Date/time formatting |
| `dialoguer` 0.12 | Unused — dead dependency |
| `marked` 0.3 | Unused — dead dependency |
| `markdown` 0.1 | Unused — dead dependency |

## Key Design Patterns

1. **Async via background threads + mpsc**: Main event loop is synchronous. HTTP requests run on spawned OS threads with individual tokio runtimes. Communication via `std::sync::mpsc::channel` with `try_recv()` polling.
2. **Dual API protocol**: Same streaming infrastructure handles both Ollama native JSON and OpenAI-compatible SSE format, branching per request based on whether the model is cloud-hosted.
3. **Output rendering cache**: Multi-layer caching avoids recomputing markdown render + text wrap every frame.
4. **Composable dialog system**: `FileActionDialog` builder pattern reduces code duplication across save/export/load dialogs.
5. **Agentic tool loop**: Send message -> execute tool calls -> send results -> repeat until no more calls or max rounds reached.

## Notes for Future Development

- `app.rs` at 3062 lines is a candidate for splitting (networking, tools, sessions, commands).
- No trait abstractions or plugin system — adding new tools requires modifying `app.rs` directly.
- No tests exist — any new features should ideally include tests.
- Consider adding a `tests/` directory or inline `#[cfg(test)]` modules.
