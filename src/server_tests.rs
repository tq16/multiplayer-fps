use super::*;

fn participant(id: u32, bot: bool, x: f32) -> (Participant, ConnectedPlayer) {
    let key = if bot {
        Participant::Bot(id)
    } else {
        Participant::Human(SocketAddr::from(([127, 0, 0, 1], 40000 + id as u16)))
    };
    (
        key,
        ConnectedPlayer {
            bot: bot.then(|| Bot::new(id, &Maze::new())),
            id,
            username: format!("Test-{id}"),
            player: Player { x, ..Player::new() },
            input: MovementInput::default(),
            sequence: 0,
            last_seen: Instant::now(),
            last_shot_id: 0,
            health: MAX_HEALTH,
            score: 0,
            deaths: 0,
            last_shot_at: None,
            protected_until: Instant::now(),
        },
    )
}

#[test]
fn humans_and_bots_share_cooldown_two_hit_respawn_and_protection() {
    for bot_shooter in [false, true] {
        let maze = Maze::new();
        let (shooter_key, shooter) = participant(1, bot_shooter, 1.5);
        let (target_key, mut target) = participant(2, !bot_shooter, 3.5);
        target.protected_until = Instant::now() + RESPAWN_PROTECTION;
        let mut players = HashMap::from([(shooter_key, shooter), (target_key, target)]);
        resolve_shot(1, &maze, &mut players);
        assert_eq!(players[&target_key].health, 2);
        players.get_mut(&target_key).unwrap().protected_until = Instant::now();
        players.get_mut(&shooter_key).unwrap().last_shot_at = None;
        resolve_shot(1, &maze, &mut players);
        assert_eq!(players[&target_key].health, 1);
        resolve_shot(1, &maze, &mut players);
        assert_eq!(
            players[&target_key].health, 1,
            "Cooldown must reject repeated shots"
        );
        players.get_mut(&shooter_key).unwrap().last_shot_at = Some(Instant::now() - SHOT_COOLDOWN);
        resolve_shot(1, &maze, &mut players);
        assert_eq!(players[&target_key].health, 2);
        assert_eq!(players[&target_key].deaths, 1);
        assert_eq!(players[&shooter_key].score, 1);
        assert!(players[&target_key].protected_until > Instant::now());
        let p = &players[&target_key].player;
        assert!(maze.can_occupy(p.x, p.y, PLAYER_RADIUS));
    }
}

#[test]
fn server_shots_do_not_cross_walls() {
    let (shooter_key, shooter) = participant(1, true, 1.5);
    let (target_key, target) = participant(2, false, 7.5);
    let mut players = HashMap::from([(shooter_key, shooter), (target_key, target)]);
    resolve_shot(1, &Maze::new(), &mut players);
    assert_eq!(players[&target_key].health, 2);
    assert_eq!(players[&shooter_key].score, 0);
}
