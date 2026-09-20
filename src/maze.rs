const LAYOUT: [&str; 15] = [
    "###############",
    "#.....#.......#",
    "#.###.#.#####.#",
    "#.#...#.....#.#",
    "#.#.#######.#.#",
    "#.#.........#.#",
    "#.#####.#####.#",
    "#.....#.#.....#",
    "#####.#.#.###.#",
    "#.....#...#...#",
    "#.#########.#.#",
    "#...........#.#",
    "#.###########.#",
    "#.............#",
    "###############",
];

pub struct Maze {
    tiles: Vec<Vec<bool>>,
}

impl Maze {
    pub fn new() -> Self {
        let tiles = LAYOUT
            .iter()
            .map(|row| row.bytes().map(|tile| tile == b'#').collect())
            .collect();

        Self { tiles }
    }

    pub fn width(&self) -> usize {
        self.tiles[0].len()
    }

    pub fn height(&self) -> usize {
        self.tiles.len()
    }

    pub fn is_wall(&self, x: i32, y: i32) -> bool {
        if x < 0 || y < 0 {
            return true;
        }

        self.tiles
            .get(y as usize)
            .and_then(|row| row.get(x as usize))
            .copied()
            .unwrap_or(true)
    }

    pub fn can_occupy(&self, x: f32, y: f32, radius: f32) -> bool {
        if !x.is_finite() || !y.is_finite() || !radius.is_finite() || radius <= 0.0 {
            return false;
        }

        if x - radius < 0.0
            || y - radius < 0.0
            || x + radius >= self.width() as f32
            || y + radius >= self.height() as f32
        {
            return false;
        }

        let min_x = (x - radius).floor() as i32;
        let max_x = (x + radius).floor() as i32;
        let min_y = (y - radius).floor() as i32;
        let max_y = (y + radius).floor() as i32;

        for tile_y in min_y..=max_y {
            for tile_x in min_x..=max_x {
                if !self.is_wall(tile_x, tile_y) {
                    continue;
                }

                let nearest_x = x.clamp(tile_x as f32, tile_x as f32 + 1.0);
                let nearest_y = y.clamp(tile_y as f32, tile_y as f32 + 1.0);

                let dx = x - nearest_x;
                let dy = y - nearest_y;

                if dx * dx + dy * dy <= radius * radius {
                    return false;
                }
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spawn_is_open() {
        assert!(Maze::new().can_occupy(1.5, 1.5, 0.2));
    }

    #[test]
    fn walls_and_outside_are_blocked() {
        let maze = Maze::new();

        assert!(maze.is_wall(0, 0));
        assert!(maze.is_wall(-1, 1));
        assert!(maze.is_wall(15, 1));
        assert!(!maze.can_occupy(0.5, 0.5, 0.2));
    }

    #[test]
    fn player_radius_cannot_overlap_wall() {
        let maze = Maze::new();

        assert!(!maze.can_occupy(1.1, 1.5, 0.2));
        assert!(maze.can_occupy(1.3, 1.5, 0.2));
    }
}
