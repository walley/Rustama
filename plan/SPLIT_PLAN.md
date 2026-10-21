# Plan: Split `src/app.rs` into modules

Status: **PLANNED — no code moved yet.** This doc is the roadmap.

## Why / goal

`src/app.rs` is ~7,900 lines and mixes ~14 distinct concerns. AGENTS.md already
flags it as the top refactor candidate. Splitting shrinks each file, groups
tests next to their code, and is a prerequisite for ever adding a `lib.rs` /
`tests/` integration suite later.

## Current anatomy (line ranges)

| Region | Lines | Contents |
|--------|-------|----------|
| enums/types | 135–323 | `InputMode`, `Focus`, dialog focuses, `SettingsFocus`, `ChatMessage`, `ExportFormat`, `TokenStats`, `StreamChunk` |
| terminal | 324–899 | `TerminalBuffer`, `TerminalCore`, `TerminalQueries`, `TerminalState`, `key_to_pty_bytes`, `decode_terminal_escapes` |
| App struct | 972–1103 | the big `pub struct App` (~50 fields) |
| App impl | 1104–4745 | all ~230 methods |
| free fns | 4746–5280 | rotate_log, usage parse, scrollbar, truncate, dirs, session paths, `params_for_model` |
| tools | 5281–6260 | `get_tool_definitions`, `execute_tool_call`, `grep_regex`, `unified_diff`, `parse_text_tool_calls`, http clients, `retry_countdown`, `needs_continuation`, `TOOL_NAMES`, `MAX_AUTO_CONTINUES` |
| networking | 7403–7750 | `extract_context_length`, `build_chat_body`, `stream_chat_request`, `SendOutcome` |
| tests | inline `mod`s | `usage_tests`, `scrollbar_tests`, `truncate_tests`, `sessions_dir_tests`, `continuation_tests`, `menu_focus_tests`, `tests`, `chat_body_tests` |

## Key enabler

Rust allows **multiple `impl App` blocks across files** in the same crate. App's
fields are already mostly `pub`, so a sibling module (`src/commands.rs`) can do
`impl App { fn slash_ctx(&self) {} }` and reach the fields directly. This is
what makes Phase 2 low-friction. Free functions/tests must be `pub(crate)` /
moved into their new module, but nothing becomes `pub` in a public-API sense
(the crate has no `lib.rs`, so everything is internal).

---

## Phase 1 — Extract free-standing code (no `App` dependency) — LOW risk, do first

Move code + its inline tests together. Each new file becomes `mod x;` in `main.rs`
(or a `lib.rs` if one is added). Mark cross-module callers' fns `pub(crate)`.

1. **`src/terminal.rs`** (~565 prod + ~450 test) — `TerminalBuffer`, `TerminalCore`,
   `TerminalQueries`, `TerminalState`, `key_to_pty_bytes`, `decode_terminal_escapes`,
   `TERMINAL_RAW_LOG_MAX`, + all `cursor_*` / `terminal_*` / `pty_*` / key-mapping tests.
   Self-contained (portable-pty, vt100). **Zero dependency on App.** Verify `main.rs`
   still compiles (it reads `app.terminal_state.*`).

2. **`src/util.rs`** (~150 prod + test) — `scrollbar_scroll_offset`, `chrono_now`,
   `truncate`, `common_prefix`, `patch_tool_call_ids`, `needs_continuation`,
   + `truncate_tests`, `scrollbar_tests`, `continuation_tests`. Pure helpers.

3. **`src/sessions.rs`** (~150 prod + test) — `sessions_dir`, `xdg_documents_dir`,
   `generate_session_id`, `dirs_home`, + `sessions_dir_tests`. Pure path/session fns.

4. **`src/tools.rs`** (~1000 prod + test) — `get_tool_definitions`, `TOOL_NAMES`,
   `execute_tool_call`, `grep_regex`, `parse_text_tool_calls`, `unified_diff`,
   `build_blocking_client` + the `read_file_offset_limit` / `unified_diff_*` /
   `edit_file_result_contains_diff` tests. Depends on `config` only. `MAX_AUTO_CONTINUES`
   stays or moves here with `needs_continuation`.

5. **`src/http.rs`** (~small) — `build_reqwest_client`, `build_blocking_client`,
   `retry_countdown`, `send_with_retry`, `SendOutcome`, + `retry_countdown_*` and
   `mock_server_429_flow` tests. Shared by networking + `fetch_models_async` /
   `fetch_ctx_async`.

