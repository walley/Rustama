# Example configuration

Example copies of every file Rustama reads from its configuration directory.
All values are placeholders — no real API keys, tokens, or personal notes.

## Usage

```bash
mkdir -p ~/.config/rustama
cp config.example/* ~/.config/rustama/
```

Then edit the files:

- `rustama.conf` — main settings (endpoint, model, limits, LSP, …)
- `models_ollama.conf` — sampling parameters for local Ollama models
- `models_cloud.conf` — cloud (OpenAI-compatible) model definitions — **put your real API keys here**
- `coder.prompt` / `assistant.prompt` — system prompts referenced by `system_prompt_coder` / `system_prompt_assistant`
- `AGENTS.md` — instructions appended to every system prompt (project/agent-wide rules)
- `example.theme` — an optional colour theme, selected with `theme = example` in `rustama.conf`

Rustama creates any missing file with its built-in defaults on first run, so
copying only the files you want to customise is fine. Sections in
`models_cloud.conf` whose `api_key` still starts with `YOUR_` are ignored
until you fill them in.

See the main [README](../README.md) for the full key reference.
