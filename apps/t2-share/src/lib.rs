// SPDX-License-Identifier: GPL-3.0-or-later
//! Quick Share (Nearby Share) for Linux on T2 Macs.
//!
//! The protocol follows NearDrop's PROTOCOL.md and the Nearby protobuf
//! definitions. Parts of the UKEY2 handshake and the secure channel are
//! adapted from rquickshare, see THIRD-PARTY-NOTICES.md.

pub mod config;
pub mod endpoint;
pub mod mdns;
pub mod notify;
pub mod proto;
pub mod receive;
pub mod secure;
pub mod service;
