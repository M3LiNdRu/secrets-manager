# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Secrets Manager is a minimal encrypted secrets manager CLI that stores and retrieves key/value secrets from a single encrypted text file. It uses GPG for encryption at rest and is implemented in Rust.

**Key characteristics:**
- Single encrypted file containing all secrets (default: `secrets.enc`)
- Line-based storage format: `key:value:timestamp` (RFC3339)
- Keys and values containing `:` are automatically base64-encoded with `b64:` prefix
- Three operations: `add`, `get`, and `list`
- GPG-backed encryption using local keyring
- Config stored in `$HOME/.secrets-manager/config.toml`

## Development Commands

**Build:**
```bash
cargo build              # Debug build
cargo build --release   # Release build
```

**Run:**
```bash
cargo run -- <command>
# Examples:
cargo run -- add db_password
# (prompts for value with no terminal echo, asks for confirmation)
cargo run -- get db_password
cargo run -- list
cargo run -- list --with-timestamps
```

**Test:**
```bash
cargo test              # Run all tests
cargo test --lib       # Run lib tests only
```

**Lint/Format:**
```bash
cargo clippy           # Lint checks
cargo fmt              # Format code
cargo fmt -- --check  # Check if formatting is needed
```

**Build Debian package:**
```bash
cargo deb --no-build  # Create .deb after building release
```

## Architecture

The codebase is organized into modular components following a trait-based design pattern:

**Core modules (src/):**
- **main.rs**: CLI entry point using `clap` for argument parsing. Handles three subcommands: `add`, `get`, and `list`. Manages config loading/prompting and configuration resolution (CLI args → env vars → config file → defaults).
- **lib.rs**: Exports `SecretsManager<S>` (generic over `SecretsStore` trait), validation functions. Contains integration tests that set up temporary GPG keyrings.
- **model.rs**: Data structures (`Entry`, `Secrets`). Implements `Secrets::from_lines()` (parses plaintext records) and `to_lines()` (serializes with base64 encoding for special characters). Handles both plain and base64-encoded formats for backward compatibility.
- **store.rs**: `SecretsStore` trait and `FileSecretsStore` implementation. Handles encryption/decryption via `crypto` module and file I/O. Uses builder pattern for configuration (`with_recipient`, `with_gnupghome`).
- **crypto.rs**: Low-level GPG interaction via subprocess. `encrypt_text()` and `decrypt_text()` wrap `gpg` command execution. Handles terminal detection for interactive vs. non-interactive modes.
- **config.rs**: Config file I/O at `~/.secrets-manager/config.toml`. `load()` reads config, `prompt_for_missing()` prompts user for recipient/file path if needed.

**Data Flow:**
1. CLI parses args/env/config
2. `FileSecretsStore` orchestrates encryption/decryption
3. `SecretsManager` performs CRUD operations on `Secrets` model
4. `Secrets` handles parsing/serialization (base64 encoding if needed)
5. `crypto` module wraps GPG subprocess calls

## Key Implementation Details

**Secure Value Input:**
- The `add` command prompts for the value interactively using `rpassword` crate
- Terminal echo is disabled during value input (no plaintext on screen)
- A confirmation prompt asks the user to enter the value again (catches typos)
- Values are never stored in shell history or visible on the command line
- Non-interactive mode (e.g., piped input in CI) is rejected with a clear error

**Configuration precedence:** CLI args > environment variables > `~/.secrets-manager/config.toml` > defaults

**Encryption/Decryption:**
- Uses `gpg` subprocess with stdin/stdout pipes
- Interactive mode: spawns `pinentry` for passphrase entry if needed
- Non-interactive mode: batch mode for CI/automation
- Always outputs ASCII-armored PGP messages

**Base64 Encoding:**
- Triggered when key or value contains `:` character
- Format: `b64:<base64_key>:<base64_value>:<timestamp>`
- Maintains backward compatibility with plaintext records

**Error Handling:**
- `anyhow` for error propagation
- Non-zero exit codes: 1 (key not found), 2 (general error)
- Detailed error messages with context

**Testing:**
- Tests set up temporary GPG homes with generated keys
- Creates temp secrets files for each test
- Tests cover: basic add/get, updates, colon handling

## Configuration File

Location: `$HOME/.secrets-manager/config.toml`

```toml
file = "/path/to/secrets.enc"
gpg_recipient = "email@example.com"
```

Permissions: 0600 (owner read/write only on Unix systems)

## Dependencies

Main: `anyhow`, `base64`, `clap`, `chrono`, `rpassword`, `serde`, `toml`

Dev: `tempfile`

**Notable:**
- `rpassword` — provides secure terminal input with echo suppression for password/secret prompts

## Common Tasks

**Add a secret:**
```bash
export SECRETS_MANAGER_GPG_RECIPIENT='you@example.com'
cargo run -- add mykey
# (prompts for value securely, then asks for confirmation)
```

**Retrieve a secret:**
```bash
cargo run -- get mykey
```

**List all keys with timestamps:**
```bash
cargo run -- list --with-timestamps
```

**Use custom GPG keyring:**
```bash
cargo run -- --gnupghome ~/.config/gnupg add mykey
```

## Release Process

Uses semantic-release with conventional commits:
- Bumps version in `Cargo.toml`
- Builds release binary
- Creates Debian package
- Pushes to GitHub with release notes

Triggered on push to `master` branch.
