mod auth;
mod target;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "vault", about = "Vault-R: a developer-first secrets and .env vault", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
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
        /// <repo>/<env>
        target: String,
        /// Write to a file instead of stdout
        #[arg(long)]
        file: Option<PathBuf>,
    },
    /// Merge a .env file into an environment
    Import {
        /// <repo>/<env>
        target: String,
        file: PathBuf,
        /// Create the repo/environment if they don't exist yet
        #[arg(long)]
        create: bool,
    },
    /// Run a command with the environment's variables injected
    Run {
        /// <repo>/<env>
        target: String,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        cmd: Vec<String>,
    },
    /// Print a single variable's value
    Get {
        /// <repo>/<env>
        target: String,
        key: String,
    },
    /// Create or update a single variable (respects linked-group propagation)
    Set {
        /// <repo>/<env>
        target: String,
        /// KEY=VALUE
        assignment: String,
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
    /// Manage the recovery kit
    Recovery {
        #[command(subcommand)]
        command: RecoveryCommand,
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
        Command::Export { target, file } => {
            let vault = auth::unlock()?;
            let (_, env) = target::resolve_env(&vault, &target, false)?;
            let text = vault.export_env_text(&env.id)?;
            match file {
                Some(path) => std::fs::write(&path, text)?,
                None => print!("{text}"),
            }
        }
        Command::Import { target, file, create } => {
            let vault = auth::unlock()?;
            let (_, env) = target::resolve_env(&vault, &target, create)?;
            let text = std::fs::read_to_string(&file)?;
            let count = vault.import_env_text(&env.id, &text)?;
            println!("Imported {count} variable(s) into {target}.");
        }
        Command::Run { target, mut cmd } => {
            let vault = auth::unlock()?;
            let (_, env) = target::resolve_env(&vault, &target, false)?;
            if cmd.first().map(String::as_str) == Some("--") {
                cmd.remove(0);
            }
            let Some((program, args)) = cmd.split_first() else {
                return Err("usage: vault run <repo>/<env> -- <command> [args...]".into());
            };
            let vars = vault.list_variables(&env.id)?;
            let status = std::process::Command::new(program)
                .args(args)
                .envs(vars.into_iter().map(|v| (v.key, v.value)))
                .status()?;
            std::process::exit(status.code().unwrap_or(1));
        }
        Command::Get { target, key } => {
            let vault = auth::unlock()?;
            let (_, env) = target::resolve_env(&vault, &target, false)?;
            let vars = vault.list_variables(&env.id)?;
            match vars.into_iter().find(|v| v.key == key) {
                Some(v) => println!("{}", v.value),
                None => return Err(format!("no such key '{key}' in {target}").into()),
            }
        }
        Command::Set { target, assignment } => {
            let vault = auth::unlock()?;
            let (_, env) = target::resolve_env(&vault, &target, true)?;
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
            println!("Set {key} in {target}.");
        }
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
            let current = rpassword::prompt_password("Current master password: ")?;
            let new = rpassword::prompt_password("New master password: ")?;
            let confirmation = rpassword::prompt_password("Confirm new master password: ")?;
            if new != confirmation {
                return Err("passwords do not match".into());
            }
            vault.change_password(&current, &new)?;
            println!("Master password changed.");
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
                let code = rpassword::prompt_password("Recovery code: ")?;
                let mut vault = vault_core::Vault::open_with_recovery(&code)?;
                println!("Recovery code accepted. Choose a new master password.");
                let new = rpassword::prompt_password("New master password: ")?;
                let confirmation = rpassword::prompt_password("Confirm new master password: ")?;
                if new != confirmation {
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
    }
    Ok(())
}
