# Maze Wars

A Rust/Macroquad multiplayer first-person maze game with raycast walls, eye-shaped
opponents, a minimap, health, scores, deaths, and an FPS display. A standalone UDP
server owns movement, collision, combat, spawning, and AI. Clients render snapshots
and apply local facing immediately for responsive mouse controls.

## Build and run

Install Rust with Cargo (edition 2024; validated with Rust 1.92), a native linker,
and a working desktop graphics/OpenGL environment for the client. Linux needs the
X11/OpenGL runtime libraries used by Macroquad/Miniquad. The server and connection
probe need no graphical display. Dependencies are locked in `Cargo.lock`; there
are no downloaded artwork files, new dependencies, or additional GUI frameworks.

```sh
cargo build --release
# Default: fixed level 1, no bots, listen on all IPv4 interfaces, UDP 34254
cargo run --release --bin server

# Preserve existing level and bind arguments
cargo run --release --bin server -- --level 2
cargo run --release --bin server -- --level 3 --bind 0.0.0.0:34254

# Generate a 21x21 maze on the server
cargo run --release --bin server -- --generated --seed 12345

# Bots work on both kinds of maze
cargo run --release --bin server -- --level 1 --bots 3
cargo run --release --bin server -- --generated --seed 12345 --bots 3

# Open the graphical client in another terminal (or on another computer)
cargo run --release

# The original terminal-only connection diagnostic remains available
cargo run --release --bin connection_test
cargo run --release --bin server -- --help
```

`--level` accepts 1, 2, or 3. `--generated` and `--level` are mutually exclusive.
`--seed` requires `--generated`, accepts an unsigned 64-bit number, and defaults to
0. The same seed always produces the same maze. `--bots` accepts 0 through 6;
the total capacity remains 16, reserving at least 10 places for real clients.
Bots are named `BOT-1`, `BOT-2`, etc. in server combat logs, with unique player IDs
assigned before human IDs. They render as ordinary opponents.

## Connecting and history

The window opens directly onto the connection screen. Enter a server IP and port,
a username of 1–20 printable characters, and an optional local host alias of up
to 40 characters. The default address is `127.0.0.1:34254`. Use a numeric IPv4
address or bracketed IPv6, such as `[::1]:34254`; DNS hostnames are not accepted.
Click a field to select its text, use Tab/Shift+Tab between fields, Ctrl+A to
replace text, Backspace/Delete to erase, and Enter or **Connect** to join.

The bounded UDP handshake runs in a worker thread. The window continues to render
while connecting, and network failures return to an editable screen with **Retry
connection**. Leaving play returns to this same screen. The standalone server
continues to use command-line arguments.

Successful joins save the address, username, and alias in a versioned JSON file.
The 20 most recently used distinct addresses are retained. Select a saved row to
fill the fields, then Connect. **Remove** deletes that row; use the wheel or
Up/Down buttons to browse a longer list. Nothing sensitive such as passwords is
stored, and aliases are never sent to the server.

History locations:

- Linux/other Unix: `$XDG_CONFIG_HOME/maze-wars/hosts.json`, or
  `$HOME/.config/maze-wars/hosts.json` when XDG_CONFIG_HOME is unset/non-absolute.
- macOS: `$HOME/Library/Application Support/maze-wars/hosts.json`.
- Windows: `%APPDATA%/maze-wars/hosts.json`.

A missing file starts an empty history. Missing configuration paths, malformed or
oversized files, permission errors, and failed saves show a warning without
preventing play. Writes use a temporary file and rename; a failed save retains
history in memory for the session and preserves the previous on-disk file.
Concurrent instances do not merge histories; the last successful save wins.

## Controls and combat

| Action | Control |
| --- | --- |
| Capture mouse and start playing | Left click |
| Move / strafe | W/S / A/D |
| Turn | Mouse or left/right arrows |
| Fire | Left click while captured |
| Release mouse | Tab or Escape |
| Return to connection screen | Escape while mouse is released |
| Quit | Close the window |

Players start with two health points. Two successful hits defeat a player: the
shooter gains one point, the victim gains one death and immediately respawns at
a free open cell with full health. Spawn protection lasts two seconds; firing
ends the shooter's own protection. Shot cooldown is 350 ms. Walls block shots,
and the nearest opponent on the aiming line is the candidate target.

Each requested shot triggers one brief energy flash, expanding crosshair, and
fading pulse. These effects do not confirm hits or change damage. Health, scores,
and deaths come from the server. Clicking to capture the mouse does not fire.
Mouse delta remains unscaled by frame time; keyboard turning and effects use
elapsed time, while authoritative movement runs at a fixed 60 Hz.

## Architecture and modules

Clients send `Join`, sequenced `Input` (movement, yaw, monotonically increasing
shot request ID), and best-effort `Leave` messages. The server associates a human
with its UDP source address and ID, rejects stale input, bounds incoming work per
tick, stops stale movement after 250 ms, and removes silent clients after 10 s.
Clients time out after five seconds without a valid snapshot.

A successful join receives `Welcome`; a failed join receives `Rejected`.
`Snapshot` contains a tick plus compact tuples of ID, quantized position/yaw,
health, score, and deaths. Bot entries use exactly the same tuples as humans.
The server registry distinguishes human addresses from bot identities, preventing
fake bot socket addresses and sharing the existing movement/combat rules.

Generated `Welcome` messages add a version, dimensions, and seed. Generation uses
iterative randomized depth-first backtracking and project-owned SplitMix64 with
explicit wrapping `u64` arithmetic and a fixed neighbor order. Version 1 has a
pinned layout test. Clients reject unsupported versions or dimensions (odd sizes
7–51 are supported by the generator; the CLI chooses 21). Full mazes are never
included in snapshots. All three original fixed layouts are unchanged. Fixed
welcomes remain compatible with the old format; generated servers advertise level
0 so old clients reject them instead of playing on a different maze. Use matching
client/server builds for the bonus features.

