// SPDX-License-Identifier: GPL-3.0-or-later
//! Command line front end: receive Quick Share transfers in the shared WLAN.

use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use log::{info, warn};
use tokio::net::TcpListener;

use t2_share::endpoint::random_endpoint_id;
use t2_share::mdns::Advertisement;
use t2_share::receive::{self, Incoming, Outcome};

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Show up in Android's share sheet and receive files and texts
    Receive {
        /// Name shown on the phone, the hostname by default
        #[arg(long)]
        name: Option<String>,
        /// Directory for received files, the XDG download directory by default
        #[arg(long)]
        dir: Option<PathBuf>,
        /// Accept every transfer without asking
        #[arg(long)]
        yes: bool,
        /// TCP port, a free one by default
        #[arg(long, default_value_t = 0)]
        port: u16,
    },
    /// Run as the background service: trusted devices are accepted, others
    /// are asked through a desktop notification
    Daemon {
        /// Name shown on the phone, the hostname by default
        #[arg(long)]
        name: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    match Cli::parse().command {
        Command::Receive { name, dir, yes, port } => receive_loop(name, dir, yes, port).await,
        Command::Daemon { name } => t2_share::service::run(&name.unwrap_or_else(default_name)).await,
    }
}

async fn receive_loop(name: Option<String>, dir: Option<PathBuf>, yes: bool, port: u16) -> Result<()> {
    let dir = dir
        .or_else(dirs::download_dir)
        .or_else(dirs::home_dir)
        .context("no download directory")?;
    std::fs::create_dir_all(&dir).with_context(|| format!("create {}", dir.display()))?;
    let name = name.unwrap_or_else(default_name);

    let listener = TcpListener::bind(("0.0.0.0", port)).await.context("listen")?;
    let port = listener.local_addr()?.port();
    let advertisement = Advertisement::start(random_endpoint_id(), &name, port)?;
    info!("visible as {name:?} on TCP port {port}, saving to {}", dir.display());

    let consent: receive::Consent = Arc::new(move |incoming: Incoming| {
        Box::pin(async move {
            if yes {
                describe(&incoming);
                return true;
            }
            tokio::task::spawn_blocking(move || ask(&incoming)).await.unwrap_or(false)
        })
    });

    let mut announce = tokio::time::interval(Duration::from_secs(10));
    loop {
        tokio::select! {
            accepted = listener.accept() => {
                let (stream, peer) = accepted?;
                info!("connection from {peer}");
                let dir = dir.clone();
                let consent = consent.clone();
                tokio::spawn(async move {
                    match receive::handle(stream, dir, consent).await {
                        Ok(Outcome::Received { files, texts }) => {
                            for file in files {
                                println!("saved {}", file.display());
                            }
                            for text in texts {
                                println!("text: {text}");
                            }
                        }
                        Ok(Outcome::Rejected) => println!("rejected"),
                        Ok(Outcome::Cancelled) => println!("cancelled by the sender"),
                        Err(err) => warn!("transfer failed: {err:#}"),
                    }
                });
            }
            _ = announce.tick() => {
                if let Err(err) = advertisement.announce() {
                    warn!("{err:#}");
                }
            }
            _ = tokio::signal::ctrl_c() => {
                info!("stopping");
                return Ok(());
            }
        }
    }
}

fn describe(incoming: &Incoming) {
    let sender = incoming.sender.as_deref().unwrap_or("a hidden device");
    println!("{sender} ({:?}) wants to send, PIN {}", incoming.device_type, incoming.pin);
    for file in &incoming.files {
        println!("  {} ({} bytes)", file.name, file.size);
    }
    for text in &incoming.texts {
        println!("  text: {text}");
    }
}

fn ask(incoming: &Incoming) -> bool {
    describe(incoming);
    print!("accept? [y/N] ");
    let _ = std::io::stdout().flush();
    let mut line = String::new();
    if std::io::stdin().lock().read_line(&mut line).is_err() {
        return false;
    }
    matches!(line.trim(), "y" | "Y" | "yes" | "j" | "ja")
}

fn default_name() -> String {
    std::fs::read_to_string("/proc/sys/kernel/hostname")
        .map(|h| h.trim().to_string())
        .ok()
        .filter(|h| !h.is_empty())
        .unwrap_or_else(|| "Linux".to_string())
}
