use macroquad::prelude::*;
use multiplayer_fps::client::NetworkClient;
use multiplayer_fps::host_history::{self, HostHistory, SavedHost};
use std::io;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, TryRecvError};

const INK: Color = Color::new(0.04, 0.055, 0.08, 1.0);
const PANEL: Color = Color::new(0.085, 0.11, 0.15, 1.0);
const ACCENT: Color = Color::new(0.2, 0.9, 0.7, 1.0);
const MUTED: Color = Color::new(0.57, 0.66, 0.75, 1.0);

#[derive(Default)]
struct TextField {
    value: String,
    selected: bool,
}

impl TextField {
    fn edit(&mut self, limit: usize) {
        let control = is_key_down(KeyCode::LeftControl)
            || is_key_down(KeyCode::RightControl)
            || is_key_down(KeyCode::LeftSuper)
            || is_key_down(KeyCode::RightSuper);
        if control && is_key_pressed(KeyCode::A) {
            self.selected = true;
        }
        if is_key_pressed(KeyCode::Backspace) || is_key_pressed(KeyCode::Delete) {
            if self.selected {
                self.value.clear();
            } else {
                self.value.pop();
            }
            self.selected = false;
        }
        while let Some(character) = get_char_pressed() {
            if character.is_control() || control {
                continue;
            }
            if self.selected {
                self.value.clear();
                self.selected = false;
            }
            if self.value.chars().count() < limit {
                self.value.push(character);
            }
        }
    }
}

enum Status {
    Ready,
    Connecting {
        receiver: Receiver<io::Result<NetworkClient>>,
        host: SavedHost,
    },
    Error(String),
}

struct ConnectionScreen {
    fields: [TextField; 3],
    focus: usize,
    status: Status,
    history: HostHistory,
    path: Option<PathBuf>,
    warning: Option<String>,
    scroll: usize,
}

impl ConnectionScreen {
    fn new() -> Self {
        let path = host_history::config_path();
        let (history, warning) = match &path {
            Some(path) => match HostHistory::load(path) {
                Ok(history) => (history, None),
                Err(error) => (
                    HostHistory::default(),
                    Some(format!("Could not load history: {error}")),
                ),
            },
            None => (
                HostHistory::default(),
                Some("No configuration directory; history is session-only.".into()),
            ),
        };
        Self {
            fields: [
                TextField {
                    value: "127.0.0.1:34254".into(),
                    selected: true,
                },
                TextField {
                    value: "Player".into(),
                    selected: false,
                },
                TextField::default(),
            ],
            focus: 0,
            status: Status::Ready,
            history,
            path,
            warning,
            scroll: 0,
        }
    }

    fn save(&mut self) {
        if let Some(path) = &self.path {
            self.warning = self.history.save(path).err().map(|error| {
                format!("History could not be saved (kept for this session): {error}")
            });
        }
    }

    fn connect(&mut self) {
        let host = match SavedHost::validate(
            &self.fields[0].value,
            &self.fields[1].value,
            &self.fields[2].value,
        ) {
            Ok(host) => host,
            Err(error) => {
                self.status = Status::Error(error);
                return;
            }
        };
        let address = match host.address.parse() {
            Ok(address) => address,
            Err(error) => {
                self.status = Status::Error(format!("Invalid server address: {error}"));
                return;
            }
        };
        let username = host.username.clone();
        let (sender, receiver) = mpsc::channel();
        // The worker owns the socket during the bounded handshake. On success,
        // ownership moves to the render thread through the channel exactly once.
        match std::thread::Builder::new()
            .name("udp-connect".into())
            .spawn(move || {
                let _ = sender.send(NetworkClient::connect(address, username));
            }) {
            Ok(_) => self.status = Status::Connecting { receiver, host },
            Err(error) => {
                self.status = Status::Error(format!("Could not start connection: {error}"))
            }
        }
    }

    fn poll(&mut self) -> Option<NetworkClient> {
        let Status::Connecting { receiver, host } = &self.status else {
            return None;
        };
        match receiver.try_recv() {
            Ok(Ok(client)) => {
                self.history.remember(host.clone());
                self.save();
                self.status = Status::Ready;
                Some(client)
            }
            Ok(Err(error)) => {
                self.status = Status::Error(error.to_string());
                None
            }
            Err(TryRecvError::Disconnected) => {
                self.status = Status::Error("Connection worker stopped. Please retry.".into());
                None
            }
            Err(TryRecvError::Empty) => None,
        }
    }

