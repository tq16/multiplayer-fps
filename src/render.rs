use macroquad::prelude::*;

use crate::maze::Maze;
use crate::player::{PLAYER_RADIUS, Player};
use crate::raycast::cast_ray;

const FOV_DEGREES: f32 = 66.0;
const MAX_RAYS: usize = 800;

const CEILING: Color = Color::new(0.04, 0.05, 0.08, 1.0);
const FLOOR: Color = Color::new(0.09, 0.11, 0.15, 1.0);
const WALL: Color = Color::new(0.62, 0.69, 0.80, 1.0);
const ACCENT: Color = Color::new(0.2, 0.9, 0.7, 1.0);
const MAP_WALL: Color = Color::new(0.32, 0.39, 0.49, 1.0);

pub fn draw_game(maze: &Maze, player: &Player) {
    draw_world(maze, player);
    draw_minimap(maze, player);
    draw_hud();
}

fn draw_world(maze: &Maze, player: &Player) {
    let width = screen_width();
    let height = screen_height();

    clear_background(CEILING);
    draw_rectangle(0.0, height * 0.5, width, height * 0.5, FLOOR);

    if width <= 0.0 || height <= 0.0 {
        return;
    }

    let direction_x = player.angle.cos();
    let direction_y = player.angle.sin();

    let half_fov_tangent = (FOV_DEGREES.to_radians() * 0.5).tan();

    // The camera plane points to the player's right.
    let plane_x = -direction_y * half_fov_tangent;
    let plane_y = direction_x * half_fov_tangent;

    let projection_distance = width / (2.0 * half_fov_tangent);
    let ray_count = (width.ceil() as usize).clamp(1, MAX_RAYS);
    let column_width = width / ray_count as f32;

    for column in 0..ray_count {
        let camera_x = 2.0 * (column as f32 + 0.5) / ray_count as f32 - 1.0;

        let ray_x = direction_x + plane_x * camera_x;
        let ray_y = direction_y + plane_y * camera_x;

        let Some(hit) = cast_ray(maze, player.x, player.y, ray_x, ray_y) else {
            continue;
        };

        // Camera-plane rays give perpendicular distance,
        // keeping straight walls from appearing curved.
        let distance = hit.distance.max(0.001);
        let wall_height = projection_distance / distance;

        let top = ((height - wall_height) * 0.5).max(0.0);
        let bottom = ((height + wall_height) * 0.5).min(height);

        let side_shade = if hit.horizontal_side { 0.72 } else { 1.0 };
        let distance_shade = 1.0 / (1.0 + distance * 0.10);
        let brightness = side_shade * distance_shade;

        let color = Color::new(
            WALL.r * brightness,
            WALL.g * brightness,
            WALL.b * brightness,
            1.0,
        );

        // Round boundaries to avoid tiny gaps between columns.
        let left = (column as f32 * column_width).floor();
        let right = ((column + 1) as f32 * column_width).ceil();

        draw_rectangle(left, top, right - left, bottom - top, color);
    }
}

fn draw_minimap(maze: &Maze, player: &Player) {
    let map_size = (screen_width() * 0.23)
        .min(screen_height() * 0.32)
        .min(220.0);

    let tile_size = map_size / maze.width().max(maze.height()) as f32;
    let map_width = maze.width() as f32 * tile_size;
    let map_height = maze.height() as f32 * tile_size;

    let offset_x = screen_width() - map_width - 20.0;
    let offset_y = 20.0;

    draw_rectangle(
        offset_x - 5.0,
        offset_y - 5.0,
        map_width + 10.0,
        map_height + 10.0,
        BLACK,
    );

    for y in 0..maze.height() {
        for x in 0..maze.width() {
            let color = if maze.is_wall(x as i32, y as i32) {
                MAP_WALL
            } else {
                FLOOR
            };

            draw_rectangle(
                offset_x + x as f32 * tile_size,
                offset_y + y as f32 * tile_size,
                tile_size,
                tile_size,
                color,
            );
        }
    }

    let player_x = offset_x + player.x * tile_size;
    let player_y = offset_y + player.y * tile_size;

    draw_line(
        player_x,
        player_y,
        player_x + player.angle.cos() * tile_size * 0.8,
        player_y + player.angle.sin() * tile_size * 0.8,
        2.0,
        WHITE,
    );

    draw_circle(
        player_x,
        player_y,
        (PLAYER_RADIUS * tile_size).max(2.0),
        ACCENT,
    );
}

fn draw_hud() {
    draw_rectangle(14.0, 14.0, 185.0, 64.0, BLACK);
    draw_text("MAZE WARS", 24.0, 40.0, 28.0, WHITE);

    let fps = get_fps();
    let fps_color = if fps > 50 { ACCENT } else { ORANGE };

    draw_text(&format!("FPS: {fps}"), 24.0, 65.0, 22.0, fps_color);

    let center_x = screen_width() * 0.5;
    let center_y = screen_height() * 0.5;

    draw_line(
        center_x - 6.0,
        center_y,
        center_x + 6.0,
        center_y,
        2.0,
        WHITE,
    );

    draw_line(
        center_x,
        center_y - 6.0,
        center_x,
        center_y + 6.0,
        2.0,
        WHITE,
    );

    draw_rectangle(0.0, screen_height() - 38.0, screen_width(), 38.0, BLACK);

    draw_text(
        "W/S: forward/back   A/D: strafe   Arrows: turn   ESC: quit",
        20.0,
        screen_height() - 13.0,
        20.0,
        WHITE,
    );
}
