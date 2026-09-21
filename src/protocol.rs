use std::io;

use serde::{Deserialize, Serialize};

pub const DEFAULT_SERVER_ADDRESS: &str = "0.0.0.0:34254";
pub const MAX_PACKET_SIZE: usize = 1200;
pub const MAX_USERNAME_CHARS: usize = 20;
pub const MAX_PLAYERS: usize = 16;
pub const POSITION_SCALE: f32 = 1000.0;

// ID, x, y, angle, health, score, deaths.
// Coordinates and angle are scaled by 1000.
pub type WirePlayer = (u32, i32, i32, i32, u8, u32, u32);

#[derive(Debug, Serialize, Deserialize)]
pub enum ClientMessage {
    Join {
        username: String,
    },

    Input {
        player_id: u32,
        sequence: u64,
        forward: f32,
        strafe: f32,
        yaw: f32,
        shot_id: u64,
    },

    Leave {
        player_id: u32,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ServerMessage {
    Welcome {
        player_id: u32,
        level: u8,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        generated: Option<crate::generator::GeneratedMaze>,
    },
    Rejected {
        reason: String,
    },
    Snapshot {
        tick: u64,
        players: Vec<WirePlayer>,
    },
}

pub fn valid_username(username: &str) -> bool {
    !username.trim().is_empty()
        && username.chars().count() <= MAX_USERNAME_CHARS
        && !username.chars().any(char::is_control)
}

pub fn encode<T: Serialize>(message: &T) -> io::Result<Vec<u8>> {
    let bytes = serde_json::to_vec(message)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;

    if bytes.len() > MAX_PACKET_SIZE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Network message exceeds packet limit.",
        ));
    }

    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn welcome_descriptors_fit_and_old_fixed_welcomes_still_decode() {
        let message = ServerMessage::Welcome {
            player_id: u32::MAX,
            level: 0,
            generated: Some(crate::generator::GeneratedMaze {
                width: 51,
                height: 51,
                ..crate::generator::GeneratedMaze::new(u64::MAX)
            }),
        };
        let bytes = encode(&message).unwrap();
        assert!(bytes.len() < 200);
        assert!(matches!(
            serde_json::from_slice::<ServerMessage>(&bytes).unwrap(),
            ServerMessage::Welcome {
                generated: Some(_),
                ..
            }
        ));
        assert!(matches!(
            serde_json::from_str::<ServerMessage>(r#"{"Welcome":{"player_id":1,"level":2}}"#)
                .unwrap(),
            ServerMessage::Welcome {
                generated: None,
                level: 2,
                ..
            }
        ));
    }

    #[test]
    fn rejects_invalid_usernames() {
        assert!(!valid_username("   "));
        assert!(!valid_username("player\nname"));
        assert!(!valid_username(&"a".repeat(21)));
        assert!(valid_username("Taqi"));
    }

    #[test]
    fn full_snapshot_fits_packet_limit() {
        // Largest coordinates in the largest generated 51×51 maze,
        // plus maximum IDs, scores, and deaths.
        let players = vec![(u32::MAX, 51_000, 51_000, 6_284, 2, u32::MAX, u32::MAX,); MAX_PLAYERS];

        let message = ServerMessage::Snapshot {
            tick: u64::MAX,
            players,
        };

        assert!(encode(&message).is_ok());
    }
}
