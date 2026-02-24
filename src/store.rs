use crate::crypto::{decrypt_text, encrypt_text};
use crate::Secrets;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

pub trait SecretsStore: Clone {
    fn load(&self) -> Result<Secrets>;
    fn save(&self, secrets: &Secrets) -> Result<()>;
}

#[derive(Clone)]
pub struct FileSecretsStore {
    path: PathBuf,
    password: String,
}

impl FileSecretsStore {
    pub fn new(path: PathBuf, password: String) -> Self {
        Self { path, password }
    }

    fn read_encrypted(&self) -> Result<Option<String>> {
        if !self.path.exists() {
            return Ok(None);
        }
        let bytes =
            std::fs::read(&self.path).with_context(|| format!("read {}", self.path.display()))?;
        let text = String::from_utf8(bytes).context("secrets file is not valid UTF-8")?;
        Ok(Some(text))
    }

    fn write_encrypted(&self, text: &str) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("create dir {}", parent.display()))?;
            }
        }
        std::fs::write(&self.path, text.as_bytes())
            .with_context(|| format!("write {}", self.path.display()))?;
        Ok(())
    }
}

impl SecretsStore for FileSecretsStore {
    fn load(&self) -> Result<Secrets> {
        let Some(enc_text) = self.read_encrypted()? else {
            return Ok(Secrets::default());
        };

        let plaintext = decrypt_text(enc_text.trim(), &self.password).context("decrypt")?;
        let lines = plaintext.lines().map(|s| s.to_string()).collect::<Vec<_>>();
        Secrets::from_lines(lines).context("parse decrypted records")
    }

    fn save(&self, secrets: &Secrets) -> Result<()> {
        let mut plaintext = String::new();
        for (idx, line) in secrets.to_lines().into_iter().enumerate() {
            if idx > 0 {
                plaintext.push('\n');
            }
            plaintext.push_str(&line);
        }

        let enc_text = encrypt_text(&plaintext, &self.password).context("encrypt")?;
        self.write_encrypted(&enc_text)
    }
}

impl AsRef<Path> for FileSecretsStore {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}
