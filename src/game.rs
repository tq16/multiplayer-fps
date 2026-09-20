use macroquad::prelude::*;

use crate::controls::Controls;
use crate::hud;
use crate::render;
use multiplayer_fps::client::NetworkClient;
use multiplayer_fps::maze::Maze;
use multiplayer_fps::player::{MovementInput, Player};

pub async fn run(mut client: NetworkClient) {
    let Some(maze) = Maze::from_level(client.level) else {
        eprintln!("Unsupported level: {}", client.level);
        return;
    };
    let mut controls = Controls::default();
    let mut connection_error: Option<String> = None;
    let mut initialized = false;
    let mut previous_shot_id = 0;

    loop {
        if is_key_pressed(KeyCode::Escape) && !controls.is_captured() {
            break;
        }

        let input = if initialized && connection_error.is_none() {
            controls.read()
        } else {
            MovementInput::default()
        };

        if controls.shot_id() > previous_shot_id {
            previous_shot_id = controls.shot_id();
            println!("Shot requested: {previous_shot_id}");
        }

        if connection_error.is_none() {
            if let Err(error) = client.update(&input, controls.angle(), controls.shot_id()) {
                eprintln!("{error}");
                connection_error = Some(error.to_string());
                controls.release();
            }
        }

        if let Some(player) = client.local_player() {
            if !initialized {
                controls.set_angle(player.angle);
                initialized = true;
            }

            // Render our latest facing angle without waiting for a reply.
            let camera_player = Player {
                x: player.x,
                y: player.y,
                angle: controls.angle(),
            };

            render::draw_game(&maze, &camera_player, &client.players, client.player_id);
            hud::draw_player_stats(client.health, client.score, client.deaths);
            draw_text(
                &format!("Level: {}", client.level),
                24.0,
                228.0,
                22.0,
                WHITE,
            );
        } else {
            clear_background(BLACK);
            draw_text("Waiting for game state...", 30.0, 50.0, 28.0, WHITE);
        }

        if let Some(error) = &connection_error {
            draw_rectangle(
                0.0,
                screen_height() * 0.5 - 45.0,
                screen_width(),
                90.0,
                BLACK,
            );

            draw_text(error, 25.0, screen_height() * 0.5, 24.0, RED);

            draw_text(
                "Press ESC to close, then reconnect.",
                25.0,
                screen_height() * 0.5 + 28.0,
                22.0,
                WHITE,
            );
        } else if initialized && !controls.is_captured() {
            draw_rectangle(
                0.0,
                screen_height() * 0.5 - 30.0,
                screen_width(),
                60.0,
                BLACK,
            );

            draw_text(
                "Click to play | ESC to quit",
                25.0,
                screen_height() * 0.5 + 8.0,
                28.0,
                WHITE,
            );
        }

        next_frame().await;
    }
}
