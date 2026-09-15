# AGENTS.md — Rustama Codebase Reference

## Overview

Rustama is a terminal UI (TUI) client for Ollama and cloud AI models, written in Rust (edition 2024, v0.3.0). It provides a chat interface with streaming responses, markdown rendering, syntax highlighting, an agentic tool system, and an embedded terminal panel (a real PTY the model can drive with tools).

## Build, Test, Run

```bash
cargo build            # dev build
cargo build --release
cargo run
cargo test             # ~100 tests; some PTY/retry tests are slow (~60s total)
cargo test <name>      # e.g. cargo test menu_ / throbber / read_file
# slow acid tests (ignored by default):
cargo test --release pty_acid -- --ignored --nocapture
```

Tests live in inline `#[cfg(test)]` modules at the bottom of each source file (no `tests/` dir). Some tests spawn real PTYs — don't run them inside Rustama's own embedded terminal tool unless necessary; prefer the `bash` tool. `cargo test` on this codebase takes ~60s (the `mock_server_429_flow` test alone is 60s) — plan timeouts accordingly.

## Project Structure

```
Rustama/
├── Cargo.toml
├── src/
│   ├── main.rs              # Entry point, event loop, ALL rendering (~1900 lines)
│   ├── app.rs               # App state, input handling, networking, tools, sessions (~7100 lines)
│   ├── config.rs            # INI config parsing/saving, cloud models, pricing (~1170 lines)
│   ├── ui.rs                # Theme, Button, dialogs, MainMenu widget (~1660 lines)
│   └── primary_selection.rs # X11 primary-selection clipboard provider (~400 lines)
└── AGENTS.md                # this file
```

## Architecture

### Event Loop (`main.rs`)

- `crossterm` raw mode + alternate screen + mouse capture; panic hook restores the terminal.
- `run_app()` polls events every 50ms; every event goes through the single `handle_event()` helper (first poll + `poll(ZERO)` drain loop). **Keyboard/mouse are never blocked** — even while streaming or waiting to retry. Only *sending* is blocked while `app.is_loading` (both send paths show a `⚠ Request in progress…` status warning).
- All rendering on the main thread with `ratatui`. `ui(f, app)` lays out: menu bar, output+status+input (optionally split with the terminal panel), hint bar, keybar, then dialog overlays.
- Streaming chunks arrive via `mpsc::channel`, polled with `try_recv()` in `app.check_responses()` / `check_model_responses()` each loop iteration.
- `app.terminal_state.flush_replies()` answers the embedded PTY's terminal queries every frame — without it crossterm-based child apps hang.

### Application Logic (`app.rs`) — the big one

Key types: `App` (all state), `ChatMessage` (User/Assistant/System/App/Thinking/FileContent/ToolCall/ToolResult), `InputMode` (Normal/Input/Menu), `Focus` (Output/Input/Terminal), `StreamChunk` (Text/Thinking/StatusTick/Stats/Truncated/Done/Error/ToolCalls), `TokenStats`.

