use crate::maze::Maze;
use crate::player::Player;
use crate::raycast::cast_ray;

// Half the width of the eye drawn in opponents.rs.
const HIT_RADIUS: f32 = 0.325;

pub fn find_target<'a>(
    maze: &Maze,
    shooter: &Player,
    targets: impl IntoIterator<Item = (u32, &'a Player)>,
) -> Option<u32> {
    let direction_x = shooter.angle.cos();
    let direction_y = shooter.angle.sin();

    let wall = cast_ray(maze, shooter.x, shooter.y, direction_x, direction_y)?;

    let mut closest_distance = wall.distance;
    let mut closest_target = None;

    for (id, target) in targets {
        let dx = target.x - shooter.x;
        let dy = target.y - shooter.y;

        // How far ahead of the shooter is this target?
        let forward = dx * direction_x + dy * direction_y;

        if forward <= 0.0 {
            continue;
        }

        // How far away from the aiming line is this target?
        let sideways = dx * -direction_y + dy * direction_x;

        if sideways.abs() > HIT_RADIUS {
            continue;
        }

        // Only hit the nearest target before the wall.
        if forward < closest_distance {
            closest_distance = forward;
            closest_target = Some(id);
        }
    }

    closest_target
}

#[cfg(test)]
mod tests {
    use super::*;

    fn player(x: f32, y: f32) -> Player {
        Player { x, y, angle: 0.0 }
    }

    #[test]
    fn hits_player_in_front() {
        let maze = Maze::new();
        let shooter = player(1.5, 1.5);
        let target = player(3.5, 1.5);

        assert_eq!(find_target(&maze, &shooter, [(2, &target)]), Some(2),);
    }

    #[test]
    fn wall_blocks_shot() {
        let maze = Maze::new();
        let shooter = player(1.5, 1.5);
        let target = player(7.5, 1.5);

        assert_eq!(find_target(&maze, &shooter, [(2, &target)]), None,);
    }

    #[test]
    fn nearest_player_is_hit() {
        let maze = Maze::new();
        let shooter = player(1.5, 1.5);
        let near = player(2.5, 1.5);
        let far = player(4.5, 1.5);

        assert_eq!(
            find_target(&maze, &shooter, [(3, &far), (2, &near)]),
            Some(2),
        );
    }

    #[test]
    fn misses_player_outside_aim() {
        let maze = Maze::new();
        let shooter = player(1.5, 1.5);
        let target = player(1.5, 3.5);

        assert_eq!(find_target(&maze, &shooter, [(2, &target)]), None,);
    }

    #[test]
    fn misses_player_behind_shooter() {
        let maze = Maze::new();
        let shooter = player(3.5, 1.5);
        let target = player(1.5, 1.5);

        assert_eq!(find_target(&maze, &shooter, [(2, &target)]), None,);
    }
}
