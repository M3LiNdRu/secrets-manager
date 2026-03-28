## Secrets Manager (encrypted text file)

A minimal secrets manager CLI that stores and retrieves **key/value** secrets from a **single encrypted text file**.

Each secret is stored as one line in the decrypted content using this format:

```text
key:value:timestamp
```

Where `timestamp` is the last time the `key:value` entry was updated.

### Goals

- Keep the implementation simple: one encrypted file, line-based records.
- Provide only two operations: add and get.
- Make it easy to version-control the tool itself while keeping secrets encrypted at rest.

## Data format

When decrypted, the secrets file is plain text with **one entry per line**:

```text
db_password:s3cr3t:2026-02-24T10:15:30Z
api_key:abc123:2026-02-24T10:20:00Z
```

Notes:

- `key` should not contain `:`.
- `value` should not contain `:` (or it must be encoded/escaped by the implementation).
- `timestamp` will be written by the application when adding/updating entries.

## CLI operations

The application supports two commands:

### `add <key> <value>`

Stores a new key/value secret.

Behavior:

- If `key` does not exist, create a new entry.
- If `key` already exists, update its `value` and `timestamp`.

Example:

```bash
secrets-manager add db_password s3cr3t
```

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

cargo run -- add db_password s3cr3t
cargo run -- get db_password
```

## Error handling (expected)

- `get <key>` returns a non-zero exit code if the key is not found.
- `add <key> <value>` returns a non-zero exit code on invalid input (e.g., missing args) or encryption/decryption failures.

## Project status

This repository currently contains the project documentation and will be extended with the actual implementation (CLI + encryption + storage parsing).
