# Vault-R

**One place for every environment variable, across every repo and every environment — encrypted on
your own machine, with a CLI so you never hand-copy a `.env` file again.**

[![CI](https://github.com/paslavskyi9roman/Vault-R/actions/workflows/ci.yml/badge.svg)](https://github.com/paslavskyi9roman/Vault-R/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

<!--
TODO: add a screenshot and a short demo GIF, then uncomment. This is the single
highest-impact thing on the page — most people decide here.

  Suggested captures:
    docs/screenshots/main.png     the variables table with an environment selected
    docs/screenshots/safety.png   the Safety panel showing a git leak finding
    docs/screenshots/demo.gif     `vault link` -> `vault run -- node server.js`

![Vault-R](docs/screenshots/main.png)
-->

## Why another secrets tool?

Most secrets managers are a hosted service: your credentials live on someone else's machine and you
pay per seat. Most local alternatives are general-purpose password managers that know nothing about
`repo/environment` layouts or `.env` files.

Vault-R is the other combination — **local-first and built specifically for developer secrets**:

- **Nothing leaves your machine.** The vault is a single encrypted file in your OS app-data
  directory. There is no account, no server, no telemetry.
- **Cross-repo linked secrets.** The same `DATABASE_URL` used by three services stays in sync
  everywhere it is used; edit it once.
- **It tells you what is already broken.** The Safety panel asks git what it can actually see —
  committed `.env` files, vault values pasted into tracked files, `.env` files `.gitignore` misses.
- **A real CLI.** `vault run -- node server.js` injects variables into a child process, so a `.env`
  file never has to exist on disk.
- **Full version history**, per environment, with one-click restore.

## Install

> **Note:** prebuilt binaries are not published yet — build from source for now.

```sh
git clone https://github.com/paslavskyi9roman/Vault-R.git
cd Vault-R
npm install
npm run tauri dev
```

The first launch walks you through creating a master password and, optionally, importing your first
`.env`.

Building the CLI:

```sh
cargo build -p vault-cli --release   # binary at target/release/vault(.exe)
```

Linux needs the [Tauri system dependencies](https://v2.tauri.app/start/prerequisites/) (WebKitGTK
and friends) for the desktop app. The CLI has no such requirement.

## Quick start with the CLI

```sh
vault init                      # first-time setup (prompts for a master password)
vault import api-gateway/local .env --create   # pull an existing file in

cd ~/code/api-gateway
vault link api-gateway/local    # bind this directory to that repo/environment
vault run -- node server.js     # run with the variables injected, no .env on disk
```

Because the directory→environment mapping lives *in the vault* and never in a file inside the
directory, a cloned repo can never opt itself into your secrets.

## What it does

### Safety checks

The *Safety* panel (and `vault scan` / `vault health`) answers a question you didn't ask: what is
already wrong with the secrets you have?

- **Git leak guard.** For every directory linked with `vault link`, Vault-R asks git what it can
  see: a tracked `.env` file, a vault value pasted verbatim into a tracked file, or an untracked
  `.env` that `.gitignore` does not cover. One click adds the missing `.gitignore` patterns.

  Two things it deliberately does *not* do. It never puts a secret **value** in a finding — reports
  carry key names, paths and line numbers, so they are safe to screenshot or paste into an issue.
  And it never pretends `.gitignore` fixed a commit: anything already tracked is in your history, so
  the finding says plainly that the credential is compromised and needs rotating (`git rm --cached`
  is your next step). Rewriting history is out of scope, permanently.

- **Secret health.** Empty values, placeholders (`changeme`, `your-api-key`, `xxx`), values
  untouched for more than 90 days, and per-variable rotation policies that have elapsed. It also
  finds identical values stored in different environments *without* being linked, and offers to link
  them in one click.

Both err heavily toward silence — trivial values, plain configuration words like `production`, and
`.env.example` files are excluded — because a scanner that cries wolf gets turned off once and never
turned back on.

### Backups and recovery

- **Automatic backups.** The last 10 copies of the vault file are kept in `backups/` next to it,
  written on every unlock and before anything destructive. They are copies of the encrypted file —
  no new crypto, and nothing is ever written in plaintext.
- **Manual backups.** *Settings → Export encrypted backup*, or `vault backup <path>`. A backup opens
  with the master password it had *when the copy was taken*.
- **Recovery kit.** *Settings → Create recovery kit* generates a one-time code that unlocks the
  vault without the master password. It is shown once and stored only in wrapped form. Losing your
  password without a kit means losing the vault.
- **Auto-lock.** The vault locks itself after a configurable idle period (default 15 minutes), and
  locking reclaims any secret still on the clipboard.

## How it works

### Stack

- **GUI**: Tauri 2 + React 18 + TypeScript (Vite), state via Zustand.
- **Core**: `crates/vault-core` — all storage, crypto, and business logic. Shared by both the GUI
  and CLI.
- **CLI**: `crates/vault-cli` — builds the `vault` binary.

### Storage & encryption

The vault is a SQLite database (bundled, no system dependency) that exists in the clear only in
memory. On disk it is a single self-describing file, `vault.db.enc` in the OS app-data directory:

```
VAULT-R2 | header length | header (JSON) | AES-256-GCM(data key, SQLite image)
```

The database is encrypted under a random 32-byte **data key**. The header holds a *key slot* per way
of unlocking the vault — one for the master password, optionally one for a recovery code — each
storing the data key wrapped under `Argon2id(secret)` (64 MiB / 3 iterations, per-slot salt). That
indirection is what makes changing the master password a rewrite of one slot rather than a
re-encryption of every secret, and what lets a recovery code exist at all.

"Remember on this device" stores the data key (never the password) in the OS keychain (Credential
Manager / Keychain / Secret Service). Because it is the data key, a remembered device keeps working
after a password change.

This is a deliberate, disclosed deviation from SQLCipher: SQLCipher's vendored build requires
Perl/OpenSSL toolchains that are painful on Windows. AES-256-GCM + Argon2id via pure-Rust crates
gives the same at-rest guarantee without that dependency.

Vaults created before this format are upgraded automatically the first time they are unlocked with a
master password, and their plaintext `vault.meta.json` sidecar is removed.

## CLI reference

The CLI (`vault`) reads and writes the same vault as the GUI.

```sh
vault list                      # repos + environments
vault export api-gateway/local  # dotenv text to stdout (--file to write instead)
vault export api-gateway/local --format json   # or yaml | shell | docker
vault import api-gateway/local .env --create   # merge a file in (creates repo/env if missing)
vault run api-gateway/local -- node server.js  # inject vars into a child process
vault get api-gateway/local DATABASE_URL
vault set api-gateway/local NEW_KEY=value
vault check api-gateway/local   # non-zero exit if a required variable is missing/empty (CI gate)
vault diff api-gateway/local api-gateway/staging  # non-zero exit if the two environments differ

# Linked directories
vault link api-gateway/local    # run from inside the project directory
vault run -- node server.js     # resolves from the linked project
vault unlink                    # remove the link for the current directory
vault projects                  # list every linked directory

# Safety checks. `scan` exits non-zero when it finds anything, so it works as a
# pre-commit hook or a CI gate.
vault scan                      # is anything in this repo visible to git?
vault scan --linked             # …across every linked directory
vault scan --fix                # also add the suggested .gitignore patterns
vault health api-gateway/local  # empty, placeholder, stale and duplicated secrets
vault health --all              # …across the whole vault

vault gen --hex --length 32     # random secret to stdout (--base64 | --alnum | --words)
vault env duplicate api-gateway/local staging --with-values  # copy an env's keys

vault rename api-gateway billing-api        # rename a repo…
vault rename api-gateway/local dev          # …or an environment
vault mv api-gateway/local OLD_KEY NEW_KEY  # rename a variable's key
vault rm api-gateway/local STALE_KEY        # delete a variable
vault rm api-gateway/staging                # delete an environment (prompts)
vault rm api-gateway --yes                  # delete a repo, no prompt (for scripts)

vault backup ./vault-backup.vrbackup        # encrypted copy
vault restore ./vault-backup.vrbackup       # replace this device's vault
vault passwd                                # change the master password
vault recovery generate                     # create a recovery kit
vault recovery unlock                       # forgot the password? use the kit
vault recovery status

vault completions powershell > vault-completion.ps1   # or bash | zsh | fish | elvish
```

Add `--remember` to `vault init` to store the data key in the OS keychain so later commands skip the
password prompt.

Every destructive command prompts before acting; pass `--yes` (`-y`) to skip the prompt in scripts.
A prompt that cannot be answered — a closed stdin, for instance — counts as "no".

`run`, `export`, `import`, `get`, `set`, `check` and `health` all accept an optional `<repo>/<env>`
target; when it is omitted they resolve from the current directory via `vault link` (walking up to
the nearest linked ancestor), and fail with an actionable message if nothing here was ever linked.

`vault scan` needs `git` on your `PATH` — it asks git what is tracked rather than guessing — and
says so rather than reporting a clean repository if git is missing. With `--fix` it exits non-zero
only when a finding is already committed, since those need a rotation that no `.gitignore` edit can
substitute for.

## Development

```sh
cargo test --workspace   # vault-core + vault-cli tests
cargo clippy --workspace --all-targets -- -D warnings
npx tsc --noEmit
npm run build
```

The leak-guard tests build real git repositories in temp directories, so `git` must be on your
`PATH` to run the full suite.

See [CONTRIBUTING.md](CONTRIBUTING.md) before opening a pull request.

## Security

Vault-R stores credentials; please report vulnerabilities privately rather than as a public issue.
See [SECURITY.md](SECURITY.md) for the process and for the scope of what counts.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in
this work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.
