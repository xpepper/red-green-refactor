# red-green-refactor

Orchestrate a Red–Green–Refactor loop with three LLM roles (tester, implementor, refactorer). Each step applies a JSON patch, runs tests, and commits to git. Works with Gemini and OpenAI-compatible APIs (e.g., DeepSeek, GitHub Models); mock mode for offline runs.

Note: If you previously used `tdd-pair`, everything is the same—just use the new binary name. Existing config files like `tdd-pair.yaml` still work when passed via `--config`.

## Features
- Three roles with independent models/providers per step
- JSON patch protocol for safe file edits
- Git commit at the end of each role (clean history, easy rollback)
- Test runner is configurable (Cargo, pytest, npm, etc.)
- Refactor step auto-reverts if tests break

## Install
```bash
# Build
cargo build --release
# Create a sample config
./target/release/red-green-refactor init-config --out red-green-refactor.yaml
```

## Configure
Edit your YAML (e.g., `red-green-refactor.yaml`) to pick providers and your test command.

- Provider kinds: `gemini`, `open_ai`, `mock`
- OpenAI-compatible (DeepSeek, Perplexity, Groq, OpenRouter, GitHub Models, local servers) uses `kind: open_ai` + `base_url` + `api_key_env`
- Optional header customization for OpenAI-compatible:
  - `api_key_header`: custom header name (default: `Authorization`)
  - `api_key_prefix`: prefix for header value (default: `"Bearer "`; set to `""` for raw keys)

Example (Gemini tester/refactorer + DeepSeek implementor):
```yaml
tester:
  provider:
    kind: gemini
    model: gemini-1.5-pro
    api_key_env: GEMINI_API_KEY
  system_prompt: "Add exactly one failing test. Output ONLY JSON LlmPatch."

implementor:
  provider:
    kind: open_ai
    model: deepseek-coder
    base_url: https://api.deepseek.com/v1
    api_key_env: DEEPSEEK_API_KEY
  system_prompt: "Make tests pass with the smallest change. Output ONLY JSON LlmPatch."

refactorer:
  provider:
    kind: gemini
    model: gemini-1.5-pro
    api_key_env: GEMINI_API_KEY
  system_prompt: "Refactor without changing behavior. Output ONLY JSON LlmPatch."

test_cmd: "cargo test --color never"
max_context_bytes: 200000
```

Export keys (adjust to your config):
```bash
export GEMINI_API_KEY=your_gemini_key
export DEEPSEEK_API_KEY=your_deepseek_key
```

### GitHub Models (OpenAI-compatible)
- Most setups work with standard Bearer auth (defaults):
```yaml
provider:
  kind: open_ai
  model: gpt-4o-mini
  base_url: https://models.inference.ai.azure.com/openai
  api_key_env: GITHUB_TOKEN  # or GITHUB_MODELS_TOKEN
  # uses defaults: Authorization + "Bearer "
```
- If your endpoint expects an `api-key` header without Bearer:
```yaml
provider:
  kind: open_ai
  model: gpt-4o-mini
  base_url: https://models.inference.ai.azure.com/openai
  api_key_env: GITHUB_MODELS_API_KEY
  api_key_header: api-key
  api_key_prefix: ""
```
Note: Some GitHub-hosted endpoints may require additional headers (e.g., `X-GitHub-Api-Version`). If you need that, open an issue—support can be added easily.

## Run on a kata

### Option A: Bowling Game (Rust)
```bash
# New kata project
cargo new bowling_kata
cd bowling_kata
git init

# From the red-green-refactor repo (adjust path if needed)
../red-green-refactor/target/release/red-green-refactor --project . --config ../red-green-refactor/red-green-refactor.yaml
# Or continuous mode
../red-green-refactor/target/release/red-green-refactor --project . --config ../red-green-refactor/red-green-refactor.yaml run
```
What happens each cycle:
- Tester adds one failing test and commits
- Implementor makes tests pass and commits (up to configurable retries)
- Refactorer improves code and commits; if tests fail, that commit is automatically reverted

Inspect:
```bash
git --no-pager log --oneline
```

### Option B: Mars Rover (Python + pytest)
```bash
# Simple Python project with pytest
mkdir mars_rover && cd mars_rover
git init
python3 -m venv .venv
. .venv/bin/activate
pip install pytest

# Edit your YAML to use pytest
# test_cmd: "pytest -q"

# Run red-green-refactor
../red-green-refactor/target/release/red-green-refactor --project . --config ../red-green-refactor/red-green-refactor.yaml
```

## Providers
- Gemini: `kind: gemini`, set `api_key_env` (e.g., `GEMINI_API_KEY`). Models like `gemini-1.5-pro`.
- OpenAI-compatible (e.g., DeepSeek, GitHub Models): `kind: open_ai`, set `base_url` and `api_key_env`. Optional:
  - `api_key_header` (e.g., `api-key`)
  - `api_key_prefix` (e.g., `""` for raw keys)
- Mock: `kind: mock` for offline dry runs (appends to `red-green-refactor-mock.log`).

## Notes
- Context is collected from `src/**`, `tests/**`, `Cargo.toml`, README and Markdown files, truncated at `max_context_bytes`.
- Each role must output only a JSON `LlmPatch`:
  - `files`: list of edits `{ path, mode: "rewrite"|"append", content }`
  - `commit_message` (optional)
- Implementor retries: set `implementor_max_attempts` (default 3). On exhaustion, the tool branches `attempts/implementor-...` and resets to the tester commit.
- Git repo is auto-initialized; refactor commit is reverted if tests break.

## Troubleshooting
- Missing API key: ensure `api_key_env` matches your exported variable.
- Tests not running: set `test_cmd` to your runner (e.g., `pytest -q`, `npm test`, `mvn -q test`).
- Large repos: raise `max_context_bytes`.
- Broken refactor: the tool hard-resets the last commit; re-run to continue.

## Commands
```bash
# One cycle (default)
./target/release/red-green-refactor --project <path> --config red-green-refactor.yaml
# Continuous
./target/release/red-green-refactor --project <path> --config red-green-refactor.yaml run
# Generate sample config
./target/release/red-green-refactor init-config --out red-green-refactor.yaml
```