6. **`src/chat.rs`** (~350 prod + test) — `build_chat_body`, `stream_chat_request`,
   `extract_context_length`, `params_for_model`, + `chat_body_tests` and the
   `extract_context_length_*` tests. Depends on `http.rs`, `tools.rs`, `config`.
   The two App call sites (`send_to_ollama_async_with`, `send_tool_results_async`)
   become thin wrappers.

7. **`src/usage.rs`** (~250 prod + test) — `TokenStats`, `StreamChunk`,
   `parse_token_stats`, `usage_from_json`, `parse_usage_stats`, + `usage_tests`.
   (`format_token_stats`/`session_cost`/`format_session_usage` are App methods →
   move in Phase 2 or keep in app.rs.)

8. **`src/types.rs`** (~200) — `InputMode`, `Focus`, all dialog-focus enums,
   `SettingsFocus`, `ChatMessage`, `ExportFormat` + their small `impl`s. Kept apart
   so later phases and `main.rs`/`ui.rs` import types from one place.

### Phase 1 verification
- `cargo build` warning-free, `cargo test` green (full suite).
- `main.rs` / `ui.rs` imports updated to `crate::terminal::…`, `crate::chat::…` etc.

---

## Phase 2 — Split App's `impl` blocks by concern — MEDIUM risk

Keep `App` struct + `App::new()` in `app.rs`; move groups of methods into sibling
modules as `impl App { … }`. Order by cohesion + independence:

9. **`src/commands.rs`** — `handle_slash_command`, `slash_help`, `slash_ctx`,
   `slash_model`, `slash_list`, `slash_tools`, `slash_config`, `slash_log`,
   `slash_status`, `slash_usage`, `slash_system`, `slash_setsystem`, `log_event`.
   Cleanest seam (whole slash subsystem is self-contained).

10. **`src/dialogs.rs`** — model dialog (`open_model_dialog`, `fetch_models_async`,
    `check_model_responses`, `check_ctx_response`, `fetch_ctx_async`, dialog key/click),
    settings dialog (`open_settings_dialog`, `handle_settings_dialog_*`, `confirm_settings`),
    save/load dialogs, file dialog (`refresh_file_list`, `navigate_into`,
    `open_selected_file`, `handle_file_dialog_*`, `adjust_scroll`,
    `build_file_action_dialog`), about/quit dialogs.

11. **`src/input.rs`** — `handle_global_key`, `handle_output_key`, `handle_input_key`,
    `handle_menu_key`, `handle_click`, `handle_mouse_drag/up`, `tab_complete_model`,
    `open_menu`, `handle_menu_action`, `execute_terminal_tool`.

12. **`src/sessions_app.rs`** — App methods `save_session`, `load_session`,
    `autosave_session`, `new_session`, `save_config`, `open_load_session_dialog`,
    `execute_load_session`, export/save-dialog actions.

13. **`src/output.rs`** — `build_system_content`, `scroll_*`, `set_auto_scroll`,
    `reroll_spinner`, `output_line_at_row`, `selected_text`, scrollbar drag helpers.

### Phase 2 verification
- Same: warning-free build, full `cargo test` green.
- `main.rs` `use app::{…}` list shrinks but public names stay reachable via re-exports
  (`pub(crate) use crate::commands::*;` in `app.rs` or direct `crate::commands::…`).

---

## Phase 3 (optional, later) — library + integration tests

- Add `src/lib.rs` re-exporting the modules, turning the bin into a thin
  `main.rs` that calls into the lib. Only then does a `tests/` dir (black-box
  integration tests against the public API) become possible/valuable.
- Keep unit tests inline in their modules; do **not** move them to `tests/`.

---

## Do / don't

- **Do** move tests *with* their code into the new module (keeps `#[cfg(test)]`
  access to privates).
- **Do** keep `─── section ───` comment banners; add them per module.
- **Do** one module per commit/PR; run the full suite each time.
- **Don't** change behavior while refactoring — pure moves + visibility tweaks.
- **Don't** touch the slow `mock_server_429_flow` / retry tests for speed.
- **Don't** add `tests/` until Phase 3.

## Suggested execution order
`terminal → util → sessions → tools → http → chat → usage → types →`
then Phase 2 `commands → dialogs → input → sessions_app → output`,
verifying after each.