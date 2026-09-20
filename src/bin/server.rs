use std::io;

use multiplayer_fps::protocol::DEFAULT_SERVER_ADDRESS;
use multiplayer_fps::server;

fn invalid_input(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

fn main() -> io::Result<()> {
    let mut address = DEFAULT_SERVER_ADDRESS.to_string();
    let mut level: u8 = 1;
    let mut args = std::env::args().skip(1);

    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--level" => {
                let value = args
                    .next()
                    .ok_or_else(|| invalid_input("--level needs a number."))?;

                level = value
                    .parse()
                    .map_err(|_| invalid_input("Level must be 1, 2, or 3."))?;

                if !(1..=3).contains(&level) {
                    return Err(invalid_input("Level must be 1, 2, or 3."));
                }
            }

            "--bind" => {
                address = args
                    .next()
                    .ok_or_else(|| invalid_input("--bind needs an IP:port."))?;
            }

            "--help" | "-h" => {
                println!("Usage: server [--level 1|2|3] [--bind IP:port]");
                return Ok(());
            }

            _ => {
                return Err(invalid_input(&format!(
                    "Unknown argument: {argument}. Use --help."
                )));
            }
        }
    }

    server::run(&address, level)
}
