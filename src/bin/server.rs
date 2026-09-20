use std::collections::HashMap;
use std::io;
use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

use multiplayer_fps::protocol::{
    ClientMessage, DEFAULT_SERVER_ADDRESS, MAX_PACKET_SIZE, ServerMessage, valid_username,
};

const MAX_PLAYERS: usize = 64;
const PLAYER_TIMEOUT: Duration = Duration::from_secs(30);

struct ConnectedPlayer {
    id: u32,
    username: String,
    last_seen: Instant,
}

fn main() -> io::Result<()> {
    let address = std::env::args()
        .nth(1)
        .unwrap_or_else(|| DEFAULT_SERVER_ADDRESS.to_string());

    let socket = UdpSocket::bind(&address)?;
    socket.set_read_timeout(Some(Duration::from_millis(100)))?;

    let mut players: HashMap<SocketAddr, ConnectedPlayer> = HashMap::new();
    let mut next_id: u32 = 1;

    // An extra byte helps detect packets larger than our limit.
    let mut buffer = [0_u8; MAX_PACKET_SIZE + 1];

    println!("Server listening on {}", socket.local_addr()?);
    println!("Capacity: {MAX_PLAYERS} players");
    println!("Press Ctrl+C to stop.");

    loop {
        players.retain(|address, player| {
            let active = player.last_seen.elapsed() < PLAYER_TIMEOUT;

            if !active {
                println!("{} ({address}) timed out", player.username);
            }

            active
        });

        let (size, address) = match socket.recv_from(&mut buffer) {
            Ok(packet) => packet,
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) =>
            {
                continue;
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {
                continue;
            }
            Err(error) => return Err(error),
        };

        if size > MAX_PACKET_SIZE {
            continue;
        }

        let message = match serde_json::from_slice::<ClientMessage>(&buffer[..size]) {
            Ok(message) => message,
            Err(_) => continue,
        };

        let response = match message {
            ClientMessage::Join { username } => {
                let username = username.trim().to_string();

                if !valid_username(&username) {
                    ServerMessage::Rejected {
                        reason: "Use a name of 1–20 characters without control characters."
                            .to_string(),
                    }
                } else if let Some(player) = players.get_mut(&address) {
                    player.last_seen = Instant::now();

                    ServerMessage::Welcome {
                        player_id: player.id,
                    }
                } else if players.len() >= MAX_PLAYERS {
                    ServerMessage::Rejected {
                        reason: "Server is full.".to_string(),
                    }
                } else if next_id == u32::MAX {
                    ServerMessage::Rejected {
                        reason: "Player IDs exhausted. Restart the server.".to_string(),
                    }
                } else {
                    let player_id = next_id;
                    next_id += 1;

                    println!("{username} joined from {address}, ID: {player_id}");

                    players.insert(
                        address,
                        ConnectedPlayer {
                            id: player_id,
                            username,
                            last_seen: Instant::now(),
                        },
                    );

                    ServerMessage::Welcome { player_id }
                }
            }
        };

        let bytes = serde_json::to_vec(&response)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;

        if let Err(error) = socket.send_to(&bytes, address) {
            eprintln!("Could not reply to {address}: {error}");
        }
    }
}
