use std::path::PathBuf;

/// XDG-aware path resolution for Fyndra.
///
/// Hierarchy:
/// - `$FYNDRA_CONFIG` (explicit file override) takes precedence.
/// - Deprecated `$EVERYTHING_CONFIG` is honored with a warning if `FYNDRA_CONFIG` is unset.
/// - Otherwise `$XDG_CONFIG_HOME/fyndra/config.toml` or `~/.config/fyndra/config.toml`.
///
/// Cache follows `$XDG_CACHE_HOME/fyndra/index.evth` or `~/.cache/fyndra/index.evth`.
/// Legacy `everything` paths are migrated **non-destructively** (copy, not move).
fn xdg_config_home() -> PathBuf {
    if let Ok(v) = std::env::var("XDG_CONFIG_HOME") {
        if !v.is_empty() {
            return PathBuf::from(v);
        }
    }
    dirs_fallback_home().join(".config")
}

fn xdg_cache_home() -> PathBuf {
    if let Ok(v) = std::env::var("XDG_CACHE_HOME") {
        if !v.is_empty() {
            return PathBuf::from(v);
        }
    }
    dirs_fallback_home().join(".cache")
}

fn xdg_data_home() -> PathBuf {
    if let Ok(v) = std::env::var("XDG_DATA_HOME") {
        if !v.is_empty() {
            return PathBuf::from(v);
        }
    }
    dirs_fallback_home().join(".local/share")
}

fn dirs_fallback_home() -> PathBuf {
    std::env::var("HOME").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("/tmp"))
}

pub fn config_dir() -> PathBuf {
    xdg_config_home().join("fyndra")
}

pub fn cache_dir() -> PathBuf {
    xdg_cache_home().join("fyndra")
}

pub fn data_dir() -> PathBuf {
    xdg_data_home().join("fyndra")
}

pub fn config_file() -> PathBuf {
    if let Ok(v) = std::env::var("FYNDRA_CONFIG") {
        if !v.is_empty() {
            return PathBuf::from(v);
        }
    }
    if let Ok(v) = std::env::var("EVERYTHING_CONFIG") {
        if !v.is_empty() {
            eprintln!("warning: EVERYTHING_CONFIG is deprecated, use FYNDRA_CONFIG");
            return PathBuf::from(v);
        }
    }
    config_dir().join("config.toml")
}

pub fn cache_file() -> PathBuf {
    if let Ok(v) = std::env::var("FYNDRA_CACHE") {
        if !v.is_empty() {
            return PathBuf::from(v);
        }
    }
    cache_dir().join("index.evth")
}

/// Ensure XDG directories exist. Idempotent.
pub fn ensure_dirs() -> std::io::Result<()> {
    for d in [config_dir(), cache_dir(), data_dir()] {
        std::fs::create_dir_all(d)?;
    }
    Ok(())
}

/// Non-destructive migration from legacy `everything` paths.
///
/// - Copies `~/.config/everything/config.toml` → `~/.config/fyndra/config.toml` if target missing.
/// - Leaves legacy cache untouched (rebuildable); old `~/.cache/everything/` is ignored.
pub fn migrate_legacy() -> std::io::Result<Option<PathBuf>> {
    let legacy_config = xdg_config_home().join("everything").join("config.toml");
    let new_config = config_file();

    // Only migrate if legacy exists and new does not, and we're using default path
    let using_default =
        std::env::var("FYNDRA_CONFIG").is_err() && std::env::var("EVERYTHING_CONFIG").is_err();
    if using_default && legacy_config.exists() && !new_config.exists() {
        if let Some(parent) = new_config.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(&legacy_config, &new_config)?;
        eprintln!(
            "Fyndra: migrated legacy config {} → {}",
            legacy_config.display(),
            new_config.display()
        );
        return Ok(Some(new_config));
    }
    Ok(None)
}

pub fn legacy_cache_exists() -> bool {
    let legacy = xdg_cache_home().join("everything");
    legacy.exists()
}