- **Input**: `handle_global_key()` is the entry point; dispatches to Normal-mode nav, `handle_input_key()` (textarea, Enter sends, Alt+Enter newline), or Menu mode. Terminal focus forwards raw keys to the PTY via `key_to_pty_bytes()` (Ctrl+G releases).
- **Slash commands** in `handle_slash_command()` (~line 3770): `/help /model|/use /list /tools /config /log /maxrounds /temp /topp /topk /fpen /ppen /effort /maxtokens /seed /setsystem /session save|rename|load /status /quit`. Slash-command parameter changes are session-only; the Settings dialog (F9 → Settings) persists via `save_config()` / `save_model_params()`.
- **Networking**: `send_to_ollama_async_with()` and `send_tool_results_async()` spawn background OS threads, each with its own single-threaded tokio runtime; `send_with_retry()` handles retries with `retry_countdown()` StatusTick updates (`app.retrying` is only a status-bar flag now, it gates nothing). Streaming parses Ollama JSON lines or OpenAI SSE. Request bodies are built by `build_chat_body()` (cloud → OpenAI-style `max_tokens`/`reasoning_effort`; Ollama → `options.*` + top-level `think`).
- **System prompt**: `App::build_system_content()` = configured `system_prompt` (or the built-in agentic default) + the application-global **`~/.config/rustama/AGENTS.md`** appended under a `# AGENTS.md` heading (`config::load_agents_md()`; missing/empty file ignored).
- **Auto-continue**: `needs_continuation()` detects answers that end mid-thought (transitional "Let me run X:" with no tool call) and auto-sends "continue"; cap `MAX_AUTO_CONTINUES = 10` per turn.
- **Tools**: schemas in `build_tools_json()`; execution in `execute_tool_call()`; text-fallback parsing in `parse_text_tool_calls()`. 13 tools: `read_file` (supports `offset`/`limit` line windows with `[showing lines X-Y of Z]` markers), `write_file`, `edit_file`, `bash` (30s timeout, sudo blocked), `list_files`, `search_files`, `search_content`, `fetch_url`, `web_search`, `terminal_open/send/read/close`. Tool loop capped by `max_tool_rounds` (default 10).
- **Embedded terminal**: `TerminalState` (portable-pty + vt100). Reader thread feeds a vt100 `Parser` (screen model rendered cell-by-cell in `render_terminal_panel()`, colors via `vt100_style()`) and a raw `TerminalBuffer` (model-facing incremental cursor reads). `TerminalQueries` vt100 callback + `flush_replies()` answer cursor-position/DA queries. PTY resizes with the panel; F6/click focuses, Ctrl+G releases. Rendering honors application-cursor-key mode (`application_cursor()`).
- **Sessions**: JSON at `$XDG_DOCUMENTS/rustama/<id>.session.rustama` (`~/rustama` fallback). `save_session()`/`load_session()`/`autosave_session()`/`new_session()`.
- **Usage/cost**: `TokenStats`, `session_usage`, `format_token_stats()`, `session_cost()` (cloud pricing from config).

### Configuration (`config.rs`)

Hand-rolled INI parser (no external crate). Files:

- `~/.config/rustama/rustama.conf` — main config (`ollama_url`, `model`, `save_path`, `agentic`, `timeout_secs`, `logging`, `logfile`, `system_prompt`, `proxy`, `max_tool_rounds`, `max_retries`, `terminal_width_pct`, `justify`, `hintbar`). Missing keys are backfilled with defaults on load.
- `~/.config/rustama/AGENTS.md` — app-global agent instructions appended to every system prompt (see above).
- `~/.config/rustama/cloud_models.conf` — per-cloud-model `api_url`/`api_key`/`api_model` + per-model params + optional pricing.
- `~/.config/rustama/model_params.conf` — Ollama per-model params: `[default]` base + `[model-name]` overrides; legacy global temp/top_p/top_k migrated into `[default]` on first run.
- `ModelParams`: temperature, top_p, top_k, frequency/presence penalties, `max_output_tokens`, `reasoning_effort`, `seed`.
- `update_ini_section()` rewrites one section in place, preserving comments and other sections — used by all `save_*` functions.
- Booleans accept English (true/yes/on/1) and Hungarian (igen).

### UI Widgets (`ui.rs`)

- `Theme` — all colors in one struct; `dark()` is the runtime preset, `default()` is `#[cfg(test)]`. Includes `menu_selected_bg/fg` (black/white) and `menu_unselected_bg/fg` (`MC_GREEN`/white). `pub const MC_GREEN` (turquoise `Rgb(0,187,187)`) is shared by the keybar and menu.
- `MainMenu` — menu bar + dropdowns. **Single source of truth**: `MENUS` order + `menu_title()`; `menu_spans()` computes column spans from title widths. `x_offset_for()`, `bar_col_to_menu()` (mouse hit-test), `next_menu()/prev_menu()`, and `render_bar()` all derive from it — never hardcode offsets. Render methods take `&Theme`.
- `Button`, `Dropdown`, `FileActionDialog` (composable builder: labels/inputs/dropdowns/file lists/buttons, `render()` + `hit_test()`), `MessageBox`, `ConfirmationBox`, `dialog_block()`.

### Rendering (`main.rs`)

