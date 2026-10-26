//! Stateless tool implementations and tool-call parsing.
//!
//! Every tool named in [`TOOL_NAMES`] is executed by
//! [`execute_tool_call`]: a thin `match` that parses the arguments once
//! and dispatches to one function per tool. The functions themselves
//! carry all the logic (and each tool's own result-size cap where it
//! needs one), so adding a tool is: write the function, add a match arm,
//! add the name to [`TOOL_NAMES`] and to the JSON schemas in
//! `app.rs::get_tool_definitions`.
//!
//! Stateful tools (terminal PTY, LSP) do not live here — they need
//! `App` state and are dispatched by `App::execute_tool` *before* this
//! stateless dispatcher is consulted.
//!
//! Every tool that takes a `path` argument accepts shell-style tilde
//! notation: [`expand_tilde`] resolves a leading `~`/`~user` to a home
//! directory before the path reaches the filesystem (or, for the LSP
//! tools, via the same rewrite in `App::execute_tool`).

use crate::app::safe_prefix;

/// Hard cap on one tool result's size. Guards the API request payload:
/// a single oversized tool result (e.g. `bash` dumping a huge file)
/// would otherwise push the request past the payload limit. Tools that
/// can naturally produce huge output (search_content, fetch_url) cap
/// themselves at the source; this is the last-resort net applied to
/// every tool result before it is sent.
pub(crate) const MAX_TOOL_RESULT_BYTES: usize = 50_000;

/// Truncates a tool result to [`MAX_TOOL_RESULT_BYTES`] bytes, keeping
/// the head and appending a marker with the original size.
pub(crate) fn cap_tool_result(content: &str) -> String {
    if content.len() > MAX_TOOL_RESULT_BYTES {
        format!(
            "{}... [truncated, {} bytes]",
            safe_prefix(content, MAX_TOOL_RESULT_BYTES),
            content.len()
        )
    } else {
        content.to_string()
    }
}

/// Unified stateless tool dispatcher: parses `args_json` once and calls
/// the tool's own function. Terminal and LSP tools are handled earlier
/// by `App::execute_tool`; an unknown name yields an error listing the
/// same names the parser/dispatcher accept.
pub(crate) fn execute_tool_call(name: &str, args_json: &str, proxy: &Option<String>) -> String {
    let args: serde_json::Value = serde_json::from_str(args_json).unwrap_or(serde_json::json!({}));

    match name {
        "read_file" => read_file(&args),
        "write_file" => write_file(&args),
        "edit_file" => edit_file(&args),
        "bash" => bash(&args),
        "list_files" => list_files(&args),
        "search_files" => search_files(&args),
        "search_content" => search_content(&args),
        "fetch_url" => fetch_url(&args, proxy),
        "web_search" => web_search(&args, proxy),
        _ => {
            // Single source of truth: the unknown-tool error lists the
            // same names the parser/dispatcher accept.
            format!(
                "Unknown tool: '{}'. Available tools: {}",
                name,
                TOOL_NAMES.join(", ")
            )
        }
    }
}

/// Expands a leading `~` in a tool path argument to an absolute path.
/// Neither `std::fs` nor the `glob` crate understand shell-style tilde
/// notation, so a model-supplied `~/git/Rustama` would otherwise fail
/// with "No such file or directory".
///
///   * `~`          → the user's home directory (`$HOME`)
///   * `~/rest`     → `$HOME/rest`
///   * `~user/rest` → that user's home directory (from `/etc/passwd`)
///
/// Anything else — a `~` that is not the leading component, or a
/// `~user` that cannot be resolved — is returned unchanged, so the tool
/// reports the underlying filesystem error as before.
pub(crate) fn expand_tilde(path: &str) -> String {
    if !path.starts_with('~') {
        return path.to_string();
    }
    if path == "~" {
        return home_dir().unwrap_or_else(|| path.to_string());
    }
    if let Some(rest) = path.strip_prefix("~/") {
        return match home_dir() {
            Some(home) => format!("{}/{}", home, rest),
            None => path.to_string(),
        };
    }
    // `~user[/rest]`: resolve via the passwd database (unix only).
    #[cfg(unix)]
    {
        let (user, rest) = match path[1..].split_once('/') {
            Some((user, rest)) => (user, rest),
            None => (&path[1..], ""),
        };
        if !user.is_empty()
            && let Some(user_home) = passwd_home(user)
        {
            return if rest.is_empty() {
                user_home
            } else {
                format!("{}/{}", user_home, rest)
            };
        }
    }
    path.to_string()
}

