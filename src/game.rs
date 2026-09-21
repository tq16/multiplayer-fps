use macroquad::prelude::*;

use crate::controls::Controls;
use crate::effects::ShotEffects;
use crate::hud;
use crate::render;
use multiplayer_fps::client::NetworkClient;
use multiplayer_fps::maze::Maze;
use multiplayer_fps::player::{MovementInput, Player};

pub async fn run(mut client: NetworkClient) -> Option<String> {
    let Some(maze) = client
        .generated
        .map_or_else(|| Maze::from_level(client.level), |config| config.build())
    else {
        eprintln!("Unsupported level: {}", client.level);
        return Some("Server selected an unsupported maze.".into());
    };
    let mut controls = Controls::default();
    let mut initialized = false;
    let mut previous_shot_id = 0;
    let mut effects = ShotEffects::default();

    loop {
        if is_key_pressed(KeyCode::Escape) && !controls.is_captured() {
            return None;
        }

        let input = if initialized {
            controls.read()
        } else {
            MovementInput::default()
        };

        effects.update(get_frame_time());
        if controls.shot_id() > previous_shot_id {
            previous_shot_id = controls.shot_id();
            effects.trigger();
        }

        if let Err(error) = client.update(&input, controls.angle(), controls.shot_id()) {
            controls.release();
            return Some(error.to_string());
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
            effects.draw();
            hud::draw_player_stats(client.health, client.score, client.deaths);
            draw_text(
                client.generated.map_or_else(
                    || format!("Level: {}", client.level),
                    |config| format!("Generated | Seed: {}", config.seed),
                ),
                24.0,
                228.0,
                22.0,
                WHITE,
            );
        } else {
            clear_background(BLACK);
            draw_text("Waiting for game state...", 30.0, 50.0, 28.0, WHITE);
        }

        if initialized && !controls.is_captured() {
            draw_rectangle(
                0.0,
                screen_height() * 0.5 - 30.0,
                screen_width(),
                60.0,
                BLACK,
            );

            draw_text(
                "Click to play | ESC to connection screen",
                25.0,
                screen_height() * 0.5 + 8.0,
                28.0,
                WHITE,
            );
        }

        next_frame().await;
    }
}
