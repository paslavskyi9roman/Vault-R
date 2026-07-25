# Contributing to Vault-R

Thanks for taking the time. Vault-R stores people's production credentials, so
the bar for changes is a little higher than for a typical app — this document
explains what that means in practice.

## Getting set up

You need a [Rust toolchain](https://rustup.rs), Node 22 (what CI builds on), and
`git` on your `PATH`. Linux also needs the Tauri system dependencies (WebKitGTK and friends);
see the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/).

```sh
npm install
npm run tauri dev
```

## Before you open a pull request

```sh
cargo test --workspace          # vault-core + vault-cli
cargo clippy --workspace --all-targets -- -D warnings
npx tsc --noEmit
npm run build                   # Rollup resolves things tsc alone does not
```

CI runs exactly these four. The git leak-guard tests build real git repositories in
temp directories, so the full suite needs `git` available — they set their own
identity per repository, so your global git config is not involved.

## Project layout

| Path                | What lives there                                          |
| ------------------- | --------------------------------------------------------- |
| `crates/vault-core` | All storage, crypto and business logic. Shared by GUI+CLI. |
| `crates/vault-cli`  | The `vault` binary.                                        |
| `src-tauri`         | Tauri shell: commands, app state, window lifecycle.        |
| `src`               | React + TypeScript front end, state in Zustand.            |

New behaviour belongs in `vault-core` wherever it can, so the GUI and the CLI
cannot drift apart. The Tauri commands in `src-tauri/src/commands.rs` should
stay thin wrappers.

## Things that need extra care

- **Never log, print, or write a secret value.** The leak-guard reports carry
  key names, paths and line numbers precisely so they are safe to paste into an
  issue. Keep it that way.
- **Schema changes are migrations.** Add a new `Migration` in
  `crates/vault-core/src/db.rs` with the next version number. Do not edit an
  existing migration — vaults in the wild have already run it.
- **Crypto changes need a written rationale** in the PR description. The
  storage format, the Argon2id parameters and the key-slot indirection are
  documented in the README; if you change them, change the README too.
- **Anything destructive prompts first**, and a prompt that cannot be answered
  counts as "no".

## Tests

Rust tests live in `crates/vault-core/tests/vault_tests.rs`. New behaviour
should come with a test; bug fixes should come with a test that fails before
the fix. Test names read as sentences describing the guarantee
(`unlinking_the_second_to_last_member_dissolves_the_group`) — please match that
style.

## Commits and pull requests

Write commit messages in the imperative mood, explaining *why* rather than
restating the diff. Keep unrelated reformatting out of feature PRs; the
codebase is not currently `rustfmt`-clean, so a blanket `cargo fmt` inside a
feature branch buries the actual change.

## Reporting security issues

Please do **not** open a public issue for a vulnerability. See
[SECURITY.md](SECURITY.md).
