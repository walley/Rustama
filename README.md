# Rustama 🦀

A Rust-based TUI (Terminal User Interface) client for Ollama with agentic capabilities.

## Overview

Rustama is a modern, terminal-based interface for interacting with [Ollama](https://ollama.ai/) models. Built with Rust, it provides a responsive TUI experience with support for agentic interactions, allowing you to leverage local language models directly from your terminal.

## Features

- 🎨 **Modern TUI Interface** - Built with [ratatui](https://github.com/ratatui-org/ratatui)
- ⚡ **Async/Await Support** - Powered by [tokio](https://tokio.rs/) for responsive interactions
- 🤖 **Agentic Mode** - Support for agent-based interactions with language models
- 📡 **Streaming Responses** - Real-time streaming of model outputs
- 🔧 **Configurable** - Customizable key bindings and settings

## Requirements

- Rust 2024 edition or later
- Ollama service running locally or remotely
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

Rustama can be configured using the `config.keys` file for custom key bindings and other settings.

## Usage

Once running, Rustama provides a terminal interface for:
- Sending prompts to Ollama models
- Receiving and streaming responses in real-time
- Managing conversation history
- Using agentic modes for more advanced interactions

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

Built with ❤️in Rust YAY!

