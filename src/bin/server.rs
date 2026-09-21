use std::io;

use multiplayer_fps::generator::GeneratedMaze;
use multiplayer_fps::protocol::DEFAULT_SERVER_ADDRESS;
use multiplayer_fps::server;

fn invalid_input(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

fn main() -> io::Result<()> {
    let mut address = DEFAULT_SERVER_ADDRESS.to_string();
    let mut level: u8 = 1;
    let mut bots = 0;
    let mut generated = false;
    let mut seed = None;
    let mut fixed_selected = false;
    let mut args = std::env::args().skip(1);

    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--bots" => {
                bots = args
                    .next()
                    .ok_or_else(|| invalid_input("--bots needs a count."))?
                    .parse::<usize>()
                    .map_err(|_| invalid_input("Invalid bot count."))?;
            }
            "--generated" => generated = true,
            "--seed" => {
                seed = Some(
                    args.next()
                        .ok_or_else(|| invalid_input("--seed needs a numeric seed."))?
                        .parse::<u64>()
                        .map_err(|_| invalid_input("Seed must be an unsigned 64-bit integer."))?,
                );
            }
            "--level" => {
                fixed_selected = true;
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
                println!(
                    "Usage: server [--level 1|2|3 | --generated [--seed N]] [--bind IP:port] [--bots 0..6]"
                );
                return Ok(());
            }

            _ => {
                return Err(invalid_input(&format!(
                    "Unknown argument: {argument}. Use --help."
                )));
            }
        }
    }

    if (generated && fixed_selected) || (seed.is_some() && !generated) {
        return Err(invalid_input(
            "Use --level or --generated; --seed requires --generated.",
        ));
    }
    let options = server::ServerOptions {
        bots,
        level: if generated { 0 } else { level },
        generated: generated.then(|| GeneratedMaze::new(seed.unwrap_or(0))),
    };
    server::run_with_options(&address, options)
}
