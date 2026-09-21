use crate::bot::Bot;
use crate::combat;
use crate::generator::GeneratedMaze;

use crate::maze::Maze;
use crate::player::{MovementInput, PLAYER_RADIUS, Player};
use crate::protocol::{
    ClientMessage, MAX_PACKET_SIZE, MAX_PLAYERS, POSITION_SCALE, ServerMessage, encode,
    valid_username,
};
use std::collections::HashMap;
use std::io;
use std::net::{SocketAddr, UdpSocket};
use std::thread;
use std::time::{Duration, Instant};

pub const MAX_BOTS: usize = MAX_PLAYERS - 10;

#[derive(Clone, Copy)]
pub struct ServerOptions {
    pub bots: usize,
    pub level: u8,
    pub generated: Option<GeneratedMaze>,
}

impl Default for ServerOptions {
    fn default() -> Self {
        Self {
            bots: 0,
            level: 1,
            generated: None,
        }
    }
}

const MAX_HEALTH: u8 = 2;
const SHOT_COOLDOWN: Duration = Duration::from_millis(350);
const RESPAWN_PROTECTION: Duration = Duration::from_secs(2);
const TICK_DURATION: Duration = Duration::from_nanos(16_666_667);
const PLAYER_TIMEOUT: Duration = Duration::from_secs(10);
const INPUT_TIMEOUT: Duration = Duration::from_millis(250);
const MAX_PACKETS_PER_TICK: usize = 256;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Participant {
    Human(SocketAddr),
    Bot(u32),
}

struct ConnectedPlayer {
    bot: Option<Bot>,
    id: u32,
    username: String,
    player: Player,
    input: MovementInput,
    sequence: u64,
    last_seen: Instant,
    last_shot_id: u64,
    health: u8,
    score: u32,
    deaths: u32,
    last_shot_at: Option<Instant>,
    protected_until: Instant,
}

fn find_spawn(maze: &Maze, players: &HashMap<Participant, ConnectedPlayer>) -> Option<Player> {
    for y in 0..maze.height() {
        for x in 0..maze.width() {
            let x = x as f32 + 0.5;
            let y = y as f32 + 0.5;

            let occupied = players.values().any(|other| {
                let dx = other.player.x - x;
                let dy = other.player.y - y;
                dx * dx + dy * dy < 1.0
            });

            if !occupied && maze.can_occupy(x, y, PLAYER_RADIUS) {
                return Some(Player { x, y, angle: 0.0 });
            }
        }
    }

    None
}

fn reply(socket: &UdpSocket, address: SocketAddr, message: ServerMessage) -> io::Result<()> {
    let bytes = encode(&message)?;

    match socket.send_to(&bytes, address) {
        Ok(_) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::WouldBlock => Ok(()),
        Err(error) => {
            eprintln!("Could not send to {address}: {error}");
            Ok(())
        }
    }
}

