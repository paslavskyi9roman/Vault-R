mod auth;
mod report;
mod target;

use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;
use zeroize::Zeroizing;

#[derive(Parser)]
#[command(name = "vault", about = "Vault-R: a developer-first secrets and .env vault", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

/// `--format` choices for `vault export`, mapped onto
/// [`vault_core::dotenv::ExportFormat`].
#[derive(Clone, Copy, ValueEnum)]
enum ExportFormatArg {
    Dotenv,
    Json,
    Yaml,
    Shell,
    Docker,
}

impl From<ExportFormatArg> for vault_core::dotenv::ExportFormat {
    fn from(f: ExportFormatArg) -> Self {
        match f {
            ExportFormatArg::Dotenv => Self::Dotenv,
            ExportFormatArg::Json => Self::Json,
            ExportFormatArg::Yaml => Self::Yaml,
            ExportFormatArg::Shell => Self::Shell,
            ExportFormatArg::Docker => Self::Docker,
        }
    }
}

#[derive(Subcommand)]
enum Command {
    /// Initialize a brand-new vault protected by a master password
    Init {
        /// Remember the derived key in the OS keychain so future commands skip the password prompt
        #[arg(long)]
        remember: bool,
    },
    /// List repositories and environments
    List,
    /// Print an environment's variables as .env text
    Export {
        /// <repo>/<env>; omit to resolve from the current directory's linked project
        target: Option<String>,
        /// Write to a file instead of stdout
        #[arg(long)]
        file: Option<PathBuf>,
        /// Output format (default: dotenv)
        #[arg(long, value_enum)]
        format: Option<ExportFormatArg>,
    },
    /// Merge a .env file into an environment
    Import {
        /// [<repo>/<env>] <file> -- omit the target to resolve from the current directory
        #[arg(num_args = 1..=2, required = true)]
        args: Vec<String>,
        /// Create the repo/environment if they don't exist yet
        #[arg(long)]
        create: bool,
    },
    /// Run a command with the environment's variables injected
    Run {
        /// [<repo>/<env>] -- <command> [args...] -- omit the target to resolve from the current directory
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        rest: Vec<String>,
    },
    /// Print a single variable's value
    Get {
        /// [<repo>/<env>] <key> -- omit the target to resolve from the current directory
        #[arg(num_args = 1..=2, required = true)]
        args: Vec<String>,
    },
    /// Create or update a single variable (respects linked-group propagation)
    Set {
        /// [<repo>/<env>] <KEY=VALUE> -- omit the target to resolve from the current directory
        #[arg(num_args = 1..=2, required = true)]
        args: Vec<String>,
    },
    /// Link the current directory to a repo/environment, so future commands
    /// here can omit the target
    Link {
        /// <repo>/<env>
        target: String,
    },
    /// Remove the link for the current directory
    Unlink,
    /// List linked directories
    Projects,
    /// Environment-level operations
    Env {
        #[command(subcommand)]
        command: EnvCommand,
    },
    /// Rename a repository or an environment
    Rename {
        /// <repo> or <repo>/<env>
        target: String,
        new_name: String,
    },
    /// Rename a variable's key within its environment
    Mv {
        /// <repo>/<env>
        target: String,
        key: String,
        new_key: String,
    },
    /// Delete a variable, an environment, or a whole repository
    Rm {
        /// <repo> or <repo>/<env>
        target: String,
        /// Omit to delete the repo/environment itself
        key: Option<String>,
        /// Skip the confirmation prompt (for scripts)
        #[arg(long, short = 'y')]
        yes: bool,
    },
    /// Write an encrypted copy of the vault to a file
    Backup { path: PathBuf },
    /// Replace this device's vault with a backup file
    Restore {
        path: PathBuf,
        /// Skip the confirmation prompt (for scripts)
        #[arg(long, short = 'y')]
        yes: bool,
    },
    /// Change the master password
    Passwd,
    /// Forget the data key remembered on this device (undoes `init --remember`)
    Forget,
    /// Manage the recovery kit
    Recovery {
        #[command(subcommand)]
        command: RecoveryCommand,
    },
    /// Fail (non-zero exit) if any required variable is missing or empty --
    /// a CI gate
    Check {
        /// <repo>/<env>; omit to resolve from the current directory's linked project
        target: Option<String>,
    },
    /// Compare two environments; exits non-zero if they differ (for CI gates)
    Diff {
        /// <repo>/<env>
        target_a: String,
        /// <repo>/<env>
        target_b: String,
    },
    /// Generate a random secret and print it to stdout
    Gen {
        /// Hex characters (default)
        #[arg(long, conflicts_with_all = ["base64", "alnum", "words"])]
        hex: bool,
        /// URL-safe base64 characters
        #[arg(long, conflicts_with_all = ["hex", "alnum", "words"])]
        base64: bool,
        /// Letters and digits only
        #[arg(long, conflicts_with_all = ["hex", "base64", "words"])]
        alnum: bool,
        /// A hyphen-joined passphrase
        #[arg(long, conflicts_with_all = ["hex", "base64", "alnum"])]
        words: bool,
        /// Character count (or word count with --words)
        #[arg(long)]
        length: Option<usize>,
    },
    /// Check a project directory for secrets that git can see; exits non-zero
    /// if anything is found, so it works as a pre-commit hook or a CI gate
    Scan {
        /// Directory to scan (default: the current one). Pass --linked to scan
        /// every folder registered with `vault link` instead.
        path: Option<PathBuf>,
        /// Scan every linked project directory
        #[arg(long, conflicts_with = "path")]
        linked: bool,
        /// Add the suggested patterns to .gitignore
        #[arg(long)]
        fix: bool,
    },
    /// Report placeholder, empty, stale and duplicated secrets
    Health {
        /// <repo>/<env>; omit to resolve from the current directory's linked
        /// project, or pass --all for the whole vault
        target: Option<String>,
        /// Report on every environment in the vault
        #[arg(long, conflicts_with = "target")]
        all: bool,
    },
    /// Print a shell completion script to stdout
    Completions {
        /// Which shell to generate completions for (PowerShell is the primary platform)
        shell: clap_complete::Shell,
    },
}

