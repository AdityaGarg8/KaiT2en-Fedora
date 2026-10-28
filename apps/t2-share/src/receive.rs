// SPDX-License-Identifier: GPL-3.0-or-later
//! Receiving side (server) of a Quick Share transfer over one TCP
//! connection: connection request, UKEY2, paired key exchange, the
//! introduction with the user's consent, then payload chunks.
//!
//! The frame sequence follows NearDrop PROTOCOL.md and rquickshare
//! `core_lib/src/hdl/inbound.rs` (commit 378d8ae, GPL-3.0).

use std::collections::HashMap;
use std::fs::File;
use std::os::unix::fs::FileExt;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use log::{debug, info, warn};
use prost::Message;
use rand::Rng;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::endpoint::{DeviceType, parse_endpoint_info};
use crate::proto::location::nearby::connections as nc;
use crate::proto::sharing::nearby as sn;
use crate::secure::{SecureChannel, ServerHandshake};

/// Largest frame accepted from the wire, Android sends file chunks well
/// below this.
const MAX_FRAME: usize = 5 * 1024 * 1024;
/// Largest bytes payload, these carry protobuf frames and short texts.
const MAX_BYTES_PAYLOAD: i64 = 5 * 1024 * 1024;
/// Android expects a keep-alive at least every 10 s.
const KEEPALIVE: Duration = Duration::from_secs(5);
/// Silence after which the peer is considered gone.
const READ_TIMEOUT: Duration = Duration::from_secs(60);

