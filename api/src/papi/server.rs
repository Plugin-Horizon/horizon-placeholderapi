//! The built-in `server` expansion: `%server_<name>%`, and `%server_time_<pattern>%` for the local
//! time in a Java `SimpleDateFormat`-style pattern (see `timefmt`).
//!
//! As with `player`, PlaceholderAPI's own copy is an eCloud expansion whose formats are not in its
//! repository; the formats here follow the names (README.md, "Built-in placeholders").

use super::expansion::Expansion;
use super::snapshot::Live;
use super::timefmt;
use super::{BUILTIN_AUTHOR, VERSION};

pub const IDENTIFIER: &str = "server";

/// The server's name as the placeholders report it.
pub const SERVER_NAME: &str = "Horizon";

const PLACEHOLDERS: &[&str] = &["name", "online", "max_players", "motd", "tps", "time_<pattern>"];

/// The current time: milliseconds since the epoch, and the offset east of UTC in seconds.
pub type Clock = Box<dyn Fn() -> (i64, i32) + Send + Sync>;

pub struct ServerExpansion {
    live: Live,
    clock: Clock,
}

impl ServerExpansion {
    pub fn new(live: Live, clock: Clock) -> ServerExpansion {
        ServerExpansion { live, clock }
    }
}

impl Expansion for ServerExpansion {
    fn identifier(&self) -> &str {
        IDENTIFIER
    }

    fn author(&self) -> &str {
        BUILTIN_AUTHOR
    }

    fn version(&self) -> &str {
        VERSION
    }

    fn placeholders(&self) -> Vec<String> {
        PLACEHOLDERS.iter().map(|p| format!("%server_{p}%")).collect()
    }

    fn on_request(&self, _player: Option<u128>, params: &str) -> Option<String> {
        if let Some(pattern) = params.strip_prefix("time_") {
            let (epoch_ms, offset) = (self.clock)();
            return timefmt::format(pattern, epoch_ms, offset);
        }
        let snap = self.live.lock().unwrap_or_else(|e| e.into_inner());
        Some(match params {
            "name" => SERVER_NAME.to_owned(),
            "online" => snap.online().to_string(),
            "max_players" => snap.max_players.to_string(),
            "motd" => snap.motd.clone(),
            "tps" => format!("{:.2}", snap.tps),
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::snapshot::{PlayerSnap, Snapshot};
    use std::sync::{Arc, Mutex};

    const NEW_YEAR: i64 = 1_704_067_200_000;

    fn expansion(snap: Snapshot) -> ServerExpansion {
        let live: Live = Arc::new(Mutex::new(snap));
        let clock: Clock = Box::new(|| (NEW_YEAR, 0));
        ServerExpansion::new(live, clock)
    }

    fn two_online() -> Snapshot {
        Snapshot {
            players: vec![PlayerSnap::new(1, "A"), PlayerSnap::new(2, "B")],
            tps: 19.5,
            max_players: 50,
            motd: "&aA Horizon server".into(),
        }
    }

    #[test]
    fn the_server_values() {
        let e = expansion(two_online());
        assert_eq!(e.on_request(None, "name").unwrap(), "Horizon");
        assert_eq!(e.on_request(None, "online").unwrap(), "2");
        assert_eq!(e.on_request(None, "max_players").unwrap(), "50");
        assert_eq!(e.on_request(None, "motd").unwrap(), "&aA Horizon server");
        assert_eq!(e.on_request(None, "tps").unwrap(), "19.50");
    }

    #[test]
    fn the_server_values_do_not_need_a_player() {
        let e = expansion(two_online());
        assert!(e.on_request(Some(1), "online").is_some());
        assert!(e.on_request(None, "online").is_some());
    }

    #[test]
    fn time_uses_the_clock() {
        let e = expansion(Snapshot::default());
        assert_eq!(e.on_request(None, "time_yyyy-MM-dd HH:mm").unwrap(), "2024-01-01 00:00");
        assert_eq!(e.on_request(None, "time_").unwrap(), "");
        assert_eq!(e.on_request(None, "time_x"), None);
    }

    #[test]
    fn time_without_its_underscore_is_unknown() {
        let e = expansion(Snapshot::default());
        assert_eq!(e.on_request(None, "time"), None);
    }

    #[test]
    fn an_unknown_server_parameter_is_left_alone() {
        let e = expansion(Snapshot::default());
        assert_eq!(e.on_request(None, "ram"), None);
        assert_eq!(e.on_request(None, ""), None);
    }

    #[test]
    fn the_placeholders_are_listed_for_info() {
        let e = expansion(Snapshot::default());
        let list = e.placeholders();
        assert!(list.contains(&"%server_time_<pattern>%".to_owned()));
        assert_eq!(list.len(), 6);
    }
}
