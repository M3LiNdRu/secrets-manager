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

    /// Password used to encrypt/decrypt (defaults to $SECRETS_MANAGER_PASSWORD)
    #[arg(long)]
    password: Option<String>,

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

    let password = match cli
        .password
        .or_else(|| std::env::var("SECRETS_MANAGER_PASSWORD").ok())
    {
        Some(p) if !p.is_empty() => p,
        _ => bail!("missing password; provide --password or set SECRETS_MANAGER_PASSWORD"),
    };

    let store = FileSecretsStore::new(file, password);
    let manager = SecretsManager::new(store);

    match cli.command {
        Command::Add { key, value } => {
            manager.add(&key, &value)?;
            Ok(0)
        }
        Command::Get { key } => match manager.get(&key)? {
            Some(value) => {
                println!("{value}");
                Ok(0)
            }
            None => Ok(1),
        },
    }
}
