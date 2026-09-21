//! Real loopback UDP tests, with child servers killed even if an assertion fails.
use multiplayer_fps::client::NetworkClient;
use multiplayer_fps::maze::Maze;
use multiplayer_fps::player::{MovementInput, PLAYER_RADIUS};
use std::io::{BufRead, BufReader};
use std::net::SocketAddr;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

struct Server(Child);
impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn server(args: &[&str]) -> (Server, SocketAddr) {
    let child = Command::new(env!("CARGO_BIN_EXE_server"))
        .args(["--bind", "127.0.0.1:0"])
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    let mut server = Server(child);
    let mut line = String::new();
    let mut output = BufReader::new(server.0.stdout.take().unwrap());
    output.read_line(&mut line).unwrap();
    thread::spawn(move || {
        for line in output.lines() {
            if line.is_err() {
                break;
            }
        }
    });
    let address = line
        .trim()
        .strip_prefix("Server listening on ")
        .unwrap()
        .parse()
        .unwrap();
    // Keep stdout open: server combat/join logging must not hit a broken pipe.
    (server, address)
}

fn pump(clients: &mut [NetworkClient], duration: Duration) {
    let until = Instant::now() + duration;
    while Instant::now() < until {
        for client in &mut *clients {
            client.update(&MovementInput::default(), 0.0, 0).unwrap();
        }
        thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn fixed_and_generated_clients_share_server_state() {
    for args in [
        vec!["--level", "1"],
        vec!["--level", "2"],
        vec!["--level", "3"],
        vec!["--generated", "--seed", "12345"],
    ] {
        let (_server, address) = server(&args);
        let mut clients = [
            NetworkClient::connect(address, "Alice".into()).unwrap(),
            NetworkClient::connect(address, "Bob".into()).unwrap(),
        ];
        pump(&mut clients, Duration::from_millis(100));
        assert_ne!(clients[0].player_id, clients[1].player_id);
        assert_eq!(clients[0].generated, clients[1].generated);
        assert_eq!(clients[0].level, clients[1].level);
        let maze = clients[0]
            .generated
            .map_or_else(|| Maze::from_level(clients[0].level), |g| g.build())
            .unwrap();
        for client in &clients {
            assert_eq!(client.players.len(), 2);
            for (_, p) in &client.players {
                assert!(maze.can_occupy(p.x, p.y, PLAYER_RADIUS));
            }
        }
        if args.contains(&"--generated") {
            assert_eq!(clients[0].generated.unwrap().seed, 12345);
        }
    }
}

#[test]
fn six_bots_leave_ten_human_slots_and_move_in_normal_snapshots() {
    let (_server, address) = server(&["--generated", "--seed", "12345", "--bots", "6"]);
    let mut clients: Vec<_> = (0..10)
        .map(|i| NetworkClient::connect(address, format!("Human-{i}")).unwrap())
        .collect();
    pump(&mut clients, Duration::from_millis(100));
    let before = clients[0].players.clone();
    assert_eq!(before.len(), 16);
    assert!(NetworkClient::connect(address, "Overflow".into()).is_err());
    pump(&mut clients, Duration::from_secs(1));
    let maze = clients[0].generated.unwrap().build().unwrap();
    let mut moving_bots = 0;
    for &(id, p) in &clients[0].players {
        assert!(maze.can_occupy(p.x, p.y, PLAYER_RADIUS - 0.002));
        if id <= 6 {
            let old = before.iter().find(|(old_id, _)| *old_id == id).unwrap().1;
            moving_bots += usize::from((p.x - old.x).hypot(p.y - old.y) > 0.1);
        }
    }
    assert!(moving_bots > 0);
    let leaving = clients.pop().unwrap();
    let leaving_id = leaving.player_id;
    drop(leaving);
    pump(&mut clients, Duration::from_millis(100));
    assert!(!clients[0].players.iter().any(|(id, _)| *id == leaving_id));
    assert!(NetworkClient::connect(address, "Replacement".into()).is_ok());
}

#[test]
fn server_rejects_invalid_cli_combinations() {
    for args in [
        vec!["--bots", "7"],
        vec!["--level", "4"],
        vec!["--seed", "1"],
        vec!["--generated", "--level", "2"],
    ] {
        assert!(
            !Command::new(env!("CARGO_BIN_EXE_server"))
                .args(args)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .unwrap()
                .success()
        );
    }
}
