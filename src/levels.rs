pub const EASY: [&str; 15] = [
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

pub const MEDIUM: [&str; 15] = [
    "###############",
    "#.#.#...#.....#",
    "#.#.#.#.#.#.#.#",
    "#.#.......#...#",
    "#.#.#########.#",
    "#.#.#...#.....#",
    "#.###.#.#.#####",
    "#.....#.#.....#",
    "#######.#.#.#.#",
    "#.....#.#...#.#",
    "#.###.#.#####.#",
    "#...#.#.......#",
    "#.#.#####.#.#.#",
    "#.#.........#.#",
    "###############",
];

pub const HARD: [&str; 15] = [
    "###############",
    "#.#.#.........#",
    "#.#.#.###.###.#",
    "#.#...#.#...#.#",
    "#.#.###.###.###",
    "#.#.......#...#",
    "#.###.#######.#",
    "#.#...#.....#.#",
    "#.#.###.###.#.#",
    "#.#.#...#...#.#",
    "#.###.###.###.#",
    "#...#.#.#.#...#",
    "###.#.#.#.#.#.#",
    "#.....#.....#.#",
    "###############",
];

pub fn layout(level: u8) -> Option<&'static [&'static str; 15]> {
    match level {
        1 => Some(&EASY),
        2 => Some(&MEDIUM),
        3 => Some(&HARD),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn neighbors(x: usize, y: usize) -> [(usize, usize); 4] {
        [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)]
    }

    fn is_open(map: &[&str; 15], x: usize, y: usize) -> bool {
        map[y].as_bytes()[x] == b'.'
    }

    fn dead_ends(map: &[&str; 15]) -> usize {
        let mut count = 0;

        for y in 1..14 {
            for x in 1..14 {
                if !is_open(map, x, y) {
                    continue;
                }

                let exits = neighbors(x, y)
                    .iter()
                    .filter(|&&(nx, ny)| is_open(map, nx, ny))
                    .count();

                if exits == 1 {
                    count += 1;
                }
            }
        }

        count
    }

    #[test]
    fn all_levels_are_enclosed_and_connected() {
        for level in 1..=3 {
            let map = layout(level).unwrap();

            for row in map {
                assert_eq!(row.len(), 15);
                assert!(row.bytes().all(|tile| tile == b'#' || tile == b'.'));
                assert_eq!(row.as_bytes()[0], b'#');
                assert_eq!(row.as_bytes()[14], b'#');
            }

            assert!(map[0].bytes().all(|tile| tile == b'#'));
            assert!(map[14].bytes().all(|tile| tile == b'#'));
            assert!(is_open(map, 1, 1));

            let mut visited = HashSet::new();
            let mut pending = vec![(1, 1)];
            visited.insert((1, 1));

            while let Some((x, y)) = pending.pop() {
                for (nx, ny) in neighbors(x, y) {
                    if is_open(map, nx, ny) && visited.insert((nx, ny)) {
                        pending.push((nx, ny));
                    }
                }
            }

            let open_tiles = map
                .iter()
                .flat_map(|row| row.bytes())
                .filter(|&tile| tile == b'.')
                .count();

            assert_eq!(visited.len(), open_tiles, "Disconnected level {level}");
        }
    }

    #[test]
    fn difficulty_increases() {
        assert!(dead_ends(&EASY) < dead_ends(&MEDIUM));
        assert!(dead_ends(&MEDIUM) < dead_ends(&HARD));
    }

    #[test]
    fn invalid_levels_are_rejected() {
        assert!(layout(0).is_none());
        assert!(layout(4).is_none());
    }
}
