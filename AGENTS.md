# AGENTS.md — Rustama Codebase Reference

## Overview

Rustama is a terminal UI (TUI) client for Ollama and cloud AI models, written in Rust (edition 2024, v0.4.0). It provides a chat interface with streaming responses, markdown rendering, syntax highlighting, an agentic tool system, and an embedded terminal panel (a real PTY the model can drive with tools).

## Build, Test, Run

```bash
cargo build            # dev build
cargo build --release
cargo run
cargo test             # ~130 tests; some PTY/retry tests are slow (~60s total)
cargo test <name>      # e.g. cargo test menu_ / throbber / read_file / lsp_
# slow acid tests (ignored by default):
cargo test --release pty_acid -- --ignored --nocapture
cargo test --release lsp_real -- --ignored --nocapture   # needs rust-analyzer installed
```

Tests live in inline `#[cfg(test)]` modules at the bottom of each source file (no `tests/` dir). Some tests spawn real PTYs — don't run them inside Rustama's own embedded terminal tool unless necessary; prefer the `bash` tool. `cargo test` on this codebase takes ~60s (the `mock_server_429_flow` test alone is 60s) — plan timeouts accordingly. LSP tests use a hermetic in-process mock server; only the ignored `lsp_real_rust_analyzer` acid test spawns the real binary.

## Project Structure

```
Rustama/
├── Cargo.toml
├── src/
│   ├── main.rs              # Entry point, event loop, ALL rendering (~1940 lines)
│   ├── app.rs               # App state, input handling, networking, tools, sessions (~8200 lines)
│   ├── config.rs            # INI config parsing/saving, cloud models, pricing (~1330 lines)
│   ├── ui.rs                # Theme, Button, dialogs, MainMenu widget (~1670 lines)
│   ├── lsp.rs               # LSP client (rust-analyzer): process, framing, diagnostics (~1100 lines)
│   └── primary_selection.rs # X11 primary-selection clipboard provider (~400 lines)
├── LSP_PLAN.md              # Design record: LSP/rust-analyzer support (implemented)
├── SPLIT_PLAN.md            # Roadmap: split app.rs into modules (planned)
└── AGENTS.md                # this file
```

## Architecture

### Event Loop (`main.rs`)

- `crossterm` raw mode + alternate screen + mouse capture; panic hook restores the terminal.
- `run_app()` polls events every 50ms; every event goes through the single `handle_event()` helper (first poll + `poll(ZERO)` drain loop). **Keyboard/mouse are never blocked** — even while streaming or waiting to retry. Only *sending* is blocked while `app.is_loading` (both send paths show a `⚠ Request in progress…` status warning). **F7 "Stop"** interrupts the in-flight request: `App::stop_request()` raises the shared `request_cancel: Arc<AtomicBool>` (checked in the stream loop and `retry_countdown`) and instantly clears `is_loading`, committing partial text with a `⏹ Stopped by user` marker.
- All rendering on the main thread with `ratatui`. `ui(f, app)` lays out: menu bar, output+status+input (optionally split with the terminal panel), hint bar, keybar, then dialog overlays.
- Streaming chunks arrive via `mpsc::channel`, polled with `try_recv()` in `app.check_responses()` / `check_model_responses()` each loop iteration (same pattern for the num_ctx auto-detect in `check_ctx_response()`).
- `app.terminal_state.flush_replies()` answers the embedded PTY's terminal queries every frame — without it crossterm-based child apps hang.

### Application Logic (`app.rs`) — the big one

Key types: `App` (all state), `ChatMessage` (User/Assistant/System/App/Thinking/FileContent/ToolCall/ToolResult), `InputMode` (Normal/Input/Menu), `Focus` (Output/Input/Terminal), `StreamChunk` (Text/Thinking/StatusTick/Stats/Truncated/Done/Error/ToolCalls), `TokenStats`.

