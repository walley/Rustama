//! LSP client — compiler feedback for the agentic tools.
//!
//! Supports two transports:
//! - **Stdio**: spawns a child process (e.g. rust-analyzer) with piped
//!   stdin/stdout.
//! - **TCP**: connects to an already-running LSP server over a TCP socket
//!   (e.g. Kotlin LSP on 127.0.0.1:9999).
//!
//! Architecture (mirrors `TerminalState` in app.rs):
//!
//! - Messages use the `lsp-server` crate's `Message::read`/`Message::write`
//!   framing (`Content-Length` headers + JSON-RPC) over the chosen
//!   transport.
//! - A reader thread routes incoming messages: `Response`s resolve pending
//!   requests (id → oneshot sender map), `publishDiagnostics` notifications
//!   update the shared diagnostics store, and server→client requests get
//!   minimal canned replies so the server never stalls waiting on us.
//! - All concurrency is std `mpsc`/`Mutex` — no tokio (the app is
//!   sync-first). Requests block the calling thread with a timeout plus a
//!   shared cancel flag, exactly like the `bash` tool's 30 s poll loop.
//!
//! Lifecycle: `start()` connects + handshakes (`initialize`/`initialized`),
//! `shutdown()` sends `shutdown`/`exit` and closes the connection;
//! `Drop` guarantees no orphan server survives.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

use lsp_server::{Message, Notification, Request, RequestId, Response};

/// How to connect to the LSP server.
pub enum LspConnect {
    /// Spawn `bin` as a child process with piped stdio (rust-analyzer).
    Stdio { bin: String },
    /// Connect to `addr` (e.g. "127.0.0.1:9999") via TCP (Kotlin LSP).
    Tcp { addr: String },
}

/// The underlying transport of an LSP connection.
enum LspTransport {
    /// Child process communicating over stdio pipes.
    Stdio { child: Child },
    /// TCP socket connection to an already-running server.
    Tcp,
}

/// Outbound half of the LSP connection — any `Write + Send` type works.
/// Production uses `ChildStdin` (stdio) or `TcpStream` (TCP socket).

/// Default timeout for LSP requests (same ballpark as the bash tool's 30 s).
pub const LSP_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
/// Timeout for the `initialize` handshake (first load indexes the crate
/// graph, so this is more generous than a normal request).
pub const LSP_INIT_TIMEOUT: Duration = Duration::from_secs(60);

/// Connection state of the LSP subprocess.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LspStatus {
    /// Never started, or cleanly stopped.
    Stopped,
    /// Child spawned, `initialize` handshake in flight.
    Starting,
    /// Handshake done; requests may be sent.
    Running,
    /// Spawn/handshake/reader failed; message describes why.
    Failed(String),
}

/// Diagnostics pushed by the server, keyed by (absolute) file path. A
/// per-file version counter lets requesters wait for "analysis newer than
/// X" without parsing server progress notifications.
#[derive(Debug, Default)]
pub struct DiagStore {
    pub by_path: HashMap<PathBuf, Vec<lsp_types::Diagnostic>>,
    versions: HashMap<PathBuf, u64>,
}

impl DiagStore {
    fn publish(&mut self, path: PathBuf, diags: Vec<lsp_types::Diagnostic>) {
        let v = self.versions.entry(path.clone()).or_insert(0);
        *v += 1;
        self.by_path.insert(path, diags);
    }

    /// Version counter for `path` (0 = never published).
    pub fn version(&self, path: &Path) -> u64 {
        self.versions.get(path).copied().unwrap_or(0)
    }

    /// Sum of diagnostics across all files, split by severity
    /// (errors, warnings).
    pub fn summary(&self) -> (usize, usize) {
        let mut errors = 0;
        let mut warnings = 0;
        for diags in self.by_path.values() {
            for d in diags {
                match d.severity {
                    Some(lsp_types::DiagnosticSeverity::ERROR) => errors += 1,
                    Some(lsp_types::DiagnosticSeverity::WARNING) => warnings += 1,
                    _ => {}
                }
            }
        }
        (errors, warnings)
    }
}

/// Shared interior state between the client handle and the reader thread.
struct Shared {
    pending: Mutex<HashMap<i64, mpsc::Sender<Response>>>,
    diags: Mutex<DiagStore>,
    status: Mutex<LspStatus>,
    next_id: AtomicI64,
    /// Set when the server advertised pull diagnostics
    /// (`textDocument/diagnostic`) in its capabilities.
    pull_supported: AtomicBool,
    /// Files currently open in the server (didOpen sent, no didClose —
    /// documents stay open for the whole session).
    open_docs: Mutex<std::collections::HashSet<PathBuf>>,
    /// Monotonic document version counter for didOpen/didChange (versions
    /// must increase per document; a global counter satisfies that).
    doc_version: AtomicI64,
}

/// A running (or stopped) LSP session. Cheap to keep around while stopped;
/// `start()` is explicit because the connection is heavy.
pub struct LspClient {
    shared: Arc<Shared>,
    transport: Option<LspTransport>,
    stdin: Option<Arc<Mutex<Box<dyn Write + Send>>>>,
    reader: Option<std::thread::JoinHandle<()>>,
    /// Set to make the reader thread exit (used on shutdown/restart).
    stop_reader: Arc<AtomicBool>,
    root: PathBuf,
    /// Server binary (stdio mode) or address (TCP mode), remembered for
    /// restarts.
    server_bin: String,
    /// Directory for the LSP server's stderr log file (rustama-lsp.log).
    /// Defaults to current directory; set via `set_log_path()`.
    log_path: PathBuf,
    /// Session id embedded in the LSP log filename so concurrent instances
    /// don't clobber each other's logs.
    session_id: String,
}

impl std::fmt::Debug for LspClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LspClient")
            .field("status", &self.status())
            .field("root", &self.root)
            .field("server_bin", &self.server_bin)
            .finish()
    }
}

impl Default for LspClient {
    fn default() -> Self {
        Self::new()
    }
}

impl LspClient {
    /// A stopped client with no workspace. Call `start()` to launch.
    pub fn new() -> Self {
        LspClient {
            shared: Arc::new(Shared {
                pending: Mutex::new(HashMap::new()),
                diags: Mutex::new(DiagStore::default()),
                status: Mutex::new(LspStatus::Stopped),
                next_id: AtomicI64::new(1),
                pull_supported: AtomicBool::new(false),
                open_docs: Mutex::new(std::collections::HashSet::new()),
                doc_version: AtomicI64::new(0),
            }),
            transport: None,
            stdin: None,
            reader: None,
            stop_reader: Arc::new(AtomicBool::new(false)),
            root: PathBuf::new(),
            server_bin: String::new(),
            log_path: PathBuf::new(),
            session_id: String::new(),
        }
    }

    pub fn status(&self) -> LspStatus {
        self.shared.status.lock().unwrap().clone()
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Set the directory for `rustama-lsp.log` (LSP server stderr).
    /// Call before `start()`.
    pub fn set_log_path(&mut self, path: PathBuf) {
        self.log_path = path;
    }

    /// Set the session id embedded in the LSP log filename.
    pub fn set_session_id(&mut self, id: &str) {
        self.session_id = id.to_string();
    }

    /// Connects to the LSP server rooted at `root` and performs the LSP
    /// handshake. `connect` specifies whether to spawn a child process
    /// (stdio) or connect to a TCP socket. Any previous server is shut
    /// down first (workspace switch).
    pub fn start(&mut self, root: PathBuf, connect: LspConnect) -> Result<(), String> {
        self.shutdown();
        self.root = root;
        match &connect {
            LspConnect::Stdio { bin } => self.server_bin = bin.clone(),
            LspConnect::Tcp { addr } => self.server_bin = addr.clone(),
        }
        *self.shared.status.lock().unwrap() = LspStatus::Starting;

        // Build the transport, stdin writer, and reader. We must extract
        // stdout/stdin from the child/stream *before* storing the transport,
        // since `ChildStdout`/`TcpStream` are not `Clone` and we need
        // separate handles for reading vs writing.
        let (stdin_writer, reader, transport) = match connect {
            LspConnect::Stdio { bin } => {
                let log_file = self
                    .log_path
                    .join(format!("rustama-lsp-{}.log", self.session_id));
                let stderr: Stdio = std::fs::File::create(&log_file)
                    .map(Stdio::from)
                    .unwrap_or(Stdio::null());
                let mut child = Command::new(&bin)
                    .current_dir(&self.root)
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(stderr)
                    .spawn()
                    .map_err(|e| format!("failed to spawn {}: {}", bin, e))?;
                let stdin = child.stdin.take().ok_or("no stdin on LSP child")?;
                let stdout = child.stdout.take().ok_or("no stdout on LSP child")?;
                let reader: Box<dyn BufRead + Send> = Box::new(BufReader::new(stdout));
                let transport = LspTransport::Stdio { child };
                (Box::new(stdin) as Box<dyn Write + Send>, reader, transport)
            }
            LspConnect::Tcp { addr } => {
                let stream = TcpStream::connect(&addr)
                    .map_err(|e| format!("failed to connect to {}: {}", addr, e))?;
                // One stream handle for writing (stdin), one for reading.
                let write_stream = stream
                    .try_clone()
                    .map_err(|e| format!("failed to clone TcpStream: {}", e))?;
                let reader: Box<dyn BufRead + Send> = Box::new(BufReader::new(stream));
                let transport = LspTransport::Tcp;
                (
                    Box::new(write_stream) as Box<dyn Write + Send>,
                    reader,
                    transport,
                )
            }
        };
        let stdin: Arc<Mutex<Box<dyn Write + Send>>> = Arc::new(Mutex::new(stdin_writer));

        self.stop_reader = Arc::new(AtomicBool::new(false));
        let reader_thread = spawn_reader(
            reader,
            Arc::clone(&stdin),
            Arc::clone(&self.shared),
            Arc::clone(&self.stop_reader),
        );
        self.reader = Some(reader_thread);
        self.stdin = Some(stdin);
        self.transport = Some(transport);

        // --- handshake ---
        let root_uri = path_to_uri(&self.root)?;
        let init_params = build_initialize_params(root_uri)?;
        let result = self.request_value(
            "initialize",
            init_params,
            LSP_INIT_TIMEOUT,
            &AtomicBool::new(false),
        )?;
        let init: lsp_types::InitializeResult = serde_json::from_value(result)
            .map_err(|e| format!("bad initialize result: {}", e))?;
        self.shared
            .pull_supported
            .store(init.capabilities.diagnostic_provider.is_some(), Ordering::Relaxed);
        self.notify("initialized", serde_json::json!({}))?;
        *self.shared.status.lock().unwrap() = LspStatus::Running;
        Ok(())
    }

    /// Sends a notification. Errors if no server is running.
    pub fn notify(&self, method: &str, params: serde_json::Value) -> Result<(), String> {
        let notif = Notification::new(method.to_string(), params);
        self.send(&Message::Notification(notif))
    }

    /// Sends a request and blocks for the response (timeout + cancel flag).
    /// Returns the raw `result` JSON.
    pub fn request_value(
        &self,
        method: &str,
        params: impl serde::Serialize,
        timeout: Duration,
        cancel: &AtomicBool,
    ) -> Result<serde_json::Value, String> {
        if *self.shared.status.lock().unwrap() == LspStatus::Stopped {
            return Err("LSP server not running".to_string());
        }
        let id = self.shared.next_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = mpsc::channel();
        self.shared.pending.lock().unwrap().insert(id, tx);
        let req = Request::new(RequestId::from(id as i32), method.to_string(), params);
        if let Err(e) = self.send(&Message::Request(req)) {
            self.shared.pending.lock().unwrap().remove(&id);
            return Err(e);
        }
        let start = Instant::now();
        loop {
            if cancel.load(Ordering::Relaxed) {
                self.shared.pending.lock().unwrap().remove(&id);
                return Err("cancelled".to_string());
            }
            let remaining = timeout.saturating_sub(start.elapsed());
            if remaining.is_zero() {
                self.shared.pending.lock().unwrap().remove(&id);
                return Err(format!("LSP request {} timed out", method));
            }
            match rx.recv_timeout(remaining.min(Duration::from_millis(100))) {
                Ok(resp) => {
                    if let Some(err) = resp.error {
                        return Err(format!("{}: {}", method, err.message));
                    }
                    return Ok(resp.result.unwrap_or(serde_json::Value::Null));
                }
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Err(format!("LSP server died during {}", method));
                }
            }
        }
    }

