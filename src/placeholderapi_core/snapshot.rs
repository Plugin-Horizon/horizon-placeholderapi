//! What the built-in expansions read: the players online and the server's numbers, copied from the
//! engine every tick by the glue. The expansions never call the engine themselves, so they work
//! from any thread and in the tests.

use std::sync::{Arc, Mutex};

/// The state shared by the glue (writer) and the built-in expansions (readers).
pub type Live = Arc<Mutex<Snapshot>>;

/// Ticks a second the server keeps up, when it ticks normally.
pub const FULL_TPS: f64 = 20.0;

/// The player list's default size when server.properties does not say (Horizon's default too).
pub const DEFAULT_MAX_PLAYERS: i64 = 20;

/// One player online, as the built-in expansions see them.
#[derive(Clone, Debug, PartialEq)]
pub struct PlayerSnap {
    pub uuid: u128,
    pub name: String,
    /// The name a mod shows for the player, if one set it.
    pub nickname: Option<String>,
    /// `minecraft:overworld`, `minecraft:the_nether`, `minecraft:the_end`, or another dimension id.
    pub dimension: String,
    pub position: [f64; 3],
    /// 0 survival, 1 creative, 2 adventure, 3 spectator.
    pub game_mode: i32,
    pub health: f32,
    /// Round-trip time in milliseconds.
    pub latency: i32,
    pub xp_level: i32,
}

impl PlayerSnap {
    /// A player with the defaults of a fresh survival player, for tests and for the glue to fill in.
    pub fn new(uuid: u128, name: &str) -> PlayerSnap {
        PlayerSnap {
            uuid,
            name: name.to_owned(),
            nickname: None,
            dimension: "minecraft:overworld".to_owned(),
            position: [0.0, 64.0, 0.0],
            game_mode: 0,
            health: 20.0,
            latency: 0,
            xp_level: 0,
        }
    }
}

/// The server as the built-in expansions see it.
#[derive(Clone, Debug, PartialEq)]
pub struct Snapshot {
    pub players: Vec<PlayerSnap>,
    /// Ticks a second, as `tps_of` computes it.
    pub tps: f64,
    pub max_players: i64,
    /// The motd from server.properties, colour codes included.
    pub motd: String,
}

impl Default for Snapshot {
    fn default() -> Snapshot {
        Snapshot {
            players: Vec::new(),
            tps: FULL_TPS,
            max_players: DEFAULT_MAX_PLAYERS,
            motd: String::new(),
        }
    }
}

/// The ticks a second the server keeps up: the tick rate, or 1000 / milliseconds per tick when
/// that is lower (Horizon's `/tick query` reports the two numbers the same way).
pub fn tps_of(rate: f32, mspt: f64) -> f64 {
    let rate = f64::from(rate);
    if mspt > 0.0 {
        rate.min(1000.0 / mspt)
    } else {
        rate
    }
}

impl Snapshot {
    /// Takes this tick's players and tick numbers.
    pub fn update(&mut self, players: Vec<PlayerSnap>, rate: f32, mspt: f64) {
        self.players = players;
        self.tps = tps_of(rate, mspt);
    }

    /// Reads `max-players` and `motd` from the text of server.properties. Keys it does not know,
    /// comments and lines without `=` are ignored; a bad `max-players` keeps the default.
    pub fn set_properties(&mut self, text: &str) {
        self.max_players = DEFAULT_MAX_PLAYERS;
        self.motd = String::new();
        for line in text.lines() {
            let line = line.trim_start();
            if line.starts_with('#') || line.starts_with('!') {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let value = value.trim_start();
            match key.trim() {
                "max-players" => {
                    if let Ok(n) = value.trim().parse::<i64>() {
                        self.max_players = n;
                    }
                }
                "motd" => self.motd = value.to_owned(),
                _ => {}
            }
        }
    }

    pub fn player(&self, uuid: u128) -> Option<&PlayerSnap> {
        self.players.iter().find(|p| p.uuid == uuid)
    }

    /// The online player with this name, in any case (`/papi parse` takes names as typed).
    pub fn find_by_name(&self, name: &str) -> Option<&PlayerSnap> {
        self.players.iter().find(|p| p.name.eq_ignore_ascii_case(name))
    }

    pub fn online(&self) -> usize {
        self.players.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tps_is_the_rate_unless_the_ticks_are_too_slow() {
        assert_eq!(tps_of(20.0, 0.0), 20.0, "no tick measured yet");
        assert_eq!(tps_of(20.0, 25.0), 20.0, "40 ticks a second are possible, 20 are asked");
        assert!((tps_of(20.0, 60.0) - 16.666).abs() < 0.01, "60 ms a tick keeps up with about 16.7");
    }

    #[test]
    fn update_takes_the_players_and_the_tps() {
        let mut snap = Snapshot::default();
        snap.update(vec![PlayerSnap::new(1, "A")], 20.0, 60.0);
        assert_eq!(snap.online(), 1);
        assert!((snap.tps - 16.666).abs() < 0.01);
        snap.update(Vec::new(), 20.0, 10.0);
        assert_eq!(snap.online(), 0);
        assert_eq!(snap.tps, 20.0);
    }

    #[test]
    fn players_are_found_by_uuid_and_by_name_in_any_case() {
        let snap = Snapshot {
            players: vec![PlayerSnap::new(7, "MixedCase")],
            ..Snapshot::default()
        };
        assert_eq!(snap.player(7).unwrap().name, "MixedCase");
        assert!(snap.player(8).is_none());
        assert_eq!(snap.find_by_name("mixedcase").unwrap().uuid, 7);
        assert!(snap.find_by_name("other").is_none());
    }

    #[test]
    fn server_properties_give_the_player_list_size_and_the_motd() {
        let mut snap = Snapshot::default();
        snap.set_properties(
            "#Minecraft server properties\nmax-players=50\nmotd=A Horizon &aserver\nlevel-name=world\n",
        );
        assert_eq!(snap.max_players, 50);
        assert_eq!(snap.motd, "A Horizon &aserver");
    }

    #[test]
    fn a_missing_or_bad_value_keeps_the_default() {
        let mut snap = Snapshot::default();
        snap.set_properties("max-players=lots\n");
        assert_eq!(snap.max_players, DEFAULT_MAX_PLAYERS);
        assert_eq!(snap.motd, "");
        snap.set_properties("");
        assert_eq!(snap.max_players, DEFAULT_MAX_PLAYERS);
    }

    #[test]
    fn a_reread_forgets_the_old_values() {
        let mut snap = Snapshot::default();
        snap.set_properties("max-players=5\nmotd=old\n");
        snap.set_properties("max-players=9\n");
        assert_eq!(snap.max_players, 9);
        assert_eq!(snap.motd, "");
    }
}
