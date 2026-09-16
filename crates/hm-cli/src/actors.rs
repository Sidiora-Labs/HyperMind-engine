#![allow(clippy::verbose_bit_mask)]

use anyhow::{Context, Result, ensure};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use clap::Subcommand;
use hm_core::ActorId;
use hm_ledger::keyring::{KeyHierarchy, OsEntropy};
use hm_serve::actor::{ActorConfig, ActorEngine};
use hm_serve::config::{ServerConfig, load};
use serde_json::{Value, json};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

#[derive(Debug, Subcommand)]
pub enum Command {
    Add {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        actor: u16,
    },
    List {
        #[arg(long)]
        config: PathBuf,
    },
    RotateKeys {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        new_kek_file: PathBuf,
    },
    Shred {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        actor: u16,
        #[arg(long)]
        confirm: u16,
    },
}

pub fn operation_lock(path: &Path) -> Result<File> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .open(path.with_extension("ops-lock"))?;
    file.try_lock()
        .context("another daemon or administrative operation holds this configuration")?;
    Ok(file)
}

pub fn require_offline(config: &ServerConfig) -> Result<()> {
    ensure!(
        std::os::unix::net::UnixStream::connect(&config.socket_path).is_err(),
        "stop the daemon before this offline operation"
    );
    Ok(())
}

pub async fn open(config: &ServerConfig, actor: u16) -> Result<ActorEngine> {
    ensure!(
        config.actors.iter().any(|entry| entry.actor == actor),
        "actor is not configured"
    );
    Ok(ActorEngine::open(ActorConfig {
        actor_directory: config.actor_directory(actor),
        actor: ActorId::new(actor),
        user: config.user,
        kek: config.kek,
        projection_map_bytes: config.projection_map_bytes,
    })
    .await?)
}

fn replace_config(path: &Path, text: &str) -> Result<()> {
    let pending = path.with_extension("conf-pending");
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&pending)?;
    file.write_all(text.as_bytes())?;
    file.sync_all()?;
    fs::rename(&pending, path)?;
    File::open(path.parent().context("configuration has no directory")?)?.sync_all()?;
    Ok(())
}

pub async fn execute(command: Command) -> Result<Value> {
    match command {
        Command::List { config } => {
            let config = load(config)?;
            Ok(
                json!({"ok":true,"actors":config.actors.iter().map(|entry| json!({"actor":entry.actor,"directory":config.actor_directory(entry.actor),"present":config.actor_directory(entry.actor).is_dir(),"shredded":config.actor_directory(entry.actor).join("keys/DELETION_RECEIPT").exists()})).collect::<Vec<_>>()}),
            )
        }
        Command::Add {
            config: path,
            actor,
        } => {
            let _lock = operation_lock(&path)?;
            let mut config = load(&path)?;
            require_offline(&config)?;
            ensure!(
                actor != 0 && !config.actors.iter().any(|entry| entry.actor == actor),
                "actor must be new and nonzero"
            );
            ensure!(
                !config.actor_directory(actor).exists(),
                "actor directory already exists; refusing to adopt it"
            );
            let mut token = [0; 32];
            getrandom::fill(&mut token)?;
            config
                .actors
                .push(hm_serve::config::ActorCapability { actor, token });
            open(&config, actor).await?.shutdown().await?;
            let original = fs::read_to_string(&path)?;
            let retained = original
                .lines()
                .filter(|line| {
                    let id = line
                        .strip_prefix("actor=")
                        .and_then(|value| value.split_once(':'))
                        .and_then(|(id, _)| id.parse::<u16>().ok());
                    id.is_none_or(|id| {
                        !config
                            .actor_directory(id)
                            .join("keys/DELETION_RECEIPT")
                            .exists()
                    })
                })
                .collect::<Vec<_>>()
                .join("\n");
            replace_config(
                &path,
                &format!(
                    "{}\nactor={actor}:{}\n",
                    retained.trim_end(),
                    crate::hex(&token)
                ),
            )?;
            Ok(json!({"ok":true,"actor":actor,"config":path,"restart_required":true}))
        }
        Command::RotateKeys {
            config: path,
            new_kek_file,
        } => rotate(&path, &new_kek_file),
        Command::Shred {
            config: path,
            actor,
            confirm,
        } => {
            ensure!(
                actor == confirm,
                "--confirm must equal the actor id; shredding is irreversible"
            );
            let _lock = operation_lock(&path)?;
            let config = load(&path)?;
            require_offline(&config)?;
            let handle = open(&config, actor).await?;
            let receipt = handle.crypto_delete().await?;
            if config.actors.len() > 1 {
                let original = fs::read_to_string(&path)?;
                let text = original
                    .lines()
                    .filter(|line| !line.starts_with(&format!("actor={actor}:")))
                    .collect::<Vec<_>>()
                    .join("\n")
                    + "\n";
                replace_config(&path, &text)?;
            }
            Ok(
                json!({"ok":true,"actor":actor,"irreversible":true,"receipt_base64":STANDARD.encode(receipt),"receipt_path":config.actor_directory(actor).join("keys/DELETION_RECEIPT"),"add_new_actor_before_restart":config.actors.len() == 1}),
            )
        }
    }
}

