mod connection_screen;
mod controls;
mod effects;
mod game;
mod hud;
mod opponents;
mod render;

use macroquad::prelude::*;

fn window_conf() -> Conf {
    Conf {
        window_title: "Maze Wars".to_string(),
        window_width: 1000,
        window_height: 800,
        ..Default::default()
    }
}

fn main() {
    macroquad::Window::from_config(window_conf(), connection_screen::run());
}
