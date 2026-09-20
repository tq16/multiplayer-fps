use std::io;

use multiplayer_fps::client::NetworkClient;

fn main() -> io::Result<()> {
    let client = NetworkClient::from_prompt()?;

    println!("Connection test passed. Player ID: {}", client.player_id);

    Ok(())
}
