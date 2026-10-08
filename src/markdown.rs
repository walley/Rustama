//! Markdown rendering — assistant/model text → styled ratatui `Line`s.
//!
//! Extracted from `main.rs`'s rendering: `render_markdown()` is the single
//! entry point, used by `render_output()` for assistant messages, file
//! contents and the streaming text. Everything else in this module is
//! private and exists only to serve it:
//!
//! - `flush_line` — commit the accumulated inline spans as a finished line
//! - `highlight_code` — `syntect` highlighting for fenced code blocks
//! - `normalize_code_fences` — pre-pass fixing indented ``` fences
//!
//! The code-block background for fence headers, highlighted code lines and
//! thinking-block code is the shared `ui::CODE_BLOCK_BG` — the same value
//! `wrap_and_justify_lines()` (main.rs) keys on to leave code lines
//! unwrapped/unjustified.

use crate::ui::CODE_BLOCK_BG;
use pulldown_cmark::{
    CodeBlockKind, Event as MdEvent, Options as MdOptions, Parser as MdParser, Tag, TagEnd,
};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

/// Render a markdown string (an assistant message body) as styled lines.
///
/// Runs `pulldown-cmark` over the text and converts the parser events into
/// styled `Line`s: `#`-prefixed yellow+bold headings, bold/italic emphasis,
/// cyan inline code, fenced code blocks (with a `[LANG]`/`[CODE]` header
/// and `syntect` highlighting), Unicode box-drawing tables, green `  • `
/// list bullets and `─` rules. `+---+`-style ASCII-art frame lines some
/// models draw around their answers are filtered out before parsing, and
/// indented code fences are normalized by `normalize_code_fences`.
pub fn render_markdown(text: &str) -> Vec<Line<'static>> {
    let mut md_options = MdOptions::empty();
    md_options.insert(MdOptions::ENABLE_STRIKETHROUGH);
    md_options.insert(MdOptions::ENABLE_TABLES);

    let filtered: Vec<&str> = text
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return true;
            }
            if trimmed.starts_with('+')
                && trimmed.ends_with('+')
                && trimmed.chars().all(|c| c == '+' || c == '-' || c == ' ')
            {
                return false;
            }
            true
        })
        .collect();

    let cleaned = normalize_code_fences(&filtered).join("\n");

    let parser = MdParser::new_ext(&cleaned, md_options);
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut current_spans: Vec<Span<'static>> = Vec::new();
    let mut bold = false;
    let mut italic = false;
    let mut in_code_block = false;
    let mut code_block_lang: Option<String> = None;
    let mut code_block_text = String::new();
    let mut in_table = false;
    let mut table_headers: Vec<String> = Vec::new();
    let mut table_rows: Vec<Vec<String>> = Vec::new();
    let mut current_row: Vec<String> = Vec::new();
    let mut is_header_row = false;

    for event in parser {
        match event {
            MdEvent::Start(tag) => match tag {
                Tag::Heading { level, .. } => {
                    flush_line(&mut lines, &mut current_spans);
                    let prefix = match level {
                        pulldown_cmark::HeadingLevel::H1 => "# ",
                        pulldown_cmark::HeadingLevel::H2 => "## ",
                        pulldown_cmark::HeadingLevel::H3 => "### ",
                        pulldown_cmark::HeadingLevel::H4 => "#### ",
                        pulldown_cmark::HeadingLevel::H5 => "##### ",
                        pulldown_cmark::HeadingLevel::H6 => "###### ",
                    };
                    current_spans.push(Span::styled(
                        prefix.to_string(),
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ));
                    bold = true;
                }
                Tag::CodeBlock(kind) => {
                    flush_line(&mut lines, &mut current_spans);
                    in_code_block = true;
                    code_block_text.clear();
                    code_block_lang = match kind {
                        CodeBlockKind::Fenced(lang) if !lang.is_empty() => Some(lang.to_string()),
                        _ => None,
                    };
                    let header = match &code_block_lang {
                        Some(lang) => format!("[{}]", lang.to_uppercase()),
                        None => "[CODE]".to_string(),
                    };
                    lines.push(Line::from(Span::styled(
                        header,
                        Style::default().fg(Color::Yellow).bg(CODE_BLOCK_BG),
                    )));
                }
                Tag::Table(_alignment) => {
                    flush_line(&mut lines, &mut current_spans);
                    in_table = true;
                    table_headers.clear();
                    table_rows.clear();
                    current_row.clear();
                    is_header_row = true;
                }
                Tag::TableHead => {
                    is_header_row = true;
                    current_row.clear();
                }
                Tag::TableRow => {
                    current_row.clear();
                }
                Tag::TableCell => {}
                Tag::Emphasis => italic = true,
                Tag::Strong => bold = true,
                Tag::Item => {
                    flush_line(&mut lines, &mut current_spans);
                    current_spans.push(Span::styled("  • ", Style::default().fg(Color::Green)));
                }
                _ => {}
            },
            MdEvent::End(tag_end) => match tag_end {
                TagEnd::Paragraph => {
                    flush_line(&mut lines, &mut current_spans);
                    lines.push(Line::from(""));
                }
                TagEnd::Heading(_) => {
                    bold = false;
                    flush_line(&mut lines, &mut current_spans);
                    lines.push(Line::from(""));
                }
                TagEnd::CodeBlock => {
                    in_code_block = false;
                    let code_lines = highlight_code(&code_block_text, code_block_lang.as_deref());
                    for line in code_lines {
                        lines.push(line);
                    }
                    lines.push(Line::from(""));
                }
                TagEnd::Table => {
                    if !table_headers.is_empty() {
                        let col_widths: Vec<usize> = table_headers
                            .iter()
                            .enumerate()
                            .map(|(i, h)| {
                                let data_max = table_rows
                                    .iter()
                                    .filter_map(|r| r.get(i))
                                    .map(|c| c.len())
                                    .max()
                                    .unwrap_or(0);
                                h.len().max(data_max).max(3)
                            })
                            .collect();

                        let border_style = Style::default().fg(Color::DarkGray);

                        // Top border: ┌───┬───┬───┐
                        let mut top = String::from("┌");
                        for (i, &w) in col_widths.iter().enumerate() {
                            top.push_str(&"─".repeat(w + 2));
                            if i < col_widths.len() - 1 {
                                top.push('┬');
                            }
                        }
                        top.push('┐');
                        lines.push(Line::from(Span::styled(top, border_style)));

                        // Header row: │ a │ b │ c │
                        let mut header_spans: Vec<Span> = Vec::new();
                        for (i, h) in table_headers.iter().enumerate() {
                            let w = col_widths[i];
                            let padded = format!("{:<width$}", h, width = w);
                            header_spans.push(Span::styled(
                                format!("│ {} ", padded),
                                Style::default()
                                    .fg(Color::White)
                                    .bg(Color::Rgb(60, 60, 80))
                                    .add_modifier(Modifier::BOLD),
                            ));
                        }
                        header_spans.push(Span::styled("│", border_style));
                        lines.push(Line::from(header_spans));

                        // Separator: ├───┼───┼───┤
                        let mut sep = String::from("├");
                        for (i, &w) in col_widths.iter().enumerate() {
                            sep.push_str(&"─".repeat(w + 2));
                            if i < col_widths.len() - 1 {
                                sep.push('┼');
                            }
                        }
                        sep.push('┤');
                        lines.push(Line::from(Span::styled(sep, border_style)));

                        // Data rows: │ x │ y │ z │
                        for row in &table_rows {
                            let mut row_spans: Vec<Span> = Vec::new();
                            for (i, cell) in row.iter().enumerate() {
                                let w = col_widths.get(i).copied().unwrap_or(10);
                                let padded = format!("{:<width$}", cell, width = w);
                                row_spans
                                    .push(Span::styled(format!("│ {} ", padded), Style::default()));
                            }
                            row_spans.push(Span::styled("│", border_style));
                            lines.push(Line::from(row_spans));
                        }

                        // Bottom border: └───┴───┴───┘
                        let mut bottom = String::from("└");
                        for (i, &w) in col_widths.iter().enumerate() {
                            bottom.push_str(&"─".repeat(w + 2));
                            if i < col_widths.len() - 1 {
                                bottom.push('┴');
                            }
                        }
                        bottom.push('┘');
                        lines.push(Line::from(Span::styled(bottom, border_style)));
                        lines.push(Line::from(""));
                    }
                    in_table = false;
                }
                TagEnd::TableHead => {
                    table_headers = current_row.clone();
                    is_header_row = false;
                }
                TagEnd::TableRow => {
                    if !is_header_row {
                        table_rows.push(current_row.clone());
                    }
                    is_header_row = false;
                }
                TagEnd::TableCell => {
                    let cell_text = current_spans
                        .iter()
                        .map(|s| s.content.to_string())
                        .collect::<String>();
                    current_row.push(cell_text);
                    current_spans.clear();
                }
                TagEnd::Emphasis => italic = false,
                TagEnd::Strong => bold = false,
                TagEnd::Item => {
                    flush_line(&mut lines, &mut current_spans);
                }
                _ => {}
            },
            MdEvent::Text(text) => {
                if in_code_block {
                    code_block_text.push_str(&text);
                } else {
                    let style = if in_table && is_header_row {
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        let mut s = Style::default();
                        if bold {
                            s = s.add_modifier(Modifier::BOLD);
                        }
                        if italic {
                            s = s.add_modifier(Modifier::ITALIC);
                        }
                        s
                    };
                    current_spans.push(Span::styled(text.to_string(), style));
                }
            }
            MdEvent::Code(code) => {
                current_spans.push(Span::styled(
                    format!("`{}`", code),
                    Style::default().fg(Color::Cyan).bg(Color::Rgb(40, 40, 60)),
                ));
            }
            MdEvent::SoftBreak | MdEvent::HardBreak => {
                if in_code_block {
                    code_block_text.push('\n');
                } else if in_table {
                    current_spans.clear();
                } else {
                    flush_line(&mut lines, &mut current_spans);
                }
            }
            MdEvent::Rule => {
                flush_line(&mut lines, &mut current_spans);
                lines.push(Line::from(Span::styled(
                    "─────────────────────────────────────",
                    Style::default().fg(Color::DarkGray),
                )));
                lines.push(Line::from(""));
            }
            _ => {}
        }
    }

    flush_line(&mut lines, &mut current_spans);

    if lines.is_empty() {
        lines.push(Line::from(""));
    }

    lines
}

