use anyhow::{bail, Result};
use chrono::SecondsFormat;
use clap::{Parser, Subcommand};
use rpassword::prompt_password;

use secrets_manager::{FileSecretsStore, SecretsManager};

mod config;

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
    Add { key: String },
    /// Retrieve a secret value by key
    Get { key: String },
    /// List all stored keys, optionally filtered by pattern
    List {
        /// Optional pattern to filter keys (case-insensitive substring match)
        pattern: Option<String>,
        /// Show timestamps for each key
        #[arg(long)]
        with_timestamps: bool,
    },
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

    let mut cfg = config::load()?;

    let mut file = cli
        .file
        .or_else(|| std::env::var_os("SECRETS_MANAGER_FILE").map(std::path::PathBuf::from))
        .or_else(|| cfg.file.as_ref().map(std::path::PathBuf::from));

    let mut recipient = cli
        .recipient
        .or_else(|| std::env::var("SECRETS_MANAGER_GPG_RECIPIENT").ok())
        .or_else(|| cfg.gpg_recipient.clone());

    let gnupghome = cli.gnupghome.or_else(|| {
        std::env::var_os("SECRETS_MANAGER_GNUPGHOME")
            .or_else(|| std::env::var_os("GNUPGHOME"))
            .map(std::path::PathBuf::from)
    });

    // Prompt for missing defaults on first run.
    let needs_recipient = matches!(cli.command, Command::Add { .. });
    let needs_file = !matches!(cli.command, Command::List { .. });

    if (needs_file && file.is_none()) || (needs_recipient && recipient.is_none()) {
        cfg.file = file.as_ref().map(|p| p.to_string_lossy().to_string());
        cfg.gpg_recipient = recipient.clone();

        let (cfg2, changed) = config::prompt_for_missing(cfg, needs_recipient, needs_file)?;
        if changed {
            config::save(&cfg2)?;
        }

        file = file.or_else(|| cfg2.file.as_ref().map(std::path::PathBuf::from));
        recipient = recipient.or_else(|| cfg2.gpg_recipient.clone());
    }

    let file = file.unwrap_or_else(|| std::path::PathBuf::from("secrets.enc"));

    match cli.command {
        Command::Add { key } => {
            if !config::is_interactive() {
                bail!("add requires an interactive terminal to prompt for the secret value");
            }

            let value = prompt_password(format!("Value for '{key}': "))?;
            if value.is_empty() {
                bail!("value must not be empty");
            }

            let confirm = prompt_password(format!("Confirm value for '{key}': "))?;
            if value != confirm {
                bail!("values do not match");
            }

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
        Command::List {
            pattern,
            with_timestamps,
        } => {
            let mut store = FileSecretsStore::new(file);
            if let Some(home) = gnupghome {
                store = store.with_gnupghome(home);
            }
            let manager = SecretsManager::new(store);

            let keys = manager.list(pattern.as_deref())?;
            if keys.is_empty() {
                return Ok(0);
            }

            for (key, timestamp) in keys {
                if with_timestamps {
                    println!(
                        "{} {}",
                        key,
                        timestamp.to_rfc3339_opts(SecondsFormat::Secs, true)
                    );
                } else {
                    println!("{}", key);
                }
            }
            Ok(0)
        }
    }
}