/// What the sender offers, shown to the user before accepting.
#[derive(Clone, Debug)]
pub struct Incoming {
    pub sender: Option<String>,
    pub device_type: DeviceType,
    pub pin: String,
    pub files: Vec<IncomingFile>,
    pub texts: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct IncomingFile {
    pub name: String,
    pub size: i64,
}

/// Asked once per transfer, true accepts. The future runs as its own task,
/// keep-alives continue while it waits for the user.
pub type Consent =
    Arc<dyn Fn(Incoming) -> Pin<Box<dyn Future<Output = bool> + Send>> + Send + Sync>;

#[derive(Debug)]
pub enum Outcome {
    Received { files: Vec<PathBuf>, texts: Vec<String> },
    Rejected,
    Cancelled,
}

struct PendingFile {
    path: PathBuf,
    file: File,
    size: i64,
    written: i64,
}

struct Session {
    channel: SecureChannel,
    writer: OwnedWriteHalf,
    bytes: HashMap<i64, Vec<u8>>,
    files: HashMap<i64, PendingFile>,
    text_ids: HashMap<i64, String>,
    done_files: Vec<PathBuf>,
    done_texts: Vec<String>,
}

/// Runs one incoming connection to its end.
pub async fn handle(stream: TcpStream, dir: PathBuf, consent: Consent) -> Result<Outcome> {
    let peer = stream.peer_addr().ok();
    let (reader, writer) = stream.into_split();
    let (tx, mut rx) = mpsc::channel(16);
    let reader_task = tokio::spawn(read_frames(reader, tx));
    let result = run(&mut rx, writer, dir, consent, peer.map(|p| p.ip())).await;
    reader_task.abort();
    if let Err(err) = &result {
        warn!("connection from {peer:?}: {err:#}");
    }
    result
}

async fn run(
    rx: &mut mpsc::Receiver<Vec<u8>>,
    mut writer: OwnedWriteHalf,
    dir: PathBuf,
    consent: Consent,
    peer_ip: Option<std::net::IpAddr>,
) -> Result<Outcome> {
    let request = nc::OfflineFrame::decode(next(rx).await?.as_slice())?;
    let request = v1(&request, nc::v1_frame::FrameType::ConnectionRequest)?
        .connection_request
        .as_ref()
        .ok_or_else(|| anyhow!("connection request without body"))?;
    let (device_type, sender) = parse_endpoint_info(request.endpoint_info())
        .ok_or_else(|| anyhow!("connection request without endpoint info"))?;
    info!("connection request from {sender:?} ({device_type:?})");
    // Identity candidates for trusted devices, logged until it is clear
    // which of them stay the same between transfers.
    info!(
        "identity: endpoint_id {:?} endpoint_info {} peer MAC {:?}",
        request.endpoint_id(),
        hex(request.endpoint_info()),
        peer_mac(peer_ip),
    );

    let (handshake, server_init) = ServerHandshake::accept(&next(rx).await?)?;
    write_frame(&mut writer, &server_init).await?;
    let (channel, pin) = handshake.finish(&next(rx).await?)?;
    info!("UKEY2 done, PIN {pin}");

    let response = nc::OfflineFrame::decode(next(rx).await?.as_slice())?;
    v1(&response, nc::v1_frame::FrameType::ConnectionResponse)?;
    let accept = offline_frame(nc::V1Frame {
        r#type: Some(nc::v1_frame::FrameType::ConnectionResponse.into()),
        connection_response: Some(nc::ConnectionResponseFrame {
            response: Some(nc::connection_response_frame::ResponseStatus::Accept.into()),
            os_info: Some(nc::OsInfo {
                r#type: Some(nc::os_info::OsType::Linux.into()),
            }),
            ..Default::default()
        }),
        ..Default::default()
    });
    write_frame(&mut writer, &accept.encode_to_vec()).await?;

    let mut session = Session {
        channel,
        writer,
        bytes: HashMap::new(),
        files: HashMap::new(),
        text_ids: HashMap::new(),
        done_files: Vec::new(),
        done_texts: Vec::new(),
    };
    session
        .send_sharing(sn::V1Frame {
            r#type: Some(sn::v1_frame::FrameType::PairedKeyEncryption.into()),
            paired_key_encryption: Some(sn::PairedKeyEncryptionFrame {
                secret_id_hash: Some(random_bytes(6)),
                signed_data: Some(random_bytes(72)),
                ..Default::default()
            }),
            ..Default::default()
        })
        .await?;

    let mut keepalive = tokio::time::interval(KEEPALIVE);
    let mut pending_consent: Option<JoinHandle<bool>> = None;
    let mut accepted = false;
    loop {
        tokio::select! {
            raw = next(rx) => {
                let frame = session.channel.open(&raw?)?;
                let Some(v1) = frame.v1 else { continue };
                match v1.r#type() {
                    nc::v1_frame::FrameType::KeepAlive => {}
                    nc::v1_frame::FrameType::Disconnection => {
                        return Ok(if accepted { session.outcome() } else { Outcome::Cancelled });
                    }
                    nc::v1_frame::FrameType::PayloadTransfer => {
                        let transfer = v1.payload_transfer.unwrap_or_default();
                        if let Some(setup) = session.on_payload(transfer)? {
                            match setup.v1.map(|v| (v.r#type(), v)) {
                                Some((sn::v1_frame::FrameType::PairedKeyEncryption, v)) => {
                                    if let Some(pke) = &v.paired_key_encryption {
                                        info!(
                                            "identity: secret_id_hash {} signed_data {} optional_signed_data {}",
                                            hex(pke.secret_id_hash()),
                                            hex(pke.signed_data()),
                                            hex(pke.optional_signed_data()),
                                        );
                                    }
                                    session.send_sharing(sn::V1Frame {
                                        r#type: Some(sn::v1_frame::FrameType::PairedKeyResult.into()),
                                        paired_key_result: Some(sn::PairedKeyResultFrame {
                                            status: Some(sn::paired_key_result_frame::Status::Unable.into()),
                                            ..Default::default()
                                        }),
                                        ..Default::default()
                                    }).await?;
                                }
                                Some((sn::v1_frame::FrameType::PairedKeyResult, _)) => {}
                                Some((sn::v1_frame::FrameType::Introduction, v)) => {
                                    let intro = v.introduction.unwrap_or_default();
                                    let incoming = session.prepare(&intro, &dir, &pin, sender.clone(), device_type)?;
                                    let consent = consent.clone();
                                    pending_consent = Some(tokio::spawn(consent(incoming)));
                                }
                                Some((sn::v1_frame::FrameType::Cancel, _)) => {
                                    info!("sender cancelled");
                                    session.discard();
                                    return Ok(Outcome::Cancelled);
                                }
                                Some((other, _)) => debug!("ignoring sharing frame {other:?}"),
                                None => {}
                            }
                        }
                        if accepted && session.complete() {
                            session.send_offline(nc::V1Frame {
                                r#type: Some(nc::v1_frame::FrameType::Disconnection.into()),
                                disconnection: Some(nc::DisconnectionFrame::default()),
                                ..Default::default()
                            }).await?;
                            return Ok(session.outcome());
                        }
                    }
                    other => debug!("ignoring offline frame {other:?}"),
                }
            }
            answer = async { pending_consent.as_mut().unwrap().await }, if pending_consent.is_some() => {
                pending_consent = None;
                let status = if answer.unwrap_or(false) {
                    accepted = true;
                    sn::connection_response_frame::Status::Accept
                } else {
                    sn::connection_response_frame::Status::Reject
                };
                session.send_sharing(sn::V1Frame {
                    r#type: Some(sn::v1_frame::FrameType::Response.into()),
                    connection_response: Some(sn::ConnectionResponseFrame {
                        status: Some(status.into()),
                    }),
                    ..Default::default()
                }).await?;
                if !accepted {
                    session.discard();
                    return Ok(Outcome::Rejected);
                }
            }
            _ = keepalive.tick() => {
                session.send_offline(nc::V1Frame {
                    r#type: Some(nc::v1_frame::FrameType::KeepAlive.into()),
                    keep_alive: Some(nc::KeepAliveFrame { ack: Some(false), ..Default::default() }),
                    ..Default::default()
                }).await?;
            }
        }
    }
}

impl Session {
    /// Collects payload chunks. Returns a sharing frame once a bytes payload
    /// that carries one is complete.
    fn on_payload(&mut self, transfer: nc::PayloadTransferFrame) -> Result<Option<sn::Frame>> {
        let header = transfer.payload_header.unwrap_or_default();
        let chunk = transfer.payload_chunk.unwrap_or_default();
        let id = header.id();
        let last = chunk.flags() & 1 != 0;
        match header.r#type() {
            nc::payload_transfer_frame::payload_header::PayloadType::Bytes => {
                if header.total_size() > MAX_BYTES_PAYLOAD {
                    bail!("bytes payload of {} bytes", header.total_size());
                }
                let buffer = self.bytes.entry(id).or_default();
                if chunk.offset() != buffer.len() as i64 {
                    bail!("bytes payload chunk at {} after {} bytes", chunk.offset(), buffer.len());
                }
                buffer.extend_from_slice(chunk.body());
                if !last {
                    return Ok(None);
                }
                let data = self.bytes.remove(&id).unwrap_or_default();
                if let Some(title) = self.text_ids.remove(&id) {
                    let text = String::from_utf8_lossy(&data).into_owned();
                    info!("received text {title:?}");
                    self.done_texts.push(text);
                    return Ok(None);
                }
                Ok(Some(sn::Frame::decode(data.as_slice())?))
            }
            nc::payload_transfer_frame::payload_header::PayloadType::File => {
                let pending = self
                    .files
                    .get_mut(&id)
                    .ok_or_else(|| anyhow!("file payload {id} was not announced"))?;
                if chunk.offset() != pending.written {
                    bail!("file chunk at {} after {} bytes", chunk.offset(), pending.written);
                }
                let body = chunk.body();
                if pending.written + body.len() as i64 > pending.size {
                    bail!("file {} grows past its announced size", pending.path.display());
                }
                pending.file.write_all_at(body, pending.written as u64)?;
                pending.written += body.len() as i64;
                if last {
                    let pending = self.files.remove(&id).expect("present above");
                    pending.file.sync_all()?;
                    info!("received {}", pending.path.display());
                    self.done_files.push(pending.path);
                }
                Ok(None)
            }
            other => bail!("unsupported payload type {other:?}"),
        }
    }