/// The user's home directory, or `None` when `$HOME` is unset/empty.
fn home_dir() -> Option<String> {
    std::env::var("HOME").ok().filter(|h| !h.is_empty())
}

/// Looks up `user`'s home directory in `/etc/passwd`. Lines are
/// `name:passwd:uid:gid:gecos:home:shell`, so the home directory is
/// field 6 (0-based index 5) of the first line naming `user`.
#[cfg(unix)]
fn passwd_home(user: &str) -> Option<String> {
    let passwd = std::fs::read_to_string("/etc/passwd").ok()?;
    passwd.lines().find_map(|line| {
        let fields: Vec<&str> = line.split(':').collect();
        if fields.first() == Some(&user) && fields.len() > 5 {
            Some(fields[5].to_string())
        } else {
            None
        }
    })
}

/// `read_file`: whole file, or a 1-based `offset`/`limit` window with a
/// "showing lines X-Y of Z" footer.
fn read_file(args: &serde_json::Value) -> String {
    let path = expand_tilde(args["path"].as_str().unwrap_or(""));
    // 1-based first line; default: start of file.
    let offset = args["offset"].as_u64().unwrap_or(1).max(1) as usize;
    let limit = args["limit"].as_u64().map(|n| n as usize);
    match std::fs::read_to_string(&path) {
        Ok(content) => {
            let total_lines = content.lines().count();
            if offset == 1 && limit.is_none() {
                return content;
            }
            let window: Vec<&str> = match limit {
                Some(n) => content.lines().skip(offset - 1).take(n).collect(),
                None => content.lines().skip(offset - 1).collect(),
            };
            if window.is_empty() {
                return format!(
                    "(no lines in range: file has {} lines, offset {})",
                    total_lines, offset
                );
            }
            let first = offset;
            let last = offset + window.len() - 1;
            let mut out = window.join("\n");
            if last < total_lines {
                out.push_str(&format!(
                    "\n[... {} more lines — showing lines {}-{} of {}. Use offset/limit to read more.]",
                    total_lines - last, first, last, total_lines
                ));
            } else {
                out.push_str(&format!(
                    "\n[showing lines {}-{} of {}]",
                    first, last, total_lines
                ));
            }
            out
        }
        Err(e) => format!("Error reading file: {}", e),
    }
}

/// `write_file`: create or overwrite a file.
fn write_file(args: &serde_json::Value) -> String {
    let path = expand_tilde(args["path"].as_str().unwrap_or(""));
    let content = args["content"].as_str().unwrap_or("");
    match std::fs::write(&path, content) {
        Ok(()) => format!("Successfully wrote {} bytes to {}", content.len(), path),
        Err(e) => format!("Error writing file: {}", e),
    }
}

/// `edit_file`: replace the first occurrence of `old_string` and return
/// a unified diff of the change.
fn edit_file(args: &serde_json::Value) -> String {
    let path = expand_tilde(args["path"].as_str().unwrap_or(""));
    let old = args["old_string"].as_str().unwrap_or("");
    let new = args["new_string"].as_str().unwrap_or("");
    match std::fs::read_to_string(&path) {
        Ok(content) => {
            if let Some(pos) = content.find(old) {
                let mut new_content = content.clone();
                new_content.replace_range(pos..pos + old.len(), new);
                match std::fs::write(&path, &new_content) {
                    Ok(()) => {
                        let diff = unified_diff(&content, &new_content, &path, 3);
                        format!("Successfully edited {}\n{}", path, diff)
                    }
                    Err(e) => format!("Error writing file: {}", e),
                }
            } else {
                format!("Error: old_string not found in {}", path)
            }
        }
        Err(e) => format!("Error reading file: {}", e),
    }
}

