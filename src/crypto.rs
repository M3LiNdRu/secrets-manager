use anyhow::{bail, Context, Result};
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

fn run_gpg(input: &[u8], args: &[&str], homedir: Option<&Path>) -> Result<Vec<u8>> {
    let mut cmd = Command::new("gpg");
    cmd.args(["--batch", "--yes", "--no-tty", "--pinentry-mode", "loopback"]);

    if let Some(home) = homedir {
        cmd.arg("--homedir").arg(home);
    }

    cmd.args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn().context("spawn gpg")?;

    {
        let stdin = child.stdin.as_mut().context("open gpg stdin")?;
        stdin.write_all(input).context("write gpg stdin")?;
    }

    let output = child.wait_with_output().context("wait for gpg")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stderr = stderr.trim();
        if stderr.is_empty() {
            bail!("gpg failed with exit status: {}", output.status);
        }
        bail!("gpg failed: {stderr}");
    }

    Ok(output.stdout)
}

/// Encrypt plaintext using the given GPG recipient (public key).
///
/// Output is an ASCII-armored PGP message (text-safe for a single file).
pub fn encrypt_text(plaintext: &str, recipient: &str, homedir: Option<&Path>) -> Result<String> {
    let recipient = recipient.trim();
    if recipient.is_empty() {
        bail!("missing GPG recipient");
    }

    let out = run_gpg(
        plaintext.as_bytes(),
        &[
            "--armor",
            "--trust-model",
            "always",
            "--encrypt",
            "--recipient",
            recipient,
        ],
        homedir,
    )
    .context("gpg encrypt")?;

    String::from_utf8(out).context("gpg output is not UTF-8")
}

/// Decrypt an ASCII-armored (or binary) PGP message.
pub fn decrypt_text(ciphertext: &str, homedir: Option<&Path>) -> Result<String> {
    let out = run_gpg(ciphertext.as_bytes(), &["--decrypt"], homedir).context("gpg decrypt")?;
    String::from_utf8(out).context("decrypted output is not UTF-8")
}