#[derive(Subcommand)]
enum RecoveryCommand {
    /// Create a recovery code, invalidating any previous one
    Generate,
    /// Report whether this vault has a recovery kit
    Status,
    /// Unlock with a recovery code and set a new master password
    Unlock,
}

#[derive(Subcommand)]
enum EnvCommand {
    /// Copy an environment's keys into a new one in the same repo
    Duplicate {
        /// <repo>/<env>
        target: String,
        new_env: String,
        /// Also copy values (default is to copy keys with blank values)
        #[arg(long)]
        with_values: bool,
    },
}

/// Asks for a yes/no on stdin. Returns `false` on anything but an explicit yes,
/// and on a closed stdin, so a piped invocation without `--yes` never destroys
/// anything by accident.
fn confirm(prompt: &str) -> bool {
    use std::io::Write;
    print!("{prompt} [y/N] ");
    let _ = std::io::stdout().flush();
    let mut answer = String::new();
    if std::io::stdin().read_line(&mut answer).is_err() {
        return false;
    }
    matches!(answer.trim().to_lowercase().as_str(), "y" | "yes")
}

/// Splits everything typed after `run` into an optional explicit target and
/// the child command. The first token is tried against `try_target` unless
/// it is a literal `--`; if that resolves (right shape *and* an actual
/// repo/env), it is consumed as the target. Otherwise it is left in place as
/// the command's own first argument -- so a command whose first token
/// happens to contain a slash (`node_modules/.bin/eslint`) is never silently
/// swallowed as a bogus target. A leading `--` separator, if present after
/// the target (or at the very start), is stripped either way.
fn split_run_args<T>(
    mut rest: Vec<String>,
    try_target: impl FnOnce(&str) -> Option<T>,
) -> (Option<T>, Vec<String>) {
    let explicit = if rest.first().map(String::as_str) != Some("--") {
        rest.first().and_then(|t| try_target(t))
    } else {
        None
    };
    if explicit.is_some() {
        rest.remove(0);
    }
    if rest.first().map(String::as_str) == Some("--") {
        rest.remove(0);
    }
    (explicit, rest)
}

