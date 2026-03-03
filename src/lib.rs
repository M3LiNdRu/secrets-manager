mod crypto;
mod model;
mod store;

pub use model::{Entry, Secrets};
pub use store::{FileSecretsStore, SecretsStore};

use anyhow::{bail, Context, Result};

#[derive(Clone)]
pub struct SecretsManager<S: SecretsStore> {
    store: S,
}

impl<S: SecretsStore> SecretsManager<S> {
    pub fn new(store: S) -> Self {
        Self { store }
    }

    pub fn add(&self, key: &str, value: &str) -> Result<()> {
        validate_key_value(key, value)?;

        let mut secrets = self.store.load().context("load secrets")?;
        secrets.upsert(key.to_string(), value.to_string());
        self.store.save(&secrets).context("save secrets")?;
        Ok(())
    }

    pub fn get(&self, key: &str) -> Result<Option<String>> {
        validate_key(key)?;
        let secrets = self.store.load().context("load secrets")?;
        Ok(secrets.get(key).map(|e| e.value.clone()))
    }
}

fn validate_key(key: &str) -> Result<()> {
    if key.is_empty() {
        bail!("key must not be empty");
    }
    if key.contains(':') {
        bail!("key must not contain ':'");
    }
    Ok(())
}

fn validate_key_value(key: &str, value: &str) -> Result<()> {
    validate_key(key)?;
    if value.is_empty() {
        bail!("value must not be empty");
    }
    if value.contains(':') {
        bail!("value must not contain ':'");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Context;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use std::process::Command;
    use tempfile::tempdir;

    fn setup_test_gpg_home() -> (tempfile::TempDir, String) {
        let home = tempdir().unwrap();

        #[cfg(unix)]
        {
            let perms = std::fs::Permissions::from_mode(0o700);
            std::fs::set_permissions(home.path(), perms).unwrap();
        }

        let email = format!("test-{}@example.com", std::process::id());
        let user_id = format!("Test User <{email}>");

        let status = Command::new("gpg")
            .arg("--homedir")
            .arg(home.path())
            .arg("--batch")
            .arg("--yes")
            .arg("--quiet")
            .arg("--no-tty")
            .arg("--pinentry-mode")
            .arg("loopback")
            .arg("--passphrase")
            .arg("")
            .arg("--quick-generate-key")
            .arg(&user_id)
            .arg("default")
            .arg("default")
            .arg("never")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .context("generate test gpg key")
            .unwrap();

        assert!(status.success(), "failed to generate test gpg key");
        (home, email)
    }

    #[test]
    fn add_then_get_returns_value() {
        let (gpg_home, recipient) = setup_test_gpg_home();
        let dir = tempdir().unwrap();
        let path = dir.path().join("secrets.txt");

        let store = FileSecretsStore::new(path)
            .with_gnupghome(gpg_home.path().to_path_buf())
            .with_recipient(recipient);
        let manager = SecretsManager::new(store);

        manager.add("db_password", "s3cr3t").unwrap();
        let value = manager.get("db_password").unwrap();
        assert_eq!(value.as_deref(), Some("s3cr3t"));
    }

    #[test]
    fn add_updates_existing_key() {
        let (gpg_home, recipient) = setup_test_gpg_home();
        let dir = tempdir().unwrap();
        let path = dir.path().join("secrets.txt");

        let store = FileSecretsStore::new(path)
            .with_gnupghome(gpg_home.path().to_path_buf())
            .with_recipient(recipient);
        let manager = SecretsManager::new(store);

        manager.add("api_key", "abc123").unwrap();
        manager.add("api_key", "def456").unwrap();

        let value = manager.get("api_key").unwrap();
        assert_eq!(value.as_deref(), Some("def456"));
    }
}
