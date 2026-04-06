## Secrets Manager (encrypted text file)

A minimal secrets manager CLI that stores and retrieves **key/value** secrets from a **single encrypted text file**.

Each secret is stored as one line in the decrypted content using this format:

```text
key:value:timestamp
```

Where `timestamp` is the last time the `key:value` entry was updated.

### Goals

- Keep the implementation simple: one encrypted file, line-based records.
- Provide only three operations: add, get, and list.
- Make it easy to version-control the tool itself while keeping secrets encrypted at rest.
- Prevent secrets from appearing in shell history or terminal output.

## Data format

When decrypted, the secrets file is plain text with **one entry per line**:

```text
db_password:s3cr3t:2026-02-24T10:15:30Z
api_key:abc123:2026-02-24T10:20:00Z
```

If a key or value contains `:`, both are automatically base64-encoded (prefixed with `b64:`):

```text
b64:aW5qZWN0ZWQ6a2V5:dmFsdWU6d2l0aHRlc3Q=:2026-02-24T10:20:00Z
```

Notes:

- `key` and `value` can now contain `:` (they'll be base64-encoded automatically).
- Neither can contain newlines (they would break the line-based format).
- `timestamp` is always plain text in RFC3339 format.

## CLI operations

The application supports three commands:

### `add <key>`

Stores a new key/value secret interactively.

Behavior:

- Prompts for the secret value with no terminal echo (secure input).
- Asks for confirmation to prevent typos.
- If `key` does not exist, creates a new entry.
- If `key` already exists, updates its `value` and `timestamp`.

Example:

```bash
$ secrets-manager add db_password
Value for 'db_password': 
Confirm value for 'db_password': 
```

The value is never displayed on screen and never stored in shell history.

### `get <key>`

Retrieves the value by key.

Example:

```bash
secrets-manager get db_password
```

Expected output (example):

```text
s3cr3t
```

### `list [pattern] [--with-timestamps]`

Lists all stored keys, optionally filtered by pattern.

Options:

- `pattern` (optional): Case-insensitive substring filter.
- `--with-timestamps`: Show the timestamp each key was last updated.

Examples:

```bash
secrets-manager list                              # List all keys
secrets-manager list db                           # List keys containing "db"
secrets-manager list --with-timestamps            # List all keys with timestamps
secrets-manager list api --with-timestamps        # List matching keys with timestamps
```

## Encryption

Secrets are stored **encrypted at rest** in a single file. The exact encryption mechanism (e.g., GPG, libsodium, OpenSSL, etc.) will be defined during implementation, but the intent is:

- Decrypt the file in-memory for reads/writes.
- Re-encrypt the full content back to disk after modifications.
- Avoid leaving decrypted content on disk.

## Running this implementation

This Rust implementation uses your local `gpg` keyring:

- Provide the recipient (key id / fingerprint / email) via `--recipient <...>` or `SECRETS_MANAGER_GPG_RECIPIENT`.
	- Required for `add` (so the file can be encrypted on save).
	- Not required for `get`.
- Choose the encrypted file via `--file <path>` or `SECRETS_MANAGER_FILE` (defaults to `./secrets.enc`).
- Optionally set `--gnupghome <path>` (or `SECRETS_MANAGER_GNUPGHOME` / `GNUPGHOME`) to use a non-default keyring.

### First-run configuration

If `SECRETS_MANAGER_GPG_RECIPIENT` / `--recipient` (for `add`) and/or `SECRETS_MANAGER_FILE` / `--file` are not set, the CLI will prompt you on first run and persist defaults to:

```text
$HOME/.secrets-manager/config.toml
```

Notes:

- CLI flags and environment variables take precedence over the config file.
- On Linux/Unix, the config directory is enforced as `0700` and the config file as `0600`.

Examples:

```bash
export SECRETS_MANAGER_GPG_RECIPIENT='you@example.com'

cargo run -- add db_password
# (prompts for value securely, then asks for confirmation)
cargo run -- get db_password
cargo run -- list --with-timestamps
```

## Error handling (expected)

- `get <key>` returns exit code 1 if the key is not found, 0 on success.
- `add <key>` returns a non-zero exit code on:
  - Non-interactive terminal (e.g., piped input in CI)
  - Empty value provided
  - Confirmation values don't match
  - Missing GPG recipient
  - Encryption/decryption failures
- `list [pattern]` returns 0 on success (empty result is not an error).

## Security Features

- **No plaintext in terminal**: Secret values are never echoed or displayed on screen during `add`.
- **No shell history leakage**: Values are prompted interactively, not passed as CLI arguments.
- **Confirmation prompt**: Prevents typos in critical secrets by asking you to confirm.
- **GPG encryption**: Secrets are encrypted at rest using your local GnuPG keyring.
- **Secure config storage**: Configuration file permissions enforced to 0600 (owner read/write only).
