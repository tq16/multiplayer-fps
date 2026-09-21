use macroquad::prelude::*;

const PULSE_DURATION: f32 = 0.22;

/// Presentation only: a request is not confirmation that a shot hit anything.
#[derive(Default)]
pub struct ShotEffects {
    age: Option<f32>,
}

impl ShotEffects {
    pub fn trigger(&mut self) {
        self.age = Some(0.0);
    }

    pub fn update(&mut self, dt: f32) {
        if let Some(age) = &mut self.age {
            *age += dt.max(0.0);
            if *age >= PULSE_DURATION {
                self.age = None;
            }
        }
    }

    pub fn draw(&self) {
        let x = screen_width() * 0.5;
        let y = screen_height() * 0.5;
        let strength = self.age.map_or(0.0, |age| 1.0 - age / PULSE_DURATION);
        let gap = 4.0 + 12.0 * strength * strength;
        let color = Color::new(0.8 + 0.2 * strength, 1.0, 1.0, 0.9);
        for (dx, dy) in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
            draw_line(
                x + dx * gap,
                y + dy * gap,
                x + dx * (gap + 7.0),
                y + dy * (gap + 7.0),
                2.0,
                color,
            );
        }
        if let Some(age) = self.age {
            draw_circle_lines(
                x,
                y,
                12.0 + age * 150.0,
                1.5,
                Color::new(0.3, 0.95, 1.0, strength * 0.6),
            );
            let flash = (1.0 - age / 0.065).max(0.0);
            if flash > 0.0 {
                draw_poly(
                    x,
                    y,
                    8,
                    18.0 * flash,
                    22.5,
                    Color::new(0.4, 0.95, 1.0, flash * 0.65),
                );
                draw_circle(x, y, 5.0 * flash, Color::new(1.0, 1.0, 0.9, flash));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effect_expires_by_elapsed_time_at_different_frame_rates() {
        for fps in [60, 120, 240] {
            let mut effects = ShotEffects::default();
            effects.trigger();
            for _ in 0..fps / 10 {
                effects.update(1.0 / fps as f32);
            }
            assert!((effects.age.unwrap() - 0.1).abs() < 0.0001);
            for _ in 0..fps {
                effects.update(1.0 / fps as f32);
            }
            assert!(effects.age.is_none());
        }
    }
}