    fn send(&self, msg: &Message) -> Result<(), String> {
        let stdin = self.stdin.as_ref().ok_or("LSP server not running")?;
        let mut guard = stdin.lock().unwrap();
        // NB: `Message::write` takes `&mut impl Write` (Sized) — pass the
        // trait-object *reference* as the writer (blanket impl on &mut W).
        let mut w: &mut dyn Write = &mut *guard;
        msg.write(&mut w)
            .and_then(|()| w.flush())
            .map_err(|e| format!("LSP write failed: {}", e))
    }

    /// Politely shuts the server down (`shutdown` + `exit`), then closes the
    /// connection. Safe to call on an already-stopped client.
    pub fn shutdown(&mut self) {
        if self.transport.is_some() {
            if *self.shared.status.lock().unwrap() == LspStatus::Running {
                let _ = self.request_value(
                    "shutdown",
                    serde_json::Value::Null,
                    Duration::from_secs(5),
                    &AtomicBool::new(false),
                );
                let _ = self.notify("exit", serde_json::Value::Null);
            }
            self.stop_reader.store(true, Ordering::Relaxed);
            self.kill_child();
        }
        *self.shared.status.lock().unwrap() = LspStatus::Stopped;
    }

    /// Closes the transport: for stdio, waits for/kills the child process;
    /// for TCP, nothing extra to do (streams are closed by dropping stdin/
    /// reader).
    fn kill_child(&mut self) {
        if let Some(LspTransport::Stdio { mut child }) = self.transport.take() {
            // exit notification gives it a moment; then escalate.
            for _ in 0..20 {
                if matches!(child.try_wait(), Ok(Some(_))) {
                    break;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            if matches!(child.try_wait(), Ok(None)) {
                let _ = child.kill();
            }
            let _ = child.wait();
        }
        self.stdin = None;
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
        // Fail every request still in flight.
        self.shared.pending.lock().unwrap().clear();
    }

    // ── text document sync ────────────────────────────────────────────

    /// Ensures the server has the current on-disk content of `path`:
    /// sends `didOpen` the first time, `didChange` (full content) after
    /// that. This is the only sync entry point tools need.
    pub fn sync_file(&self, path: &Path) -> Result<(), String> {
        let abs = absolutize(path);
        let already_open = self.shared.open_docs.lock().unwrap().contains(&abs);
        if already_open {
            self.did_change(&abs)
        } else {
            self.did_open(&abs)?;
            self.shared.open_docs.lock().unwrap().insert(abs);
            Ok(())
        }
    }

    /// Opens `path` in the server with its current on-disk content.
    fn did_open(&self, path: &Path) -> Result<(), String> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("cannot read {}: {}", path.display(), e))?;
        let uri = path_to_uri(path)?;
        let version = self.shared.doc_version.fetch_add(1, Ordering::Relaxed) as i32 + 1;
        self.notify(
            "textDocument/didOpen",
            serde_json::json!({
                "textDocument": {
                    "uri": uri,
                    "languageId": language_id_for(path),
                    "version": version,
                    "text": text,
                }
            }),
        )
    }

