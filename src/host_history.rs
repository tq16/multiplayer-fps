//! Small, bounded, versioned local history; all I/O failures are recoverable.
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use crate::protocol::valid_username;
use serde::{Deserialize, Serialize};

pub const MAX_HOSTS: usize = 20;
const MAX_FILE_BYTES: u64 = 64 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedHost {
    pub address: String,
    pub username: String,
    pub alias: String,
}

impl SavedHost {
    pub fn validate(address: &str, username: &str, alias: &str) -> Result<Self, String> {
        let address = address
            .trim()
            .parse::<SocketAddr>()
            .map_err(|_| "Enter an IP and port, e.g. 127.0.0.1:34254 (IPv6: [::1]:34254).")?;
        if address.port() == 0 || address.ip().is_unspecified() || address.ip().is_multicast() {
            return Err("Use a specific server IP and a port from 1 to 65535.".into());
        }
        let username = username.trim();
        if !valid_username(username) {
            return Err("Username must contain 1–20 printable characters.".into());
        }
        let alias = alias.trim();
        if alias.chars().count() > 40 || alias.chars().any(char::is_control) {
            return Err("Host alias must be at most 40 printable characters.".into());
        }
        Ok(Self {
            address: address.to_string(),
            username: username.into(),
            alias: alias.into(),
        })
    }
}

#[derive(Serialize, Deserialize)]
pub struct HostHistory {
    version: u8,
    pub hosts: Vec<SavedHost>,
}

impl Default for HostHistory {
    fn default() -> Self {
        Self {
            version: 1,
            hosts: Vec::new(),
        }
    }
}

impl HostHistory {
    pub fn remember(&mut self, host: SavedHost) {
        self.hosts.retain(|old| old.address != host.address);
        self.hosts.insert(0, host);
        self.hosts.truncate(MAX_HOSTS);
    }

    pub fn load(path: &Path) -> io::Result<Self> {
        let file = match File::open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(error) => return Err(error),
        };
        let mut bytes = Vec::new();
        file.take(MAX_FILE_BYTES + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_FILE_BYTES {
            return Err(invalid_data("History is too large."));
        }
        let history: Self =
            serde_json::from_slice(&bytes).map_err(|e| invalid_data(&e.to_string()))?;
        if history.version != 1 || history.hosts.len() > MAX_HOSTS {
            return Err(invalid_data(
                "Unsupported history format or too many hosts.",
            ));
        }
        let mut validated = Self::default();
        for host in history.hosts.into_iter().rev() {
            let host = SavedHost::validate(&host.address, &host.username, &host.alias)
                .map_err(|e| invalid_data(&e))?;
            validated.remember(host);
        }
        Ok(validated)
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let bytes = serde_json::to_vec_pretty(self).map_err(|e| invalid_data(&e.to_string()))?;
        // Atomic replacement preserves the previous history if a write fails.
        let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&temporary)?;
        let result = (|| {
            file.write_all(&bytes)?;
            file.sync_all()?;
            drop(file);
            fs::rename(&temporary, path)
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result
    }
}

fn invalid_data(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

pub fn config_path() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    let base = std::env::var_os("APPDATA").map(PathBuf::from);
    #[cfg(target_os = "macos")]
    let base = std::env::var_os("HOME")
        .map(|home| PathBuf::from(home).join("Library/Application Support"));
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")));
    base.filter(|p| p.is_absolute())
        .map(|p| p.join("maze-wars/hosts.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_and_deduplication() {
        assert!(SavedHost::validate("0.0.0.0:0", "player", "").is_err());
        assert!(SavedHost::validate("localhost", "player", "").is_err());
        assert!(SavedHost::validate("127.0.0.1:34254", "", "").is_err());
        let host = SavedHost::validate("127.0.0.1:34254", " Player ", " LAN ").unwrap();
        let mut history = HostHistory::default();
        history.remember(host.clone());
        history.remember(host.clone());
        assert_eq!(history.hosts, [host]);
    }

    #[test]
    fn history_round_trip_missing_corrupt_and_unwritable_paths() {
        let directory =
            std::env::temp_dir().join(format!("maze-wars-history-test-{}", std::process::id()));
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("hosts.json");
        assert!(
            HostHistory::load(&directory.join("missing.json"))
                .unwrap()
                .hosts
                .is_empty()
        );
        let mut history = HostHistory::default();
        history.remember(SavedHost::validate("[::1]:34254", "Player", "Home").unwrap());
        history.save(&path).unwrap();
        history.save(&path).unwrap();
        assert_eq!(HostHistory::load(&path).unwrap().hosts, history.hosts);
        assert!(history.save(&path.join("not-a-directory.json")).is_err());
        fs::write(&path, b"not json").unwrap();
        assert!(HostHistory::load(&path).is_err());
        fs::write(&path, vec![b' '; MAX_FILE_BYTES as usize + 1]).unwrap();
        assert!(HostHistory::load(&path).is_err());
        fs::remove_dir_all(directory).unwrap();
    }
}
