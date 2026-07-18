use vault_core::error::{Result, VaultError};
use vault_core::models::{Environment, Repo};
use vault_core::Vault;

/// Splits a `<repo>/<env>` CLI target into its two name components.
pub fn parse_target(target: &str) -> Result<(String, String)> {
    let (repo, env) = target.split_once('/').ok_or_else(|| {
        VaultError::InvalidInput(format!("expected <repo>/<env>, got '{target}'"))
    })?;
    if repo.is_empty() || env.is_empty() {
        return Err(VaultError::InvalidInput(format!(
            "expected <repo>/<env>, got '{target}'"
        )));
    }
    Ok((repo.to_string(), env.to_string()))
}

/// Resolves a `<repo>/<env>` target to its stored records, optionally
/// creating the repo and/or environment if they don't exist yet.
pub fn resolve_env(vault: &Vault, target: &str, create: bool) -> Result<(Repo, Environment)> {
    let (repo_name, env_name) = parse_target(target)?;

    let repo = vault.list_repos()?.into_iter().find(|r| r.name == repo_name);
    let repo = match repo {
        Some(r) => r,
        None if create => vault.create_repo(&repo_name)?,
        None => return Err(VaultError::Missing(format!("repo '{repo_name}'"))),
    };

    let env = vault
        .list_environments(&repo.id)?
        .into_iter()
        .find(|e| e.name == env_name);
    let env = match env {
        Some(e) => e,
        None if create => vault.create_environment(&repo.id, &env_name)?,
        None => {
            return Err(VaultError::Missing(format!(
                "environment '{env_name}' in repo '{repo_name}'"
            )))
        }
    };

    Ok((repo, env))
}

/// Resolves an optional `<repo>/<env>` CLI target: if given, resolves it
/// normally (optionally creating). If omitted, resolves the linked project
/// for the current directory, per `vault link`.
pub fn resolve_target_or_cwd(
    vault: &Vault,
    target: Option<&str>,
    create: bool,
) -> Result<(Repo, Environment)> {
    match target {
        Some(t) => resolve_env(vault, t, create),
        None => resolve_cwd(vault),
    }
}

/// Resolves the linked project for the current directory (or the nearest
/// linked ancestor). Fails with an actionable message if nothing here was
/// ever linked with `vault link`.
pub fn resolve_cwd(vault: &Vault) -> Result<(Repo, Environment)> {
    let cwd = std::env::current_dir()?;
    vault.resolve_project(&cwd)?.ok_or_else(|| {
        VaultError::InvalidInput(
            "no linked project here — run 'vault link <repo>/<env>' in this directory".into(),
        )
    })
}

/// Resolves a target that names either a whole repo (`api-gateway`) or one of
/// its environments (`api-gateway/local`).
pub fn resolve_repo_or_env(
    vault: &Vault,
    target: &str,
) -> Result<(Repo, Option<Environment>)> {
    match target.split_once('/') {
        Some(_) => {
            let (repo, env) = resolve_env(vault, target, false)?;
            Ok((repo, Some(env)))
        }
        None => {
            let repo = vault
                .list_repos()?
                .into_iter()
                .find(|r| r.name == target)
                .ok_or_else(|| VaultError::Missing(format!("repo '{target}'")))?;
            Ok((repo, None))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::parse_target;

    #[test]
    fn splits_repo_and_env() {
        let (repo, env) = parse_target("api-gateway/local").unwrap();
        assert_eq!(repo, "api-gateway");
        assert_eq!(env, "local");
    }

    #[test]
    fn rejects_missing_slash() {
        assert!(parse_target("no-slash-here").is_err());
    }

    #[test]
    fn rejects_empty_repo_or_env() {
        assert!(parse_target("/local").is_err());
        assert!(parse_target("repo/").is_err());
    }

    #[test]
    fn splits_on_first_slash_only() {
        let (repo, env) = parse_target("repo/env/extra").unwrap();
        assert_eq!(repo, "repo");
        assert_eq!(env, "env/extra");
    }
}
