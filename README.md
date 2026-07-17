# Vault-R

A developer-first, local-first secrets/`.env` vault. One place for every environment variable across
every repo and every environment, with cross-repo "linked" secrets that stay in sync everywhere they're
used, full version history, and a CLI so you never have to hand-copy a `.env` file again.

## Stack

- **GUI**: Tauri 2 + React 18 + TypeScript (Vite), state via Zustand.
- **Core**: `crates/vault-core` — all storage, crypto, and business logic. Shared by both the GUI and CLI.
- **CLI**: `crates/vault-cli` — builds the `vault` binary.

### Storage & encryption

The vault is a SQLite database (bundled, no system dependency) that is only ever written to disk as a
single AES-256-GCM encrypted blob (`vault.db.enc`, in the OS app-data directory). The encryption key is
derived from your master password via Argon2id (64 MiB / 3 iterations). While the app is unlocked, a
plaintext working copy exists in the app-data directory and is deleted again on lock/clean exit; a
leftover copy from an abnormal crash is never trusted and is discarded on the next unlock. "Remember on
this device" stores the derived key (not the password) in the OS keychain (Credential Manager / Keychain
/ Secret Service).

This is a deliberate, disclosed deviation from SQLCipher: SQLCipher's vendored build requires Perl/OpenSSL
toolchains that are painful on Windows. AES-256-GCM + Argon2id via pure-Rust crates gives the same
at-rest guarantee without that dependency.

## Getting started

```sh
npm install
npm run tauri dev
```

The first launch walks you through creating a master password and (optionally) importing your first
`.env`.

## CLI

The CLI (`vault`) reads/writes the same vault as the GUI.

```sh
cargo build -p vault-cli --release
# binary at target/release/vault(.exe)

vault init                      # first-time setup (prompts for a master password)
vault list                      # repos + environments
vault export api-gateway/local  # dotenv text to stdout (--file to write instead)
vault import api-gateway/local .env --create   # merge a file in (creates repo/env if missing)
vault run api-gateway/local -- node server.js  # inject vars into a child process, no .env file needed
vault get api-gateway/local DATABASE_URL
vault set api-gateway/local NEW_KEY=value
```

Add `--remember` to `vault init` to store the derived key in the OS keychain so later commands skip the
password prompt.

## Development

```sh
cargo test --workspace   # vault-core + vault-cli tests
cargo clippy --workspace --all-targets
npx tsc --noEmit
```

See [PLAN.md](PLAN.md) for the original design/implementation plan.
