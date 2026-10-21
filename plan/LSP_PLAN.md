# Plan: Add Language Server Protocol (LSP) support — rust-analyzer

Status: **IMPLEMENTED (Phases 1–3 + 5 done; Phase 4 diagnostics panel
skipped).** This doc is kept as the design record; AGENTS.md describes the
shipped feature. Deviations from the original plan:

- `lsp-server`'s `Connection::stdio()` is server-oriented, so the client
  uses its `Message::read`/`Message::write` framing directly over the
  child process's pipes instead.
- The mock LSP server is an in-process thread over `std::io::pipe()`
  pairs (not a re-executed binary) — hermetic without banner pollution.
- Everything lives in `src/lsp.rs` (`lsp_state.rs` / `workspace.rs`
  folded in, as the plan allowed).
- Stretch tool `lsp_definition` landed as the `go_to_definition` agentic
  tool (`textDocument/definition`, `path`/`line`/`column` args, 1-based,
  results formatted with a source excerpt by `format_locations`).
- The remaining stretch tools shipped too: `lsp_hover`
  (`textDocument/hover`, rendered hover text via `format_hover_contents`),
  `lsp_references` (`textDocument/references`, optional
  `include_declaration` arg, locations formatted by `format_locations`),
  and `lsp_completion` (`textDocument/completion`, `label — detail` list;
  both `CompletionList` and bare-array response shapes handled). All three
  share the `lsp_position_args`/`lsp_position_preflight` helpers in
  `app.rs` (1-based → 0-based conversion, server-status + `sync_file`
  preflight). Only the Phase 4 diagnostics UI panel remains unimplemented.

## What this feature is

Rustama is an agentic AI coding client: the model reads/edits files and runs
`bash`/terminal tools on a codebase. Today it is **blind** — after editing a
`.rs` file it has no idea whether the change compiles. Adding an **LSP client
(rust-analyzer)** gives the AI real compiler feedback, turning it from a
"search-and-replace robot" into something that can fix its own type errors.

Primary goal: **a `lsp_diagnostics` tool** (+ auto-feedback after edits) so the
AI sees rust-analyzer diagnostics and iterates until clean.

## Architectural fit (verified against the code)

- Tools run **synchronously on the main thread** in `execute_tool_call()`, e.g.
  `bash` uses a 30 s `try_wait()` poll loop. An LSP request tool follows the
  same pattern (send request, block on a response with timeout + cancel).
- The event loop already uses the **mpsc + `try_recv()` poll** pattern
  (`check_model_responses`, `check_ctx_response`). The LSP reader thread reuses
  it: drain incoming messages in a new `check_lsp_responses()`.
- `TerminalState` proves a self-contained background-thread submodule pattern.
  `src/lsp.rs` mirrors it.
- There is **no workspace-root concept yet** — `bash` runs with no `current_dir`,
  and file tools take absolute/relative paths. LSP needs a root → add one.
- The UI splits chat | terminal (a right panel). A diagnostics **panel** is a
  stretch goal reusing the same layout machinery.

## Crates to add (`Cargo.toml`)

- **`lsp-types`** — the LSP protocol types (`InitializeParams`, `PublishDiagnosticsParams`, `Location`, …). Non-negotiable.
- **`lsp-server`** (rust-lang org) — `Connection::stdio()` handles the
  `Content-Length` framing + request/response plumbing over rust-analyzer's
  stdio. Optional but recommended; hand-rolling framing is only ~40 lines if we
  want zero deps. **Decision: use both.**

Concurrency is std/`std::sync` + `mpsc`/`oneshot` — no tokio needed here (mirrors
`TerminalState`; the app is sync-first).

---

## Module layout

```
src/lsp.rs            LspClient: child process, Connection, reader thread,
                      diagnostics store, request/notification helpers
src/lsp_state.rs      LspStore: shared diagnostics (Mutex) + pending-request
                      correlation (id -> oneshot)         [may fold into lsp.rs]
src/workspace.rs      workspace-root resolution (config, /workspace command)  [small]
```

Tool schemas, `TOOL_NAMES`, and the `execute_tool_call` arm live where the other
tools do (see "Agentic integration"). The LSP client itself is standalone.

---

## Phase 1 — Core LSP client (no UI, no tools)

1. `src/lsp.rs` + `mod lsp;` in `main.rs`.
2. **Process + handshake**: spawn `rust-analyzer` (`Command::new`), stderr → log
   file, `Connection::stdio()`. Send `initialize`, await response, send
   `initialized`, then `workspace/didChangeConfiguration` (if needed). On failure
   → status message, `lsp = off`.
3. **Reader thread**: loop `connection.receiver.recv()`; route:
   - `Response` → resolve the pending request's oneshot (match by `id`).
   - `Notification(publishDiagnostics)` → parse `Diagnostic[]`, store in
     `LspStore` keyed by `textDocument.uri` (+ workspace summary counts).
   - `Notification(shutdown/exit/error)` → log, mark stopped.
