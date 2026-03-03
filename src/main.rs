use anyhow::{bail, Result};
use clap::{Parser, Subcommand};

use secrets_manager::{FileSecretsStore, SecretsManager};

#[derive(Parser)]
#[command(
    name = "secrets-manager",
    version,
    about = "Minimal encrypted secrets manager"
)]
struct Cli {
    /// Path to the encrypted secrets file (defaults to $SECRETS_MANAGER_FILE or ./secrets.enc)
    #[arg(long)]
    file: Option<std::path::PathBuf>,

    /// GPG recipient (key id / fingerprint / email). Defaults to $SECRETS_MANAGER_GPG_RECIPIENT.
    ///
    /// Required for `add` (encryption on save). Not required for `get`.
    #[arg(long)]
    recipient: Option<String>,

    /// Optional GnuPG home directory to use (defaults to $SECRETS_MANAGER_GNUPGHOME or $GNUPGHOME)
    #[arg(long)]
    gnupghome: Option<std::path::PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Store or update a key/value secret
    Add { key: String, value: String },
    /// Retrieve a secret value by key
    Get { key: String },
}

fn main() {
    let exit_code = match run() {
        Ok(code) => code,
        Err(err) => {
            eprintln!("{err:#}");
            2
        }
    };
    std::process::exit(exit_code);
}

fn run() -> Result<i32> {
    let cli = Cli::parse();

    let file = cli
        .file
        .or_else(|| std::env::var_os("SECRETS_MANAGER_FILE").map(std::path::PathBuf::from))
        .unwrap_or_else(|| std::path::PathBuf::from("secrets.enc"));

    let recipient = cli
        .recipient
        .or_else(|| std::env::var("SECRETS_MANAGER_GPG_RECIPIENT").ok());

    let gnupghome = cli.gnupghome.or_else(|| {
        std::env::var_os("SECRETS_MANAGER_GNUPGHOME")
            .or_else(|| std::env::var_os("GNUPGHOME"))
            .map(std::path::PathBuf::from)
    });

    match cli.command {
        Command::Add { key, value } => {
            let Some(recipient) = recipient else {
                bail!(
                    "missing GPG recipient; provide --recipient or set SECRETS_MANAGER_GPG_RECIPIENT"
                );
            };

            let mut store = FileSecretsStore::new(file);
            if let Some(home) = gnupghome {
                store = store.with_gnupghome(home);
            }
            let store = store.with_recipient(recipient);
            let manager = SecretsManager::new(store);
            manager.add(&key, &value)?;
            Ok(0)
        }
        Command::Get { key } => {
            let mut store = FileSecretsStore::new(file);
            if let Some(home) = gnupghome {
                store = store.with_gnupghome(home);
            }
            let manager = SecretsManager::new(store);

            match manager.get(&key)? {
                Some(value) => {
                    println!("{value}");
                    Ok(0)
                }
                None => Ok(1),
            }
        }
    }
}
