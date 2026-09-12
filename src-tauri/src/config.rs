use crate::models::AppConfig;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

pub fn config_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("config.json")
}

pub fn load_config(path: &Path) -> Result<AppConfig> {
    if !path.exists() {
        return Ok(AppConfig::default());
    }
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read config at {}", path.display()))?;
    let config: AppConfig =
        serde_json::from_str(&raw).with_context(|| "failed to parse config.json")?;
    Ok(config)
}

pub fn save_config(path: &Path, config: &AppConfig) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create config dir {}", parent.display()))?;
    }
    let raw = serde_json::to_string_pretty(config)?;
    std::fs::write(path, raw)
        .with_context(|| format!("failed to write config at {}", path.display()))?;
    Ok(())
}

/// Resolve private key path: explicit path, or first existing default key.
pub fn resolve_private_key_path(explicit: Option<&str>) -> Result<PathBuf> {
    if let Some(p) = explicit {
        let trimmed = p.trim();
        if !trimmed.is_empty() {
            let path = PathBuf::from(trimmed);
            if path.exists() {
                return Ok(path);
            }
            anyhow::bail!("private key not found: {}", path.display());
        }
    }

    let home = dirs::home_dir().context("cannot determine home directory")?;
    let ssh_dir = home.join(".ssh");
    for name in ["id_ed25519", "id_rsa", "id_ecdsa", "id_ed25519_sk", "id_rsa_sk"] {
        let candidate = ssh_dir.join(name);
        if candidate.exists() {
            return Ok(candidate);
        }
    }
    anyhow::bail!(
        "no default SSH private key found under {}",
        ssh_dir.display()
    );
}

pub fn default_key_hint() -> String {
    dirs::home_dir()
        .map(|h| h.join(".ssh").join("id_ed25519").display().to_string())
        .unwrap_or_else(|| "~/.ssh/id_ed25519".into())
}