- `render_output()` — chat history with multi-layer caching (`cached_output`/`cached_wrapped`/`cached_msg_count`/`cached_streaming_len`/`cached_width`); re-renders markdown only when history or width changes.
- `render_markdown()` — `pulldown-cmark` → styled `Line`s: headings, bold/italic, code blocks, tables (Unicode box-drawing), lists, rules, inline code. `normalize_code_fences()` fixes indented fences first.
- `highlight_code()` — `syntect` "base16-ocean.dark", lazy `OnceLock`.
- `wrap_and_justify_lines()` — wrapping + optional justification; code/table lines are never justified.
- `throbber()` — status-bar spinner; style picked **randomly per request** from `app::SPINNERS` (5 frame sets: braille circle/dots, quadrants, equalizer, trigrams) via `App::reroll_spinner()` (time-based PRNG, no rand crate); frame ticks on wall-clock every 100ms. Idle shows 🧘.
- `render_keybar()` — MC-style F-key strip, always the last line; hint bar above it only when `hintbar = on`.

## Dependencies (notable)

`ratatui 0.30` + `crossterm 0.29` + `ratatui-textarea 0.9`; `reqwest 0.12` (rustls) + `tokio 1.37` (background threads only); `pulldown-cmark 0.10`; `syntect 5` (default-fancy); `portable-pty 0.9` + `vt100 0.16`; `arboard 3` + `x11rb 0.14` (primary selection); `glob`, `regex`, `chrono`, `urlencoding`, `serde`/`serde_json`. Dead deps still in Cargo.toml: `dialoguer`, `marked`, `markdown`.

## Key Design Patterns

1. **Async via threads + mpsc** — sync main loop; HTTP on spawned threads with per-thread tokio runtimes; `try_recv()` polling.
2. **Dual API protocol** — one streaming infrastructure branches per request: Ollama native JSON vs OpenAI-compatible SSE.
3. **Render caching** — markdown/wrap recomputed only on change, not per frame.
4. **Never block the UI** — input always works; only sending is guarded (`is_loading`) with a user-visible warning.
5. **Single source of truth for layout-ish math** — e.g. menu spans computed from titles, not magic numbers. Keep it that way.

## Conventions & Gotchas

- **Style**: 4-space indent, `snake_case`, doc comments on public items, keep the `─── section ───` comment banners. Run `cargo build` (must be warning-free) and `cargo test` before declaring done.
- **`app.rs` is huge (~7100 lines)** — the known candidate for splitting (networking / tools / sessions / commands). If a task touches a coherent chunk, extracting a module is welcome but not required.
- **Adding a tool** = edit three places in app.rs: schema in `build_tools_json()`, arm in `execute_tool_call()`, name in `TOOL_NAMES` (+ the "Available tools" list in the unknown-tool error).
- **Adding a slash command** = arm in `handle_slash_command()` + help text in `slash_help()`.
- **Adding a menu item** = extend `item_names()` + `action_for()`; offsets/hit-testing are automatic (see `menu_spans()`).
- **The retry countdown tests and `mock_server_429_flow` are slow** (~60s); don't "fix" them for being slow.
- **Terminal tool escape sequences** are documented in a comment above `key_to_pty_bytes()`; TUI apps in the embedded terminal need application-cursor-key sequences (`ESC O A/B/C/D`), see `application_cursor()`. `terminal_send` writes input **verbatim** (no auto-Enter) — the model appends `\r` itself when it wants to submit a line.
- There are ~20 unit-test modules (`usage_tests`, `scrollbar_tests`, `truncate_tests`, `sessions_dir_tests`, `continuation_tests`, `menu_focus_tests`, `chat_body_tests`, `pricing_tests`, `throbber_tests`, …). New features should add tests to the matching module (or a new one).

## Recent Changes (keep this section current)

- `terminal_send` no longer appends a newline/Enter — input is written exactly as given; the model must include `\r` explicitly to submit a line (`TerminalState::send_input`, `pty_send_input_appends_nothing` test). Fixes batched arrow keys (`ESC O B` — printable bytes!) triggering Enter in TUI apps like mc.
- App-global `~/.config/rustama/AGENTS.md` loaded into every system prompt (`config::load_agents_md`, `App::build_system_content`).
- Keyboard no longer blocked during retry backoff; send attempts while busy show a warning (event dispatch unified in `handle_event()`).
- Random per-request throbber spinner (`SPINNERS`, `App::reroll_spinner`).
- `read_file` tool gained `offset`/`limit` line-window params.
- Menu colors moved into `Theme` (`menu_*` fields); menu bar offsets/hit-testing now computed from menu titles (`menu_spans()`).

## Notes for Future Development

- No trait abstractions or plugin system — adding tools requires editing `app.rs` directly.
- Dead dependencies (`dialoguer`, `marked`, `markdown`) could be removed.
- No CI; `cargo test` is the gate.
