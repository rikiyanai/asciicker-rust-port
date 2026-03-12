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

impl PoseUpdate {
    /// Create a PoseUpdate at a specific position (for testing).
    #[cfg(test)]
    pub fn test_at(x: f32, y: f32, z: f32) -> Self {
        Self {
            pos: [x, y, z],
            ..Default::default()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_join_request_roundtrip() {
        let msg = NetMessage::Join(JoinRequest::new("TestPlayer"));
        let bytes = encode_message(&msg).unwrap();
        let decoded = decode_message(&bytes).unwrap();
        assert_eq!(msg, decoded);
    }

    #[test]
    fn test_exit_notice_roundtrip() {
        let msg = NetMessage::Exit(ExitNotice { reason: 1 });
        let bytes = encode_message(&msg).unwrap();
        let decoded = decode_message(&bytes).unwrap();
        assert_eq!(msg, decoded);
    }

    #[test]
    fn test_pose_update_roundtrip() {
        let msg = NetMessage::Pose(PoseUpdate {
            anim: 3,
            frame: 7,
            action_mount: 1,
            pos: [100.5, -200.25, 50.0],
            dir: std::f32::consts::PI,
            sprite: 42,
        });
        let bytes = encode_message(&msg).unwrap();
        let decoded = decode_message(&bytes).unwrap();
        assert_eq!(msg, decoded);
    }

    #[test]
    fn test_talk_message_roundtrip() {
        let msg = NetMessage::Talk(TalkMessage::new("Hello, world!"));
        let bytes = encode_message(&msg).unwrap();
        let decoded = decode_message(&bytes).unwrap();
        assert_eq!(msg, decoded);
    }

    #[test]
    fn test_max_length_name_truncation() {
        let long_name = "A".repeat(100);
        let join = JoinRequest::new(&long_name);
        assert_eq!(join.name.len(), MAX_NAME_LEN);

        // Round-trip the truncated message
        let msg = NetMessage::Join(join);
        let bytes = encode_message(&msg).unwrap();
        let decoded = decode_message(&bytes).unwrap();
        assert_eq!(msg, decoded);
    }

    #[test]
    fn test_max_length_talk_truncation() {
        let long_text = "B".repeat(500);
        let talk = TalkMessage::new(&long_text);
        assert_eq!(talk.text.len(), MAX_TALK_LEN);

        let msg = NetMessage::Talk(talk);
        let bytes = encode_message(&msg).unwrap();
        let decoded = decode_message(&bytes).unwrap();
        assert_eq!(msg, decoded);
    }

    #[test]
    fn test_float_preservation() {
        // Verify exact float values survive serialization
        let pos = [1.23456789_f32, -9.87654321, 0.00001];
        let dir = 3.14159265_f32;
        let msg = NetMessage::Pose(PoseUpdate {
            anim: 0,
            frame: 0,
            action_mount: 0,
            pos,
            dir,
            sprite: 0,
        });
        let bytes = encode_message(&msg).unwrap();
        let decoded = decode_message(&bytes).unwrap();
        if let NetMessage::Pose(pose) = decoded {
            assert_eq!(pose.pos, pos, "Float positions must be bit-exact");
            assert_eq!(pose.dir, dir, "Float direction must be bit-exact");
        } else {
            panic!("Expected Pose variant");
        }
    }

    #[test]
    fn test_pose_update_default() {
        let pose = PoseUpdate::default();
        assert_eq!(pose.anim, 0);
        assert_eq!(pose.frame, 0);
        assert_eq!(pose.action_mount, 0);
        assert_eq!(pose.pos, [0.0, 0.0, 0.0]);
        assert_eq!(pose.dir, 0.0);
        assert_eq!(pose.sprite, 0);
    }

    /// R16-F206 FIX: Deterministic test for all message types (no timing dependency).
    #[test]
    fn test_protocol_roundtrip_all_message_types() {
        let messages = vec![
            NetMessage::Join(JoinRequest::new("Player1")),
            NetMessage::Exit(ExitNotice { reason: 0 }),
            NetMessage::Pose(PoseUpdate {
                anim: 5,
                frame: 12,
                action_mount: 2,
                pos: [1.0, 2.0, 3.0],
                dir: 1.5707,
                sprite: 100,
            }),
            NetMessage::Talk(TalkMessage::new("gg")),
        ];

        for msg in &messages {
            let bytes = encode_message(msg).unwrap();
            let decoded = decode_message(&bytes).unwrap();
            assert_eq!(
                msg,
                &decoded,
                "Round-trip failed for {:?}",
                std::mem::discriminant(msg)
            );
        }
    }
}