/// Exits with the child's own exit code, or -- on Unix, where a process
/// killed by a signal reports no exit code at all -- the conventional
/// `128 + signal` shells use, so a `vault run`-wrapped command that was
/// interrupted (Ctrl+C, a timeout, etc.) is reported the same way running
/// it directly would have been.
fn exit_with_child_status(status: std::process::ExitStatus) -> ! {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(signal) = status.signal() {
            std::process::exit(128 + signal);
        }
    }
    std::process::exit(status.code().unwrap_or(1));
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    match cli.command {
        Command::Init { remember } => {
            auth::init(remember)?;
            println!("Vault initialized.");
        }
        Command::List => {
            let vault = auth::unlock()?;
            let summaries = vault.list_repo_summaries()?;
            if summaries.is_empty() {
                println!("(no repositories yet)");
            }
            for repo in summaries {
                println!("{}", repo.name);
                for env in repo.envs {
                    println!("  {}/{}  ({} vars)", repo.name, env.name, env.var_count);
                }
            }
        }
        Command::Export { target, file, format } => {
            let vault = auth::unlock()?;
            let (_, env) = target::resolve_target_or_cwd(&vault, target.as_deref(), false)?;
            let format = format.unwrap_or(ExportFormatArg::Dotenv).into();
            let text = vault.export_env_as(&env.id, format)?;
            match file {
                Some(path) => std::fs::write(&path, text)?,
                None => print!("{text}"),
            }
        }
        Command::Import { args, create } => {
            let (target, file) = match args.as_slice() {
                [file] => (None, file.clone()),
                [t, file] => (Some(t.clone()), file.clone()),
                _ => return Err("usage: vault import [<repo>/<env>] <file>".into()),
            };
            let vault = auth::unlock()?;
            let (repo, env) = target::resolve_target_or_cwd(&vault, target.as_deref(), create)?;
            let text = std::fs::read_to_string(&file)?;
            let count = vault.import_env_text(&env.id, &text)?;
            println!("Imported {count} variable(s) into {}/{}.", repo.name, env.name);
        }
        Command::Run { rest } => {
            let vault = auth::unlock()?;
            let (explicit, rest) =
                split_run_args(rest, |t| target::resolve_env(&vault, t, false).ok());
            let (_, env) = match explicit {
                Some(found) => found,
                None => target::resolve_cwd(&vault)?,
            };
            let Some((program, args)) = rest.split_first() else {
                return Err("usage: vault run [<repo>/<env>] -- <command> [args...]".into());
            };
            let vars = vault.list_variables(&env.id)?;
            // Inherits stdin/stdout/stderr (the default for `.status()`), so
            // the child's output streams straight through with no buffering.
            let status = std::process::Command::new(program)
                .args(args)
                .envs(vars.into_iter().map(|v| (v.key, v.value)))
                .status()?;
            exit_with_child_status(status);
        }
        Command::Get { args } => {
            let (target, key) = match args.as_slice() {
                [key] => (None, key.clone()),
                [t, key] => (Some(t.clone()), key.clone()),
                _ => return Err("usage: vault get [<repo>/<env>] <key>".into()),
            };
            let vault = auth::unlock()?;
            let (repo, env) = target::resolve_target_or_cwd(&vault, target.as_deref(), false)?;
            let vars = vault.list_variables(&env.id)?;
            match vars.into_iter().find(|v| v.key == key) {
                Some(v) => println!("{}", v.value),
                None => return Err(format!("no such key '{key}' in {}/{}", repo.name, env.name).into()),
            }
        }
        Command::Set { args } => {
            let (target, assignment) = match args.as_slice() {
                [assignment] => (None, assignment.clone()),
                [t, assignment] => (Some(t.clone()), assignment.clone()),
                _ => return Err("usage: vault set [<repo>/<env>] <KEY=VALUE>".into()),
            };
            let vault = auth::unlock()?;
            let (repo, env) = target::resolve_target_or_cwd(&vault, target.as_deref(), true)?;
            let (key, value) = assignment.split_once('=').ok_or("expected KEY=VALUE")?;
            let existing = vault
                .list_variables(&env.id)?
                .into_iter()
                .find(|v| v.key == key);
            match existing {
                Some(v) => vault.update_variable_value(&v.id, value)?,
                None => {
                    vault.add_variable(&env.id, key, value)?;
                }
            }
            println!("Set {key} in {}/{}.", repo.name, env.name);
        }
        Command::Link { target } => {
            let vault = auth::unlock()?;
            let (repo, env) = target::resolve_env(&vault, &target, false)?;
            let cwd = std::env::current_dir()?;
            vault.link_project(&cwd, &env.id)?;
            println!("Linked {} to {}/{}.", cwd.display(), repo.name, env.name);
        }
        Command::Unlink => {
            let vault = auth::unlock()?;
            let cwd = std::env::current_dir()?;
            vault.unlink_project(&cwd)?;
            println!("Unlinked {}.", cwd.display());
        }
        Command::Projects => {
            let vault = auth::unlock()?;
            let projects = vault.list_projects()?;
            if projects.is_empty() {
                println!("(no linked projects yet)");
            }
            for p in projects {
                println!("{}  ->  {}/{}", p.path, p.repo_name, p.env_name);
            }
        }
        Command::Env { command } => match command {
            EnvCommand::Duplicate { target, new_env, with_values } => {
                let vault = auth::unlock()?;
                let (repo, env) = target::resolve_env(&vault, &target, false)?;
                vault.duplicate_environment(&env.id, &new_env, with_values)?;
                println!("Duplicated {}/{} to {}/{}.", repo.name, env.name, repo.name, new_env);
            }
        },
        Command::Rename { target, new_name } => {
            let vault = auth::unlock()?;
            let (repo, env) = target::resolve_repo_or_env(&vault, &target)?;
            match env {
                Some(env) => {
                    vault.rename_environment(&env.id, &new_name)?;
                    println!("Renamed {}/{} to {}/{}.", repo.name, env.name, repo.name, new_name);
                }
                None => {
                    vault.rename_repo(&repo.id, &new_name)?;
                    println!("Renamed {} to {}.", repo.name, new_name);
                }
            }
        }
        Command::Mv { target, key, new_key } => {
            let vault = auth::unlock()?;
            let (_, env) = target::resolve_env(&vault, &target, false)?;
            let var = vault
                .list_variables(&env.id)?
                .into_iter()
                .find(|v| v.key == key)
                .ok_or_else(|| format!("no such key '{key}' in {target}"))?;
            vault.rename_variable_key(&var.id, &new_key)?;
            println!("Renamed {key} to {new_key} in {target}.");
        }
        Command::Rm { target, key, yes } => {
            let vault = auth::unlock()?;
            let (repo, env) = target::resolve_repo_or_env(&vault, &target)?;
            match (env, key) {
                (Some(env), Some(key)) => {
                    let var = vault
                        .list_variables(&env.id)?
                        .into_iter()
                        .find(|v| v.key == key)
                        .ok_or_else(|| format!("no such key '{key}' in {target}"))?;
                    if !yes && !confirm(&format!("Delete {key} from {target}?")) {
                        return Err("aborted".into());
                    }
                    vault.delete_variable(&var.id)?;
                    println!("Deleted {key} from {target}.");
                }
                (Some(env), None) => {
                    let count = vault.list_variables(&env.id)?.len();
                    if !yes
                        && !confirm(&format!(
                            "Delete environment {}/{} and its {count} variable(s)?",
                            repo.name, env.name
                        ))
                    {
                        return Err("aborted".into());
                    }
                    vault.delete_environment(&env.id)?;
                    println!("Deleted {}/{}.", repo.name, env.name);
                }
                (None, Some(_)) => {
                    return Err("to delete a variable, name its environment: <repo>/<env>".into())
                }
                (None, None) => {
                    let envs = vault.list_environments(&repo.id)?.len();
                    if !yes
                        && !confirm(&format!(
                            "Delete repository {} and its {envs} environment(s)?",
                            repo.name
                        ))
                    {
                        return Err("aborted".into());
                    }
                    vault.delete_repo(&repo.id)?;
                    println!("Deleted {}.", repo.name);
                }
            }
        }
        Command::Backup { path } => {
            let vault = auth::unlock()?;
            vault.export_backup(&path)?;
            println!("Wrote an encrypted backup to {}.", path.display());
        }
        Command::Restore { path, yes } => {
            // Deliberately does not unlock first: restoring swaps the file the
            // vault lives in, so there must be nothing open on top of it.
            if !yes
                && !confirm(&format!(
                    "Replace this device's vault with {}? A copy of the current one is kept.",
                    path.display()
                ))
            {
                return Err("aborted".into());
            }
            vault_core::backup::restore_backup(&path)?;
            println!("Restored. Unlock with the master password that protected that backup.");
        }
        Command::Passwd => {
            let mut vault = auth::unlock()?;
            let current = Zeroizing::new(rpassword::prompt_password("Current master password: ")?);
            let new = Zeroizing::new(rpassword::prompt_password("New master password: ")?);
            let confirmation =
                Zeroizing::new(rpassword::prompt_password("Confirm new master password: ")?);
            if *new != *confirmation {
                return Err("passwords do not match".into());
            }
            vault.change_password(&current, &new)?;
            println!("Master password changed.");
        }
        Command::Forget => {
            auth::forget()?;
            println!("Forgot the remembered key. The next command will ask for the master password.");
        }
        Command::Recovery { command } => match command {
            RecoveryCommand::Status => {
                let vault = auth::unlock()?;
                if vault.needs_migration() {
                    println!(
                        "This vault is in the legacy format. Unlock it once with \
                         `vault list` and your master password to upgrade it."
                    );
                } else if vault.has_recovery_code() {
                    println!("This vault has a recovery kit.");
                } else {
                    println!("No recovery kit. Run `vault recovery generate` to create one.");
                }
            }
            RecoveryCommand::Unlock => {
                let code = Zeroizing::new(rpassword::prompt_password("Recovery code: ")?);
                let mut vault = vault_core::Vault::open_with_recovery(&code)?;
                println!("Recovery code accepted. Choose a new master password.");
                let new = Zeroizing::new(rpassword::prompt_password("New master password: ")?);
                let confirmation =
                    Zeroizing::new(rpassword::prompt_password("Confirm new master password: ")?);
                if *new != *confirmation {
                    return Err("passwords do not match".into());
                }
                vault.reset_password(&new)?;
                println!("Master password set. Your recovery code still works.");
            }
            RecoveryCommand::Generate => {
                let mut vault = auth::unlock()?;
                if vault.has_recovery_code()
                    && !confirm("Replace the existing recovery kit? The old code stops working.")
                {
                    return Err("aborted".into());
                }
                let code = vault.generate_recovery_code()?;
                println!("Recovery code: {code}");
                println!();
                println!("Save this somewhere safe and offline. It will not be shown again,");
                println!("and anyone holding it can read every secret in this vault.");
            }
        },
        Command::Check { target } => {
            let vault = auth::unlock()?;
            let (repo, env) = target::resolve_target_or_cwd(&vault, target.as_deref(), false)?;
            let missing = vault.required_and_empty(&env.id)?;
            if missing.is_empty() {
                println!("All required variables are set in {}/{}.", repo.name, env.name);
                return Ok(());
            }
            eprintln!("Missing required variable(s) in {}/{}:", repo.name, env.name);
            for v in &missing {
                eprintln!("  {}", v.key);
            }
            std::process::exit(1);
        }
        Command::Diff { target_a, target_b } => {
            let vault = auth::unlock()?;
            let (_, env_a) = target::resolve_env(&vault, &target_a, false)?;
            let (_, env_b) = target::resolve_env(&vault, &target_b, false)?;
            let rows = vault.diff_environments(&env_a.id, &env_b.id)?;
            if rows.is_empty() {
                println!("{target_a} and {target_b} match.");
                return Ok(());
            }
            for row in &rows {
                match row.kind.as_str() {
                    "added" => println!(
                        "+ {} (only in {target_b}) = {}",
                        row.key,
                        row.new_value.as_deref().unwrap_or("")
                    ),
                    "removed" => println!(
                        "- {} (only in {target_a}) = {}",
                        row.key,
                        row.old_value.as_deref().unwrap_or("")
                    ),
                    "changed" => println!(
                        "~ {} : {} -> {}",
                        row.key,
                        row.old_value.as_deref().unwrap_or(""),
                        row.new_value.as_deref().unwrap_or("")
                    ),
                    _ => {}
                }
            }
            std::process::exit(1);
        }
        Command::Gen { hex: _, base64, alnum, words, length } => {
            let kind = if base64 {
                vault_core::crypto::SecretKind::Base64Url
            } else if alnum {
                vault_core::crypto::SecretKind::Alphanumeric
            } else if words {
                vault_core::crypto::SecretKind::Passphrase
            } else {
                vault_core::crypto::SecretKind::Hex
            };
            let default_len = if words { 5 } else { 32 };
            let secret = vault_core::crypto::generate_secret(kind, length.unwrap_or(default_len))?;
            println!("{secret}");
        }
        Command::Scan { path, linked, fix } => {
            let vault = auth::unlock()?;
            let reports = if linked {
                let reports = vault.scan_linked_projects()?;
                if reports.is_empty() {
                    println!("(no linked projects yet — run `vault link <repo>/<env>` in one)");
                }
                reports
            } else {
                let dir = match path {
                    Some(p) => p,
                    None => std::env::current_dir()?,
                };
                vec![vault.scan_directory(&dir)?]
            };

            let mut any_findings = false;
            let mut needs_rotation = false;
            for report in &reports {
                print!("{}", report::leak_report_text(report));
                any_findings |= report.has_findings();
                needs_rotation |= report.findings.iter().any(|f| f.needs_rotation);
                if fix {
                    apply_fix(report)?;
                }
            }

            // Without --fix, any finding is a failure. With it, the .gitignore
            // write has handled everything *except* what is already committed,
            // and that genuinely still needs a human: rotate the credential.
            if if fix { needs_rotation } else { any_findings } {
                std::process::exit(1);
            }
        }
        Command::Health { target, all } => {
            let vault = auth::unlock()?;
            let (label, report) = if all {
                ("this vault".to_string(), vault.health_report()?)
            } else {
                let (repo, env) = target::resolve_target_or_cwd(&vault, target.as_deref(), false)?;
                let label = format!("{}/{}", repo.name, env.name);
                let report = vault.health_report_for_env(&env.id)?;
                (label, report)
            };
            print!("{}", report::health_report_text(&label, &report));
        }
        Command::Completions { shell } => {
            let mut cmd = Cli::command();
            let name = cmd.get_name().to_string();
            clap_complete::generate(shell, &mut cmd, name, &mut std::io::stdout());
        }
    }
    Ok(())
}