/// `bash`: run a command via `sh -c` with a 30s timeout. `sudo` is
/// rejected outright.
fn bash(args: &serde_json::Value) -> String {
    let command = args["command"].as_str().unwrap_or("");
    let trimmed = command.trim_start();
    if trimmed.starts_with("sudo ") || trimmed == "sudo" {
        return "Error: sudo is not permitted. You do not have elevated privileges and cannot run commands as root.".to_string();
    }
    use std::io::Read;
    use std::process::Stdio;
    match std::process::Command::new("sh")
        .arg("-c")
        .arg(command)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null())
        .spawn()
    {
        Ok(mut child) => {
            let start = std::time::Instant::now();
            let timeout = std::time::Duration::from_secs(30);
            loop {
                match child.try_wait() {
                    Ok(Some(status)) => {
                        let mut stdout = String::new();
                        let mut stderr = String::new();
                        if let Some(ref mut out) = child.stdout {
                            let _ = out.read_to_string(&mut stdout);
                        }
                        if let Some(ref mut err) = child.stderr {
                            let _ = err.read_to_string(&mut stderr);
                        }
                        let mut result = String::new();
                        if !stdout.is_empty() {
                            result.push_str(&stdout);
                        }
                        if !stderr.is_empty() {
                            if !result.is_empty() {
                                result.push('\n');
                            }
                            result.push_str(&stderr);
                        }
                        if result.is_empty() {
                            result = format!("(exit {})", status);
                        }
                        return result;
                    }
                    Ok(None) => {
                        if start.elapsed() > timeout {
                            let _ = child.kill();
                            let _ = child.wait();
                            return "(command timed out after 30s)".to_string();
                        }
                        std::thread::sleep(std::time::Duration::from_millis(50));
                    }
                    Err(e) => return format!("Error: {}", e),
                }
            }
        }
        Err(e) => format!("Error executing command: {}", e),
    }
}

/// `list_files`: directory entries, `dir/` suffixed, sorted.
fn list_files(args: &serde_json::Value) -> String {
    let path = expand_tilde(args["path"].as_str().unwrap_or("."));
    match std::fs::read_dir(path) {
        Ok(entries) => {
            let mut files: Vec<String> = entries
                .filter_map(|e| e.ok())
                .map(|e| {
                    let name = e.file_name().to_string_lossy().to_string();
                    let is_dir = e.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
                    if is_dir { format!("{}/", name) } else { name }
                })
                .collect();
            files.sort();
            files.join("\n")
        }
        Err(e) => format!("Error listing directory: {}", e),
    }
}

/// `search_files`: glob pattern → matching paths.
fn search_files(args: &serde_json::Value) -> String {
    let pattern = args["pattern"].as_str().unwrap_or("*");
    let path = expand_tilde(args["path"].as_str().unwrap_or("."));
    let full_pattern = format!("{}/{}", path, pattern);
    match glob::glob(&full_pattern) {
        Ok(paths) => {
            let results: Vec<String> = paths
                .filter_map(|p| p.ok())
                .map(|p| p.display().to_string())
                .collect();
            if results.is_empty() {
                "No files found".to_string()
            } else {
                results.join("\n")
            }
        }
        Err(e) => format!("Error with glob pattern: {}", e),
    }
}

/// `search_content`: regex over files matching the `include` glob.
///
/// The result is capped **right here, right after `grep_regex`
/// returns** — the source of the potentially huge output — so every
/// consumer (agentic loop, session, API request) sees the same bounded
/// string. `grep_regex` already bounds the match list; this caps the
/// final joined result including its newline separators.
fn search_content(args: &serde_json::Value) -> String {
    let pattern = args["pattern"].as_str().unwrap_or("");
    let path = expand_tilde(args["path"].as_str().unwrap_or("."));
    let include = args["include"].as_str().unwrap_or("*");

    match grep_regex(pattern, &path, include) {
        Ok(results) => {
            if results.is_empty() {
                "No matches found".to_string()
            } else {
                // Cap immediately after the grep — this is where the
                // size can explode (huge log lines), so this is where
                // it must be bounded.
                cap_tool_result(&results.join("\n"))
            }
        }
        Err(e) => format!("Error searching: {}", e),
    }
}

/// `fetch_url`: GET a URL, capping the response body at 50k bytes.
fn fetch_url(args: &serde_json::Value, proxy: &Option<String>) -> String {
    let url = args["url"].as_str().unwrap_or("");
    if url.is_empty() {
        "Error: no URL provided".to_string()
    } else {
        let client = build_blocking_client(proxy);
        match client.get(url).send() {
            Ok(resp) => {
                let status = resp.status();
                if !status.is_success() {
                    format!("HTTP error: {}", status)
                } else {
                    match resp.text() {
                        Ok(text) => {
                            if text.len() > MAX_TOOL_RESULT_BYTES {
                                let prefix = safe_prefix(&text, MAX_TOOL_RESULT_BYTES);
                                format!(
                                    "{}...[truncated, total {} bytes]",
                                    prefix,
                                    text.len()
                                )
                            } else {
                                text
                            }
                        }
                        Err(e) => format!("Error reading response: {}", e),
                    }
                }
            }
            Err(e) => format!("Error fetching URL: {}", e),
        }
    }
}