fn handle_message(
    socket: &UdpSocket,
    address: SocketAddr,
    message: ClientMessage,
    maze: &Maze,
    players: &mut HashMap<Participant, ConnectedPlayer>,
    next_id: &mut u32,
    options: ServerOptions,
) -> io::Result<()> {
    match message {
        ClientMessage::Join { username } => {
            let username = username.trim().to_string();

            let response = if !valid_username(&username) {
                ServerMessage::Rejected {
                    reason: "Use a username of 1–20 printable characters.".into(),
                }
            } else if let Some(existing) = players.get_mut(&Participant::Human(address)) {
                existing.last_seen = Instant::now();

                ServerMessage::Welcome {
                    player_id: existing.id,
                    level: options.level,
                    generated: options.generated,
                }
            } else if players.len() >= MAX_PLAYERS {
                ServerMessage::Rejected {
                    reason: "Server is full.".into(),
                }
            } else if *next_id == u32::MAX {
                ServerMessage::Rejected {
                    reason: "Player IDs exhausted. Restart the server.".into(),
                }
            } else if let Some(player) = find_spawn(maze, players) {
                let id = *next_id;
                *next_id += 1;

                println!("{username} joined from {address}, ID: {id}");

                players.insert(
                    Participant::Human(address),
                    ConnectedPlayer {
                        bot: None,
                        id,
                        username,
                        player,
                        input: MovementInput::default(),
                        sequence: 0,
                        last_seen: Instant::now(),
                        last_shot_id: 0,
                        health: MAX_HEALTH,
                        score: 0,
                        deaths: 0,
                        last_shot_at: None,
                        protected_until: Instant::now() + RESPAWN_PROTECTION,
                    },
                );

                ServerMessage::Welcome {
                    player_id: id,
                    level: options.level,
                    generated: options.generated,
                }
            } else {
                ServerMessage::Rejected {
                    reason: "No free spawn position.".into(),
                }
            };

            reply(socket, address, response)?;
        }

        ClientMessage::Input {
            player_id,
            sequence,
            forward,
            strafe,
            yaw,
            shot_id,
        } => {
            if !forward.is_finite() || !strafe.is_finite() || !yaw.is_finite() {
                return Ok(());
            }

            let Some(connected) = players.get_mut(&Participant::Human(address)) else {
                return Ok(());
            };

            if connected.id != player_id || sequence <= connected.sequence {
                return Ok(());
            }

            connected.sequence = sequence;
            connected.last_seen = Instant::now();

            connected.player.angle = yaw.rem_euclid(std::f32::consts::TAU);

            connected.input = MovementInput {
                forward: forward.clamp(-1.0, 1.0),
                strafe: strafe.clamp(-1.0, 1.0),
                turn: 0.0,
            };

            if shot_id > connected.last_shot_id {
                connected.last_shot_id = shot_id;
                resolve_shot(player_id, maze, players);
            }
        }

        ClientMessage::Leave { player_id } => {
            let matches = players
                .get(&Participant::Human(address))
                .map(|player| player.id == player_id)
                .unwrap_or(false);

            if matches && let Some(player) = players.remove(&Participant::Human(address)) {
                println!("{} left", player.username);
            }
        }
    }

    Ok(())
}

fn resolve_shot(shooter_id: u32, maze: &Maze, players: &mut HashMap<Participant, ConnectedPlayer>) {
    let now = Instant::now();

    let shooter_position = {
        let Some(shooter) = players.values_mut().find(|player| player.id == shooter_id) else {
            return;
        };

        if let Some(last_shot) = shooter.last_shot_at
            && now.duration_since(last_shot) < SHOT_COOLDOWN
        {
            return;
        }

        shooter.last_shot_at = Some(now);

        // Shooting ends your own spawn protection.
        shooter.protected_until = now;

        Player {
            x: shooter.player.x,
            y: shooter.player.y,
            angle: shooter.player.angle,
        }
    };

    let target_id = combat::find_target(
        maze,
        &shooter_position,
        players
            .values()
            .filter(|player| player.id != shooter_id)
            .map(|player| (player.id, &player.player)),
    );

    let Some(target_id) = target_id else {
        println!("Player {shooter_id} missed.");
        return;
    };

    {
        let Some(target) = players.values_mut().find(|player| player.id == target_id) else {
            return;
        };

        if now < target.protected_until {
            println!("{} still has spawn protection.", target.username);
            return;
        }

        if target.health > 1 {
            target.health -= 1;

            println!(
                "{} was hit! Health: {}/{}",
                target.username, target.health, MAX_HEALTH,
            );

            return;
        }
    }

    // The next hit defeats the target. Find a free respawn position.
    let Some(mut spawn) = find_spawn(maze, players) else {
        println!("No free respawn position; defeat was not applied.");
        return;
    };

    if let Some(target) = players.values_mut().find(|player| player.id == target_id) {
        // Preserve facing so mouse controls remain consistent.
        spawn.angle = target.player.angle;

        if let Some(bot) = &mut target.bot {
            bot.reset();
        }
        target.player = spawn;
        target.health = MAX_HEALTH;
        target.input = MovementInput::default();
        target.deaths = target.deaths.saturating_add(1);
        target.protected_until = now + RESPAWN_PROTECTION;

        println!(
            "{} was defeated and respawned. Deaths: {}",
            target.username, target.deaths,
        );
    }

    if let Some(shooter) = players.values_mut().find(|player| player.id == shooter_id) {
        shooter.score = shooter.score.saturating_add(1);

        println!(
            "{} earned a point! Score: {}",
            shooter.username, shooter.score,
        );
    }
}

pub fn run(address: &str, level: u8) -> io::Result<()> {
    run_with_options(
        address,
        ServerOptions {
            level,
            ..Default::default()
        },
    )
}