    fn draw(&mut self) {
        // Fixed logical canvas, fitted to both dimensions, including tiny windows.
        let width = screen_width().max(1.0);
        let height = screen_height().max(1.0);
        let scale = (width / 800.0).min(height / 760.0);
        let logical_width = width / scale;
        let logical_height = height / scale;
        let offset = vec2(
            (logical_width - 800.0) * 0.5,
            (logical_height - 760.0) * 0.5,
        );
        let mut camera = Camera2D::from_display_rect(Rect::new(
            -offset.x,
            -offset.y,
            logical_width,
            logical_height,
        ));
        // Camera2D applies a screen Y inversion; positive zoom keeps GUI Y downward.
        camera.zoom.y = camera.zoom.y.abs();
        set_camera(&camera);
        let mouse = vec2(mouse_position().0 / scale, mouse_position().1 / scale) - offset;
        let busy = matches!(self.status, Status::Connecting { .. });
        let clicked = is_mouse_button_pressed(MouseButton::Left) && !busy;
        clear_background(INK);
        draw_rectangle(36.0, 30.0, 728.0, 700.0, PANEL);
        draw_rectangle(36.0, 30.0, 5.0, 700.0, ACCENT);
        draw_text("MAZE WARS", 66.0, 80.0, 38.0, WHITE);
        draw_text(
            "Multiplayer arena / Connect to a server",
            67.0,
            110.0,
            21.0,
            MUTED,
        );
        let labels = ["Server IP and port", "Username", "Host alias (optional)"];
        if !busy && is_key_pressed(KeyCode::Tab) {
            self.fields[self.focus].selected = false;
            let backwards = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);
            self.focus = (self.focus + if backwards { 2 } else { 1 }) % 3;
            self.fields[self.focus].selected = true;
        }
        for (index, label) in labels.iter().enumerate() {
            let y = 145.0 + index as f32 * 78.0;
            draw_text(*label, 67.0, y, 20.0, MUTED);
            let rect = Rect::new(66.0, y + 9.0, 668.0, 40.0);
            if clicked && rect.contains(mouse) {
                self.fields[self.focus].selected = false;
                self.focus = index;
                self.fields[index].selected = true;
            }
            draw_rectangle(rect.x, rect.y, rect.w, rect.h, INK);
            draw_rectangle_lines(
                rect.x,
                rect.y,
                rect.w,
                rect.h,
                1.5,
                if index == self.focus && !busy {
                    ACCENT
                } else {
                    MUTED
                },
            );
            let field = &self.fields[index];
            let text = if index == self.focus && !busy && get_time().fract() < 0.5 {
                format!("{}|", field.value)
            } else {
                field.value.clone()
            };
            if index == self.focus && field.selected && !busy {
                draw_rectangle(
                    rect.x + 6.0,
                    rect.y + 5.0,
                    rect.w - 12.0,
                    rect.h - 10.0,
                    Color::new(0.1, 0.3, 0.3, 1.0),
                );
            }
            fitted_text(
                &text,
                rect.x + 10.0,
                rect.y + 27.0,
                22.0,
                rect.w - 20.0,
                WHITE,
            );
        }
        if !busy {
            self.fields[self.focus].edit([64, 20, 40][self.focus]);
        } else {
            while get_char_pressed().is_some() {}
        }
        if button(
            Rect::new(66.0, 366.0, 668.0, 43.0),
            if busy {
                "Connecting..."
            } else if matches!(self.status, Status::Error(_)) {
                "Retry connection"
            } else {
                "Connect"
            },
            mouse,
            clicked,
            !busy,
        ) || (!busy && is_key_pressed(KeyCode::Enter))
        {
            self.connect();
        }
        let (message, color) = match &self.status {
            Status::Ready => (
                "Tab: next field  /  Ctrl+A: replace text  /  Enter: connect".to_string(),
                MUTED,
            ),
            Status::Connecting { .. } => (
                format!(
                    "Waiting for UDP reply{} (up to 5 seconds)",
                    ".".repeat((get_time() * 3.0) as usize % 4)
                ),
                ACCENT,
            ),
            Status::Error(error) => (format!("{error} Edit the fields and retry."), ORANGE),
        };
        fitted_text(&message, 66.0, 436.0, 19.0, 668.0, color);
        draw_text("SAVED HOSTS", 66.0, 478.0, 22.0, WHITE);
        draw_text("Select a host, then Connect", 410.0, 478.0, 18.0, MUTED);
        let max_scroll = self.history.hosts.len().saturating_sub(3);
        if !busy && mouse.y > 480.0 {
            let wheel = mouse_wheel().1;
            if wheel < 0.0 {
                self.scroll = (self.scroll + 1).min(max_scroll);
            }
            if wheel > 0.0 {
                self.scroll = self.scroll.saturating_sub(1);
            }
        }
        self.scroll = self.scroll.min(max_scroll);
        if self.history.hosts.is_empty() {
            draw_text(
                "Successful connections appear here.",
                66.0,
                518.0,
                20.0,
                MUTED,
            );
        }
        let mut remove = None;
        for (row, index) in (self.scroll..self.history.hosts.len()).take(3).enumerate() {
            let host = &self.history.hosts[index];
            let rect = Rect::new(66.0, 492.0 + row as f32 * 48.0, 556.0, 40.0);
            let label = if host.alias.is_empty() {
                format!("{}  /  {}", host.address, host.username)
            } else {
                format!("{}  /  {}  /  {}", host.alias, host.address, host.username)
            };
            if button(rect, &label, mouse, clicked, !busy) {
                self.fields[0].value = host.address.clone();
                self.fields[1].value = host.username.clone();
                self.fields[2].value = host.alias.clone();
                self.status = Status::Ready;
            }
            if button(
                Rect::new(632.0, rect.y, 102.0, 40.0),
                "Remove",
                mouse,
                clicked,
                !busy,
            ) {
                remove = Some(index);
            }
        }
        if let Some(index) = remove {
            self.history.hosts.remove(index);
            self.save();
        }
        if max_scroll > 0 {
            if button(
                Rect::new(566.0, 642.0, 78.0, 28.0),
                "Up",
                mouse,
                clicked,
                !busy,
            ) {
                self.scroll = self.scroll.saturating_sub(1);
            }
            if button(
                Rect::new(654.0, 642.0, 80.0, 28.0),
                "Down",
                mouse,
                clicked,
                !busy,
            ) {
                self.scroll = (self.scroll + 1).min(max_scroll);
            }
        }
        if let Some(warning) = &self.warning {
            fitted_text(warning, 66.0, 696.0, 18.0, 668.0, ORANGE);
        } else {
            draw_text(
                "UDP  /  Two hits to defeat  /  Server-authoritative combat",
                66.0,
                696.0,
                18.0,
                MUTED,
            );
        }
        set_default_camera();
    }
}

