# App-wide agent instructions

This file is appended to the system prompt of every mode (chat, coder,
assistant). Keep it short — it costs tokens on every request. Use it for
preferences that apply to all projects.

## Terminal tool

When driving an interactive full-screen TUI app through the terminal tool,
send arrow keys using the application cursor key sequences:

- Up:    `ESC O A`
- Down:  `ESC O B`
- Right: `ESC O C`
- Left:  `ESC O D`

TUI programs enable application cursor key mode, so the ANSI forms
(`ESC [ A` …) are silently ignored and the cursor appears not to move.
`terminal_send` writes input verbatim — append `\r` yourself to submit a
line.

## Working style

- Read a file before editing it, and keep changes consistent with the
  surrounding code.
- Run the project's build and tests after making changes.
- Do not invent commands or options; check `--help` when unsure.