/// `web_search`: scrape DuckDuckGo's HTML result page for snippets/links.
fn web_search(args: &serde_json::Value, proxy: &Option<String>) -> String {
    let query = args["query"].as_str().unwrap_or("");
    if query.is_empty() {
        "Error: no search query provided".to_string()
    } else {
        let search_url = format!(
            "https://html.duckduckgo.com/html/?q={}",
            urlencoding::encode(query)
        );
        let client = build_blocking_client(proxy);
        match client.get(&*search_url).send() {
            Ok(resp) => match resp.text() {
                Ok(html) => {
                    let mut results = Vec::new();
                    for line in html.lines() {
                        if line.contains("result__snippet") || line.contains("result__a") {
                            let cleaned = line
                                .replace(
                                    "<a rel=\"nofollow\" class=\"result__a\" href=\"",
                                    "",
                                )
                                .replace("<a class=\"result__snippet\" href=\"", "")
                                .replace("</a>", "")
                                .replace("<span class=\"result__snippet\">", "")
                                .replace("</span>", "")
                                .replace("<b>", "")
                                .replace("</b>", "")
                                .trim()
                                .to_string();
                            if !cleaned.is_empty() && cleaned.len() > 5 {
                                results.push(cleaned);
                            }
                        }
                    }
                    if results.is_empty() {
                        "No results found".to_string()
                    } else {
                        results.join("\n")
                    }
                }
                Err(e) => format!("Error reading search results: {}", e),
            },
            Err(e) => format!("Error performing search: {}", e),
        }
    }
}

/// Regex search over the files matching `path/include`, returning
/// `file:line: match` entries. Bounded three ways so a single call can
/// never explode: known app logs are excluded entirely, single matching
/// lines are truncated at 2000 bytes, and the total result is capped at
/// [`MAX_TOOL_RESULT_BYTES`].
fn grep_regex(pattern: &str, path: &str, include: &str) -> Result<Vec<String>, String> {
    let re = regex::Regex::new(pattern).map_err(|e| format!("Invalid regex: {}", e))?;
    let glob_pattern = format!("{}/{}", path, include);

    // Files we never want to search: app logs contain REQUEST/RESPONSE
    // payloads (including full tool schemas) and can be hundreds of MB.
    // Searching them returns huge lines that bloat tool results and can
    // push API request payloads over the size limit.
    const EXCLUDED_NAMES: &[&str] = &["rustama.log", "rustama-lsp.log"];

    let mut results = Vec::new();
    let mut total_bytes = 0usize;

    if let Ok(entries) = glob::glob(&glob_pattern) {
        for entry in entries.flatten() {
            let name = entry
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            if EXCLUDED_NAMES.iter().any(|e| name.starts_with(e)) {
                continue;
            }
            if entry.is_file() && let Ok(content) = std::fs::read_to_string(&entry) {
                for (line_num, line) in content.lines().enumerate() {
                    if re.is_match(line) {
                        let entry_line = format!("{}:{}: {}", entry.display(), line_num + 1, line);
                        // Truncate overly long lines (e.g. log lines that embed
                        // entire JSON payloads on one line).
                        let entry_line = if entry_line.len() > 2000 {
                            format!("{}... [truncated, {} bytes]", &entry_line[..2000], entry_line.len())
                        } else {
                            entry_line
                        };
                        if total_bytes + entry_line.len() > MAX_TOOL_RESULT_BYTES {
                            results.push(format!(
                                "...[search results truncated at {} bytes total]",
                                MAX_TOOL_RESULT_BYTES
                            ));
                            return Ok(results);
                        }
                        total_bytes += entry_line.len();
                        results.push(entry_line);
                    }
                }
            }
        }
    }
    Ok(results)
}

pub(crate) const TOOL_NAMES: &[&str] = &[
    "read_file",
    "write_file",
    "edit_file",
    "bash",
    "list_files",
    "search_files",
    "search_content",
    "fetch_url",
    "web_search",
    "terminal_open",
    "terminal_send",
    "terminal_read",
    "terminal_close",
    "lsp_diagnostics",
    "lsp_definition",
    "lsp_symbol_info",
    "lsp_references",
    "lsp_completion",
];

