# Security Policy

Vault-R stores credentials. If you believe you have found a vulnerability,
please report it privately so it can be fixed before it is public.

## Reporting a vulnerability

**Please do not open a public GitHub issue.**

Use GitHub's private vulnerability reporting: go to the **Security** tab of
this repository and choose **Report a vulnerability**. That opens a private
advisory visible only to the maintainers.

Please include what you were doing, what you observed, and — if you have one —
a minimal reproduction. **Never include a real secret value** in a report; key
names, file paths and line numbers are enough.

You can expect an initial response within a week. If a report is confirmed, a
fix and an advisory will be published together.

## Scope

In scope:

- Recovering vault contents without the master password or a recovery code.
- Weaknesses in the on-disk format, the key-slot design, or the Argon2id
  parameters described in the README.
- Secret values escaping where they should not go: log output, crash dumps,
  the clipboard past its intended lifetime, leak-guard reports, or any file
  written in plaintext.
- Anything that lets a cloned repository opt itself into your secrets.
- Compromise of the webview leading to secret disclosure.

Out of scope:

- An attacker who already has your unlocked vault and an active session.
- Physical access to an unlocked machine with the vault unlocked.
- Rewriting git history for already-committed secrets — this is deliberately
  not a feature; committed credentials must be rotated.
- Findings that require you to disable the app's own safety prompts.

## Known design decisions

These are deliberate and documented, not oversights:

- Vault-R uses AES-256-GCM with Argon2id key derivation rather than SQLCipher.
  The reasoning is in the README.
- "Remember on this device" stores the *data key* in the OS keychain, not the
  master password. Anyone able to read that keychain entry can open the vault.
- Automatic backups are copies of the already-encrypted file. A backup opens
  with whatever master password was set when the copy was taken.
