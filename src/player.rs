use crate::maze::Maze;
use std::f32::consts::TAU;

pub const MOVE_SPEED: f32 = 3.0;
const TURN_SPEED: f32 = 2.5;
const MAX_DELTA_TIME: f32 = 0.1;

pub const PLAYER_RADIUS: f32 = 0.2;

#[derive(Default)]
pub struct MovementInput {
    pub forward: f32,
    pub strafe: f32,
    pub turn: f32,
}

#[derive(Clone, Copy)]
pub struct Player {
    pub x: f32,
    pub y: f32,
    pub angle: f32,
}

impl Default for Player {
    fn default() -> Self {
        Self::new()
    }
}

impl Player {
    pub fn new() -> Self {
        Self {
            x: 1.5,
            y: 1.5,
            angle: 0.0,
        }
    }

    pub fn update(&mut self, input: &MovementInput, delta_time: f32, maze: &Maze) {
        if !delta_time.is_finite()
            || !input.forward.is_finite()
            || !input.strafe.is_finite()
            || !input.turn.is_finite()
        {
            return;
        }

        let dt = delta_time.clamp(0.0, MAX_DELTA_TIME);

        self.angle = (self.angle + input.turn.clamp(-1.0, 1.0) * TURN_SPEED * dt).rem_euclid(TAU);

        let mut forward = input.forward.clamp(-1.0, 1.0);
        let mut strafe = input.strafe.clamp(-1.0, 1.0);

        // Keep diagonal movement at the same speed.
        let length = (forward * forward + strafe * strafe).sqrt();

        if length > 1.0 {
            forward /= length;
            strafe /= length;
        }

        let dx = (self.angle.cos() * forward - self.angle.sin() * strafe) * MOVE_SPEED * dt;
        let dy = (self.angle.sin() * forward + self.angle.cos() * strafe) * MOVE_SPEED * dt;

        let distance = (dx * dx + dy * dy).sqrt();
        let steps = (distance / (PLAYER_RADIUS * 0.5)).ceil().max(1.0) as usize;

        let step_x = dx / steps as f32;
        let step_y = dy / steps as f32;

        for _ in 0..steps {
            // Separate axes let the player slide along walls.
            if maze.can_occupy(self.x + step_x, self.y, PLAYER_RADIUS) {
                self.x += step_x;
            }

            if maze.can_occupy(self.x, self.y + step_y, PLAYER_RADIUS) {
                self.y += step_y;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn movement_and_turning_match_at_60_120_and_240_fps() {
        let maze = Maze::new();
        for fps in [60, 120, 240] {
            let mut moving = Player::new();
            let mut turning = Player::new();
            for _ in 0..fps {
                moving.update(
                    &MovementInput {
                        forward: 1.0,
                        ..Default::default()
                    },
                    1.0 / fps as f32,
                    &maze,
                );
                turning.update(
                    &MovementInput {
                        turn: 1.0,
                        ..Default::default()
                    },
                    1.0 / fps as f32,
                    &maze,
                );
            }
            assert!((moving.x - 4.5).abs() < 0.0001);
            assert!((turning.angle - 2.5).abs() < 0.0001);
        }
    }

    #[test]
    fn player_cannot_walk_through_wall() {
        let maze = Maze::new();
        let mut player = Player::new();

        let input = MovementInput {
            forward: -1.0,
            ..Default::default()
        };

        for _ in 0..100 {
            player.update(&input, 1.0 / 60.0, &maze);
        }

        assert!(player.x >= 1.0 + PLAYER_RADIUS);
        assert!(maze.can_occupy(player.x, player.y, PLAYER_RADIUS));
    }

    #[test]
    fn diagonal_movement_is_not_faster() {
        let maze = Maze::new();
        let mut straight = Player::new();
        let mut diagonal = Player::new();

        straight.update(
            &MovementInput {
                forward: 1.0,
                ..Default::default()
            },
            0.05,
            &maze,
        );

        diagonal.update(
            &MovementInput {
                forward: 1.0,
                strafe: 1.0,
                ..Default::default()
            },
            0.05,
            &maze,
        );

        let straight_distance = ((straight.x - 1.5).powi(2) + (straight.y - 1.5).powi(2)).sqrt();
        let diagonal_distance = ((diagonal.x - 1.5).powi(2) + (diagonal.y - 1.5).powi(2)).sqrt();

        assert!((straight_distance - diagonal_distance).abs() < 0.0001);
    }
}