pub fn run_with_options(address: &str, options: ServerOptions) -> io::Result<()> {
    if options.bots > MAX_BOTS {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("At most {MAX_BOTS} bots are allowed; 10 human slots are reserved."),
        ));
    }
    let socket = UdpSocket::bind(address)?;
    socket.set_nonblocking(true)?;

    let maze = options
        .generated
        .map_or_else(|| Maze::from_level(options.level), |config| config.build())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid level or unsupported generated-maze descriptor.",
            )
        })?;
    let mut players = HashMap::new();
    let mut next_id = 1;
    for index in 0..options.bots {
        let id = next_id;
        next_id += 1;
        let player = find_spawn(&maze, &players)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "No free bot spawn."))?;
        players.insert(
            Participant::Bot(id),
            ConnectedPlayer {
                bot: Some(Bot::new(id, &maze)),
                id,
                username: format!("BOT-{}", index + 1),
                player,
                input: MovementInput::default(),
                sequence: 0,
                last_seen: Instant::now(),
                last_shot_id: 0,
                health: MAX_HEALTH,
                score: 0,
                deaths: 0,
                last_shot_at: None,
                protected_until: Instant::now() + RESPAWN_PROTECTION,
            },
        );
    }
    let mut tick = 0_u64;
    let mut deadline = Instant::now();
    let mut buffer = [0_u8; MAX_PACKET_SIZE + 1];

    println!("Server listening on {}", socket.local_addr()?);
    println!(
        "Level: {}, generated: {:?}",
        options.level, options.generated
    );
    println!("Capacity: {MAX_PLAYERS} players. Press Ctrl+C to stop.");

    loop {
        players.retain(|_, player: &mut ConnectedPlayer| {
            let active = player.bot.is_some() || player.last_seen.elapsed() < PLAYER_TIMEOUT;

            if !active {
                println!("{} timed out", player.username);
            }

            active
        });

        // Bound incoming work so packets cannot starve simulation updates.
        for _ in 0..MAX_PACKETS_PER_TICK {
            let (size, address) = match socket.recv_from(&mut buffer) {
                Ok(packet) => packet,
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error),
            };

            if size > MAX_PACKET_SIZE {
                continue;
            }

            let Ok(message) = serde_json::from_slice::<ClientMessage>(&buffer[..size]) else {
                continue;
            };

            handle_message(
                &socket,
                address,
                message,
                &maze,
                &mut players,
                &mut next_id,
                options,
            )?;
        }

        let positions: Vec<_> = players.values().map(|p| (p.id, p.player)).collect();
        let mut bot_shots = Vec::new();
        for connected in players.values_mut() {
            if let Some(bot) = &mut connected.bot {
                let (input, shoot) = bot.update(
                    connected.id,
                    &mut connected.player,
                    &positions,
                    &maze,
                    TICK_DURATION.as_secs_f32(),
                );
                connected.input = input;
                if shoot {
                    bot_shots.push(connected.id);
                }
            } else if connected.last_seen.elapsed() > INPUT_TIMEOUT {
                connected.input = MovementInput::default();
            }

            connected
                .player
                .update(&connected.input, TICK_DURATION.as_secs_f32(), &maze);
        }

        for id in bot_shots {
            resolve_shot(id, &maze, &mut players);
        }

        tick += 1;

        let states = players
            .values()
            .map(|connected| {
                let player = &connected.player;

                (
                    connected.id,
                    (player.x * POSITION_SCALE).round() as i32,
                    (player.y * POSITION_SCALE).round() as i32,
                    (player.angle * POSITION_SCALE).round() as i32,
                    connected.health,
                    connected.score,
                    connected.deaths,
                )
            })
            .collect();

        let bytes = encode(&ServerMessage::Snapshot {
            tick,
            players: states,
        })?;

        for participant in players.keys() {
            let Participant::Human(address) = participant else {
                continue;
            };
            if let Err(error) = socket.send_to(&bytes, address)
                && error.kind() != io::ErrorKind::WouldBlock
            {
                eprintln!("Snapshot to {address} failed: {error}");
            }
        }

        deadline += TICK_DURATION;

        if let Some(remaining) = deadline.checked_duration_since(Instant::now()) {
            thread::sleep(remaining);
        } else {
            // Avoid a burst of catch-up ticks after a long stall.
            deadline = Instant::now();
        }
    }
}

#[cfg(test)]
#[path = "server_tests.rs"]
mod tests;
