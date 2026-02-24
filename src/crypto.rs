use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Nonce};
use anyhow::{bail, Context, Result};
use base64::Engine;
use pbkdf2::pbkdf2_hmac;
use rand::RngCore;
use sha2::Sha256;
use zeroize::Zeroize;

const MAGIC: &str = "SM1";
const PBKDF2_ITERS: u32 = 100_000;
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;

/// Encrypt plaintext into a single-line, text-safe record:
/// `SM1:<b64(salt)>:<b64(nonce)>:<b64(ciphertext)>`
pub fn encrypt_text(plaintext: &str, password: &str) -> Result<String> {
    if password.is_empty() {
        bail!("password must not be empty");
    }

    let mut salt = [0u8; SALT_LEN];
    OsRng.fill_bytes(&mut salt);

    let mut key_bytes = [0u8; 32];
    pbkdf2_hmac::<Sha256>(password.as_bytes(), &salt, PBKDF2_ITERS, &mut key_bytes);

    let cipher = Aes256Gcm::new_from_slice(&key_bytes).context("init cipher")?;

    let mut nonce_bytes = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|_| anyhow::anyhow!("encrypt failed"))?;

    let b64 = base64::engine::general_purpose::STANDARD_NO_PAD;
    let out = format!(
        "{}:{}:{}:{}",
        MAGIC,
        b64.encode(salt),
        b64.encode(nonce_bytes),
        b64.encode(ciphertext)
    );

    key_bytes.zeroize();
    Ok(out)
}

pub fn decrypt_text(enc_record: &str, password: &str) -> Result<String> {
    if password.is_empty() {
        bail!("password must not be empty");
    }

    let parts: Vec<&str> = enc_record.split(':').collect();
    if parts.len() != 4 {
        bail!("invalid encrypted file format");
    }
    if parts[0] != MAGIC {
        bail!("invalid magic header");
    }

    let b64 = base64::engine::general_purpose::STANDARD_NO_PAD;
    let salt = b64.decode(parts[1]).context("decode salt")?;
    let nonce_bytes = b64.decode(parts[2]).context("decode nonce")?;
    let ciphertext = b64.decode(parts[3]).context("decode ciphertext")?;

    if salt.len() != SALT_LEN {
        bail!("invalid salt length");
    }
    if nonce_bytes.len() != NONCE_LEN {
        bail!("invalid nonce length");
    }

    let mut key_bytes = [0u8; 32];
    pbkdf2_hmac::<Sha256>(password.as_bytes(), &salt, PBKDF2_ITERS, &mut key_bytes);

    let cipher = Aes256Gcm::new_from_slice(&key_bytes).context("init cipher")?;
    let nonce = Nonce::from_slice(&nonce_bytes);

    let plaintext_bytes = cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|_| anyhow::anyhow!("decrypt failed (wrong password or corrupted file)"))?;

    key_bytes.zeroize();

    let plaintext = String::from_utf8(plaintext_bytes).context("decrypted data is not UTF-8")?;
    Ok(plaintext)
}
