use std::io::{self, Write};
use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

use multiplayer_fps::protocol::{ClientMessage, MAX_PACKET_SIZE, ServerMessage, valid_username};

const JOIN_ATTEMPTS: usize = 5;
const RETRY_INTERVAL: Duration = Duration::from_secs(1);

fn prompt(label: &str) -> io::Result<String> {
    print!("{label}");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    Ok(input.trim().to_string())
}

fn main() -> io::Result<()> {
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
            "Use a name of 1–20 characters without control characters.",
        ));
    }

    let bind_address = if server.is_ipv4() {
        "0.0.0.0:0"
    } else {
        "[::]:0"
    };

    let socket = UdpSocket::bind(bind_address)?;

    // UDP connect selects the peer; it does not perform a handshake.
    socket.connect(server)?;

    let message = ClientMessage::Join { username };
    let bytes = serde_json::to_vec(&message)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;

    let mut buffer = [0_u8; MAX_PACKET_SIZE + 1];

    for attempt in 1..=JOIN_ATTEMPTS {
        println!("Joining {server}, attempt {attempt}/{JOIN_ATTEMPTS}...");
        socket.send(&bytes)?;

        let deadline = Instant::now() + RETRY_INTERVAL;

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

            let response = match serde_json::from_slice::<ServerMessage>(&buffer[..size]) {
                Ok(response) => response,
                Err(_) => continue,
            };

            match response {
                ServerMessage::Welcome { player_id } => {
                    println!("Connected! Your player ID is {player_id}.");
                    return Ok(());
                }
                ServerMessage::Rejected { reason } => {
                    return Err(io::Error::new(io::ErrorKind::PermissionDenied, reason));
                }
            }
        }
    }

    Err(io::Error::new(
        io::ErrorKind::TimedOut,
        "Server did not reply. Check the address and that the server is running.",
    ))
}