    /// Tells the server the file changed, re-sending full content from disk
    /// (`TextDocumentSyncKind::Full` — rust-analyzer accepts it).
    fn did_change(&self, path: &Path) -> Result<(), String> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("cannot read {}: {}", path.display(), e))?;
        let uri = path_to_uri(path)?;
        let version = self.shared.doc_version.fetch_add(1, Ordering::Relaxed) as i32 + 1;
        self.notify(
            "textDocument/didChange",
            serde_json::json!({
                "textDocument": { "uri": uri, "version": version },
                "contentChanges": [{ "text": text }],
            }),
        )
    }

    // ── diagnostics ───────────────────────────────────────────────────

    /// Snapshot of the cached diagnostics for `path` (empty if none).
    pub fn diagnostics_for(&self, path: &Path) -> Vec<lsp_types::Diagnostic> {
        self.shared
            .diags
            .lock()
            .unwrap()
            .by_path
            .get(&absolutize(path))
            .cloned()
            .unwrap_or_default()
    }

    /// Cached (errors, warnings) counts across the workspace.
    pub fn diagnostics_summary(&self) -> (usize, usize) {
        self.shared.diags.lock().unwrap().summary()
    }

    /// Diagnostics version for `path` (bumps on every publish).
    pub fn diag_version(&self, path: &Path) -> u64 {
        self.shared.diags.lock().unwrap().version(&absolutize(path))
    }

    /// Whether the server advertised pull diagnostics.
    pub fn pull_supported(&self) -> bool {
        self.shared.pull_supported.load(Ordering::Relaxed)
    }

    /// Pull diagnostics for `path` via `textDocument/diagnostic`.
    /// Caller checks `pull_supported()` first.
    pub fn pull_diagnostics(
        &self,
        path: &Path,
        timeout: Duration,
        cancel: &AtomicBool,
    ) -> Result<Vec<lsp_types::Diagnostic>, String> {
        let uri = path_to_uri(path)?;
        let result = self.request_value(
            "textDocument/diagnostic",
            serde_json::json!({
                "textDocument": { "uri": uri },
            }),
            timeout,
            cancel,
        )?;
        let report: lsp_types::DocumentDiagnosticReportResult =
            serde_json::from_value(result).map_err(|e| format!("bad diagnostic report: {}", e))?;
        Ok(extract_report_items(&report))
    }

    /// Resolves the definition(s) of the symbol at `(line, character)`
    /// (0-based) in `path` via `textDocument/definition`. Normalizes the
    /// three response shapes (single `Location`, `Location[]`, or
    /// `LocationLink[]`) into a flat list of target `Location`s. The
    /// caller must `sync_file` first so the server has the current text.
    pub fn goto_definition(
        &self,
        path: &Path,
        line: u32,
        character: u32,
        timeout: Duration,
        cancel: &AtomicBool,
    ) -> Result<Vec<lsp_types::Location>, String> {
        let uri = path_to_uri(path)?;
        let result = self.request_value(
            "textDocument/definition",
            serde_json::json!({
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character },
            }),
            timeout,
            cancel,
        )?;
        let resp: lsp_types::GotoDefinitionResponse = serde_json::from_value(result)
            .map_err(|e| format!("bad definition response: {}", e))?;
        Ok(match resp {
            lsp_types::GotoDefinitionResponse::Scalar(loc) => vec![loc],
            lsp_types::GotoDefinitionResponse::Array(locs) => locs,
            lsp_types::GotoDefinitionResponse::Link(links) => links
                .into_iter()
                .map(|l| lsp_types::Location::new(l.target_uri, l.target_range))
                .collect(),
        })
    }

    /// Resolves the hover info for the symbol at `(line, character)`
    /// (0-based) in `path` via `textDocument/hover`. Returns the rendered
    /// hover text, or an empty string if the server has nothing at that
    /// position. The caller must `sync_file` first.
    pub fn hover(
        &self,
        path: &Path,
        line: u32,
        character: u32,
        timeout: Duration,
        cancel: &AtomicBool,
    ) -> Result<String, String> {
        let uri = path_to_uri(path)?;
        let result = self.request_value(
            "textDocument/hover",
            serde_json::json!({
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character },
            }),
            timeout,
            cancel,
        )?;
        // A null result means "no hover here".
        if result.is_null() {
            return Ok(String::new());
        }
        let hover: lsp_types::Hover = serde_json::from_value(result)
            .map_err(|e| format!("bad hover response: {}", e))?;
        Ok(format_hover_contents(&hover.contents))
    }

    /// Finds all references to the symbol at `(line, character)` (0-based)
    /// in `path` via `textDocument/references`. Returns the locations as a
    /// flat list. The caller must `sync_file` first.
    pub fn references(
        &self,
        path: &Path,
        line: u32,
        character: u32,
        include_declaration: bool,
        timeout: Duration,
        cancel: &AtomicBool,
    ) -> Result<Vec<lsp_types::Location>, String> {
        let uri = path_to_uri(path)?;
        let result = self.request_value(
            "textDocument/references",
            serde_json::json!({
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character },
                "context": { "includeDeclaration": include_declaration },
            }),
            timeout,
            cancel,
        )?;
        let locs: Vec<lsp_types::Location> = serde_json::from_value(result)
            .map_err(|e| format!("bad references response: {}", e))?;
        Ok(locs)
    }

    /// Requests completions at `(line, character)` (0-based) in `path` via
    /// `textDocument/completion`. Returns a compact list of
    /// `label — detail` strings (detail optional). The caller must
    /// `sync_file` first.
    pub fn completion(
        &self,
        path: &Path,
        line: u32,
        character: u32,
        timeout: Duration,
        cancel: &AtomicBool,
    ) -> Result<Vec<String>, String> {
        let uri = path_to_uri(path)?;
        let result = self.request_value(
            "textDocument/completion",
            serde_json::json!({
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character },
            }),
            timeout,
            cancel,
        )?;
        // Null result → no completions.
        if result.is_null() {
            return Ok(Vec::new());
        }
        let resp: lsp_types::CompletionResponse = serde_json::from_value(result)
            .map_err(|e| format!("bad completion response: {}", e))?;
        let items: Vec<lsp_types::CompletionItem> = match resp {
            lsp_types::CompletionResponse::Array(items) => items,
            lsp_types::CompletionResponse::List(list) => list.items,
        };
        Ok(items
            .into_iter()
            .map(|i| match i.detail {
                Some(d) => format!("{} — {}", i.label, d),
                None => i.label,
            })
            .collect())
    }

    /// Blocks until the server's published diagnostics for `path` reach a
    /// version newer than `since` (i.e. fresh analysis arrived), the
    /// timeout elapses, or `cancel` is raised. Returns the current version
    /// (compare against `since` to tell "fresh" from "timed out").
    pub fn wait_for_diagnostics(
        &self,
        path: &Path,
        since: u64,
        timeout: Duration,
        cancel: &AtomicBool,
    ) -> u64 {
        let start = Instant::now();
        loop {
            let v = self.diag_version(path);
            if v > since {
                return v;
            }
            if cancel.load(Ordering::Relaxed) || start.elapsed() >= timeout {
                return v;
            }
            if !matches!(self.status(), LspStatus::Running | LspStatus::Starting) {
                return v;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
}

/// Manages multiple LSP server connections, one per language.
///
/// Routes requests to the appropriate client based on file path extension
/// (via [`language_id_for`]). App holds a single `LspManager` and operates
/// all LSP through it — no per-language methods needed on App.
pub struct LspManager {
    /// LSP clients keyed by language name (e.g. "rust", "kotlin").
    clients: HashMap<String, LspClient>,
    /// Directory for LSP server log files.
    log_path: PathBuf,
    /// Session id embedded in log filenames.
    session_id: String,
}

impl LspManager {
    /// A stopped manager with no clients.
    pub fn new() -> Self {
        let mut clients = HashMap::new();
        // Pre-populate with default (stopped) clients for known languages so
        // that `client_for_path` always returns a client and tools report
        // "server not running" rather than "no server configured".
        clients.insert("rust".to_string(), LspClient::new());
        clients.insert("kotlin".to_string(), LspClient::new());
        LspManager {
            clients,
            log_path: PathBuf::new(),
            session_id: String::new(),
        }
    }

    /// Set the directory for LSP server log files. Call before `start()`.
    pub fn set_log_path(&mut self, path: PathBuf) {
        let clone = path.clone();
        self.log_path = path;
        for client in self.clients.values_mut() {
            client.set_log_path(clone.clone());
        }
    }

    /// Set the session id embedded in log filenames.
    pub fn set_session_id(&mut self, id: &str) {
        self.session_id = id.to_string();
        for client in self.clients.values_mut() {
            client.set_session_id(id);
        }
    }

    /// Starts (or restarts) the LSP server for `language` rooted at `root`.
    /// `connect` specifies how to connect (stdio child process or TCP).
    pub fn start(
        &mut self,
        language: &str,
        root: PathBuf,
        connect: LspConnect,
    ) -> Result<(), String> {
        let mut client = self.clients.remove(language).unwrap_or_else(LspClient::new);
        client.set_log_path(self.log_path.clone());
        client.set_session_id(&self.session_id);
        let result = client.start(root, connect);
        if result.is_ok() {
            self.clients.insert(language.to_string(), client);
        } else {
            // Put the failed client back so its state is preserved
            self.clients.insert(language.to_string(), client);
        }
        result
    }

    /// Stops the LSP server for `language`, if one is running.
    pub fn stop(&mut self, language: &str) {
        if let Some(mut client) = self.clients.remove(language) {
            client.shutdown();
            self.clients.insert(language.to_string(), client);
        }
    }

    /// Returns the LSP client responsible for `path`, based on file
    /// extension. Returns `None` if no client is configured for that
    /// language.
    pub fn client_for_path(&self, path: &Path) -> Option<&LspClient> {
        let lang = language_id_for(path);
        self.clients.get(lang)
    }

    /// Returns the status of the LSP server for `language`.
    pub fn status_for(&self, language: &str) -> LspStatus {
        self.clients
            .get(language)
            .map(|c| c.status())
            .unwrap_or(LspStatus::Stopped)
    }

    /// Diagnostics for `path` (routes to the client for that file's
    /// language). Empty if no client is configured or no diagnostics.
    pub fn diagnostics_for(&self, path: &Path) -> Vec<lsp_types::Diagnostic> {
        match self.client_for_path(path) {
            Some(client) => client.diagnostics_for(path),
            None => Vec::new(),
        }
    }

    /// Combined (errors, warnings) across all LSP servers.
    pub fn diagnostics_summary(&self) -> (usize, usize) {
        let mut errors = 0;
        let mut warnings = 0;
        for client in self.clients.values() {
            let (e, w) = client.diagnostics_summary();
            errors += e;
            warnings += w;
        }
        (errors, warnings)
    }

    /// Diagnostics version for `path` (0 if no client configured).
    pub fn diag_version(&self, path: &Path) -> u64 {
        self.client_for_path(path)
            .map(|c| c.diag_version(path))
            .unwrap_or(0)
    }

    /// Root of the LSP server for `path`'s language.
    pub fn root_for_path(&self, path: &Path) -> Option<PathBuf> {
        self.client_for_path(path).map(|c| c.root().to_path_buf())
    }

    /// Wait for fresh diagnostics for `path`.
    pub fn wait_for_diagnostics(
        &self,
        path: &Path,
        since: u64,
        timeout: Duration,
        cancel: &AtomicBool,
    ) -> u64 {
        match self.client_for_path(path) {
            Some(client) => client.wait_for_diagnostics(path, since, timeout, cancel),
            None => 0,
        }
    }

    /// Returns the server name for `path`'s language (for error messages).
    pub fn server_name_for_path(&self, path: &Path) -> &'static str {
        match language_id_for(path) {
            "kotlin" => "Kotlin LSP",
            _ => "rust-analyzer",
        }
    }

    /// Returns the config key for `path`'s language.
    pub fn config_key_for_path(&self, path: &Path) -> &'static str {
        match language_id_for(path) {
            "kotlin" => "kotlin_lsp",
            _ => "lsp",
        }
    }

    /// Shared preflight for the position-based LSP tools: checks the
    /// server status and syncs the file. On success returns `()`;
    /// on failure returns a ready-to-return error string.
    pub fn position_preflight(&self, tool: &str, path: &Path) -> Result<(), String> {
        let Some(client) = self.client_for_path(path) else {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("unknown");
            return Err(format!(
                "{}: no LSP server configured for .{} files",
                tool, ext
            ));
        };
        let server_name = self.server_name_for_path(path);
        let key = self.config_key_for_path(path);

        match client.status() {
            LspStatus::Running => {}
            LspStatus::Stopped => {
                return Err(format!(
                    "{}: server not running. Enable it with `{key} = on` in \
                    rustama.conf (or /workspace <dir>) — requires {server_name}.",
                    tool
                ));
            }
            LspStatus::Failed(e) => return Err(format!("{}: server failed: {}", tool, e)),
            LspStatus::Starting => {} // rare; request below waits for analysis
        }
        if let Err(e) = client.sync_file(path) {
            return Err(format!("{}: cannot sync {}: {}", tool, path.display(), e));
        }
        Ok(())
    }

    /// Core of the `lsp_definition` tool: sync the file into the server,
    /// resolve the symbol at (line, column), format the target location(s).
    /// `line`/`column` are 1-based in the tool interface (matching how
    /// diagnostics are reported) but are converted to LSP's 0-based
    /// positions internally.
    pub fn goto_definition_tool(
        &self,
        path: Option<&str>,
        line: Option<u64>,
        column: Option<u64>,
        timeout: Duration,
    ) -> String {
        // A definition needs a concrete symbol position, so a path is
        // mandatory (unlike lsp_diagnostics which can fall back to a
        // workspace summary).
        let Some(path) = path else {
            return "lsp_definition: `path` is required (the file to search in) and \
                `line`/`column` (1-based) must point at the symbol."
                .to_string();
        };
        let Some(line) = line else {
            return "lsp_definition: `line` (1-based) is required.".to_string();
        };
        let Some(column) = column else {
            return "lsp_definition: `column` (1-based) is required.".to_string();
        };
        if line == 0 || column == 0 {
            return "lsp_definition: `line` and `column` are 1-based (>= 1).".to_string();
        }

        let path = PathBuf::from(path);
        if let Err(e) = self.position_preflight("lsp_definition", &path) {
            return e;
        }
        let Some(client) = self.client_for_path(&path) else {
            return "lsp_definition: no LSP server configured for this file type".to_string();
        };

        let root = client.root().to_path_buf();
        let cancel = AtomicBool::new(false);
        let locs = match client.goto_definition(
            &path,
            (line - 1) as u32,
            (column - 1) as u32,
            timeout,
            &cancel,
        ) {
            Ok(locs) => locs,
            Err(e) if e.contains("cancelled") => {
                return format!(
                    "lsp_definition: {} not analyzed yet (server busy re-analyzing — try again)",
                    path.display()
                );
            }
            Err(e) => return format!("lsp_definition: request failed: {}", e),
        };

        if locs.is_empty() {
            return format!(
                "lsp_definition: no definition found for {}:{}:{}",
                path.display(),
                line,
                column
            );
        }
        let formatted = format_locations(&locs, &root);
        format!(
            "Definition of {}:{}:{}:\n{}",
            path.display(),
            line,
            column,
            formatted.trim_end()
        )
    }

    /// Core of the `lsp_symbol_info` tool: sync the file, fetch hover info for
    /// the symbol at (line, column), return the rendered text.
    pub fn symbol_info_tool(
        &self,
        path: Option<&str>,
        line: Option<u64>,
        column: Option<u64>,
        timeout: Duration,
    ) -> String {
        let (path, line, column) = match parse_position_args("lsp_symbol_info", path, line, column)
        {
            Ok(t) => t,
            Err(e) => return e,
        };
        if let Err(e) = self.position_preflight("lsp_symbol_info", &path) {
            return e;
        }
        let Some(client) = self.client_for_path(&path) else {
            return "lsp_symbol_info: no LSP server configured for this file type".to_string();
        };
        let cancel = AtomicBool::new(false);
        let text = match client.hover(
            &path,
            (line - 1) as u32,
            (column - 1) as u32,
            timeout,
            &cancel,
        ) {
            Ok(t) => t,
            Err(e) if e.contains("cancelled") => {
                return format!(
                    "lsp_symbol_info: {} not analyzed yet (server busy re-analyzing — try again)",
                    path.display()
                );
            }
            Err(e) => return format!("lsp_symbol_info: request failed: {}", e),
        };
        if text.trim().is_empty() {
            format!(
                "lsp_symbol_info: no hover information for {}:{}:{}",
                path.display(),
                line,
                column
            )
        } else {
            format!(
                "Hover for {}:{}:{}:\n{}",
                path.display(),
                line,
                column,
                text.trim_end()
            )
        }
    }

    /// Core of the `lsp_references` tool: sync the file, list references to
    /// the symbol at (line, column).
    pub fn references_tool(
        &self,
        path: Option<&str>,
        line: Option<u64>,
        column: Option<u64>,
        include_declaration: Option<bool>,
        timeout: Duration,
    ) -> String {
        let (path, line, column) =
            match parse_position_args("lsp_references", path, line, column) {
                Ok(t) => t,
                Err(e) => return e,
            };
        if let Err(e) = self.position_preflight("lsp_references", &path) {
            return e;
        }
        let Some(client) = self.client_for_path(&path) else {
            return "lsp_references: no LSP server configured for this file type".to_string();
        };
        let cancel = AtomicBool::new(false);
        let locs = match client.references(
            &path,
            (line - 1) as u32,
            (column - 1) as u32,
            include_declaration.unwrap_or(true),
            timeout,
            &cancel,
        ) {
            Ok(l) => l,
            Err(e) if e.contains("cancelled") => {
                return format!(
                    "lsp_references: {} not analyzed yet (server busy re-analyzing — try again)",
                    path.display()
                );
            }
            Err(e) => return format!("lsp_references: request failed: {}", e),
        };
        if locs.is_empty() {
            return format!(
                "lsp_references: no references found for {}:{}:{}",
                path.display(),
                line,
                column
            );
        }
        let root = client.root().to_path_buf();
        let formatted = format_locations(&locs, &root);
        format!(
            "References to {}:{}:{} ({}):\n{}",
            path.display(),
            line,
            column,
            locs.len(),
            formatted.trim_end()
        )
    }

    /// Core of the `lsp_completion` tool: sync the file, request
    /// completions at (line, column), return the list.
    pub fn completion_tool(
        &self,
        path: Option<&str>,
        line: Option<u64>,
        column: Option<u64>,
        timeout: Duration,
    ) -> String {
        let (path, line, column) =
            match parse_position_args("lsp_completion", path, line, column) {
                Ok(t) => t,
                Err(e) => return e,
            };
        if let Err(e) = self.position_preflight("lsp_completion", &path) {
            return e;
        }
        let Some(client) = self.client_for_path(&path) else {
            return "lsp_completion: no LSP server configured for this file type".to_string();
        };
        let cancel = AtomicBool::new(false);
        let items = match client.completion(
            &path,
            (line - 1) as u32,
            (column - 1) as u32,
            timeout,
            &cancel,
        ) {
            Ok(items) => items,
            Err(e) if e.contains("cancelled") => {
                return format!(
                    "lsp_completion: {} not analyzed yet (server busy re-analyzing — try again)",
                    path.display()
                );
            }
            Err(e) => return format!("lsp_completion: request failed: {}", e),
        };
        if items.is_empty() {
            return format!(
                "lsp_completion: no completions for {}:{}:{}",
                path.display(),
                line,
                column
            );
        }
        let mut out = format!(
            "Completions at {}:{}:{}:\n",
            path.display(),
            line,
            column
        );
        for item in items {
            out.push_str(&format!("- {}\n", item));
        }
        out.trim_end().to_string()
    }

    /// Core of the `lsp_diagnostics` tool: sync the file into the server,
    /// wait for fresh analysis, format the results. Shared by the tool,
    /// the auto-feedback hook, and tests. `fallback_path` (the last edited
    /// file) is used when the tool is called without a `path` argument.
    pub fn diagnostics_tool(
        &self,
        path: Option<&str>,
        severity: Option<&str>,
        timeout: Duration,
        fallback_path: Option<&Path>,
    ) -> String {
        let filter = match severity {
            None => SeverityFilter::All,
            Some(s) => match SeverityFilter::parse(s) {
                Some(f) => f,
                None => {
                    return format!(
                        "LSP: unknown severity '{}' (use error|warning|hint|info)",
                        s
                    );
                }
            },
        };

        // If no LSP server is running, report it before doing anything else.
        if self.status_for("rust") != LspStatus::Running {
            return "LSP: server not running. Enable it with `lsp = on` in rustama.conf \
                (or /workspace <dir>) — requires rust-analyzer."
                .to_string();
        }

        // No path → workspace summary of everything published so far.
        let path = match path {
            Some(p) => PathBuf::from(p),
            None => match fallback_path {
                Some(p) => p.to_path_buf(),
                None => {
                    let (errors, warnings) = self.diagnostics_summary();
                    let root = self
                        .root_for_path(Path::new("."))
                        .map_or("(none)".to_string(), |r| r.display().to_string());
                    return format!(
                        "LSP workspace ({}): {} errors, {} warnings (cached; open a file for details)",
                        root, errors, warnings
                    );
                }
            },
        };

        // Determine which LSP client handles this file.
        let Some(client) = self.client_for_path(&path) else {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("unknown");
            return format!(
                "LSP: no server configured for .{} files",
                ext
            );
        };

        match client.status() {
            LspStatus::Running => {}
            LspStatus::Stopped => {
                let server_name = self.server_name_for_path(&path);
                let key = self.config_key_for_path(&path);
                return format!(
                    "LSP: server not running. Enable it with `{key} = on` in rustama.conf \
                    (or /workspace <dir>) — requires {server_name}."
                );
            }
            LspStatus::Failed(e) => return format!("LSP: server failed: {}", e),
            LspStatus::Starting => {} // rare; wait below handles it
        }

        if let Err(e) = client.sync_file(&path) {
            return format!("LSP: cannot sync {}: {}", path.display(), e);
        }

        let root = client.root().to_path_buf();
        let cancel = AtomicBool::new(false);

        // Pull diagnostics when the server supports it (rust-analyzer
        // does) — a direct answer beats waiting for a push. The server
        // *cancels* pulls invalidated by a fresh didChange ("server
        // cancelled the request"): retry briefly while it re-analyzes.
        let diags = if client.pull_supported() {
            let mut result = None;
            for _ in 0..10 {
                match client.pull_diagnostics(&path, timeout, &cancel) {
                    Ok(d) => {
                        result = Some(d);
                        break;
                    }
                    Err(e) if e.contains("cancelled") => {
                        std::thread::sleep(std::time::Duration::from_millis(300));
                    }
                    Err(e) => return format!("LSP: diagnostic request failed: {}", e),
                }
            }
            match result {
                Some(d) => d,
                None => {
                    return format!(
                        "LSP: diagnostics for {} not ready (server busy re-analyzing — try again)",
                        path.display()
                    );
                }
            }
        } else {
            // Push-only server: wait briefly for fresh analysis, then use
            // whatever is cached (possibly empty = clean or not ready).
            let since = self.diag_version(&path);
            let waited = std::time::Duration::from_secs(10).min(timeout);
            let v = self.wait_for_diagnostics(&path, since, waited, &cancel);
            if v == since && since == 0 {
                return format!(
                    "LSP: no diagnostics for {} yet (analysis still in progress — try again)",
                    path.display()
                );
            }
            self.diagnostics_for(&path)
        };

        let formatted = format_diagnostics(&diags, &path, &root, filter);
        if formatted.is_empty() {
            format!("LSP: no diagnostics for {} ✓", path.display())
        } else {
            format!(
                "LSP diagnostics for {}:\n{}",
                path.display(),
                formatted.trim_end()
            )
        }
    }

    /// Executes the LSP-flavored tools; `None` = not an LSP tool (the caller
    /// falls through to the stateless tool executor). `fallback_path` is the
    /// app's last-edited file, used by `lsp_diagnostics` when no path is
    /// given.
    pub fn execute_tool(
        &self,
        name: &str,
        args_json: &str,
        fallback_path: Option<&Path>,
    ) -> Option<String> {
        if name == "lsp_diagnostics" {
            let args: serde_json::Value =
                serde_json::from_str(args_json).unwrap_or(serde_json::json!({}));
            return Some(self.diagnostics_tool(
                args["path"].as_str(),
                args["severity"].as_str(),
                LSP_REQUEST_TIMEOUT,
                fallback_path,
            ));
        }
        if name == "lsp_definition" {
            let args: serde_json::Value =
                serde_json::from_str(args_json).unwrap_or(serde_json::json!({}));
            return Some(self.goto_definition_tool(
                args["path"].as_str(),
                args["line"].as_u64(),
                args["column"].as_u64(),
                LSP_REQUEST_TIMEOUT,
            ));
        }
        if name == "lsp_symbol_info" {
            let args: serde_json::Value =
                serde_json::from_str(args_json).unwrap_or(serde_json::json!({}));
            return Some(self.symbol_info_tool(
                args["path"].as_str(),
                args["line"].as_u64(),
                args["column"].as_u64(),
                LSP_REQUEST_TIMEOUT,
            ));
        }
        if name == "lsp_references" {
            let args: serde_json::Value =
                serde_json::from_str(args_json).unwrap_or(serde_json::json!({}));
            return Some(self.references_tool(
                args["path"].as_str(),
                args["line"].as_u64(),
                args["column"].as_u64(),
                args["include_declaration"].as_bool(),
                LSP_REQUEST_TIMEOUT,
            ));
        }
        if name == "lsp_completion" {
            let args: serde_json::Value =
                serde_json::from_str(args_json).unwrap_or(serde_json::json!({}));
            return Some(self.completion_tool(
                args["path"].as_str(),
                args["line"].as_u64(),
                args["column"].as_u64(),
                LSP_REQUEST_TIMEOUT,
            ));
        }
        None
    }

    /// Auto-feedback hook: after a successful edit_file/write_file tool
    /// call, append fresh diagnostics so the model sees its own type
    /// errors immediately. Gated by `enabled` (`lsp_auto_diagnostics`);
    /// no-op when the server is not running or the edit failed.
    pub fn append_edit_diagnostics(
        &self,
        enabled: bool,
        name: &str,
        args_json: &str,
        result: &mut String,
    ) {
        if !enabled || !matches!(name, "edit_file" | "write_file") || result.starts_with("Error")
        {
            return;
        }
        let args: serde_json::Value =
            serde_json::from_str(args_json).unwrap_or(serde_json::json!({}));
        let Some(path) = args["path"].as_str() else { return };
        let path = Path::new(path);
        let ext = path.extension().and_then(|e| e.to_str());
        if ext != Some("rs") && ext != Some("kt") && ext != Some("kts") {
            return;
        }
        let Some(client) = self.client_for_path(path) else {
            return;
        };
        if client.status() != LspStatus::Running {
            return;
        }
        // Fresh analysis takes a moment after didChange; give it a
        // bounded wait so the feedback reflects the edit, not the past.
        let diag = self.diagnostics_tool(
            Some(&path.to_string_lossy()),
            None,
            std::time::Duration::from_secs(15),
            Some(path),
        );
        result.push_str(&format!("\n\nDiagnostics: {}", diag));
    }
}