    /// Creates the target files for an introduction and describes the
    /// transfer for the consent prompt.
    fn prepare(
        &mut self,
        intro: &sn::IntroductionFrame,
        dir: &Path,
        pin: &str,
        sender: Option<String>,
        device_type: DeviceType,
    ) -> Result<Incoming> {
        let mut files = Vec::new();
        for meta in &intro.file_metadata {
            let path = unique_path(dir, meta.name());
            let file = File::options()
                .write(true)
                .create_new(true)
                .open(&path)
                .with_context(|| format!("create {}", path.display()))?;
            files.push(IncomingFile { name: meta.name().to_string(), size: meta.size() });
            self.files.insert(
                meta.payload_id(),
                PendingFile { path, file, size: meta.size(), written: 0 },
            );
        }
        let mut texts = Vec::new();
        for meta in &intro.text_metadata {
            texts.push(meta.text_title().to_string());
            self.text_ids.insert(meta.payload_id(), meta.text_title().to_string());
        }
        Ok(Incoming { sender, device_type, pin: pin.to_string(), files, texts })
    }

    fn complete(&self) -> bool {
        self.files.is_empty() && self.text_ids.is_empty()
    }

    fn outcome(&mut self) -> Outcome {
        Outcome::Received {
            files: std::mem::take(&mut self.done_files),
            texts: std::mem::take(&mut self.done_texts),
        }
    }

    /// Removes files created for a transfer that did not complete.
    fn discard(&mut self) {
        for (_, pending) in self.files.drain() {
            let _ = std::fs::remove_file(&pending.path);
        }
    }

    async fn send_offline(&mut self, v1: nc::V1Frame) -> Result<()> {
        let sealed = self.channel.seal(&offline_frame(v1));
        write_frame(&mut self.writer, &sealed).await
    }

