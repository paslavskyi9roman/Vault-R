# Vault-R

A developer-first, local-first secrets/`.env` vault. One place for every environment variable across
every repo and every environment, with cross-repo "linked" secrets that stay in sync everywhere they're
used, full version history, and a CLI so you never have to hand-copy a `.env` file again.

## Stack

- **GUI**: Tauri 2 + React 18 + TypeScript (Vite), state via Zustand.
- **Core**: `crates/vault-core` — all storage, crypto, and business logic. Shared by both the GUI and CLI.
- **CLI**: `crates/vault-cli` — builds the `vault` binary.

### Storage & encryption

The vault is a SQLite database (bundled, no system dependency) that exists in the clear only in memory.
On disk it is a single self-describing file, `vault.db.enc` in the OS app-data directory:

```
VAULT-R2 | header length | header (JSON) | AES-256-GCM(data key, SQLite image)
```

The database is encrypted under a random 32-byte **data key**. The header holds a *key slot* per way of
unlocking the vault — one for the master password, optionally one for a recovery code — each storing the
data key wrapped under `Argon2id(secret)` (64 MiB / 3 iterations, per-slot salt). That indirection is what
makes changing the master password a rewrite of one slot rather than a re-encryption of every secret, and
what lets a recovery code exist at all.

"Remember on this device" stores the data key (never the password) in the OS keychain (Credential Manager
/ Keychain / Secret Service). Because it is the data key, a remembered device keeps working after a
password change.

This is a deliberate, disclosed deviation from SQLCipher: SQLCipher's vendored build requires Perl/OpenSSL
toolchains that are painful on Windows. AES-256-GCM + Argon2id via pure-Rust crates gives the same
at-rest guarantee without that dependency.

Vaults created before this format are upgraded automatically the first time they are unlocked with a
master password, and their plaintext `vault.meta.json` sidecar is removed.

### Backups and recovery

- **Automatic backups.** The last 10 copies of the vault file are kept in `backups/` next to it, written
  on every unlock and before anything destructive. They are copies of the encrypted file — no new crypto,
  and nothing is ever written in plaintext.
- **Manual backups.** *Settings → Export encrypted backup*, or `vault backup <path>`. A backup opens with
  the master password it had *when the copy was taken*.
- **Recovery kit.** *Settings → Create recovery kit* generates a one-time code that unlocks the vault
  without the master password. It is shown once and stored only in wrapped form. Losing your password
  without a kit means losing the vault.
- **Auto-lock.** The vault locks itself after a configurable idle period (default 15 minutes), and locking
  reclaims any secret still on the clipboard.

### Safety checks

The *Safety* panel (and `vault scan` / `vault health`) answers a question you didn't ask: what is already
wrong with the secrets you have?

- **Git leak guard.** For every directory linked with `vault link`, Vault-R asks git what it can see: a
  tracked `.env` file, a vault value pasted verbatim into a tracked file, or an untracked `.env` that
  `.gitignore` does not cover. One click adds the missing `.gitignore` patterns.

  Two things it deliberately does *not* do. It never puts a secret **value** in a finding — reports carry
  key names, paths and line numbers, so they are safe to screenshot or paste into an issue. And it never
  pretends `.gitignore` fixed a commit: anything already tracked is in your history, so the finding says
  plainly that the credential is compromised and needs rotating (`git rm --cached` is your next step).
  Rewriting history is out of scope, permanently.

- **Secret health.** Empty values, placeholders (`changeme`, `your-api-key`, `xxx`), values untouched for
  more than 90 days, and per-variable rotation policies that have elapsed. It also finds identical values
  stored in different environments *without* being linked, and offers to link them in one click.

Both err heavily toward silence — trivial values, plain configuration words like `production`, and
`.env.example` files are excluded — because a scanner that cries wolf gets turned off once and never
turned back on.

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
vault export api-gateway/local --format json   # or yaml | shell | docker
vault import api-gateway/local .env --create   # merge a file in (creates repo/env if missing)
vault run api-gateway/local -- node server.js  # inject vars into a child process, no .env file needed
vault get api-gateway/local DATABASE_URL
vault set api-gateway/local NEW_KEY=value
vault check api-gateway/local   # non-zero exit if a required variable is missing/empty (CI gate)
vault diff api-gateway/local api-gateway/staging  # non-zero exit if the two environments differ

# Link a directory to a repo/environment so the target argument above can be
# omitted entirely -- the mapping lives in the vault, never in a file inside
# the directory, so a cloned repo can never opt itself into your secrets.
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
vault env duplicate api-gateway/local staging --with-values  # copy an env's keys (blank values by default)

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

Every destructive command prompts before acting; pass `--yes` (`-y`) to skip the prompt in scripts. A
prompt that cannot be answered — a closed stdin, for instance — counts as "no".

`run`, `export`, `import`, `get`, `set`, `check` and `health` all accept an optional `<repo>/<env>` target;
when it is omitted they resolve from the current directory via `vault link` (walking up to the nearest
linked ancestor), and fail with an actionable message if nothing here was ever linked.

`vault scan` needs `git` on your PATH — it asks git what is tracked rather than guessing — and says so
rather than reporting a clean repository if git is missing. With `--fix` it exits non-zero only when a
finding is already committed, since those need a rotation that no `.gitignore` edit can substitute for.

## Development

```sh
cargo test --workspace   # vault-core + vault-cli tests
cargo clippy --workspace --all-targets
npx tsc --noEmit
```

The leak-guard tests build real git repositories in temp directories, so `git` must be on your PATH to
run the full suite.

See [.vscode/PLAN.md](.vscode/PLAN.md) for the original design/implementation plan and
[.vscode/PHASE2.md](.vscode/PHASE2.md) for the plan that added the commands above.