fn fitted_text(text: &str, x: f32, y: f32, size: f32, width: f32, color: Color) {
    let mut shown = text.to_string();
    if measure_text(&shown, None, size as u16, 1.0).width > width {
        while !shown.is_empty()
            && measure_text(format!("{shown}..."), None, size as u16, 1.0).width > width
        {
            shown.pop();
        }
        shown.push_str("...");
    }
    draw_text(shown, x, y, size, color);
}

fn button(rect: Rect, label: &str, mouse: Vec2, clicked: bool, enabled: bool) -> bool {
    let hover = rect.contains(mouse) && enabled;
    draw_rectangle(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        if hover {
            Color::new(0.13, 0.3, 0.29, 1.0)
        } else {
            INK
        },
    );
    draw_rectangle_lines(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        1.0,
        if enabled { ACCENT } else { MUTED },
    );
    fitted_text(
        label,
        rect.x + 12.0,
        rect.y + rect.h * 0.5 + 7.0,
        20.0,
        rect.w - 24.0,
        if enabled { WHITE } else { MUTED },
    );
    clicked && hover
}

pub async fn run() {
    let mut screen = ConnectionScreen::new();
    loop {
        if let Some(client) = screen.poll() {
            set_default_camera();
            let error = crate::game::run(client).await;
            while get_char_pressed().is_some() {}
            screen.status = error.map_or(Status::Ready, Status::Error);
        }
        screen.draw();
        next_frame().await;
    }
}