Bots think at 5 Hz, choose BFS paths through open tile centers, roam when alone,
and remember a detected opponent's location for two seconds. Raycasts check
visibility through walls, with a ten-cell detection range. A 400 ms reaction delay
and limited turning rate prevent instant aiming. The ordinary server collision,
cooldown, health, scoring, respawn, and protection code handles their actions.

| File | Responsibility |
| --- | --- |
| `src/main.rs` | Window configuration and graphical application entry |
| `src/connection_screen.rs` | Connection form, worker result polling, retry and saved-host UI |
| `src/host_history.rs` | Validated, bounded local history and recoverable persistence |
| `src/game.rs` | Playing loop, maze reconstruction, effects and return to connection screen |
| `src/controls.rs` | Mouse capture, mouse/keyboard facing and shot requests |
| `src/effects.rs` | Time-based shot flash, pulse and crosshair animation |
| `src/client.rs` | UDP handshake, input transmission, snapshots and timeout |
| `src/server.rs` | 60 Hz authoritative simulation, player registry, spawning and combat resolution |
| `src/server_tests.rs` | Human/bot combat, cooldown and protection regression tests |
| `src/bin/server.rs` | Server CLI: fixed/generated maps, seed, bind and bots |
| `src/bin/connection_test.rs` | Original interactive terminal handshake probe |
| `src/bot.rs` | Bot decisions, BFS navigation, vision and reaction timing |
| `src/generator.rs` | Versioned deterministic maze carving and stable PRNG |
| `src/protocol.rs` | JSON wire types, username validation and packet size enforcement |
| `src/maze.rs` | Maze storage, dimensions and circle/wall collision |
| `src/levels.rs` | Original three fixed levels and topology tests |
| `src/player.rs` | Shared movement integration, turning and collision sliding |
| `src/combat.rs` | Ray/wall-constrained nearest-target selection |
| `src/raycast.rs` | Grid DDA wall raycasting |
| `src/render.rs` | First-person walls, minimap and FPS HUD |
| `src/opponents.rs` | Eye sprites with per-column wall occlusion |
| `src/hud.rs` | Health, score and deaths |
| `src/lib.rs` | Shared module exports |
| `tests/network.rs` | Real UDP child-server integration and CLI rejection tests |

## Packet sizes and performance

The existing maximum UDP payload remains **1,200 bytes**. Encoding refuses larger
messages and receive buffers detect oversized packets. With 16 players, maximum
IDs/scores/deaths/tick, and the largest supported map coordinates, the snapshot
is **918 bytes**. The largest generated welcome is **123 bytes**. Tests cover both.
Names and full maze layouts do not enlarge the per-tick snapshots.

World rendering stays capped at 800 raycast columns. Effects use a handful of
primitive draw calls with no assets or per-frame particle allocations. BFS visits
each cell at most once and runs only when a bot needs a route. A local graphical
smoke test observed 57–61 FPS with three bots; this is not a hardware-independent
performance guarantee. Verify sustained >50 FPS on the computers/resolutions used
for play, especially with the full 16-player population.

## LAN testing and troubleshooting

1. Start the server on one computer, normally with `--bind 0.0.0.0:34254`.
2. Find that computer's LAN IP (for example `192.168.1.42`). On each client, enter
   `192.168.1.42:34254`. Loopback `127.0.0.1` works only on the server computer.
3. Permit inbound **UDP** port 34254 through the server's firewall. TCP-only rules
   do not help. Use the chosen port if `--bind` changes it.
4. Join from two computers and check movement, wall collision, wall occlusion,
   two-hit defeats, score/death updates, and respawn protection.
5. Repeat on levels 1, 2, and 3, then on `--generated --seed 12345`, and add bots.
   Compare both clients' minimaps and the reported seed.

If joining fails, check the IP/port, server terminal output, UDP firewall rules,
VPN routing, and Wi-Fi client isolation. Binding to `127.0.0.1` prevents remote
connections. IPv6 needs an IPv6 bind such as `--bind '[::]:34254'`. For connections
across routers, UDP forwarding and a reachable server address are required.
The terminal `connection_test` separates handshake issues from rendering issues.
The protocol is intended for trusted LAN play and does not provide encryption or
Internet matchmaking.

## Validation

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
```

All 35 tests pass, including the original 18. Added tests cover deterministic/pinned maze
generation, enclosure, connectivity, dead ends, spawn collision, BFS and bot
exploration, blocked vision/shots, shared human/bot combat, cooldown/protection,
history failures, packet bounds, and 60/120/240 FPS timing. Integration tests start
and stop real child servers on ephemeral loopback ports, join all fixed/generated
maps, fill six bot plus ten human slots, reject overflow, and reconnect after
leaving. These tests need permission to bind local UDP sockets; restricted
sandboxes may require running them outside the sandbox.

The graphical smoke check covered launch, generated-map join against bots,
return to the form, reconnect, saved history, connection refusal, resizing, and
connection-screen maximize/minimize/restore with working input afterward.
Before release, manually verify on your target machines:

- Cross-computer LAN gameplay, including ten real clients if available.
- Sustained FPS at full population and different resolutions/refresh rates.
- Maximize/minimize/restore and mouse capture after resizing or focus changes.
- Shot appearance and mouse feel at 60, 120, and higher display refresh rates.
- Saved-host selection/removal across application restarts and readable warnings
  when the real configuration directory is unwritable.