4. **Sync `request()` / `notify()` helpers**:
   - `notify_open(uri)`, `notify_change(uri, text)`, `notify_save(uri)`.
   - `request<T>(method, params, timeout, cancel)` → send + block on oneshot.
5. **Lifecycle**: `shutdown` + `exit` on app quit; kill child; restart on
   workspace change. `App::stop_request()` (F7) also cancels in-flight LSP reqs.

**Verify**: unit-test message framing/correlation with a mock server; manual run
with a real rust-analyzer in a Rust repo.

## Phase 2 — Agentic integration (the killer feature)

Add a **new agentic tool `lsp_diagnostics`** (per AGENTS.md: schema in
`get_tool_definitions()`, arm in `execute_tool_call()`, name in `TOOL_NAMES`,
plus the "Available tools" list in the unknown-tool error — 4 edits in app.rs).

- **Input**: `path` (optional; defaults to the most recently edited file, else
  whole workspace), `severity` (optional filter: error|warning|hint|info).
- **Behavior**: ensure the file is open (`textDocument/didOpen` with current
  content via `read_file`), send `textDocument/diagnostic` (pull) **and** drain
  any cached `publishDiagnostics` push; format results compactly:
  `path:line:col: [error] message`.
- **Timeout** (default 30 s, like bash) + **cancel** via shared `Arc<AtomicBool>`.

**Auto-feedback** (optional but recommended): after `edit_file` / `write_file`
tool results, trigger an async `lsp_diagnostics` and append a
`Diagnostics: ...` block to the tool result, so the AI sees errors immediately
and can fix them. Gate behind config `lsp_auto_diagnostics`.

**Stretch tools** (later): `lsp_hover`, `lsp_definition`, `lsp_references`,
`lsp_completion` — same request pattern, give the AI "go-to-def" superpowers.

## Phase 3 — Workspace root + config

- **Config keys** (`rustama.conf`):
  - `lsp = on|off` (default off — rust-analyzer is a heavy process).
  - `lsp_server = rust-analyzer` (binary path, overridable).
  - `lsp_workspace = <dir>` (root dir; default = dir of `save_path`, else cwd).
  - `lsp_auto_diagnostics = on|off`.
- **`/workspace <dir>`** slash command (+ help text) to switch roots; on change,
  shutdown + relaunch LSP with the new root. Missing `Cargo.toml` → warn but
  continue (rust-analyzer still parses single files).
- `save_config()` persists the new keys; `Config` struct + backfill defaults.

## Phase 4 — Diagnostics UI panel (stretch)

- Reuse the chat|terminal split machinery to add a **Diagnostics panel**
  (F-key or menu item toggles a third pane / overlay).
- Render cached diagnostics: severity-colored rows, `file:line:col message`,
  scrollable, `enter` jumps to the file (open in editor / attach via
  `FileContent`).
- Menu item = extend `item_names()` + `action_for()` (offsets auto-computed).
- Status bar indicator: `LSP: ✓` / `LSP: 3 errors` via `render_status_bar`.

## Phase 5 — Testing

- **Mock LSP server** (unit): a small thread that speaks `Content-Length` JSON
  over a pipe, replies to `initialize` and `textDocument/diagnostic` with canned
  `Diagnostic[]` — asserts framing, request id correlation, timeout, cancel, and
  diagnostics parsing. This keeps tests hermetic (no real rust-analyzer needed).
- **Tool tests**: `lsp_diagnostics` returns formatted output; `path`/`severity`
  filtering; auto-feedback appends to `edit_file` result.
- **Config tests**: parse/backfill/save `lsp_*` keys; `/workspace` command.
- Keep tests fast; do **not** spawn real rust-analyzer in CI-style runs.

---

## Risks / notes

- **rust-analyzer is heavy** (~100 MB RAM, slow first load on big crates) →
  default `lsp = off`, explicit opt-in, and a generous-but-bounded request
  timeout with F7 cancel.
- **Blocking on main thread**: the `lsp_diagnostics` tool blocks like `bash`
  does; keep requests short and cancellable, never block the render loop while
  idle (only during a tool round).
- **Process lifecycle**: ensure `shutdown/exit` + `child.kill()` on quit and on
  every `App` drop so no orphan rust-analyzer survives.
- **Edition 2024 / sync-first**: do **not** pull tokio into the LSP path; std
  `mpsc`/`oneshot` + a reader thread matches the existing architecture.
- **No workspace concept today**: Phase 3 is a hard prerequisite for usefulness,
  not optional.

## Execution order
`Cargo deps → src/lsp.rs core (Phase 1) → Phase 3 config/workspace → Phase 2 tool +
auto-feedback → Phase 5 tests → (stretch) Phase 4 UI → AGENTS.md update`.
Run `cargo build` (warning-free) + full `cargo test` after each phase.