use serde::{Deserialize, Serialize};

pub const DEFAULT_SERVER_ADDRESS: &str = "0.0.0.0:34254";
pub const MAX_PACKET_SIZE: usize = 1200;
pub const MAX_USERNAME_CHARS: usize = 20;

#[derive(Debug, Serialize, Deserialize)]
pub enum ClientMessage {
    Join { username: String },
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ServerMessage {
    Welcome { player_id: u32 },
    Rejected { reason: String },
}

pub fn valid_username(username: &str) -> bool {
    !username.trim().is_empty()
        && username.chars().count() <= MAX_USERNAME_CHARS
        && !username.chars().any(char::is_control)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_usernames() {
        assert!(!valid_username("   "));
        assert!(!valid_username("player\nname"));
        assert!(!valid_username(&"a".repeat(21)));
        assert!(valid_username("Taqi"));
    }

    #[test]
    fn join_message_survives_encoding_and_decoding() {
        let message = ClientMessage::Join {
            username: "Taqi".to_string(),
        };

        let bytes = serde_json::to_vec(&message).unwrap();
        let decoded: ClientMessage = serde_json::from_slice(&bytes).unwrap();

        let ClientMessage::Join { username } = decoded;
        assert_eq!(username, "Taqi");
    }
}
