## What does this change?

<!-- What it does and, more usefully, why. -->

## How was it tested?

<!--
Which of these did you run, and did you add a test?

    cargo test --workspace
    cargo clippy --workspace --all-targets -- -D warnings
    npx tsc --noEmit
-->

## Checklist

- [ ] No secret value is logged, printed, or written in plaintext by this change
- [ ] Any schema change is a **new** migration, not an edit to an existing one
- [ ] Destructive behaviour still prompts (and an unanswerable prompt means "no")
- [ ] The README is updated if this changes the storage format, crypto, or CLI
- [ ] Unrelated reformatting is kept out of this PR
