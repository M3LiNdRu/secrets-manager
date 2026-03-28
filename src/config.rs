use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppConfig {
    pub gpg_recipient: Option<String>,
    pub file: Option<String>,
}

pub fn is_interactive() -> bool {
    io::stdin().is_terminal() && io::stdout().is_terminal()
}

fn home_dir() -> Result<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| anyhow!("$HOME is not set; cannot locate config directory"))
}

pub fn config_dir() -> Result<PathBuf> {
    Ok(home_dir()?.join(".secrets-manager"))
}

pub fn config_path() -> Result<PathBuf> {
    Ok(config_dir()?.join("config.toml"))
}

#[cfg(unix)]
fn chmod(path: &Path, mode: u32) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let perms = fs::Permissions::from_mode(mode);
    fs::set_permissions(path, perms)
        .with_context(|| format!("set permissions on {}", path.display()))?;
    Ok(())
}

#[cfg(unix)]
fn mode(path: &Path) -> Result<u32> {
    use std::os::unix::fs::PermissionsExt;
    let m = fs::metadata(path)
        .with_context(|| format!("stat {}", path.display()))?
        .permissions()
        .mode();
    Ok(m & 0o777)
}

#[cfg(not(unix))]
fn chmod(_path: &Path, _mode: u32) -> Result<()> {
    Ok(())
}

#[cfg(not(unix))]
fn mode(_path: &Path) -> Result<u32> {
    Ok(0)
}

fn ensure_secure_dir(dir: &Path) -> Result<()> {
    if !dir.exists() {
        fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;

        #[cfg(unix)]
        {
            chmod(dir, 0o700)?;
        }
    }

    #[cfg(unix)]
    {
        // Refuse to use an existing directory if it's too permissive.
        let actual = mode(dir)?;
        if actual != 0o700 {
            bail!(
                "config directory {} must have permissions 0700 (current: {:o}); run: chmod 700 {}",
                dir.display(),
                actual,
                dir.display()
            );
        }
    }

    Ok(())
}

fn ensure_secure_file(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        let actual = mode(path)?;
        if actual != 0o600 {
            bail!(
                "config file {} must have permissions 0600 (current: {:o}); run: chmod 600 {}",
                path.display(),
                actual,
                path.display()
            );
        }
    }
    Ok(())
}

pub fn load() -> Result<AppConfig> {
    let dir = config_dir()?;
    if dir.exists() {
        ensure_secure_dir(&dir)?;
    }

    let path = config_path()?;
    if !path.exists() {
        return Ok(AppConfig::default());
    }

    ensure_secure_file(&path)?;

    let text = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let cfg: AppConfig =
        toml::from_str(&text).with_context(|| format!("parse TOML in {}", path.display()))?;

    Ok(AppConfig {
        gpg_recipient: cfg.gpg_recipient.and_then(non_empty),
        file: cfg.file.and_then(non_empty),
    })
}

pub fn save(cfg: &AppConfig) -> Result<()> {
    let dir = config_dir()?;
    ensure_secure_dir(&dir)?;

    let path = config_path()?;

    let cfg = AppConfig {
        gpg_recipient: cfg.gpg_recipient.clone().and_then(non_empty),
        file: cfg.file.clone().and_then(non_empty),
    };

    let text = toml::to_string_pretty(&cfg).context("serialize config")?;

    write_atomically_secure(&path, text.as_bytes())?;
    ensure_secure_file(&path)?;

    Ok(())
}

fn write_atomically_secure(path: &Path, bytes: &[u8]) -> Result<()> {
    let tmp = path.with_extension("toml.tmp");

    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&tmp)
            .with_context(|| format!("open {}", tmp.display()))?;
        file.write_all(bytes)
            .with_context(|| format!("write {}", tmp.display()))?;
        file.flush()
            .with_context(|| format!("flush {}", tmp.display()))?;
    }

    #[cfg(not(unix))]
    {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&tmp)
            .with_context(|| format!("open {}", tmp.display()))?;
        file.write_all(bytes)
            .with_context(|| format!("write {}", tmp.display()))?;
        file.flush()
            .with_context(|| format!("flush {}", tmp.display()))?;
    }

    fs::rename(&tmp, path)
        .with_context(|| format!("rename {} -> {}", tmp.display(), path.display()))?;

    Ok(())
}

fn non_empty(s: String) -> Option<String> {
    if s.trim().is_empty() {
        None
    } else {
        Some(s)
    }
}

pub fn prompt_for_missing(
    mut cfg: AppConfig,
    require_recipient: bool,
    require_file: bool,
) -> Result<(AppConfig, bool)> {
    if !is_interactive() {
        return Ok((cfg, false));
    }

    let mut changed = false;

    if require_file && cfg.file.is_none() {
        let v = prompt_line("Default encrypted file path", Some("secrets.enc"), false)?;
        cfg.file = Some(v);
        changed = true;
    }

    if require_recipient && cfg.gpg_recipient.is_none() {
        let v = prompt_line(
            "Default GPG recipient (email/keyid/fingerprint)",
            None,
            true,
        )?;
        cfg.gpg_recipient = Some(v);
        changed = true;
    }

    Ok((cfg, changed))
}

fn prompt_line(label: &str, default: Option<&str>, require_non_empty: bool) -> Result<String> {
    loop {
        let mut stdout = io::stdout();
        match default {
            Some(d) => write!(stdout, "{label} [{d}]: ")?,
            None => write!(stdout, "{label}: ")?,
        }
        stdout.flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim().to_string();

        let value = if input.is_empty() {
            default.unwrap_or("").to_string()
        } else {
            input
        };

        if require_non_empty && value.trim().is_empty() {
            eprintln!("value must not be empty");
            continue;
        }

        return Ok(value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};
    use tempfile::tempdir;

    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    fn lock_env() -> std::sync::MutexGuard<'static, ()> {
        ENV_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|e| e.into_inner())
    }

    #[test]
    fn save_then_load_roundtrips() {
        let _guard = lock_env();
        let dir = tempdir().unwrap();
        std::env::set_var("HOME", dir.path());

        let cfg = AppConfig {
            gpg_recipient: Some("me@example.com".to_string()),
            file: Some("/tmp/secrets.enc".to_string()),
        };

        save(&cfg).unwrap();
        let loaded = load().unwrap();

        assert_eq!(loaded.gpg_recipient.as_deref(), Some("me@example.com"));
        assert_eq!(loaded.file.as_deref(), Some("/tmp/secrets.enc"));

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let cdir = config_dir().unwrap();
            let cpath = config_path().unwrap();

            let dmode = fs::metadata(cdir).unwrap().permissions().mode() & 0o777;
            let fmode = fs::metadata(cpath).unwrap().permissions().mode() & 0o777;

            assert_eq!(dmode, 0o700);
            assert_eq!(fmode, 0o600);
        }
    }

    #[cfg(unix)]
    #[test]
    fn load_errors_on_too_permissive_file() {
        use std::os::unix::fs::PermissionsExt;

        let _guard = lock_env();
        let dir = tempdir().unwrap();
        std::env::set_var("HOME", dir.path());

        let cdir = config_dir().unwrap();
        fs::create_dir_all(&cdir).unwrap();
        chmod(&cdir, 0o700).unwrap();

        let cpath = config_path().unwrap();
        fs::write(&cpath, "gpg_recipient = 'x'\nfile = 'y'\n").unwrap();
        fs::set_permissions(&cpath, fs::Permissions::from_mode(0o644)).unwrap();

        let err = load().unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("permissions 0600"));
    }
}