impl Default for LspManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for LspClient {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Extracts the diagnostic items from a (full) document diagnostic report.
fn extract_report_items(
    report: &lsp_types::DocumentDiagnosticReportResult,
) -> Vec<lsp_types::Diagnostic> {
    use lsp_types::{DocumentDiagnosticReport, DocumentDiagnosticReportResult};
    match report {
        DocumentDiagnosticReportResult::Report(DocumentDiagnosticReport::Full(full)) => {
            full.full_document_diagnostic_report.items.clone()
        }
        _ => Vec::new(),
    }
}

/// The reader thread: routes responses to pending waiters, stores pushed
/// diagnostics, and answers server→client requests with minimal replies so
/// rust-analyzer never blocks on us.
fn spawn_reader(
    mut stdout: Box<dyn BufRead + Send>,
    stdin: Arc<Mutex<Box<dyn Write + Send>>>,
    shared: Arc<Shared>,
    stop: Arc<AtomicBool>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        loop {
            if stop.load(Ordering::Relaxed) {
                return;
            }
            match Message::read(&mut stdout) {
                Ok(Some(Message::Response(resp))) => {
                    // Our ids are always numeric (see request_value).
                    let id: i64 = resp.id.to_string().parse().unwrap_or(-1);
                    if let Some(tx) = shared.pending.lock().unwrap().remove(&id) {
                        let _ = tx.send(resp);
                    }
                }
                Ok(Some(Message::Notification(notif))) => {
                    if notif.method == "textDocument/publishDiagnostics" {
                        if let Ok(params) =
                            serde_json::from_value::<lsp_types::PublishDiagnosticsParams>(
                                notif.params,
                            )
                        {
                            if let Some(path) = uri_to_path(params.uri.as_str()) {
                                shared.diags.lock().unwrap().publish(path, params.diagnostics);
                            }
                        }
                    }
                    // All other notifications (progress, logMessage, …) are
                    // informational only.
                }
                Ok(Some(Message::Request(req))) => {
                    // Server→client requests must always get a reply or the
                    // server may stall. Provide the minimal legal answers:
                    // - workspace/configuration → one null per requested item
                    // - everything else (workDoneProgress/create,
                    //   registerCapability, …) → null result
                    let result = if req.method == "workspace/configuration" {
                        let n = req
                            .params
                            .get("items")
                            .and_then(|i| i.as_array())
                            .map(|a| a.len())
                            .unwrap_or(0);
                        serde_json::Value::Array(vec![serde_json::Value::Null; n])
                    } else {
                        serde_json::Value::Null
                    };
                    let reply = Message::Response(Response::new_ok(req.id, result));
                    if let Ok(mut guard) = stdin.lock() {
                        let mut w: &mut dyn Write = &mut *guard;
                        let _ = reply.write(&mut w).and_then(|()| w.flush());
                    }
                }
                Ok(None) => {
                    // EOF: server exited.
                    *shared.status.lock().unwrap() = LspStatus::Stopped;
                    shared.pending.lock().unwrap().clear();
                    return;
                }
                Err(e) => {
                    *shared.status.lock().unwrap() =
                        LspStatus::Failed(format!("reader error: {}", e));
                    shared.pending.lock().unwrap().clear();
                    return;
                }
            }
        }
    })
}