/// Push the accumulated inline spans as a finished line (no-op when empty).
fn flush_line(lines: &mut Vec<Line<'static>>, spans: &mut Vec<Span<'static>>) {
    if !spans.is_empty() {
        let collected: Vec<Span> = std::mem::take(spans);
        lines.push(Line::from(collected));
    }
}

/// Syntax-highlight a code block with syntect's "base16-ocean.dark" theme
/// on the shared code-block background. The `SyntaxSet`/`ThemeSet` load
/// lazily into `OnceLock`s — the first highlighted block pays the load,
/// every later one reuses it.
fn highlight_code(code: &str, lang: Option<&str>) -> Vec<Line<'static>> {
    use syntect::easy::HighlightLines;
    use syntect::highlighting::ThemeSet;
    use syntect::parsing::SyntaxSet;

    use std::sync::OnceLock;
    static SS: OnceLock<SyntaxSet> = OnceLock::new();
    static TS: OnceLock<ThemeSet> = OnceLock::new();

    let ss = SS.get_or_init(SyntaxSet::load_defaults_newlines);
    let ts = TS.get_or_init(ThemeSet::load_defaults);

    let syntax = lang
        .and_then(|l| ss.find_syntax_by_token(l))
        .unwrap_or_else(|| ss.find_syntax_plain_text());

    let mut h = HighlightLines::new(syntax, &ts.themes["base16-ocean.dark"]);

    let bg = CODE_BLOCK_BG;
    let mut result: Vec<Line<'static>> = Vec::new();

    for line in code.lines() {
        let ranges = h.highlight_line(line, ss).unwrap_or_default();
        let mut spans: Vec<Span<'static>> = Vec::new();
        for (style, text) in ranges {
            let fg = Color::Rgb(style.foreground.r, style.foreground.g, style.foreground.b);
            spans.push(Span::styled(
                text.to_string(),
                Style::default().fg(fg).bg(bg),
            ));
        }
        result.push(Line::from(spans));
    }

    if code.ends_with('\n') || code.is_empty() {
        result.push(Line::from(Span::styled(" ", Style::default().bg(bg))));
    }

    result
}

