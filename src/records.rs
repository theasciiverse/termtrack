//! Your own best times, kept on this computer in the state folder:
//! `$XDG_STATE_HOME/termtrack` (`~/.local/state/termtrack` without it,
//! `%LOCALAPPDATA%\termtrack` on Windows), file `times`.
//!
//! One line per finished run, `<level id>\t<seconds>\t<unix time>`. The level
//! id is the level's name plus a CRC32 of its file, as upstream's online
//! leaderboard used, so an edited level starts a fresh list.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn state_dir() -> Option<PathBuf> {
    let base = match std::env::var_os("XDG_STATE_HOME").filter(|dir| !dir.is_empty()) {
        Some(dir) => PathBuf::from(dir),
        None if cfg!(windows) => PathBuf::from(std::env::var_os("LOCALAPPDATA")?),
        None => PathBuf::from(std::env::var_os("HOME")?).join(".local").join("state"),
    };
    let dir = base.join("termtrack");
    fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

pub fn level_id(level_name: &str, level_map: &[u8]) -> String {
    level_name.to_string() + &crc32fast::hash(level_map).to_string()
}

/// This level's times, fastest first.
pub fn times(level_id: &str) -> Vec<f64> {
    let Some(path) = state_dir().map(|d| d.join("times")) else { return vec![] };
    let text = fs::read_to_string(path).unwrap_or_default();
    let mut times: Vec<f64> = text
        .lines()
        .filter_map(|line| {
            let mut parts = line.split('\t');
            if parts.next()? != level_id {
                return None;
            }
            parts.next()?.parse::<f64>().ok()
        })
        .filter(|t| t.is_finite() && *t > 0.)
        .collect();
    times.sort_by(|a, b| a.total_cmp(b));
    times
}

/// Keeps a finished run.
pub fn add_time(level_id: &str, time: f64) {
    let Some(path) = state_dir().map(|d| d.join("times")) else { return };
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    match OpenOptions::new().create(true).append(true).open(&path) {
        Ok(mut file) => {
            let _ = writeln!(file, "{level_id}\t{time:.3}\t{now}");
            log::info!("time {time:.3} s kept for {level_id}");
        }
        Err(err) => log::warn!("could not keep the time in {}: {err}", path.display()),
    }
}