// ── URI / formatting helpers ──────────────────────────────────────────

/// Absolutizes a path against the process cwd (no symlink resolution —
/// matches what `file://` URIs are built from, keeping map keys stable).
pub fn absolutize(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .unwrap_or_else(|_| path.to_path_buf())
    }
}

/// Percent-encodes an absolute path into a `file://` URI string.
/// Each path segment is encoded separately so `/` separators survive.
pub fn path_to_uri(path: &Path) -> Result<String, String> {
    let abs = absolutize(path);
    let s = abs.to_string_lossy();
    let mut out = String::from("file://");
    for (i, seg) in s.split('/').enumerate() {
        if i > 0 {
            out.push('/');
        }
        out.push_str(&urlencoding::encode(seg));
    }
    Ok(out)
}

/// Inverse of `path_to_uri`: decodes a `file://` URI into a path.
/// Returns `None` for non-file URIs.
pub fn uri_to_path(uri: &str) -> Option<PathBuf> {
    let rest = uri.strip_prefix("file://")?;
    let decoded = urlencoding::decode(rest).ok()?;
    Some(PathBuf::from(decoded.into_owned()))
}

/// LSP `languageId` for a path. Handles Rust, Kotlin, and common config files.
fn language_id_for(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("rs") => "rust",
        Some("kt") => "kotlin",
        Some("kts") => "kotlin",
        Some("toml") => "toml",
        _ => "rust",
    }
}

