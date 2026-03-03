use crate::crypto::{decrypt_text, encrypt_text};
use crate::Secrets;
use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};

pub trait SecretsStore: Clone {
    fn load(&self) -> Result<Secrets>;
    fn save(&self, secrets: &Secrets) -> Result<()>;
}

#[derive(Clone)]
pub struct FileSecretsStore {
    path: PathBuf,
    recipient: Option<String>,
    gnupghome: Option<PathBuf>,
}

impl FileSecretsStore {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            recipient: None,
            gnupghome: None,
        }
    }

    pub fn with_recipient(mut self, recipient: impl Into<String>) -> Self {
        self.recipient = Some(recipient.into());
        self
    }

    pub fn with_gnupghome(mut self, gnupghome: PathBuf) -> Self {
        self.gnupghome = Some(gnupghome);
        self
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

        let plaintext =
            decrypt_text(enc_text.as_str(), self.gnupghome.as_deref()).context("decrypt")?;
        let lines = plaintext.lines().map(|s| s.to_string()).collect::<Vec<_>>();
        Secrets::from_lines(lines).context("parse decrypted records")
    }

    fn save(&self, secrets: &Secrets) -> Result<()> {
        let Some(recipient) = self.recipient.as_deref() else {
            bail!("missing GPG recipient (needed to encrypt on save)");
        };

        let mut plaintext = String::new();
        for (idx, line) in secrets.to_lines().into_iter().enumerate() {
            if idx > 0 {
                plaintext.push('\n');
            }
            plaintext.push_str(&line);
        }

        let enc_text =
            encrypt_text(&plaintext, recipient, self.gnupghome.as_deref()).context("encrypt")?;
        self.write_encrypted(&enc_text)
    }
}

impl AsRef<Path> for FileSecretsStore {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}