/// Normalize indented ``` fences before parsing.
///
/// pulldown-cmark only recognizes fences in the first few columns, so
/// models that indent their fences (e.g. inside their ASCII-art answer
/// frames) produce broken blocks. This pass tracks the opening fence's
/// indent and pads under-indented body lines (and empty lines) up to it,
/// so the whole block survives as one coherent unit; the closing fence is
/// recognized when its indent is at most the opening one's.
fn normalize_code_fences(lines: &[&str]) -> Vec<String> {
    let mut result = Vec::new();
    let mut in_code_block = false;
    let mut fence_indent: usize = 0;

    for line in lines {
        if in_code_block {
            let trimmed = line.trim_start();
            let current_indent = line.len() - trimmed.len();

            if trimmed.starts_with("```") && current_indent <= fence_indent {
                in_code_block = false;
                result.push(format!("{}{}", " ".repeat(fence_indent), trimmed));
                continue;
            }

            if line.trim().is_empty() {
                result.push(format!("{}{}", " ".repeat(fence_indent), ""));
            } else if current_indent < fence_indent {
                result.push(format!("{}{}", " ".repeat(fence_indent), trimmed));
            } else {
                result.push(line.to_string());
            }
        } else {
            let trimmed = line.trim_start();
            if trimmed.starts_with("```") {
                let indent = line.len() - trimmed.len();
                if indent > 0 {
                    in_code_block = true;
                    fence_indent = indent;
                    result.push(line.to_string());
                    continue;
                }
            }
            result.push(line.to_string());
        }
    }
    result
}