/// Shared arg validation for the position-based LSP tools
/// (`lsp_symbol_info`, `lsp_references`, `lsp_completion`): all need a
/// `path` plus 1-based `line`/`column`. Returns `Ok((path, line,
/// column))` or an error string describing the missing/invalid arg.
pub fn parse_position_args(
    tool: &str,
    path: Option<&str>,
    line: Option<u64>,
    column: Option<u64>,
) -> Result<(PathBuf, u64, u64), String> {
    let Some(path) = path else {
        return Err(format!(
            "{}: `path` is required (the file to search in) and \
            `line`/`column` (1-based) must point at the symbol.",
            tool
        ));
    };
    let Some(line) = line else {
        return Err(format!("{}: `line` (1-based) is required.", tool));
    };
    let Some(column) = column else {
        return Err(format!("{}: `column` (1-based) is required.", tool));
    };
    if line == 0 || column == 0 {
        return Err(format!("{}: `line` and `column` are 1-based (>= 1).", tool));
    }
    Ok((PathBuf::from(path), line, column))
}

/// Resolves the LSP workspace root: the configured `lsp_workspace` if set,
/// else the directory of `save_path`, else the current directory.
/// Missing `Cargo.toml` is not fatal — rust-analyzer still parses
/// single files (the caller only warns).
pub fn resolve_workspace(configured: &str, save_path: &str) -> PathBuf {
    if !configured.trim().is_empty() {
        return PathBuf::from(configured.trim());
    }
    let save_dir = std::path::Path::new(save_path)
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(PathBuf::from);
    match save_dir {
        Some(d) if d.is_dir() => d,
        _ => std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
    }
}

/// Detects whether the workspace looks like a Kotlin / Android Studio
/// project by checking for Gradle build files, AndroidManifest.xml, or
/// any `.kt`/`.kts` source files.
pub fn detect_kotlin_project(root: &Path) -> bool {
    let markers = [
        "build.gradle",
        "build.gradle.kts",
        "settings.gradle",
        "settings.gradle.kts",
        "AndroidManifest.xml",
    ];
    for marker in &markers {
        if root.join(marker).is_file() {
            return true;
        }
    }
    // Also check for any .kt/.kts source files (shallow scan of src/ and
    // the root).
    for entry in std::fs::read_dir(root).ok().into_iter().flatten() {
        if let Ok(entry) = entry {
            let path = entry.path();
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if ext == "kt" || ext == "kts" {
                    return true;
                }
            }
        }
    }
    false
}

/// Severity filter for the diagnostics tool / formatting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeverityFilter {
    All,
    Error,
    Warning,
    Information,
    Hint,
}

impl SeverityFilter {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "error" | "errors" => Some(Self::Error),
            "warning" | "warnings" => Some(Self::Warning),
            "info" | "information" => Some(Self::Information),
            "hint" => Some(Self::Hint),
            "all" => Some(Self::All),
            _ => None,
        }
    }

    fn matches(self, sev: Option<lsp_types::DiagnosticSeverity>) -> bool {
        match self {
            Self::All => true,
            Self::Error => sev == Some(lsp_types::DiagnosticSeverity::ERROR),
            Self::Warning => sev == Some(lsp_types::DiagnosticSeverity::WARNING),
            Self::Information => sev == Some(lsp_types::DiagnosticSeverity::INFORMATION),
            Self::Hint => sev == Some(lsp_types::DiagnosticSeverity::HINT),
        }
    }
}

fn severity_label(sev: Option<lsp_types::DiagnosticSeverity>) -> &'static str {
    match sev {
        Some(lsp_types::DiagnosticSeverity::ERROR) => "error",
        Some(lsp_types::DiagnosticSeverity::WARNING) => "warning",
        Some(lsp_types::DiagnosticSeverity::INFORMATION) => "info",
        Some(lsp_types::DiagnosticSeverity::HINT) => "hint",
        _ => "note",
    }
}

/// Formats diagnostics compactly, one per line:
/// `path:line:col: [severity] message` (1-based, LSP ranges are 0-based).
/// `root` is stripped from paths to keep lines short.
pub fn format_diagnostics(
    diags: &[lsp_types::Diagnostic],
    path: &Path,
    root: &Path,
    filter: SeverityFilter,
) -> String {
    let display_path = path
        .strip_prefix(root)
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| path.display().to_string());
    let mut out = String::new();
    for d in diags.iter().filter(|d| filter.matches(d.severity)) {
        let line = d.range.start.line + 1;
        let col = d.range.start.character + 1;
        let msg = d.message.lines().next().unwrap_or("");
        out.push_str(&format!(
            "{}:{}:{}: [{}] {}\n",
            display_path,
            line,
            col,
            severity_label(d.severity),
            msg
        ));
    }
    out
}

/// Formats definition locations compactly, one per line, with a source
/// excerpt so the model can see the surrounding context without a follow-up
/// `read_file`:
/// `path:line:col: <trimmed line text>` (1-based, LSP ranges are 0-based).
/// `root` is stripped from paths to keep lines short. Empty `locs` means
/// "no definition found".
pub fn format_locations(
    locs: &[lsp_types::Location],
    root: &Path,
) -> String {
    let mut out = String::new();
    for loc in locs {
        let Some(path) = uri_to_path(loc.uri.as_str()) else { continue };
        let display_path = path
            .strip_prefix(root)
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| path.display().to_string());
        let line = loc.range.start.line + 1;
        let col = loc.range.start.character + 1;
        // Trimmed one-line excerpt of the target line.
        let excerpt = std::fs::read_to_string(&path)
            .ok()
            .and_then(|text| text.lines().nth(loc.range.start.line as usize).map(|l| l.trim().to_string()))
            .filter(|l| !l.is_empty());
        match excerpt {
            Some(text) => out.push_str(&format!("{}:{}:{}: {}\n", display_path, line, col, text)),
            None => out.push_str(&format!("{}:{}:{}\n", display_path, line, col)),
        }
    }
    out
}

/// Formats the `contents` of an LSP `Hover` response into plain text,
/// collapsing the three possible shapes (markup, single marked string,
/// or an array of marked strings) into readable lines.
fn format_hover_contents(contents: &lsp_types::HoverContents) -> String {
    fn marked(ms: &lsp_types::MarkedString) -> String {
        match ms {
            lsp_types::MarkedString::String(s) => s.clone(),
            lsp_types::MarkedString::LanguageString(ls) => ls.value.clone(),
        }
    }
    match contents {
        lsp_types::HoverContents::Markup(m) => m.value.clone(),
        lsp_types::HoverContents::Scalar(ms) => marked(ms),
        lsp_types::HoverContents::Array(arr) => arr.iter().map(marked).collect::<Vec<_>>().join("\n"),
    }
}

