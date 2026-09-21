//! Version 1 maze generation is part of the wire protocol. Keep its PRNG,
//! neighbor order, and carving order stable across platforms and releases.
use crate::maze::Maze;
use serde::{Deserialize, Serialize};

pub const MAX_DIMENSION: u16 = 51;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedMaze {
    pub version: u8,
    pub width: u16,
    pub height: u16,
    pub seed: u64,
}

impl GeneratedMaze {
    pub fn new(seed: u64) -> Self {
        Self {
            version: 1,
            width: 21,
            height: 21,
            seed,
        }
    }

    pub fn build(self) -> Option<Maze> {
        if self.version != 1
            || [self.width, self.height]
                .iter()
                .any(|&n| !(7..=MAX_DIMENSION).contains(&n) || n % 2 == 0)
        {
            return None;
        }
        let width = usize::from(self.width);
        let height = usize::from(self.height);
        let mut tiles = vec![vec![true; width]; height];
        let mut random = StableRng(self.seed);
        let mut stack = vec![(1_usize, 1_usize)];
        tiles[1][1] = false;
        while let Some(&(x, y)) = stack.last() {
            let mut choices = [(0, 0); 4];
            let mut count = 0;
            for (dx, dy) in [(0, -2), (2, 0), (0, 2), (-2, 0)] {
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                if nx > 0
                    && ny > 0
                    && nx < width as i32 - 1
                    && ny < height as i32 - 1
                    && tiles[ny as usize][nx as usize]
                {
                    choices[count] = (nx as usize, ny as usize);
                    count += 1;
                }
            }
            if count == 0 {
                stack.pop();
            } else {
                let (nx, ny) = choices[random.index(count)];
                tiles[(y + ny) / 2][(x + nx) / 2] = false;
                tiles[ny][nx] = false;
                stack.push((nx, ny));
            }
        }
        Some(Maze::from_generated_tiles(tiles))
    }
}

/// SplitMix64 using only explicitly wrapping u64 operations.
pub(crate) struct StableRng(pub u64);
impl StableRng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }

    pub(crate) fn index(&mut self, length: usize) -> usize {
        (self.next() % length as u64) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::PLAYER_RADIUS;
    use std::collections::HashSet;

    fn signature(maze: &Maze) -> Vec<bool> {
        (0..maze.height())
            .flat_map(|y| (0..maze.width()).map(move |x| maze.is_wall(x as i32, y as i32)))
            .collect()
    }

    #[test]
    fn version_one_layout_is_pinned() {
        let maze = GeneratedMaze {
            width: 7,
            height: 7,
            ..GeneratedMaze::new(12345)
        }
        .build()
        .unwrap();
        let rows: Vec<String> = (0..7)
            .map(|y| {
                (0..7)
                    .map(|x| if maze.is_wall(x, y) { '#' } else { '.' })
                    .collect()
            })
            .collect();
        assert_eq!(
            rows,
            [
                "#######", "#...#.#", "###.#.#", "#.#...#", "#.###.#", "#.....#", "#######"
            ]
        );
    }

    #[test]
    fn deterministic_and_seed_dependent() {
        let first = GeneratedMaze::new(12345).build().unwrap();
        assert_eq!(
            signature(&first),
            signature(&GeneratedMaze::new(12345).build().unwrap())
        );
        assert_ne!(
            signature(&first),
            signature(&GeneratedMaze::new(12346).build().unwrap())
        );
        // Pin the PRNG to prevent accidental platform or algorithm drift.
        let mut rng = StableRng(0);
        assert_eq!(rng.next(), 0xe220a8397b1dcdaf);
        assert_eq!(rng.next(), 0x6e789e6aa1b965f4);
    }

    #[test]
    fn enclosed_connected_dead_ends_and_valid_spawns() {
        for size in [7, 21, MAX_DIMENSION] {
            for seed in [0, 1, 12345, u64::MAX] {
                let maze = GeneratedMaze {
                    width: size,
                    height: size,
                    ..GeneratedMaze::new(seed)
                }
                .build()
                .unwrap();
                let mut visited = HashSet::from([(1, 1)]);
                let mut pending = vec![(1, 1)];
                let mut dead_ends = 0;
                while let Some((x, y)) = pending.pop() {
                    let mut exits = 0;
                    assert!(maze.can_occupy(x as f32 + 0.5, y as f32 + 0.5, PLAYER_RADIUS));
                    for next in [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)] {
                        if !maze.is_wall(next.0, next.1) {
                            exits += 1;
                            if visited.insert(next) {
                                pending.push(next);
                            }
                        }
                    }
                    dead_ends += usize::from(exits == 1);
                }
                assert!(dead_ends >= 2);
                assert_eq!(
                    visited.len(),
                    signature(&maze).iter().filter(|&&wall| !wall).count()
                );
                assert!(visited.len() >= 17);
                for n in 0..i32::from(size) {
                    assert!(maze.is_wall(n, 0) && maze.is_wall(0, n));
                    assert!(
                        maze.is_wall(n, i32::from(size) - 1)
                            && maze.is_wall(i32::from(size) - 1, n)
                    );
                }
                assert!(!maze.can_occupy(0.5, 0.5, PLAYER_RADIUS));
            }
        }
    }

    #[test]
    fn rejects_unsupported_versions_and_dimensions() {
        for size in [0, 6, 8, 52, u16::MAX] {
            assert!(
                GeneratedMaze {
                    width: size,
                    ..GeneratedMaze::new(0)
                }
                .build()
                .is_none()
            );
        }
        assert!(
            GeneratedMaze {
                version: 2,
                ..GeneratedMaze::new(0)
            }
            .build()
            .is_none()
        );
    }
}