#[cfg(test)]
mod markdown_tests {
    use super::render_markdown;
    use crate::ui::CODE_BLOCK_BG;
    use ratatui::style::{Color, Modifier};
    use ratatui::text::Line;

    /// Concatenate every span of every rendered line into one string.
    fn render_text(md: &str) -> String {
        render_markdown(md)
            .iter()
            .flat_map(|l| l.spans.iter())
            .map(|s| s.content.to_string())
            .collect()
    }

    #[test]
    fn heading_prefix_is_yellow_bold() {
        let lines = render_markdown("# Title");
        let all = render_text("# Title");
        assert!(all.contains("# Title"), "heading text missing: {all:?}");
        let span = lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .find(|s| s.content.contains('#'))
            .expect("heading prefix span");
        assert_eq!(span.style.fg, Some(Color::Yellow));
        assert!(span.style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn fenced_code_has_lang_header_and_body() {
        let all = render_text("```rust\nfn main() {}\n```");
        assert!(all.contains("[RUST]"), "missing [RUST] header: {all:?}");
        assert!(all.contains("fn main() {}"), "code body missing: {all:?}");
    }

    #[test]
    fn code_block_lines_carry_the_shared_bg() {
        // Both the [RUST] header span and every highlighted body span must
        // sit on ui::CODE_BLOCK_BG — the same value wrap_and_justify_lines
        // (main.rs) keys on to leave code lines unwrapped.
        let lines = render_markdown("```rust\nfn main() {}\n```");
        let code_lines: Vec<&Line> = lines
            .iter()
            .filter(|l| l.spans.iter().any(|s| s.style.bg == Some(CODE_BLOCK_BG)))
            .collect();
        assert!(!code_lines.is_empty(), "no CODE_BLOCK_BG spans rendered");
        let body = code_lines
            .iter()
            .find(|l| {
                let text: String = l.spans.iter().map(|s| s.content.to_string()).collect();
                text.contains("fn main() {}")
            })
            .expect("highlighted body line");
        assert!(
            body.spans
                .iter()
                .all(|s| s.style.bg == Some(CODE_BLOCK_BG)),
            "body span without the shared bg"
        );
        assert_eq!(CODE_BLOCK_BG, Color::Rgb(30, 60, 120));
    }

    #[test]
    fn plain_fence_gets_code_header() {
        let all = render_text("```\nplain\n```");
        assert!(all.contains("[CODE]"), "missing [CODE] header: {all:?}");
    }

    #[test]
    fn table_renders_box_borders() {
        let all = render_text("| a | b |\n|---|---|\n| 1 | 2 |");
        assert!(all.contains('┌'), "missing top border: {all:?}");
        assert!(all.contains('└'), "missing bottom border: {all:?}");
        assert!(all.contains("│ a "), "header cell missing: {all:?}");
        assert!(all.contains("│ 1 "), "data cell missing: {all:?}");
    }

    #[test]
    fn ascii_frame_lines_are_filtered() {
        // `+---+`-style frame lines are dropped before parsing; the text
        // between them survives (as plain paragraph text).
        let all = render_text("+----------+\nanswer body\n+----------+");
        assert!(!all.contains("----------"), "frame leaked: {all:?}");
        assert!(all.contains("answer body"), "body missing: {all:?}");
    }

    #[test]
    fn empty_input_renders_one_blank_line() {
        let lines = render_markdown("");
        assert_eq!(lines.len(), 1);
    }

    #[test]
    fn indented_fence_body_is_padded() {
        let fixed = super::normalize_code_fences(&["  ```rust", "fn a() {}", "  ```"]);
        assert_eq!(fixed[0], "  ```rust");
        // Under-indented body lines are lifted to the fence's indent.
        assert_eq!(fixed[1], "  fn a() {}");
        assert_eq!(fixed[2], "  ```");
    }
}
