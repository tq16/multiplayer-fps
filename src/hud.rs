use macroquad::prelude::*;

pub fn draw_player_stats(health: u8, score: u32, deaths: u32) {
    let health_color = if health >= 2 { GREEN } else { ORANGE };

    draw_rectangle(14.0, 116.0, 250.0, 86.0, BLACK);

    draw_text(
        &format!("Health: {health}/2"),
        24.0,
        141.0,
        24.0,
        health_color,
    );

    draw_text(&format!("Score: {score}"), 24.0, 166.0, 22.0, WHITE);

    draw_text(&format!("Deaths: {deaths}"), 24.0, 191.0, 22.0, WHITE);
}
