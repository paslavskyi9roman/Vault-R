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
    }
    Ok(())
}