- **Input**: `handle_global_key()` is the entry point; dispatches to Normal-mode nav, `handle_input_key()` (textarea, Enter sends, Alt+Enter newline), or Menu mode. Terminal focus forwards raw keys to the PTY via `key_to_pty_bytes()` (F8/^G close the terminal). F9 opens the menu (also: click, or Tab in Normal mode); F8 is the terminal toggle.
- **Slash commands** in `handle_slash_command()` (~line 4050): `/help /model|/use /list /tools /config /system /setsystem /log /maxrounds /temp /topp /topk /fpen /ppen /effort /maxtokens /seed /ctx|/context /proxy /session new|save|rename|load /workspace /justify /status /usage|/cost /quit|/q|/exit`. Slash-command parameter changes are session-only; the Settings dialog (F9 → Settings) persists via `save_config()` / `save_model_params()`.
- **Networking**: `send_to_ollama_async_with()` and `send_tool_results_async()` spawn background OS threads, each with its own single-threaded tokio runtime; `send_with_retry()` handles retries with `retry_countdown()` StatusTick updates (`app.retrying` is only a status-bar flag now, it gates nothing). Streaming parses Ollama JSON lines or OpenAI SSE. Request bodies are built by `build_chat_body()` (cloud → OpenAI-style `max_tokens`/`reasoning_effort`; Ollama → `options.*` + top-level `think`).
- **System prompt**: `App::build_system_content()` = configured `system_prompt` (or the built-in agentic default) + the application-global **`~/.config/rustama/AGENTS.md`** appended under a `# AGENTS.md` heading (`config::load_agents_md()`; missing/empty file ignored).
- **Auto-continue**: `needs_continuation()` detects answers that end mid-thought (transitional "Let me run X:" with no tool call) and auto-sends "continue"; cap `MAX_AUTO_CONTINUES = 10` per turn.
- **Stop (F7)**: every request thread gets a `cancel: Arc<AtomicBool>` (stored in `app.request_cancel`); the stream loop and `retry_countdown` poll it, `send_with_retry` returns `SendOutcome::{Response,Error,Cancelled}`. `stop_request()` (F7, works with terminal focused too) drops `response_rx` so late chunks fail to send, commits partial text/thinking plus a "⏹ Stopped by user (F7)" App marker, and suppresses auto-continue/tool rounds for that turn.
- **Tools**: schemas in `get_tool_definitions()`; stateless execution in the free fn `execute_tool_call()`; text-fallback parsing in `parse_text_tool_calls()`. App-level dispatch goes through `App::execute_tool()` → `execute_terminal_tool()` / `execute_lsp_tool()` / `execute_tool_call()` (in that order). 14 tools: `read_file` (supports `offset`/`limit` line windows with `[showing lines X-Y of Z]` markers), `write_file`, `edit_file` (result includes a `unified_diff()` of the change, rendered +green/−red in `render_output`), `bash` (30s timeout, sudo blocked), `list_files`, `search_files`, `search_content`, `fetch_url`, `web_search`, `terminal_open/send/read/close`, `lsp_diagnostics`. Tool loop capped by `max_tool_rounds` (default 10).
- **Embedded terminal**: `TerminalState` (portable-pty + vt100). Reader thread feeds a vt100 `Parser` (screen model rendered cell-by-cell in `render_terminal_panel()`, colors via `vt100_style()`) and a raw `TerminalBuffer` (model-facing incremental cursor reads). `TerminalQueries` vt100 callback + `flush_replies()` answer cursor-position/DA queries. PTY resizes with the panel; F8 opens/closes it ("Term"/"CloseTerm" keybar labels), F6/click focuses, F8/^G closes while focused, Ctrl+T hides. Rendering honors application-cursor-key mode (`application_cursor()`).
- **LSP client** (`src/lsp.rs`, `App::lsp`): rust-analyzer as a child process, `lsp-server` crate's `Message::read`/`Message::write` framing over the child's pipes (`Connection::stdio()` is server-oriented — not used). Reader thread routes responses to pending oneshots, stores pushed `publishDiagnostics` in a versioned `DiagStore`, and answers server→client requests with canned replies so rust-analyzer never stalls. Sync std-only concurrency (no tokio). `sync_file()` keeps didOpen/didChange in order; `lsp_diagnostics` prefers pull (`textDocument/diagnostic`, retries through rust-analyzer's "server cancelled the request" re-analysis window) and falls back to the push cache. stderr → `rustama-lsp.log`. `Drop`/`shutdown()` guarantee no orphan server. Lifecycle: started in `App::new` when `lsp = on`, restarted by `/workspace <dir>`. Auto-feedback: after successful `edit_file`/`write_file` on a `.rs` file, fresh diagnostics are appended to the tool result (`lsp_auto_diagnostics`, `App::maybe_append_lsp_diagnostics`).
- **Sessions**: JSON at `$XDG_DOCUMENTS/rustama/<id>.session.rustama` (`~/rustama` fallback). `save_session()`/`load_session()`/`autosave_session()`/`new_session()`.
- **Usage/cost**: `TokenStats`, `session_usage`, `format_token_stats()`, `session_cost()` (cloud pricing from config).

### Configuration (`config.rs`)

Hand-rolled INI parser (no external crate). Files:

- `~/.config/rustama/rustama.conf` — main config (`ollama_url`, `model`, `save_path`, `agentic`, `timeout_secs`, `logging`, `logfile`, `system_prompt`, `proxy`, `max_tool_rounds`, `max_retries`, `terminal_width_pct`, `justify`, `hintbar`, `lsp`, `lsp_server`, `lsp_workspace`, `lsp_auto_diagnostics`). Missing keys are backfilled with defaults on load.
- `~/.config/rustama/AGENTS.md` — app-global agent instructions appended to every system prompt (see above).
- `~/.config/rustama/cloud_models.conf` — per-cloud-model `api_url`/`api_key`/`api_model` + per-model params + optional pricing.
- `~/.config/rustama/model_params.conf` — Ollama per-model params: `[default]` base + `[model-name]` overrides; legacy global temp/top_p/top_k migrated into `[default]` on first run.
- `ModelParams`: temperature, top_p, top_k, frequency/presence penalties, `max_output_tokens`, `reasoning_effort`, `seed`, `num_ctx`.
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

`ratatui 0.30` + `crossterm 0.29` + `ratatui-textarea 0.9`; `reqwest 0.12` (rustls) + `tokio 1.37` (background threads only); `pulldown-cmark 0.10`; `syntect 5` (default-fancy); `portable-pty 0.9` + `vt100 0.16`; `lsp-types 0.97` + `lsp-server 0.7` (LSP client; framing only — no `Connection`); `arboard 3` + `x11rb 0.14` (primary selection); `glob`, `regex`, `chrono`, `urlencoding`, `serde`/`serde_json`. Dead deps still in Cargo.toml: `dialoguer`, `marked`, `markdown`.

## Key Design Patterns

1. **Async via threads + mpsc** — sync main loop; HTTP on spawned threads with per-thread tokio runtimes; `try_recv()` polling.
2. **Dual API protocol** — one streaming infrastructure branches per request: Ollama native JSON vs OpenAI-compatible SSE.
3. **Render caching** — markdown/wrap recomputed only on change, not per frame.
4. **Never block the UI** — input always works; only sending is guarded (`is_loading`) with a user-visible warning.
5. **Single source of truth for layout-ish math** — e.g. menu spans computed from titles, not magic numbers. Keep it that way.

## Conventions & Gotchas

- **Style**: 4-space indent, `snake_case`, doc comments on public items, keep the `─── section ───` comment banners. Run `cargo build` (must be warning-free) and `cargo test` before declaring done.
- **`app.rs` is huge (~8300 lines)** — the known candidate for splitting; `SPLIT_PLAN.md` is the roadmap (see "Planned Features"). If a task touches a coherent chunk, extracting a module is welcome but not required.
- **Adding a tool** = 4 edits, all in app.rs: schema in `get_tool_definitions()`, arm in `execute_tool_call()`, name in `TOOL_NAMES`, and the "Available tools" list in the unknown-tool error. **Stateful tool** (needs `App`, like the terminal/LSP tools): instead of the `execute_tool_call()` arm, add an `execute_*_tool()` method returning `Option<String>` and chain it in `App::execute_tool()`.
- **Adding a slash command** = arm in `handle_slash_command()` + help text in `slash_help()`.
- **Adding a config key** = field in `Config` (+ its `Default` impl) + parse arm in `Config::parse()` + backfill entry in the `missing` list in `Config::load()` + a line in `Config::save()` (full-file rewrite). Add a Settings dialog field too if it should be user-editable.
- **Adding a menu item** = extend `item_names()` + `action_for()`; offsets/hit-testing are automatic (see `menu_spans()`).
- **The retry countdown tests and `mock_server_429_flow` are slow** (~60s); don't "fix" them for being slow.
- **Terminal tool escape sequences** are documented in a comment above `key_to_pty_bytes()`; TUI apps in the embedded terminal need application-cursor-key sequences (`ESC O A/B/C/D`), see `application_cursor()`. `terminal_send` writes input **verbatim** (no auto-Enter) — the model appends `\r` itself when it wants to submit a line.
- There are ~20 unit-test modules (`usage_tests`, `scrollbar_tests`, `truncate_tests`, `sessions_dir_tests`, `continuation_tests`, `menu_focus_tests`, `chat_body_tests`, `pricing_tests`, `throbber_tests`, …). New features should add tests to the matching module (or a new one).

## Recent Changes (keep this section current)

- **View menu → Hintbar**: new `View` submenu item "Hintbar" (below "Terminal") toggles the hint bar on/off at runtime (`MenuAction::ToggleHintbar`, flips `App::hintbar` in-memory, like Agentic Mode). Added `view_menu_maps_hintbar_action` test.
- **LSP support (rust-analyzer)**: new `src/lsp.rs` — `LspClient` spawns rust-analyzer, speaks JSON-RPC via the `lsp-server` crate's `Message::read`/`Message::write` framing over the child's pipes, reader thread routes responses/pushed diagnostics (std-only concurrency, no tokio). New `lsp_diagnostics` agentic tool (pull `textDocument/diagnostic` with retry through the "server cancelled" re-analysis window, push-cache fallback; `path`/`severity` args) — the model now gets real compiler feedback and can fix its own type errors. Auto-feedback: successful `edit_file`/`write_file` on a `.rs` file appends fresh diagnostics to the tool result (`lsp_auto_diagnostics`, default on). New config keys `lsp` (default **off**), `lsp_server`, `lsp_workspace`, `lsp_auto_diagnostics`; new `/workspace <dir>|off` slash command (restarts the server on the new root). Tool dispatch unified in `App::execute_tool()` (terminal → LSP → stateless). Server stderr goes to `rustama-lsp.log`; `shutdown()`/`Drop` leave no orphans. Tests: hermetic mock server (in-process thread over `std::io::pipe()` pairs) + `lsp_real_rust_analyzer` acid test (ignored; run with `cargo test --release lsp_real -- --ignored --nocapture`).
- **Ollama context window (`num_ctx`)**: Rustama now overcomes Ollama's default 2048-token context limit by auto-detecting the model's native context window and sending it as `options.num_ctx`. Detection queries `/api/show` and reads `model_info.<architecture>.context_length` (falls back to `llama.context_length`); triggered on startup, on model switch, and via `/ctx auto`. The detected value is a per-run default that never overwrites an explicit setting. New `ModelParams.num_ctx` (config keys `num_ctx`/`n_ctx`/`context_length`, sent only for Ollama; cloud models ignore it). New slash command `/ctx <n> | auto | off` (+ `/context` alias); "Num Ctx:" field added to the Settings dialog (F9 → Settings persists it).
- `edit_file` tool result now embeds a unified diff (`unified_diff()`, LCS-based, ±3 context lines) instead of just "Successfully edited"; `render_output` colors diff lines in tool results (+green, −red, @@ cyan).
- F7 "Stop": interrupts the in-flight request (stream / tool round / retry countdown) — `App::stop_request()`, per-request `Arc<AtomicBool>` cancel flag, `SendOutcome` enum in `send_with_retry`. Partial text is kept with a stopped marker; keybar F7 shows "Stop" only while loading; hintbar shows "F7:Stop" during requests.
- F8 is now the terminal toggle: opens+focuses the terminal when not running, closes it when running; ^G (was: release focus) also closes the terminal while focused. Keybar F8 label: "Term"/"CloseTerm"; hints live in the hintbar. F9 still opens the menu (keybar: "PullDn").
- `terminal_send` no longer appends a newline/Enter — input is written exactly as given; the model must include `\r` explicitly to submit a line (`TerminalState::send_input`, `pty_send_input_appends_nothing` test). Fixes batched arrow keys (`ESC O B` — printable bytes!) triggering Enter in TUI apps like mc.
- App-global `~/.config/rustama/AGENTS.md` loaded into every system prompt (`config::load_agents_md`, `App::build_system_content`).
- Keyboard no longer blocked during retry backoff; send attempts while busy show a warning (event dispatch unified in `handle_event()`).
- Random per-request throbber spinner (`SPINNERS`, `App::reroll_spinner`).
- `read_file` tool gained `offset`/`limit` line-window params.
- Menu colors moved into `Theme` (`menu_*` fields); menu bar offsets/hit-testing now computed from menu titles (`menu_spans()`).

## Planned Features (roadmaps)

Read the plan doc before starting a phase, follow its execution order, and update AGENTS.md as each phase lands — never document unimplemented features as if they existed. Run `cargo build` (warning-free) + full `cargo test` after each phase.

### Splitting `app.rs` — `SPLIT_PLAN.md` (PLANNED — no code moved yet)

The plan maps `app.rs`'s regions (types / terminal / tools / networking / tests) into sibling modules, enabled by multiple `impl App` blocks across files (fields are already mostly `pub`; no `lib.rs`, everything stays crate-internal). Phase 1 (free-standing code) is low-risk — do it first; move each region's inline tests with it.

### LSP support — `LSP_PLAN.md` (IMPLEMENTED)

Shipped as described in "Recent Changes" and the `app.rs`/`lsp.rs` bullets above; `LSP_PLAN.md` is kept as the design record (including deviations: `Connection::stdio()` not used, mock server is in-process). Possible follow-ups from the plan: a diagnostics UI panel (Phase 4, was skipped), and stretch tools (`lsp_hover`, `lsp_definition`, `lsp_references`, `lsp_completion`) — all follow the existing `request_value()` pattern in `lsp.rs`.

## Notes for Future Development

- No trait abstractions or plugin system — adding tools requires editing `app.rs` directly.
- `app.rs` keeps growing — see `SPLIT_PLAN.md` before adding another large subsystem to it (LSP got its own `src/lsp.rs`; follow that precedent).
- Dead dependencies (`dialoguer`, `marked`, `markdown`) could be removed.
- No CI; `cargo test` is the gate.
