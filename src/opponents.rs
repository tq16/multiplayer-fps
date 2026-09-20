use macroquad::prelude::*;

use multiplayer_fps::player::Player;

const EYE_WIDTH: f32 = 0.65;
const EYE_HEIGHT: f32 = 0.45;
const NEAR_DISTANCE: f32 = 0.05;

pub fn draw(
    camera: &Player,
    players: &[(u32, Player)],
    local_id: u32,
    wall_distances: &[f32],
    fov_degrees: f32,
) {
    if wall_distances.is_empty() {
        return;
    }

    let width = screen_width();
    let height = screen_height();

    if width <= 0.0 || height <= 0.0 {
        return;
    }

    let forward_x = camera.angle.cos();
    let forward_y = camera.angle.sin();
    let right_x = -forward_y;
    let right_y = forward_x;

    let projection = width / (2.0 * (fov_degrees.to_radians() * 0.5).tan());
    let column_width = width / wall_distances.len() as f32;

    let mut visible = Vec::with_capacity(players.len());

    for (id, player) in players {
        if *id == local_id {
            continue;
        }

        let dx = player.x - camera.x;
        let dy = player.y - camera.y;

        let depth = dx * forward_x + dy * forward_y;
        let sideways = dx * right_x + dy * right_y;

        if depth > NEAR_DISTANCE {
            visible.push((depth, sideways));
        }
    }

    // Draw farther players first so nearer ones cover them.
    visible.sort_by(|a, b| b.0.total_cmp(&a.0));

    for (depth, sideways) in visible {
        let center_x = width * 0.5 + sideways * projection / depth;
        let center_y = height * 0.5;

        let half_width = EYE_WIDTH * projection / depth * 0.5;
        let half_height = EYE_HEIGHT * projection / depth * 0.5;

        let left = center_x - half_width;
        let right = center_x + half_width;

        if right <= 0.0 || left >= width {
            continue;
        }

        let start = (left / column_width).floor().max(0.0) as usize;

        let end = (right / column_width)
            .ceil()
            .clamp(0.0, wall_distances.len() as f32) as usize;

        let brightness = (1.0 / (1.0 + depth * 0.08)).max(0.25);

        let outline = Color::new(0.08 * brightness, 0.10 * brightness, 0.13 * brightness, 1.0);

        let eye_white = Color::new(0.95 * brightness, 0.97 * brightness, brightness, 1.0);

        let iris = Color::new(0.95 * brightness, 0.50 * brightness, 0.12 * brightness, 1.0);

        for (column, wall_depth) in wall_distances.iter().enumerate().take(end).skip(start) {
            // A nearer wall hides this part of the eye.
            if depth >= *wall_depth {
                continue;
            }

            let sample_x = (column as f32 + 0.5) * column_width;
            let offset = sample_x - center_x;
            let normalized_x = offset / half_width;

            if normalized_x.abs() >= 1.0 {
                continue;
            }

            let x = (column as f32 * column_width).floor();
            let next_x = ((column + 1) as f32 * column_width).ceil();
            let stripe_width = next_x - x;

            let outer_height = half_height * (1.0 - normalized_x * normalized_x).sqrt();

            draw_stripe(
                x,
                stripe_width,
                center_y - outer_height,
                center_y + outer_height,
                outline,
            );

            let inner_width = half_width * 0.88;
            let inner_x = offset / inner_width;

            if inner_x.abs() < 1.0 {
                let inner_height = half_height * 0.80 * (1.0 - inner_x * inner_x).sqrt();

                draw_stripe(
                    x,
                    stripe_width,
                    center_y - inner_height,
                    center_y + inner_height,
                    eye_white,
                );
            }

            let iris_radius = half_height * 0.68;

            if offset.abs() < iris_radius {
                let iris_height = (iris_radius * iris_radius - offset * offset).sqrt();

                draw_stripe(
                    x,
                    stripe_width,
                    center_y - iris_height,
                    center_y + iris_height,
                    iris,
                );
            }

            let pupil_radius = iris_radius * 0.46;

            if offset.abs() < pupil_radius {
                let pupil_height = (pupil_radius * pupil_radius - offset * offset).sqrt();

                draw_stripe(
                    x,
                    stripe_width,
                    center_y - pupil_height,
                    center_y + pupil_height,
                    BLACK,
                );
            }
        }
    }
}

fn draw_stripe(x: f32, width: f32, top: f32, bottom: f32, color: Color) {
    let top = top.max(0.0);
    let bottom = bottom.min(screen_height());

    if bottom > top {
        draw_rectangle(x, top, width, bottom - top, color);
    }
}
