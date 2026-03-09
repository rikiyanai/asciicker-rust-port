//! Network protocol message types.
//!
//! Binary message types matching the C++ protocol (network.h):
//! - JoinRequest (STRUCT_REQ_JOIN)
//! - ExitNotice (STRUCT_BRC_EXIT)
//! - PoseUpdate (STRUCT_BRC_POSE)
//! - TalkMessage (STRUCT_BRC_TALK)
//!
//! Serialized via bincode 1.3.3 (little-endian, no padding, field-declaration order).
//! Wire compatibility with the C++ server is NOT required at this stage (bevy_replicon
//! adds its own framing), but field ordering matches C++ convention for future interop.
//!
//! OUT_OF_SCOPE: Combat messages (REQ_SWING, BRC_DAMAGE, etc.) require Phase 6+ combat systems.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Maximum player name length (matches C++ NAME_LEN 31).
pub const MAX_NAME_LEN: usize = 31;

/// Maximum talk message length in bytes.
pub const MAX_TALK_LEN: usize = 256;

// Compile-time size assertions (R19-004 FIX: prevent accidental struct bloat).
// These are wire-budget bounds, not exact C++ sizes (different serialization).
const _: () = assert!(
    std::mem::size_of::<PoseUpdate>() <= 64,
    "PoseUpdate exceeds 64-byte wire budget"
);
const _: () = assert!(
    std::mem::size_of::<JoinRequest>() <= 64,
    "JoinRequest exceeds 64-byte wire budget"
);
const _: () = assert!(
    std::mem::size_of::<ExitNotice>() <= 8,
    "ExitNotice exceeds 8-byte wire budget"
);

/// Client request to join the server.
///
/// C++ equivalent: STRUCT_REQ_JOIN. Name is truncated to MAX_NAME_LEN.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JoinRequest {
    /// Player display name (max 31 chars, truncated on construction).
    pub name: String,
}

impl JoinRequest {
    /// Create a new JoinRequest, truncating name to MAX_NAME_LEN characters.
    pub fn new(name: &str) -> Self {
        let truncated = if name.len() > MAX_NAME_LEN {
            name[..MAX_NAME_LEN].to_string()
        } else {
            name.to_string()
        };
        Self { name: truncated }
    }
}

/// Notice that a player has exited.
///
/// C++ equivalent: STRUCT_BRC_EXIT.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExitNotice {
    /// Reason code: 0 = normal disconnect, 1 = timeout, 2 = kicked.
    pub reason: u8,
}

/// Position/animation update broadcast.
///
/// C++ equivalent: STRUCT_BRC_POSE.
/// Component derive enables bevy_replicon replication.
/// Field order matches C++ packed struct convention (R19-003 FIX).
#[derive(Component, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PoseUpdate {
    /// Animation index.
    pub anim: u8,
    /// Animation frame.
    pub frame: u8,
    /// Action/mount flags.
    pub action_mount: u8,
    /// World position [x, y, z].
    pub pos: [f32; 3],
    /// Facing direction in radians.
    pub dir: f32,
    /// Sprite index.
    pub sprite: u16,
}

impl Default for PoseUpdate {
    fn default() -> Self {
        Self {
            anim: 0,
            frame: 0,
            action_mount: 0,
            pos: [0.0, 0.0, 0.0],
            dir: 0.0,
            sprite: 0,
        }
    }
}


/// Chat message broadcast.
///
/// C++ equivalent: STRUCT_BRC_TALK. Text is truncated to MAX_TALK_LEN bytes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TalkMessage {
    /// Chat text (max 256 bytes, truncated on construction).
    pub text: String,
}

impl TalkMessage {
    /// Create a new TalkMessage, truncating text to MAX_TALK_LEN bytes.
    pub fn new(text: &str) -> Self {
        let truncated = if text.len() > MAX_TALK_LEN {
            text[..MAX_TALK_LEN].to_string()
        } else {
            text.to_string()
        };
        Self { text: truncated }
    }
}

/// Envelope wrapping all network message types.
///
/// Used for channel-level message routing. Individual types are also
/// used directly via bevy_replicon's message system.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NetMessage {
    Join(JoinRequest),
    Exit(ExitNotice),
    Pose(PoseUpdate),
    Talk(TalkMessage),
}

/// Encode a NetMessage to bytes using bincode (little-endian).
pub fn encode_message(msg: &NetMessage) -> Result<Vec<u8>, bincode::Error> {
    bincode::serialize(msg)
}

/// Decode a NetMessage from bytes using bincode (little-endian).
pub fn decode_message(data: &[u8]) -> Result<NetMessage, bincode::Error> {
    bincode::deserialize(data)
}