    /// Sends a sharing frame as a bytes payload: one chunk with the data,
    /// one empty chunk with the last chunk flag, as Android does.
    async fn send_sharing(&mut self, v1: sn::V1Frame) -> Result<()> {
        let data = sn::Frame {
            version: Some(sn::frame::Version::V1.into()),
            v1: Some(v1),
        }
        .encode_to_vec();
        let header = nc::payload_transfer_frame::PayloadHeader {
            id: Some(rand::thread_rng().r#gen::<i64>().abs()),
            r#type: Some(nc::payload_transfer_frame::payload_header::PayloadType::Bytes.into()),
            total_size: Some(data.len() as i64),
            is_sensitive: Some(false),
            ..Default::default()
        };
        let size = data.len() as i64;
        for (offset, flags, body) in [(0, 0, data), (size, 1, Vec::new())] {
            self.send_offline(nc::V1Frame {
                r#type: Some(nc::v1_frame::FrameType::PayloadTransfer.into()),
                payload_transfer: Some(nc::PayloadTransferFrame {
                    packet_type: Some(nc::payload_transfer_frame::PacketType::Data.into()),
                    payload_header: Some(header.clone()),
                    payload_chunk: Some(nc::payload_transfer_frame::PayloadChunk {
                        offset: Some(offset),
                        flags: Some(flags),
                        body: Some(body),
                        ..Default::default()
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .await?;
        }
        Ok(())
    }
}

fn offline_frame(v1: nc::V1Frame) -> nc::OfflineFrame {
    nc::OfflineFrame {
        version: Some(nc::offline_frame::Version::V1.into()),
        v1: Some(v1),
    }
}

fn v1(frame: &nc::OfflineFrame, expected: nc::v1_frame::FrameType) -> Result<&nc::V1Frame> {
    let v1 = frame.v1.as_ref().ok_or_else(|| anyhow!("offline frame without v1"))?;
    if v1.r#type() != expected {
        bail!("expected {expected:?}, got {:?}", v1.r#type());
    }
    Ok(v1)
}

/// Reads length prefixed frames until the connection ends.
async fn read_frames(mut reader: OwnedReadHalf, tx: mpsc::Sender<Vec<u8>>) {
    loop {
        let mut len = [0u8; 4];
        if reader.read_exact(&mut len).await.is_err() {
            return;
        }
        let len = u32::from_be_bytes(len) as usize;
        if len > MAX_FRAME {
            warn!("dropping connection, frame of {len} bytes");
            return;
        }
        let mut frame = vec![0u8; len];
        if reader.read_exact(&mut frame).await.is_err() || tx.send(frame).await.is_err() {
            return;
        }
    }
}

async fn next(rx: &mut mpsc::Receiver<Vec<u8>>) -> Result<Vec<u8>> {
    match tokio::time::timeout(READ_TIMEOUT, rx.recv()).await {
        Ok(Some(frame)) => Ok(frame),
        Ok(None) => bail!("connection closed"),
        Err(_) => bail!("no frame for {} s", READ_TIMEOUT.as_secs()),
    }
}

async fn write_frame(writer: &mut OwnedWriteHalf, data: &[u8]) -> Result<()> {
    writer.write_all(&(data.len() as u32).to_be_bytes()).await?;
    writer.write_all(data).await?;
    writer.flush().await?;
    Ok(())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// The peer's MAC from the kernel neighbour table, IPv4 only.
fn peer_mac(ip: Option<std::net::IpAddr>) -> Option<String> {
    let ip = ip?.to_string();
    let table = std::fs::read_to_string("/proc/net/arp").ok()?;
    table
        .lines()
        .skip(1)
        .map(|line| line.split_whitespace().collect::<Vec<_>>())
        .find(|cols| cols.first() == Some(&ip.as_str()))
        .and_then(|cols| cols.get(3).map(|mac| mac.to_string()))
}

fn random_bytes(len: usize) -> Vec<u8> {
    let mut data = vec![0u8; len];
    rand::thread_rng().fill(data.as_mut_slice());
    data
}

/// A free path in `dir` for a name the sender chose. Only the last path
/// component is used, an existing file gets " (n)" before the extension.
fn unique_path(dir: &Path, name: &str) -> PathBuf {
    let base = Path::new(name)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .filter(|n| !n.is_empty() && n != "." && n != "..")
        .unwrap_or_else(|| "received".to_string());
    let candidate = dir.join(&base);
    if !candidate.exists() {
        return candidate;
    }
    let (stem, ext) = match base.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() => (stem.to_string(), format!(".{ext}")),
        _ => (base.clone(), String::new()),
    };
    (1..)
        .map(|n| dir.join(format!("{stem} ({n}){ext}")))
        .find(|p| !p.exists())
        .expect("unbounded range")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unique_path_strips_directories_and_numbers_duplicates() {
        let dir = std::env::temp_dir().join(format!("t2share-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(unique_path(&dir, "../../etc/passwd"), dir.join("passwd"));
        std::fs::write(dir.join("photo.jpg"), b"x").unwrap();
        assert_eq!(unique_path(&dir, "photo.jpg"), dir.join("photo (1).jpg"));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
