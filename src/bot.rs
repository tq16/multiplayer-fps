//! Server-only decisions. Movement and damage still use the shared game rules.
use std::collections::VecDeque;
use std::f32::consts::{PI, TAU};

use crate::generator::StableRng;
use crate::maze::Maze;
use crate::player::{MOVE_SPEED, MovementInput, Player};
use crate::raycast::cast_ray;

type Cell = (i32, i32);
const THINK_INTERVAL: f32 = 0.2;
const REACTION_TIME: f32 = 0.4;
const MEMORY_TIME: f32 = 2.0;
const TURN_RATE: f32 = 2.2;

fn cell(player: &Player) -> Cell {
    (player.x.floor() as i32, player.y.floor() as i32)
}

pub fn visible(maze: &Maze, from: &Player, to: &Player) -> bool {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let distance = dx.hypot(dy);
    distance > 0.01
        && distance <= 10.0
        && cast_ray(maze, from.x, from.y, dx / distance, dy / distance)
            .is_some_and(|wall| wall.distance > distance)
}

/// Includes both endpoints; empty means no path. At most one visit per tile.
pub fn path(maze: &Maze, start: Cell, goal: Cell) -> VecDeque<Cell> {
    if maze.is_wall(start.0, start.1) || maze.is_wall(goal.0, goal.1) {
        return VecDeque::new();
    }
    let index = |(x, y): Cell| y as usize * maze.width() + x as usize;
    let mut previous = vec![None; maze.width() * maze.height()];
    previous[index(start)] = Some(start);
    let mut queue = VecDeque::from([start]);
    while let Some(current) = queue.pop_front() {
        if current == goal {
            let mut result = VecDeque::from([goal]);
            let mut node = goal;
            while node != start {
                let Some(parent) = previous[index(node)] else {
                    return VecDeque::new();
                };
                result.push_front(parent);
                node = parent;
            }
            return result;
        }
        let (x, y) = current;
        for next in [(x + 1, y), (x, y + 1), (x - 1, y), (x, y - 1)] {
            if !maze.is_wall(next.0, next.1) && previous[index(next)].is_none() {
                previous[index(next)] = Some(current);
                queue.push_back(next);
            }
        }
    }
    VecDeque::new()
}

pub struct Bot {
    random: StableRng,
    open_cells: Vec<Cell>,
    route: VecDeque<Cell>,
    goal: Option<Cell>,
    memory_left: f32,
    think_in: f32,
    target_id: Option<u32>,
    reaction: f32,
}