/// Parses tool calls the model embedded as plain text (fallback for
/// models that emit JSON blobs instead of using the tools API).
/// Recognised shapes:
///
///   * `{"name": "...", "parameters": {...}}` / `{"name": "...", "arguments": {...}}`
///   * `name: {...}` (a leading identifier followed by a JSON object)
///
/// Only calls naming a known tool survive.
pub(crate) fn parse_text_tool_calls(text: &str) -> Vec<serde_json::Value> {
    let mut tool_calls = Vec::new();

    let patterns = [
        (
            r#"\{"name"\s*:\s*"(\w+)"\s*,\s*"parameters"\s*:\s*(\{[^}]*\})\}"#,
            "name",
            "parameters",
        ),
        (
            r#"\{"name"\s*:\s*"(\w+)"\s*,\s*"arguments"\s*:\s*(\{[^}]*\})\}"#,
            "name",
            "arguments",
        ),
    ];

    for (regex_pattern, _name_key, _args_key) in &patterns {
        if let Ok(re) = regex::Regex::new(regex_pattern) {
            for cap in re.captures_iter(text) {
                if let (Some(name_match), Some(args_match)) = (cap.get(1), cap.get(2)) {
                    let name = name_match.as_str().to_string();
                    let args_str = args_match.as_str().to_string();

                    if TOOL_NAMES.contains(&name.as_str()) {
                        let mut tc = serde_json::json!({
                            "name": name,
                        });
                        tc["parameters"] =
                            serde_json::from_str(&args_str).unwrap_or(serde_json::json!({}));
                        tool_calls.push(tc);
                    }
                }
            }
        }
    }

    if !tool_calls.is_empty() {
        return tool_calls;
    }

    if let Ok(re) = regex::Regex::new(r#"(\w+):\s*(\{.*\})"#) {
        for cap in re.captures_iter(text) {
            if let (Some(name_match), Some(args_match)) = (cap.get(1), cap.get(2)) {
                let name = name_match.as_str().to_string();
                let args_str = args_match.as_str().to_string();

                if TOOL_NAMES.contains(&name.as_str())
                    && let Ok(args_json) = serde_json::from_str::<serde_json::Value>(&args_str)
                {
                    let tc = serde_json::json!({
                        "name": name,
                        "parameters": args_json,
                    });
                    tool_calls.push(tc);
                }
            }
        }
    }

    tool_calls
}

/// Blocking (thread-side) HTTP client honouring the configured proxy.
fn build_blocking_client(proxy: &Option<String>) -> reqwest::blocking::Client {
    let mut builder = reqwest::blocking::Client::builder();
    if let Some(proxy_url) = proxy
        && let Ok(proxy) = reqwest::Proxy::all(proxy_url)
    {
        builder = builder.proxy(proxy);
    }
    builder
        .build()
        .unwrap_or_else(|_| reqwest::blocking::Client::new())
}

