mod maze;
mod player;
mod raycast;
mod render;

use macroquad::prelude::*;

use maze::Maze;
use player::{MovementInput, Player};

fn window_conf() -> Conf {
    Conf {
        window_title: "Maze Wars".to_string(),
        window_width: 1000,
        window_height: 800,
        ..Default::default()
    }
}

fn key_axis(positive: KeyCode, negative: KeyCode) -> f32 {
    let positive = if is_key_down(positive) { 1.0 } else { 0.0 };
    let negative = if is_key_down(negative) { 1.0 } else { 0.0 };

    positive - negative
}

#[macroquad::main(window_conf)]
async fn main() {
    let maze = Maze::new();
    let mut player = Player::new();

    loop {
        if is_key_pressed(KeyCode::Escape) {
            break;
        }

        let input = MovementInput {
            forward: key_axis(KeyCode::W, KeyCode::S),
            strafe: key_axis(KeyCode::D, KeyCode::A),
            turn: key_axis(KeyCode::Right, KeyCode::Left),
        };

        player.update(&input, get_frame_time(), &maze);
        render::draw_game(&maze, &player);

        next_frame().await;
    }
}