/// Applies a report's suggested `.gitignore` patterns, and says plainly what
/// that did *not* do — a scan that leaves the user believing a committed
/// secret is now safe would be worse than not offering the fix at all.
fn apply_fix(report: &vault_core::gitguard::LeakReport) -> Result<(), Box<dyn std::error::Error>> {
    let Some(root) = &report.git_root else {
        return Ok(());
    };
    let patterns = report.suggested_patterns();
    if patterns.is_empty() {
        return Ok(());
    }
    let added =
        vault_core::gitguard::apply_gitignore_patterns(std::path::Path::new(root), &patterns)?;
    println!("  Added {added} pattern(s) to {root}/.gitignore.");
    if report.findings.iter().any(|f| f.needs_rotation) {
        println!(
            "  This stops future commits only. Anything already tracked is still in the \
             repository's history — untrack it and rotate those credentials."
        );
    }
    Ok(())
}

#[cfg(test)]
mod arg_parsing_tests {
    use super::*;

    fn strs(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn run_with_explicit_separator_and_no_target() {
        let (explicit, cmd): (Option<()>, _) = split_run_args(strs(&["--", "npm", "start"]), |_| None);
        assert!(explicit.is_none());
        assert_eq!(cmd, strs(&["npm", "start"]));
    }

    #[test]
    fn run_with_an_explicit_target_that_resolves() {
        let rest = strs(&["api-gateway/local", "--", "npm", "start"]);
        let (explicit, cmd) =
            split_run_args(rest, |t| (t == "api-gateway/local").then(|| t.to_string()));
        assert_eq!(explicit.as_deref(), Some("api-gateway/local"));
        assert_eq!(cmd, strs(&["npm", "start"]));
    }

    #[test]
    fn run_with_no_target_and_no_separator() {
        let (explicit, cmd): (Option<()>, _) = split_run_args(strs(&["npm", "start"]), |_| None);
        assert!(explicit.is_none());
        assert_eq!(cmd, strs(&["npm", "start"]));
    }

    #[test]
    fn a_slash_shaped_first_command_token_is_not_mistaken_for_a_target() {
        // no resolver ever matches -- simulates "no such repo/env" -- so a
        // command whose own first token contains a slash must fall through
        // to the command rather than being silently swallowed as a target.
        let rest = strs(&["node_modules/.bin/eslint", "."]);
        let (explicit, cmd): (Option<()>, _) = split_run_args(rest, |_| None);
        assert!(explicit.is_none());
        assert_eq!(cmd, strs(&["node_modules/.bin/eslint", "."]));
    }

    #[test]
    fn cli_parses_run_with_no_arguments_at_all() {
        let cli = Cli::try_parse_from(["vault", "run"]).unwrap();
        assert!(matches!(cli.command, Command::Run { rest } if rest.is_empty()));
    }

    #[test]
    fn cli_parses_get_with_one_or_two_positional_args() {
        let cli = Cli::try_parse_from(["vault", "get", "KEY"]).unwrap();
        assert!(matches!(cli.command, Command::Get { args } if args == strs(&["KEY"])));

        let cli = Cli::try_parse_from(["vault", "get", "repo/env", "KEY"]).unwrap();
        assert!(matches!(cli.command, Command::Get { args } if args == strs(&["repo/env", "KEY"])));

        assert!(Cli::try_parse_from(["vault", "get"]).is_err());
        assert!(Cli::try_parse_from(["vault", "get", "a", "b", "c"]).is_err());
    }

    #[test]
    fn cli_parses_set_and_import_with_one_or_two_positional_args() {
        let cli = Cli::try_parse_from(["vault", "set", "KEY=value"]).unwrap();
        assert!(matches!(cli.command, Command::Set { args } if args == strs(&["KEY=value"])));

        let cli = Cli::try_parse_from(["vault", "import", "repo/env", "file.env"]).unwrap();
        assert!(matches!(
            cli.command,
            Command::Import { args, create: false } if args == strs(&["repo/env", "file.env"])
        ));
    }

    #[test]
    fn cli_parses_scan_with_a_path_or_linked_but_not_both() {
        let cli = Cli::try_parse_from(["vault", "scan"]).unwrap();
        assert!(matches!(
            cli.command,
            Command::Scan { path: None, linked: false, fix: false }
        ));

        let cli = Cli::try_parse_from(["vault", "scan", "--linked", "--fix"]).unwrap();
        assert!(matches!(
            cli.command,
            Command::Scan { path: None, linked: true, fix: true }
        ));

        // a directory to scan and "scan every linked directory" are different
        // questions; accepting both would have to silently ignore one
        assert!(Cli::try_parse_from(["vault", "scan", ".", "--linked"]).is_err());
    }

    #[test]
    fn cli_parses_health_with_a_target_or_all_but_not_both() {
        let cli = Cli::try_parse_from(["vault", "health"]).unwrap();
        assert!(matches!(cli.command, Command::Health { target: None, all: false }));

        let cli = Cli::try_parse_from(["vault", "health", "repo/env"]).unwrap();
        assert!(matches!(cli.command, Command::Health { target: Some(t), .. } if t == "repo/env"));

        assert!(Cli::try_parse_from(["vault", "health", "repo/env", "--all"]).is_err());
    }

    #[test]
    fn cli_parses_export_with_an_optional_target() {
        let cli = Cli::try_parse_from(["vault", "export"]).unwrap();
        assert!(matches!(cli.command, Command::Export { target: None, file: None, .. }));

        let cli = Cli::try_parse_from(["vault", "export", "repo/env"]).unwrap();
        assert!(matches!(cli.command, Command::Export { target: Some(t), .. } if t == "repo/env"));
    }
}
