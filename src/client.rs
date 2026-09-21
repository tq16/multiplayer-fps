use std::io::{self, Write};
use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

use crate::player::{MovementInput, Player};
use crate::protocol::{
    ClientMessage, MAX_PACKET_SIZE, MAX_PLAYERS, POSITION_SCALE, ServerMessage, encode,
    valid_username,
};

const SEND_INTERVAL: Duration = Duration::from_millis(16);
const SERVER_TIMEOUT: Duration = Duration::from_secs(5);
const JOIN_ATTEMPTS: usize = 5;

pub struct NetworkClient {
    socket: UdpSocket,
    pub player_id: u32,
    pub players: Vec<(u32, Player)>,
    sequence: u64,
    last_tick: u64,
    last_send: Instant,
    last_snapshot: Instant,
    pub health: u8,
    pub score: u32,
    pub deaths: u32,
    pub level: u8,
    pub generated: Option<crate::generator::GeneratedMaze>,
}

fn prompt(label: &str) -> io::Result<String> {
    print!("{label}");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    Ok(input.trim().to_string())
}

impl NetworkClient {
    pub fn from_prompt() -> io::Result<Self> {
        let address = prompt("Server IP:port [127.0.0.1:34254]: ")?;
        let address = if address.is_empty() {
            "127.0.0.1:34254"
        } else {
            &address
        };

        let server: SocketAddr = address
            .parse()
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;

        let username = prompt("Username: ")?;

        if !valid_username(&username) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Use a username of 1–20 printable characters.",
            ));
        }

        Self::connect(server, username)
    }

    pub fn connect(server: SocketAddr, username: String) -> io::Result<Self> {
        if !valid_username(&username) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Use a username of 1–20 printable characters.",
            ));
        }
        let bind_address = if server.is_ipv4() {
            "0.0.0.0:0"
        } else {
            "[::]:0"
        };

        let socket = UdpSocket::bind(bind_address)?;
        socket.connect(server)?;

        let bytes = encode(&ClientMessage::Join { username })?;
        let mut buffer = [0_u8; MAX_PACKET_SIZE + 1];

        for attempt in 1..=JOIN_ATTEMPTS {
            println!("Joining {server}, attempt {attempt}/{JOIN_ATTEMPTS}...");
            socket.send(&bytes)?;

            let deadline = Instant::now() + Duration::from_secs(1);

            loop {
                let remaining = deadline.saturating_duration_since(Instant::now());

                if remaining.is_zero() {
                    break;
                }

                socket.set_read_timeout(Some(remaining))?;

                let size = match socket.recv(&mut buffer) {
                    Ok(size) => size,
                    Err(error)
                        if matches!(
                            error.kind(),
                            io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                        ) =>
                    {
                        break;
                    }
                    Err(error) if error.kind() == io::ErrorKind::Interrupted => {
                        continue;
                    }
                    Err(error) => return Err(error),
                };

                if size > MAX_PACKET_SIZE {
                    continue;
                }

                let Ok(message) = serde_json::from_slice::<ServerMessage>(&buffer[..size]) else {
                    continue;
                };

                match message {
                    ServerMessage::Welcome {
                        player_id,
                        level,
                        generated,
                    } => {
                        if (generated.is_none() && !(1..=3).contains(&level))
                            || generated.is_some_and(|config| config.build().is_none())
                        {
                            return Err(io::Error::new(
                                io::ErrorKind::InvalidData,
                                "Server sent an unsupported level.",
                            ));
                        }
                        socket.set_read_timeout(None)?;
                        socket.set_nonblocking(true)?;

                        println!("Connected! Player ID: {player_id}, Level: {level}");
                        return Ok(Self {
                            socket,
                            player_id,
                            players: Vec::new(),
                            sequence: 0,
                            last_tick: 0,
                            last_send: Instant::now(),
                            last_snapshot: Instant::now(),
                            health: 2,
                            score: 0,
                            deaths: 0,
                            level,
                            generated,
                        });
                    }
                    ServerMessage::Rejected { reason } => {
                        return Err(io::Error::new(io::ErrorKind::PermissionDenied, reason));
                    }
                    ServerMessage::Snapshot { .. } => {}
                }
            }
        }

        Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "Server did not reply.",
        ))
    }

    pub fn update(&mut self, input: &MovementInput, yaw: f32, shot_id: u64) -> io::Result<()> {
        if self.last_send.elapsed() >= SEND_INTERVAL {
            self.sequence += 1;

            let message = ClientMessage::Input {
                player_id: self.player_id,
                sequence: self.sequence,
                forward: input.forward,
                strafe: input.strafe,
                yaw,
                shot_id,
            };

            match self.socket.send(&encode(&message)?) {
                Ok(_) => self.last_send = Instant::now(),
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {}
                Err(error) => return Err(error),
            }
        }

        let mut buffer = [0_u8; MAX_PACKET_SIZE + 1];

        for _ in 0..128 {
            let size = match self.socket.recv(&mut buffer) {
                Ok(size) => size,
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error),
            };

            if size > MAX_PACKET_SIZE {
                continue;
            }

            let Ok(ServerMessage::Snapshot { tick, players }) =
                serde_json::from_slice::<ServerMessage>(&buffer[..size])
            else {
                continue;
            };

            if tick <= self.last_tick
                || players.len() > MAX_PLAYERS
                || !players.iter().any(|state| state.0 == self.player_id)
            {
                continue;
            }

            self.last_tick = tick;
            self.last_snapshot = Instant::now();

            if let Some((_, _, _, _, health, score, deaths)) =
                players.iter().find(|state| state.0 == self.player_id)
            {
                self.health = *health;
                self.score = *score;
                self.deaths = *deaths;
            }

            self.players = players
                .into_iter()
                .map(|(id, x, y, angle, _, _, _)| {
                    (
                        id,
                        Player {
                            x: x as f32 / POSITION_SCALE,
                            y: y as f32 / POSITION_SCALE,
                            angle: angle as f32 / POSITION_SCALE,
                        },
                    )
                })
                .collect();
        }

        if self.last_snapshot.elapsed() > SERVER_TIMEOUT {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "Lost connection to the server.",
            ));
        }

        Ok(())
    }

    pub fn local_player(&self) -> Option<&Player> {
        self.players
            .iter()
            .find(|(id, _)| *id == self.player_id)
            .map(|(_, player)| player)
    }
}

impl Drop for NetworkClient {
    fn drop(&mut self) {
        let message = ClientMessage::Leave {
            player_id: self.player_id,
        };

        if let Ok(bytes) = encode(&message) {
            // Best effort. The server timeout handles a lost leave packet.
            let _ = self.socket.send(&bytes);
        }
    }
}