/// Minimal unified diff (`diff -u`-shaped) between two file contents,
/// with `context` lines around each change hunk. Used by the
/// `edit_file` tool result so the model sees exactly what changed.
pub(crate) fn unified_diff(old: &str, new: &str, path: &str, context: usize) -> String {
    #[derive(Debug, Clone, Copy, PartialEq)]
    enum Op {
        Keep(usize, usize),
        Del(usize),
        Add(usize),
    }

    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();

    let n = old_lines.len();
    let m = new_lines.len();
    // LCS table (O(n*m) — fine for file-sized inputs).
    let mut dp = vec![vec![0usize; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            dp[i][j] = if old_lines[i] == new_lines[j] {
                dp[i + 1][j + 1] + 1
            } else {
                dp[i + 1][j].max(dp[i][j + 1])
            };
        }
    }

    let mut ops: Vec<Op> = Vec::new();
    let (mut i, mut j) = (0usize, 0usize);
    while i < n && j < m {
        if old_lines[i] == new_lines[j] {
            ops.push(Op::Keep(i, j));
            i += 1;
            j += 1;
        } else if dp[i + 1][j] >= dp[i][j + 1] {
            ops.push(Op::Del(i));
            i += 1;
        } else {
            ops.push(Op::Add(j));
            j += 1;
        }
    }
    while i < n {
        ops.push(Op::Del(i));
        i += 1;
    }
    while j < m {
        ops.push(Op::Add(j));
        j += 1;
    }

    if ops.iter().all(|o| matches!(o, Op::Keep(..))) {
        return String::new();
    }

    // Keep only hunks: an op survives when a change is within `context`.
    let keep_flags: Vec<bool> = ops
        .iter()
        .enumerate()
        .map(|(idx, op)| {
            if !matches!(op, Op::Keep(..)) {
                return true;
            }
            let near_before = (idx.saturating_sub(context)..idx)
                .any(|k| !matches!(ops[k], Op::Keep(..)));
            let near_after = (idx + 1..ops.len())
                .take(context)
                .any(|k| !matches!(ops[k], Op::Keep(..)));
            near_before || near_after
        })
        .collect();

    let mut out = format!("--- {}\n+++ {}\n", path, path);
    let mut idx = 0;
    while idx < ops.len() {
        if !keep_flags[idx] {
            idx += 1;
            continue;
        }
        // Collect one hunk (consecutive flagged ops).
        let start = idx;
        let mut end = idx;
        while end < ops.len() && keep_flags[end] {
            end += 1;
        }
        let hunk = &ops[start..end];
        let old_start = hunk
            .iter()
            .find_map(|o| match o {
                Op::Keep(oi, _) | Op::Del(oi) => Some(*oi),
                Op::Add(_) => None,
            })
            .map(|oi| oi + 1)
            .unwrap_or(1);
        let new_start = hunk
            .iter()
            .find_map(|o| match o {
                Op::Keep(_, ni) | Op::Add(ni) => Some(*ni),
                Op::Del(_) => None,
            })
            .map(|ni| ni + 1)
            .unwrap_or(1);
        let old_count = hunk
            .iter()
            .filter(|o| matches!(o, Op::Keep(..) | Op::Del(_)))
            .count();
        let new_count = hunk
            .iter()
            .filter(|o| matches!(o, Op::Keep(..) | Op::Add(_)))
            .count();
        out.push_str(&format!(
            "@@ -{},{} +{},{} @@\n",
            old_start, old_count, new_start, new_count
        ));
        for op in hunk {
            match op {
                Op::Keep(oi, _) => out.push_str(&format!(" {}\n", old_lines[*oi])),
                Op::Del(oi) => out.push_str(&format!("-{}\n", old_lines[*oi])),
                Op::Add(ni) => out.push_str(&format!("+{}\n", new_lines[*ni])),
            }
        }
        idx = end;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_file_offset_limit() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("rustama-readfile-test-{}.txt", std::process::id()));
        let content = (1..=10).map(|i| format!("line{}", i)).collect::<Vec<_>>().join("\n");
        std::fs::write(&path, &content).unwrap();
        let p = path.to_string_lossy().to_string();

        // Whole file when no offset/limit given
        let out = execute_tool_call("read_file", &serde_json::json!({"path": p}).to_string(), &None);
        assert_eq!(out, content);

        // limit only
        let out = execute_tool_call(
            "read_file",
            &serde_json::json!({"path": p, "limit": 3}).to_string(),
            &None,
        );
        assert!(out.starts_with("line1\nline2\nline3"), "got: {}", out);
        assert!(out.contains("showing lines 1-3 of 10"), "got: {}", out);
        assert!(out.contains("7 more lines"), "got: {}", out);

        // offset + limit window
        let out = execute_tool_call(
            "read_file",
            &serde_json::json!({"path": p, "offset": 4, "limit": 2}).to_string(),
            &None,
        );
        assert!(out.starts_with("line4\nline5"), "got: {}", out);
        assert!(out.contains("showing lines 4-5 of 10"), "got: {}", out);

        // offset past end of file
        let out = execute_tool_call(
            "read_file",
            &serde_json::json!({"path": p, "offset": 99}).to_string(),
            &None,
        );
        assert!(out.contains("no lines in range"), "got: {}", out);

        // window reaching EOF
        let out = execute_tool_call(
            "read_file",
            &serde_json::json!({"path": p, "offset": 9, "limit": 50}).to_string(),
            &None,
        );
        assert!(out.starts_with("line9\nline10"), "got: {}", out);
        assert!(out.contains("showing lines 9-10 of 10"), "got: {}", out);
        assert!(!out.contains("more lines"), "got: {}", out);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn unified_diff_single_change_with_context() {
        let old = "one\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\n";
        let new = "one\ntwo\nthree\nfour\nFIVE\nsix\nseven\neight\nnine\nten\n";
        let diff = unified_diff(old, new, "f.txt", 3);
        assert!(diff.starts_with("--- f.txt\n+++ f.txt\n"), "header: {}", diff);
        // Change at line 5, ±3 context → hunk spans lines 2..8.
        assert!(diff.contains("@@ -2,7 +2,7 @@"), "hunk header: {}", diff);
        assert!(diff.contains(" two\n"), "context line: {}", diff);
        assert!(diff.contains(" eight\n"), "context line: {}", diff);
        assert!(diff.contains("-five\n"), "removed line: {}", diff);
        assert!(diff.contains("+FIVE\n"), "added line: {}", diff);
        // Context limits the hunk: lines 1, 9, 10 are outside ±3.
        assert!(!diff.contains(" one\n"), "outside context: {}", diff);
        assert!(!diff.contains(" nine\n"), "outside context: {}", diff);
        assert!(!diff.contains(" ten\n"), "outside context: {}", diff);
    }

    #[test]
    fn unified_diff_context_covers_whole_small_file() {
        // A change in the middle of a 7-line file with context=3
        // reaches both edges — one full-file hunk, like `diff -u -U3`.
        let old = "one\ntwo\nthree\nfour\nfive\nsix\nseven\n";
        let new = "one\ntwo\nthree\nFOUR\nfive\nsix\nseven\n";
        let diff = unified_diff(old, new, "f.txt", 3);
        assert!(diff.contains("@@ -1,7 +1,7 @@"), "hunk header: {}", diff);
        assert!(diff.contains("-four\n"), "removed: {}", diff);
        assert!(diff.contains("+FOUR\n"), "added: {}", diff);
    }

    #[test]
    fn unified_diff_insert_and_delete() {
        // Pure insertion at the top and deletion at the bottom.
        let old = "a\nb\nc\nd\ne\nf\ng\nh\ni\nj\n";
        let new = "NEW\na\nb\nc\nd\ne\nf\ng\nh\ni\n";
        let diff = unified_diff(old, new, "f.txt", 1);
        assert!(diff.contains("+NEW\n"), "insertion: {}", diff);
        assert!(diff.contains("-j\n"), "deletion: {}", diff);
        // Two separate hunks (change at line 1 and line 10, context 1).
        assert_eq!(diff.matches("@@ ").count(), 2, "two hunks: {}", diff);
    }

    #[test]
    fn unified_diff_no_change_has_no_hunks() {
        let diff = unified_diff("x\ny\n", "x\ny\n", "f.txt", 3);
        assert!(!diff.contains("@@"), "no hunks: {}", diff);
    }

    #[test]
    fn edit_file_result_contains_diff() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("rustama-editfile-test-{}.txt", std::process::id()));
        std::fs::write(&path, "alpha\nbeta\ngamma\n").unwrap();
        let p = path.to_string_lossy().to_string();
        let out = execute_tool_call(
            "edit_file",
            &serde_json::json!({"path": p, "old_string": "beta", "new_string": "BETA"})
                .to_string(),
            &None,
        );
        assert!(out.starts_with("Successfully edited"), "got: {}", out);
        assert!(out.contains("--- "), "diff header: {}", out);
        assert!(out.contains("@@ "), "hunk header: {}", out);
        assert!(out.contains("-beta\n"), "removed line: {}", out);
        assert!(out.contains("+BETA\n"), "added line: {}", out);
        assert!(out.contains(" alpha\n"), "context line: {}", out);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn unknown_tool_lists_known_names() {
        let out = execute_tool_call("definitely_not_a_tool", "{}", &None);
        assert!(out.starts_with("Unknown tool: 'definitely_not_a_tool'"), "got: {}", out);
        assert!(out.contains("search_content"), "lists tools: {}", out);
    }

    #[test]
    fn search_content_caps_result_right_after_grep() {
        // A file whose matching lines alone exceed the cap: the result
        // must come back bounded (capped at the grep call site), not
        // megabytes long.
        let dir = std::env::temp_dir();
        let path = dir.join(format!("rustama-searchcap-test-{}.txt", std::process::id()));
        let big_line = "x".repeat(2000);
        let content = (0..100)
            .map(|i| format!("{} match-{}", big_line, i))
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(&path, &content).unwrap();
        let dir_str = dir.to_string_lossy().to_string();

        let out = execute_tool_call(
            "search_content",
            &serde_json::json!({"pattern": "match-", "path": dir_str, "include": path.file_name().unwrap().to_string_lossy().to_string()}).to_string(),
            &None,
        );
        let _ = std::fs::remove_file(&path);
        assert!(
            out.len() < MAX_TOOL_RESULT_BYTES + 200,
            "search_content result must be capped, got {} bytes",
            out.len()
        );
    }

    #[test]
    fn cap_tool_result_truncates_overlong_content() {
        let big = "y".repeat(MAX_TOOL_RESULT_BYTES + 1234);
        let capped = cap_tool_result(&big);
        assert!(capped.len() < MAX_TOOL_RESULT_BYTES + 100, "not capped");
        assert!(capped.contains("truncated"), "marker missing");
        assert!(capped.contains("1234") || capped.contains(&format!("{}", big.len())), "size note missing: {}", capped.len());
        // Content under the cap passes through untouched.
        assert_eq!(cap_tool_result("small"), "small");
    }

    #[test]
    fn parse_text_tool_calls_finds_known_tools_only() {
        let text = r#"Let me check. {"name": "read_file", "parameters": {"path": "a.rs"}} and {"name": "evil", "parameters": {}} done"#;
        let calls = parse_text_tool_calls(text);
        assert_eq!(calls.len(), 1, "unknown names must be dropped: {:?}", calls);
        assert_eq!(calls[0]["name"], "read_file");
        assert_eq!(calls[0]["parameters"]["path"], "a.rs");
    }

    #[test]
    fn parse_text_tool_calls_arguments_variant_and_name_colon_form() {
        let a = parse_text_tool_calls(r#"{"name": "bash", "arguments": {"command": "ls"}}"#);
        assert_eq!(a.len(), 1);
        assert_eq!(a[0]["parameters"]["command"], "ls");

        let b = parse_text_tool_calls("list_files:\n{\"path\": \".\"}");
        assert_eq!(b.len(), 1, "name-colon fallback: {:?}", b);
        assert_eq!(b[0]["name"], "list_files");
    }

    #[test]
    fn expand_tilde_resolves_home_and_subpath() {
        let Some(home) = home_dir() else { return }; // no $HOME: nothing to assert
        assert_eq!(expand_tilde("~"), home);
        assert_eq!(expand_tilde("~/git/Rustama"), format!("{home}/git/Rustama"));
        // Trailing slash and nested tilde-free components pass through.
        assert_eq!(expand_tilde("~/x/"), format!("{home}/x/"));
    }

    #[test]
    fn expand_tilde_leaves_other_paths_untouched() {
        // Absolute, relative, tilde-free, and mid-path tildes must not
        // be rewritten (an unresolvable path must still produce the
        // tool's usual filesystem error, not a mangled path).
        assert_eq!(expand_tilde(""), "");
        assert_eq!(expand_tilde("/tmp/x.rs"), "/tmp/x.rs");
        assert_eq!(expand_tilde("src/app.rs"), "src/app.rs");
        assert_eq!(expand_tilde("."), ".");
        assert_eq!(expand_tilde("a~b/c"), "a~b/c");
        assert_eq!(expand_tilde("src/~/x"), "src/~/x");
        // Unknown user: returned unchanged.
        assert_eq!(
            expand_tilde("~no-such-user-8f3a/x"),
            "~no-such-user-8f3a/x"
        );
    }

    #[cfg(unix)]
    #[test]
    fn expand_tilde_resolves_named_user() {
        let user = std::env::var("USER").or_else(|_| std::env::var("LOGNAME"));
        let Some(user) = user.ok().filter(|u| !u.is_empty()) else { return };
        let Some(home) = home_dir() else { return };
        assert_eq!(expand_tilde(&format!("~{user}")), home);
        assert_eq!(
            expand_tilde(&format!("~{user}/git/Rustama")),
            format!("{home}/git/Rustama")
        );
    }

    #[test]
    fn list_files_accepts_tilde_path() {
        // The reported failure mode: `~/...` must list the home
        // directory, not fail with "No such file or directory".
        let out = execute_tool_call("list_files", r#"{"path": "~"}"#, &None);
        assert!(
            !out.starts_with("Error listing directory"),
            "tilde path must resolve, got: {out}"
        );

        // End-to-end: a tilde subpath reaches a real directory under $HOME.
        let Some(home) = home_dir() else { return };
        let dir = std::path::Path::new(&home)
            .join(format!("rustama-tilde-list-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::fs::write(dir.join("f.txt"), "x").unwrap();
        let rel = dir.file_name().unwrap().to_string_lossy().to_string();
        let out = execute_tool_call(
            "list_files",
            &serde_json::json!({"path": format!("~/{rel}")}).to_string(),
            &None,
        );
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(out, "f.txt\nsub/", "got: {out}");
    }

    #[test]
    fn write_file_accepts_tilde_path() {
        let Some(home) = home_dir() else { return };
        let dir = std::path::Path::new(&home)
            .join(format!("rustama-tilde-write-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let rel = dir.file_name().unwrap().to_string_lossy().to_string();
        let out = execute_tool_call(
            "write_file",
            &serde_json::json!({"path": format!("~/{rel}/tilde.md"), "content": "hi"}).to_string(),
            &None,
        );
        assert!(out.starts_with("Successfully wrote"), "got: {out}");
        let read_back = execute_tool_call(
            "read_file",
            &serde_json::json!({"path": format!("~/{rel}/tilde.md")}).to_string(),
            &None,
        );
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(read_back, "hi", "round-trip through ~/ path failed");
    }
}