/// Builds the `initialize` params: minimal capabilities + pull-diagnostic
/// support; workspace folders set to the root.
fn build_initialize_params(root_uri: String) -> Result<serde_json::Value, String> {
    let uri: lsp_types::Uri = root_uri
        .parse()
        .map_err(|e| format!("bad root URI: {:?}", e))?;
    let params = lsp_types::InitializeParams {
        process_id: Some(std::process::id()),
        #[allow(deprecated)] // rust-analyzer still honors root_uri; harmless duplicate of workspace_folders
        root_uri: Some(uri.clone()),
        capabilities: lsp_types::ClientCapabilities {
            text_document: Some(lsp_types::TextDocumentClientCapabilities {
                diagnostic: Some(lsp_types::DiagnosticClientCapabilities {
                    dynamic_registration: Some(false),
                    related_document_support: Some(false),
                }),
                publish_diagnostics: Some(lsp_types::PublishDiagnosticsClientCapabilities {
                    version_support: Some(false),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            window: Some(lsp_types::WindowClientCapabilities {
                work_done_progress: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        },
        workspace_folders: Some(vec![lsp_types::WorkspaceFolder {
            uri,
            name: "workspace".to_string(),
        }]),
        client_info: Some(lsp_types::ClientInfo {
            name: "rustama".to_string(),
            version: Some(env!("CARGO_PKG_VERSION").to_string()),
        }),
        ..Default::default()
    };
    serde_json::to_value(params).map_err(|e| e.to_string())
}

// ═══════════════════════════ tests ═══════════════════════════

#[cfg(test)]
mod lsp_tests {
    use super::*;

    // ── pure helpers ────────────────────────────────────────────────

    #[test]
    fn uri_roundtrip() {
        let path = PathBuf::from("/tmp/some dir/main.rs");
        let uri = path_to_uri(&path).unwrap();
        assert_eq!(uri, "file:///tmp/some%20dir/main.rs");
        assert_eq!(uri_to_path(&uri).unwrap(), path);
    }

    #[test]
    fn uri_rejects_non_file() {
        assert!(uri_to_path("https://example.com/x").is_none());
    }

    #[test]
    fn severity_parse_and_match() {
        assert_eq!(SeverityFilter::parse("ERROR"), Some(SeverityFilter::Error));
        assert_eq!(SeverityFilter::parse("warnings"), Some(SeverityFilter::Warning));
        assert!(SeverityFilter::parse("bogus").is_none());
        assert!(SeverityFilter::Error.matches(Some(lsp_types::DiagnosticSeverity::ERROR)));
        assert!(!SeverityFilter::Error.matches(Some(lsp_types::DiagnosticSeverity::WARNING)));
        assert!(SeverityFilter::All.matches(None));
    }

    fn fake_diag(line: u32, col: u32, sev: lsp_types::DiagnosticSeverity, msg: &str) -> lsp_types::Diagnostic {
        lsp_types::Diagnostic {
            range: lsp_types::Range {
                start: lsp_types::Position { line, character: col },
                end: lsp_types::Position { line, character: col + 1 },
            },
            severity: Some(sev),
            message: msg.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn formats_diagnostics_one_per_line() {
        let root = PathBuf::from("/ws");
        let path = root.join("src/main.rs");
        let diags = vec![
            fake_diag(0, 4, lsp_types::DiagnosticSeverity::ERROR, "mismatched types\nexpected i32"),
            fake_diag(9, 0, lsp_types::DiagnosticSeverity::WARNING, "unused variable"),
            fake_diag(2, 1, lsp_types::DiagnosticSeverity::HINT, "hint"),
        ];
        let out = format_diagnostics(&diags, &path, &root, SeverityFilter::All);
        assert_eq!(
            out,
            "src/main.rs:1:5: [error] mismatched types\nsrc/main.rs:10:1: [warning] unused variable\nsrc/main.rs:3:2: [hint] hint\n"
        );
        let errors = format_diagnostics(&diags, &path, &root, SeverityFilter::Error);
        assert_eq!(errors, "src/main.rs:1:5: [error] mismatched types\n");
    }

    #[test]
    fn formats_definition_locations_with_excerpt() {
        let root = std::env::temp_dir().join(format!("rustama-loc-{}", std::process::id()));
        let path = root.join("src/lib.rs");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            "pub fn foo() {}\npub fn bar() {}\nfn use_it() { foo(); }\n",
        )
        .unwrap();
        let uri = path_to_uri(&path).unwrap().parse().unwrap();
        let loc = lsp_types::Location::new(
            uri,
            lsp_types::Range {
                start: lsp_types::Position { line: 0, character: 4 },
                end: lsp_types::Position { line: 0, character: 7 },
            },
        );
        let out = format_locations(&[loc], &root);
        assert_eq!(out, "src/lib.rs:1:5: pub fn foo() {}\n");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn diag_store_versions_and_summary() {
        let mut store = DiagStore::default();
        let p = PathBuf::from("/ws/a.rs");
        assert_eq!(store.version(&p), 0);
        store.publish(p.clone(), vec![
            fake_diag(0, 0, lsp_types::DiagnosticSeverity::ERROR, "e"),
            fake_diag(1, 0, lsp_types::DiagnosticSeverity::WARNING, "w"),
        ]);
        assert_eq!(store.version(&p), 1);
        assert_eq!(store.summary(), (1, 1));
        store.publish(p.clone(), Vec::new());
        assert_eq!(store.version(&p), 2);
        assert_eq!(store.summary(), (0, 0));
    }

    // ── mock server (in-process thread over std::io::pipe) ──────────
    //
    // A thread speaking Content-Length JSON-RPC over a pair of OS pipes:
    // replies to `initialize` (advertising pull diagnostics) and
    // `textDocument/diagnostic` with canned items, pushes one
    // publishDiagnostics after didOpen, answers `shutdown`, and never
    // answers `test/neverReply` (timeout test). Hermetic — no real
    // rust-analyzer, no process re-exec, no libtest banner interleaving.

    fn mock_server_loop(mut reader: impl BufRead, mut writer: impl Write) {
        loop {
            let msg = match Message::read(&mut reader) {
                Ok(Some(m)) => m,
                _ => return, // EOF / garbage: client hung up
            };
            match msg {
                Message::Request(req) => {
                    if req.method == "test/neverReply" {
                        continue; // deliberately never answered (timeout test)
                    }
                    let resp = match req.method.as_str() {
                        "initialize" => Response::new_ok(
                            req.id,
                            serde_json::json!({
                                "capabilities": {
                                    "textDocumentSync": 1,
                                    "diagnosticProvider": {
                                        "interFileDependencies": false,
                                        "workspaceDiagnostics": false
                                    }
                                },
                                "serverInfo": { "name": "mock-lsp" }
                            }),
                        ),
                        "textDocument/diagnostic" => Response::new_ok(
                            req.id,
                            serde_json::json!({
                                "kind": "full",
                                "items": [{
                                    "range": {
                                        "start": {"line": 0, "character": 4},
                                        "end": {"line": 0, "character": 9}
                                    },
                                    "severity": 1,
                                    "message": "mismatched types"
                                }]
                            }),
                        ),
                        "textDocument/definition" => Response::new_ok(
                            req.id,
                            serde_json::json!([{
                                "uri": "file:///tmp/fake.rs",
                                "range": {
                                    "start": {"line": 5, "character": 2},
                                    "end": {"line": 5, "character": 6}
                                }
                            }]),
                        ),
                        "textDocument/hover" => Response::new_ok(
                            req.id,
                            serde_json::json!({
                                "contents": {
                                    "kind": "markdown",
                                    "value": "```rust\nfn foo(x: i32) -> i32\n```\n\nReturns `x` unchanged."
                                }
                            }),
                        ),
                        "textDocument/references" => Response::new_ok(
                            req.id,
                            serde_json::json!([
                                {
                                    "uri": "file:///tmp/fake.rs",
                                    "range": {
                                        "start": {"line": 0, "character": 0},
                                        "end": {"line": 0, "character": 3}
                                    }
                                },
                                {
                                    "uri": "file:///tmp/fake.rs",
                                    "range": {
                                        "start": {"line": 10, "character": 2},
                                        "end": {"line": 10, "character": 5}
                                    }
                                }
                            ]),
                        ),
                        "textDocument/completion" => Response::new_ok(
                            req.id,
                            serde_json::json!({
                                "isIncomplete": false,
                                "items": [
                                    { "label": "println!", "detail": "macro_rules" },
                                    { "label": "push" }
                                ]
                            }),
                        ),
                        "shutdown" => Response::new_ok(req.id, serde_json::Value::Null),
                        _ => Response::new_err(req.id, -32601, "method not found".to_string()),
                    };
                    Message::Response(resp).write(&mut writer).unwrap();
                    writer.flush().unwrap();
                }
                Message::Notification(notif) => {
                    if notif.method == "textDocument/didOpen" {
                        let uri = notif.params["textDocument"]["uri"].clone();
                        let push = Notification::new(
                            "textDocument/publishDiagnostics".to_string(),
                            serde_json::json!({
                                "uri": uri,
                                "diagnostics": [{
                                    "range": {
                                        "start": {"line": 2, "character": 0},
                                        "end": {"line": 2, "character": 3}
                                    },
                                    "severity": 2,
                                    "message": "unused import"
                                }]
                            }),
                        );
                        Message::Notification(push).write(&mut writer).unwrap();
                        writer.flush().unwrap();
                    }
                }
                Message::Response(_) => {}
            }
        }
    }

    /// Wires a client to a fresh in-process mock server.
    fn start_mock_client() -> LspClient {
        let (client_reader, server_writer) = std::io::pipe().unwrap();
        let (server_reader, client_writer) = std::io::pipe().unwrap();
        std::thread::spawn(move || {
            mock_server_loop(BufReader::new(server_reader), server_writer);
        });

        let mut client = LspClient::new();
        client.stop_reader = Arc::new(AtomicBool::new(false));
        let stdin: Arc<Mutex<Box<dyn Write + Send>>> =
            Arc::new(Mutex::new(Box::new(client_writer) as Box<dyn Write + Send>));
        client.reader = Some(spawn_reader(
            Box::new(BufReader::new(client_reader)) as Box<dyn BufRead + Send>,
            Arc::clone(&stdin),
            Arc::clone(&client.shared),
            Arc::clone(&client.stop_reader),
        ));
        client.stdin = Some(stdin);
        client
    }


    #[test]
    fn handshake_pull_diagnostics_and_push() {
        let mut client = start_mock_client();
        *client.shared.status.lock().unwrap() = LspStatus::Starting;

        // handshake
        let result = client
            .request_value(
                "initialize",
                build_initialize_params("file:///tmp".to_string()).unwrap(),
                Duration::from_secs(10),
                &AtomicBool::new(false),
            )
            .unwrap();
        let init: lsp_types::InitializeResult = serde_json::from_value(result).unwrap();
        assert!(init.capabilities.diagnostic_provider.is_some());
        client
            .shared
            .pull_supported
            .store(init.capabilities.diagnostic_provider.is_some(), Ordering::Relaxed);
        client.notify("initialized", serde_json::json!({})).unwrap();
        *client.shared.status.lock().unwrap() = LspStatus::Running;

        // pull diagnostics
        let path = PathBuf::from("/tmp/fake.rs");
        std::fs::write(&path, "fn main() {}\n").unwrap();
        let diags = client
            .pull_diagnostics(&path, Duration::from_secs(10), &AtomicBool::new(false))
            .unwrap();
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].message, "mismatched types");
        assert_eq!(diags[0].range.start.line, 0);

        // push diagnostics: didOpen triggers a publishDiagnostics
        client.sync_file(&path).unwrap();
        let v = client.wait_for_diagnostics(&path, 0, Duration::from_secs(10), &AtomicBool::new(false));
        assert!(v >= 1);
        let pushed = client.diagnostics_for(&path);
        assert_eq!(pushed.len(), 1);
        assert_eq!(pushed[0].message, "unused import");
        assert_eq!(pushed[0].severity, Some(lsp_types::DiagnosticSeverity::WARNING));

        // unknown method → error surfaces
        let err = client
            .request_value(
                "bogus/method",
                serde_json::Value::Null,
                Duration::from_secs(5),
                &AtomicBool::new(false),
            )
            .unwrap_err();
        assert!(err.contains("method not found"), "got: {}", err);

        // clean shutdown
        client.shutdown();
        assert_eq!(client.status(), LspStatus::Stopped);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn goto_definition_returns_locations() {
        let mut client = start_mock_client();
        *client.shared.status.lock().unwrap() = LspStatus::Running;
        // The mock replies to textDocument/definition with a Location[].
        let locs = client
            .goto_definition(
                Path::new("/tmp/fake.rs"),
                1,
                4,
                Duration::from_secs(10),
                &AtomicBool::new(false),
            )
            .unwrap();
        assert_eq!(locs.len(), 1);
        assert_eq!(locs[0].range.start.line, 5);
        assert_eq!(locs[0].range.start.character, 2);
        assert_eq!(uri_to_path(locs[0].uri.as_str()).unwrap(), PathBuf::from("/tmp/fake.rs"));
        client.shutdown();
    }

    #[test]
    fn hover_returns_rendered_text() {
        let mut client = start_mock_client();
        *client.shared.status.lock().unwrap() = LspStatus::Running;
        let text = client
            .hover(
                Path::new("/tmp/fake.rs"),
                0,
                4,
                Duration::from_secs(10),
                &AtomicBool::new(false),
            )
            .unwrap();
        assert!(
            text.contains("fn foo(x: i32) -> i32"),
            "got: {}",
            text
        );
        assert!(text.contains("Returns `x` unchanged"));
        client.shutdown();
    }

    #[test]
    fn references_returns_locations() {
        let mut client = start_mock_client();
        *client.shared.status.lock().unwrap() = LspStatus::Running;
        let locs = client
            .references(
                Path::new("/tmp/fake.rs"),
                1,
                4,
                true,
                Duration::from_secs(10),
                &AtomicBool::new(false),
            )
            .unwrap();
        assert_eq!(locs.len(), 2);
        assert_eq!(locs[0].range.start.line, 0);
        assert_eq!(locs[1].range.start.line, 10);
        client.shutdown();
    }

    #[test]
    fn completion_returns_labels_with_detail() {
        let mut client = start_mock_client();
        *client.shared.status.lock().unwrap() = LspStatus::Running;
        let items = client
            .completion(
                Path::new("/tmp/fake.rs"),
                1,
                4,
                Duration::from_secs(10),
                &AtomicBool::new(false),
            )
            .unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0], "println! — macro_rules");
        assert_eq!(items[1], "push");
        client.shutdown();
    }

    #[test]
    fn request_times_out_and_cancel_works() {
        let mut client = start_mock_client();
        *client.shared.status.lock().unwrap() = LspStatus::Running;

        // Timeout path: the mock never replies to "test/neverReply".
        let err = client
            .request_value(
                "test/neverReply",
                serde_json::Value::Null,
                Duration::from_millis(300),
                &AtomicBool::new(false),
            )
            .unwrap_err();
        assert!(err.contains("timed out"), "got: {}", err);

        // Cancel path: flag pre-set → immediate cancel error.
        let cancel = AtomicBool::new(true);
        let err = client
            .request_value(
                "textDocument/diagnostic",
                serde_json::json!({"textDocument": {"uri": "file:///tmp/x.rs"}}),
                Duration::from_secs(10),
                &cancel,
            )
            .unwrap_err();
        assert_eq!(err, "cancelled");
        client.shutdown();
    }

    #[test]
    fn request_on_stopped_client_errors() {
        let client = LspClient::new();
        let err = client
            .request_value(
                "initialize",
                serde_json::Value::Null,
                Duration::from_secs(1),
                &AtomicBool::new(false),
            )
            .unwrap_err();
        assert_eq!(err, "LSP server not running");
    }
}

/// End-to-end acid test against a *real* rust-analyzer (skipped in normal
/// runs): `cargo test --release lsp_real -- --ignored --nocapture`.
/// Spawns the server on a scratch cargo project, verifies a deliberate
/// type error is reported, then that fixing the file clears it.
#[cfg(test)]
#[ignore = "spawns real rust-analyzer (heavy, slow first load)"]
#[test]
fn lsp_real_rust_analyzer() {
    use super::*;

    let root = std::env::temp_dir().join(format!("rustama-lsp-acid-{}", std::process::id()));
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"acid\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    let main_rs = root.join("src/main.rs");
    std::fs::write(&main_rs, "fn main() {\n    let x: i32 = \"nope\";\n}\n").unwrap();

    let mut client = LspClient::new();
    client
        .start(
            root.clone(),
            LspConnect::Stdio {
                bin: "rust-analyzer".to_string(),
            },
        )
        .expect("rust-analyzer should start (is it installed?)");
    assert_eq!(client.status(), LspStatus::Running);
    assert!(client.pull_supported(), "rust-analyzer supports pull diagnostics");

    client.sync_file(&main_rs).unwrap();
    let cancel = AtomicBool::new(false);
    // First load runs `cargo metadata` + indexes the crate graph — pull
    // requests answered during that window come back empty, so retry
    // until the real analysis shows up.
    let deadline = std::time::Instant::now() + Duration::from_secs(90);
    let diags = loop {
        let diags = client
            .pull_diagnostics(&main_rs, Duration::from_secs(30), &cancel)
            .unwrap();
        if diags.iter().any(|d| d.severity == Some(lsp_types::DiagnosticSeverity::ERROR))
            || std::time::Instant::now() > deadline
        {
            break diags;
        }
        std::thread::sleep(Duration::from_secs(2));
    };
    assert!(
        diags.iter().any(|d| d.severity == Some(lsp_types::DiagnosticSeverity::ERROR)
            && d.message.contains("expected i32")),
        "expected an 'expected i32' error, got: {:?}",
        diags.iter().map(|d| &d.message).collect::<Vec<_>>()
    );

    // Fix the error → diagnostics clear.
    std::fs::write(&main_rs, "fn main() {\n    let x: i32 = 42;\n}\n").unwrap();
    client.sync_file(&main_rs).unwrap();
    let mut clean = false;
    for _ in 0..30 {
        // rust-analyzer cancels in-flight pulls when a didChange invalidates
        // them ("server cancelled the request") — treat it as "re-analyzing"
        // and retry.
        match client.pull_diagnostics(&main_rs, Duration::from_secs(30), &cancel) {
            Ok(diags) => {
                if diags
                    .iter()
                    .all(|d| d.severity != Some(lsp_types::DiagnosticSeverity::ERROR))
                {
                    clean = true;
                    break;
                }
            }
            Err(e) if e.contains("cancelled") => {}
            Err(e) => panic!("unexpected pull error: {}", e),
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    assert!(clean, "errors should clear after fixing the file");

    // lsp_definition: resolve `main` on its own line (position 1:1, 0-based
    // line 0 char 0) → should point back into main.rs.
    let mut defs = Vec::new();
    let deadline = std::time::Instant::now() + Duration::from_secs(60);
    while defs.is_empty() && std::time::Instant::now() < deadline {
        match client.goto_definition(&main_rs, 0, 0, Duration::from_secs(30), &cancel) {
            Ok(l) => defs = l,
            Err(e) if e.contains("cancelled") => {}
            Err(e) => panic!("unexpected goto_definition error: {}", e),
        }
        if defs.is_empty() {
            std::thread::sleep(Duration::from_millis(500));
        }
    }
    assert_eq!(defs.len(), 1, "expected one definition, got: {:?}", defs);
    assert_eq!(
        uri_to_path(defs[0].uri.as_str()).unwrap(),
        main_rs,
        "definition of main should be in main.rs"
    );

    // lsp_symbol_info: hover over `main` (the identifier at 0-based char 3 of
    // `fn main() {`) → should show the fn signature. NB: hovering the
    // `fn` keyword (char 0) yields keyword docs, not the signature.
    let mut hover = String::new();
    let deadline = std::time::Instant::now() + Duration::from_secs(60);
    while hover.trim().is_empty() && std::time::Instant::now() < deadline {
        match client.hover(&main_rs, 0, 4, Duration::from_secs(30), &cancel) {
            Ok(h) => hover = h,
            Err(e) if e.contains("cancelled") => {}
            Err(e) => panic!("unexpected hover error: {}", e),
        }
        if hover.trim().is_empty() {
            std::thread::sleep(Duration::from_millis(500));
        }
    }
    assert!(
        hover.contains("fn main()"),
        "expected hover to contain 'fn main()', got: {}",
        hover
    );

    // lsp_references: `main` in this scratch project is referenced once
    // (its own definition); rust-analyzer reports the declaration when
    // includeDeclaration is set. Position on the identifier, not the
    // `fn` keyword.
    let mut refs = Vec::new();
    let deadline = std::time::Instant::now() + Duration::from_secs(60);
    while refs.is_empty() && std::time::Instant::now() < deadline {
        match client.references(&main_rs, 0, 4, true, Duration::from_secs(30), &cancel) {
            Ok(r) => refs = r,
            Err(e) if e.contains("cancelled") => {}
            Err(e) => panic!("unexpected references error: {}", e),
        }
        if refs.is_empty() {
            std::thread::sleep(Duration::from_millis(500));
        }
    }
    assert!(!refs.is_empty(), "expected at least the declaration ref");

    // lsp_completion: complete at the end of `let x: i32 = ` (after the
    // fix, `42`) — asking for completions on an identifier position.
    // Position: end of line 2 (`    let x: i32 = 42;` → after "42").
    let line_text = "    let x: i32 = 42;";
    let char_pos = line_text.len() as u32;
    let deadline = std::time::Instant::now() + Duration::from_secs(60);
    let mut got_completion_reply = false;
    while !got_completion_reply && std::time::Instant::now() < deadline {
        match client.completion(&main_rs, 1, char_pos, Duration::from_secs(30), &cancel) {
            Ok(_) => got_completion_reply = true,
            Err(e) if e.contains("cancelled") => {}
            Err(e) => panic!("unexpected completion error: {}", e),
        }
        if !got_completion_reply {
            std::thread::sleep(Duration::from_millis(500));
        }
    }
    // Empty is acceptable in this edge position, but a real rust-analyzer
    // normally offers something (keywords like `if`, method calls, …).
    // Only assert the server answered; the hermetic mock covers content.
    assert!(got_completion_reply, "expected a completion reply from rust-analyzer");

    client.shutdown();
    assert_eq!(client.status(), LspStatus::Stopped);
    let _ = std::fs::remove_dir_all(&root);
}
