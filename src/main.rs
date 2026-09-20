mod controls;
mod game;
mod hud;
mod opponents;
mod render;

use std::io;

use macroquad::prelude::*;
use multiplayer_fps::client::NetworkClient;

fn window_conf() -> Conf {
    Conf {
        window_title: "Maze Wars".to_string(),
        window_width: 1000,
        window_height: 800,
        ..Default::default()
    }
}

fn main() -> io::Result<()> {
    let client = NetworkClient::from_prompt()?;

    macroquad::Window::from_config(window_conf(), game::run(client));

    Ok(())
}
