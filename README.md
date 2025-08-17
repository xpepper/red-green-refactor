# tdd-pair

Orchestrate a Red–Green–Refactor loop with three LLM roles (tester, implementor, refactorer). Each step applies a JSON patch, runs tests, and commits to git. Works with Gemini and OpenAI-compatible APIs (e.g., DeepSeek); mock mode for offline runs.

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
./target/release/tdd-pair init-config --out tdd-pair.yaml
```

## Configure
Edit `tdd-pair.yaml` to pick providers and your test command.

- Provider kinds: `gemini`, `open_ai`, `mock`
- OpenAI-compatible (DeepSeek, Groq, OpenRouter, local servers) uses `kind: open_ai` + `base_url` + `api_key_env`
- Gemini uses `kind: gemini` + `api_key_env`
- `test_cmd` runs your suite (any language)

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

## Run on a kata

### Option A: Bowling Game (Rust)
```bash
# New kata project
cargo new bowling_kata
cd bowling_kata
git init

# From the tdd-pair repo (adjust path if needed)
../tdd-pair/target/release/tdd-pair --project . --config ../tdd-pair/tdd-pair.yaml
# Or continuous mode
../tdd-pair/target/release/tdd-pair --project . --config ../tdd-pair/tdd-pair.yaml run
```
What happens each cycle:
- Tester adds one failing test and commits
- Implementor makes tests pass and commits
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

# Edit tdd-pair.yaml to use pytest
# test_cmd: "pytest -q"

# Run tdd-pair
../tdd-pair/target/release/tdd-pair --project . --config ../tdd-pair/tdd-pair.yaml
```

## Providers
- Gemini: `kind: gemini`, set `api_key_env` (e.g., `GEMINI_API_KEY`). Models like `gemini-1.5-pro`.
- OpenAI-compatible (e.g., DeepSeek): `kind: open_ai`, set `base_url` and `api_key_env` (e.g., `DEEPSEEK_API_KEY`).
- Mock: `kind: mock` for offline dry runs (appends to `README.tdd-pair.log`).

## Notes
- Context is collected from `src/**`, `tests/**`, `Cargo.toml`, README and Markdown files, truncated at `max_context_bytes`.
- Each role must output only a JSON `LlmPatch`:
  - `files`: list of edits `{ path, mode: "rewrite"|"append", content }`
  - `commit_message` (optional)
- Git repo is auto-initialized; refactor commit is reverted if tests break.

## Troubleshooting
- Missing API key: ensure `api_key_env` matches your exported variable.
- Tests not running: set `test_cmd` to your runner (e.g., `pytest -q`, `npm test`, `mvn -q test`).
- Large repos: raise `max_context_bytes`.
- Broken refactor: the tool hard-resets the last commit; re-run to continue.

## Commands
```bash
# One cycle (default)
./target/release/tdd-pair --project <path> --config tdd-pair.yaml
# Continuous
./target/release/tdd-pair --project <path> --config tdd-pair.yaml run
# Generate sample config
./target/release/tdd-pair init-config --out tdd-pair.yaml
```