fn rotate(path: &Path, key_path: &Path) -> Result<Value> {
    let _lock = operation_lock(path)?;
    let config = load(path)?;
    require_offline(&config)?;
    ensure!(
        fs::metadata(key_path)?.permissions().mode() & 0o077 == 0,
        "new KEK file must have private permissions"
    );
    let encoded = fs::read_to_string(key_path)?;
    let new_key = decode_key(encoded.trim())?;
    let journal = path.with_extension("rotation.json");
    let identity = json!({"format":"hypermind.rotation.v1","new_key_blake3":blake3::hash(&new_key).to_hex().to_string(),"actors":config.actors.iter().map(|a| a.actor).collect::<Vec<_>>()});
    if journal.exists() {
        ensure!(
            serde_json::from_slice::<Value>(&fs::read(&journal)?)? == identity,
            "different rotation journal exists"
        );
    } else {
        ensure!(new_key != config.kek, "new KEK equals existing KEK");
        for entry in &config.actors {
            if !config
                .actor_directory(entry.actor)
                .join("keys/DELETION_RECEIPT")
                .exists()
            {
                KeyHierarchy::open_or_create(
                    config.actor_directory(entry.actor),
                    ActorId::new(entry.actor),
                    config.user,
                    &config.kek,
                    &mut OsEntropy,
                    false,
                )?;
            }
        }
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .open(&journal)?;
        file.write_all(&serde_json::to_vec(&identity)?)?;
        file.sync_all()?;
        File::open(path.parent().context("configuration directory missing")?)?.sync_all()?;
    }
    let mut rotated = Vec::new();
    for entry in &config.actors {
        let directory = config.actor_directory(entry.actor);
        if directory.join("keys/DELETION_RECEIPT").exists() {
            continue;
        }
        if KeyHierarchy::open_or_create(
            &directory,
            ActorId::new(entry.actor),
            config.user,
            &new_key,
            &mut OsEntropy,
            false,
        )
        .is_err()
        {
            let pending = directory.join("keys/KEYRING.rotating");
            if pending.exists() {
                let mut suffix = [0; 8];
                getrandom::fill(&mut suffix)?;
                fs::rename(
                    &pending,
                    directory.join(format!("keys/KEYRING.interrupted-{}", crate::hex(&suffix))),
                )?;
            }
            hm_ledger::rotate::rotate_keys(
                &directory,
                ActorId::new(entry.actor),
                config.user,
                &config.kek,
                &new_key,
                &mut OsEntropy,
            )?;
        }
        rotated.push(entry.actor);
    }
    let text = fs::read_to_string(path)?
        .lines()
        .map(|line| {
            if line.starts_with("kek=") {
                format!("kek={}", crate::hex(&new_key))
            } else {
                line.into()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    replace_config(path, &text)?;
    fs::remove_file(&journal)?;
    File::open(path.parent().context("configuration directory missing")?)?.sync_all()?;
    Ok(
        json!({"ok":true,"rotated_actors":rotated,"scope":"all_nonshredded_actors_shared_kek","restart_required":true}),
    )
}

fn decode_key(text: &str) -> Result<[u8; 32]> {
    ensure!(
        text.len() == 64,
        "KEK file must contain 64 hexadecimal characters"
    );
    let mut output = [0; 32];
    for (index, value) in output.iter_mut().enumerate() {
        *value = u8::from_str_radix(&text[index * 2..index * 2 + 2], 16)?;
    }
    Ok(output)
}
