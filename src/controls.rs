use std::f32::consts::TAU;

use macroquad::prelude::*;
use multiplayer_fps::player::MovementInput;

const MOUSE_SENSITIVITY: f32 = 0.002;
const KEYBOARD_TURN_SPEED: f32 = 2.5;

#[derive(Default)]
pub struct Controls {
    captured: bool,
    angle: f32,
    previous_width: f32,
    shot_id: u64,
}

impl Controls {
    pub fn is_captured(&self) -> bool {
        self.captured
    }

    pub fn angle(&self) -> f32 {
        self.angle
    }

    pub fn set_angle(&mut self, angle: f32) {
        self.angle = angle.rem_euclid(TAU);
    }

    pub fn release(&mut self) {
        self.captured = false;
        set_cursor_grab(false);
        show_mouse(true);
    }

    pub fn read(&mut self) -> MovementInput {
        let width = screen_width();
        let resized = (width - self.previous_width).abs() > 0.5;
        self.previous_width = width;

        if is_key_pressed(KeyCode::Tab) || is_key_pressed(KeyCode::Escape) {
            self.release();
            return MovementInput::default();
        }

        if !self.captured {
            if is_mouse_button_pressed(MouseButton::Left) {
                self.captured = true;
                set_cursor_grab(true);
                show_mouse(false);
            }

            // Ignore movement on the frame when capturing the cursor.
            return MovementInput::default();
        }

        if is_mouse_button_pressed(MouseButton::Left) {
            self.shot_id = self.shot_id.saturating_add(1);
        }

        let mouse_pixels = if resized {
            0.0
        } else {
            -mouse_delta_position().x * width * 0.5
        };

        let dt = get_frame_time().clamp(0.0, 0.1);
        let keyboard_turn = key_axis(KeyCode::Right, KeyCode::Left);

        self.angle = (self.angle
            + mouse_pixels * MOUSE_SENSITIVITY
            + keyboard_turn * KEYBOARD_TURN_SPEED * dt)
            .rem_euclid(TAU);

        MovementInput {
            forward: key_axis(KeyCode::W, KeyCode::S),
            strafe: key_axis(KeyCode::D, KeyCode::A),
            // Both turning methods are now included in the facing angle.
            turn: 0.0,
        }
    }

    pub fn shot_id(&self) -> u64 {
        self.shot_id
    }
}

impl Drop for Controls {
    fn drop(&mut self) {
        self.release();
    }
}

fn key_axis(positive: KeyCode, negative: KeyCode) -> f32 {
    let positive = if is_key_down(positive) { 1.0 } else { 0.0 };
    let negative = if is_key_down(negative) { 1.0 } else { 0.0 };

    positive - negative
}