impl Bot {
    pub fn new(id: u32, maze: &Maze) -> Self {
        let open_cells = (0..maze.height() as i32)
            .flat_map(|y| {
                (0..maze.width() as i32)
                    .filter_map(move |x| (!maze.is_wall(x, y)).then_some((x, y)))
            })
            .collect();
        Self {
            random: StableRng(u64::from(id)),
            open_cells,
            route: VecDeque::new(),
            goal: None,
            memory_left: 0.0,
            think_in: 0.0,
            target_id: None,
            reaction: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.route.clear();
        self.goal = None;
        self.target_id = None;
        self.reaction = 0.0;
        self.memory_left = 0.0;
        self.think_in = 0.0;
    }

    pub fn update(
        &mut self,
        id: u32,
        player: &mut Player,
        opponents: &[(u32, Player)],
        maze: &Maze,
        dt: f32,
    ) -> (MovementInput, bool) {
        self.think_in -= dt;
        self.memory_left = (self.memory_left - dt).max(0.0);
        if self.think_in <= 0.0 {
            self.think_in = THINK_INTERVAL;
            let nearest = opponents
                .iter()
                .filter(|(other_id, other)| *other_id != id && visible(maze, player, other))
                .min_by(|a, b| {
                    let distance = |p: &Player| (p.x - player.x).hypot(p.y - player.y);
                    distance(&a.1)
                        .total_cmp(&distance(&b.1))
                        .then(a.0.cmp(&b.0))
                });
            let next_id = nearest.map(|(id, _)| *id);
            if next_id != self.target_id {
                self.reaction = 0.0;
            }
            self.target_id = next_id;
            if let Some((_, target)) = nearest {
                let destination = cell(target);
                if self.goal != Some(destination) {
                    self.route.clear();
                }
                self.goal = Some(destination);
                self.memory_left = MEMORY_TIME;
            } else if self.memory_left == 0.0
                && self.route.is_empty()
                && !self.open_cells.is_empty()
            {
                self.goal = Some(self.open_cells[self.random.index(self.open_cells.len())]);
            }
            if self.route.is_empty()
                && let Some(goal) = self.goal
            {
                self.route = path(maze, cell(player), goal);
            }
        }

        // Recheck line of sight every tick before aiming or requesting a shot.
        let target = self
            .target_id
            .and_then(|id| opponents.iter().find(|(other_id, _)| *other_id == id))
            .map(|(_, p)| p)
            .filter(|p| visible(maze, player, p));
        if target.is_some() {
            self.reaction += dt;
        } else {
            self.reaction = 0.0;
        }
        while let Some(&(x, y)) = self.route.front() {
            if (x as f32 + 0.5 - player.x).hypot(y as f32 + 0.5 - player.y) > 0.06 {
                break;
            }
            self.route.pop_front();
        }
        let waypoint = self
            .route
            .front()
            .map(|&(x, y)| (x as f32 + 0.5, y as f32 + 0.5));
        let aim = target.map(|p| (p.x, p.y)).or(waypoint);
        let mut aim_error = PI;
        if let Some((x, y)) = aim {
            let desired = (y - player.y).atan2(x - player.x);
            let difference = (desired - player.angle + PI).rem_euclid(TAU) - PI;
            player.angle =
                (player.angle + difference.clamp(-TURN_RATE * dt, TURN_RATE * dt)).rem_euclid(TAU);
            aim_error = (desired - player.angle + PI).rem_euclid(TAU) - PI;
        }
        let shoot = target.is_some() && self.reaction >= REACTION_TIME && aim_error.abs() < 0.08;
        let mut input = MovementInput::default();
        if let Some((x, y)) = waypoint {
            let dx = x - player.x;
            let dy = y - player.y;
            let distance = dx.hypot(dy);
            let speed = (distance / (MOVE_SPEED * dt).max(0.001)).min(0.75);
            if distance > 0.001 {
                input.forward =
                    (dx * player.angle.cos() + dy * player.angle.sin()) / distance * speed;
                input.strafe =
                    (-dx * player.angle.sin() + dy * player.angle.cos()) / distance * speed;
            }
        }
        (input, shoot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{generator::GeneratedMaze, player::PLAYER_RADIUS};

    #[test]
    fn bfs_uses_open_adjacent_cells_and_rejects_walls() {
        let maze = Maze::new();
        let route = path(&maze, (1, 1), (13, 13));
        assert_eq!(route.front(), Some(&(1, 1)));
        assert_eq!(route.back(), Some(&(13, 13)));
        for pair in route.iter().zip(route.iter().skip(1)) {
            assert_eq!((pair.0.0 - pair.1.0).abs() + (pair.0.1 - pair.1.1).abs(), 1);
            assert!(!maze.is_wall(pair.1.0, pair.1.1));
        }
        assert!(path(&maze, (0, 0), (1, 1)).is_empty());
    }

    #[test]
    fn walls_block_vision_and_reaction_delays_shots() {
        let maze = Maze::new();
        let mut player = Player::new();
        let clear = Player {
            x: 4.5,
            ..Player::new()
        };
        let hidden = Player {
            x: 7.5,
            ..Player::new()
        };
        assert!(visible(&maze, &player, &clear));
        assert!(!visible(&maze, &player, &hidden));
        let mut bot = Bot::new(1, &maze);
        assert!(
            !bot.update(1, &mut player, &[(2, clear)], &maze, 1.0 / 60.0)
                .1
        );
        for _ in 0..60 {
            assert!(
                !bot.update(1, &mut player, &[(2, hidden)], &maze, 1.0 / 60.0)
                    .1
            );
        }
    }

    #[test]
    fn bots_roam_without_crossing_walls_on_all_maps() {
        let maps = (1..=3)
            .map(|n| Maze::from_level(n).unwrap())
            .chain([GeneratedMaze::new(12345).build().unwrap()]);
        for maze in maps {
            let mut player = Player::new();
            let mut bot = Bot::new(1, &maze);
            let mut visited = std::collections::HashSet::new();
            for _ in 0..3600 {
                let (input, _) = bot.update(1, &mut player, &[], &maze, 1.0 / 60.0);
                player.update(&input, 1.0 / 60.0, &maze);
                assert!(maze.can_occupy(player.x, player.y, PLAYER_RADIUS));
                visited.insert(cell(&player));
            }
            assert!(
                visited.len() > 10,
                "Bot failed to explore: {} cells",
                visited.len()
            );
        }
    }
}
