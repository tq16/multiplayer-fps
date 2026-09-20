use crate::maze::Maze;
pub struct WallHit {
    pub distance: f32,
    pub horizontal_side: bool,
}

pub fn cast_ray(
    maze: &Maze,
    origin_x: f32,
    origin_y: f32,
    direction_x: f32,
    direction_y: f32,
) -> Option<WallHit> {
    if !origin_x.is_finite()
        || !origin_y.is_finite()
        || !direction_x.is_finite()
        || !direction_y.is_finite()
        || (direction_x == 0.0 && direction_y == 0.0)
    {
        return None;
    }

    let mut map_x = origin_x.floor() as i32;
    let mut map_y = origin_y.floor() as i32;

    if maze.is_wall(map_x, map_y) {
        return None;
    }

    let delta_x = if direction_x == 0.0 {
        f32::INFINITY
    } else {
        (1.0 / direction_x).abs()
    };

    let delta_y = if direction_y == 0.0 {
        f32::INFINITY
    } else {
        (1.0 / direction_y).abs()
    };

    let (step_x, mut side_x) = if direction_x < 0.0 {
        (-1, (origin_x - map_x as f32) * delta_x)
    } else {
        (1, (map_x as f32 + 1.0 - origin_x) * delta_x)
    };

    let (step_y, mut side_y) = if direction_y < 0.0 {
        (-1, (origin_y - map_y as f32) * delta_y)
    } else {
        (1, (map_y as f32 + 1.0 - origin_y) * delta_y)
    };

    // Each step crosses one tile boundary.
    let max_steps = maze.width() + maze.height() + 2;

    for _ in 0..max_steps {
        let distance;
        let horizontal_side;

        if side_x < side_y {
            distance = side_x;
            side_x += delta_x;
            map_x += step_x;
            horizontal_side = false;
        } else {
            distance = side_y;
            side_y += delta_y;
            map_y += step_y;
            horizontal_side = true;
        }

        if maze.is_wall(map_x, map_y) {
            return Some(WallHit {
                distance,
                horizontal_side,
            });
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ray_finds_wall_to_the_east() {
        let hit = cast_ray(&Maze::new(), 1.5, 1.5, 1.0, 0.0).unwrap();

        assert!((hit.distance - 4.5).abs() < 0.0001);
        assert!(!hit.horizontal_side);
    }

    #[test]
    fn ray_finds_wall_to_the_north() {
        let hit = cast_ray(&Maze::new(), 1.5, 1.5, 0.0, -1.0).unwrap();

        assert!((hit.distance - 0.5).abs() < 0.0001);
        assert!(hit.horizontal_side);
    }

    #[test]
    fn zero_direction_is_rejected() {
        assert!(cast_ray(&Maze::new(), 1.5, 1.5, 0.0, 0.0).is_none());
    }
}
