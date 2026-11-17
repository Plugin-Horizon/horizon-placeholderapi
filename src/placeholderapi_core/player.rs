//! Written for this port after the names of PlaceholderAPI's eCloud `player` expansion (no code copied).
//! The built-in `player` expansion: `%player_<name>%` for the player in the context.
//!
//! PlaceholderAPI does not ship these; its eCloud expansions of the same names do, and their exact
//! formats are not in the PlaceholderAPI repository. The formats here are our reading of the
//! names (see README.md, "Built-in placeholders"): coordinates are block coordinates, the health
//! is printed as a Java-style decimal, the game mode in Bukkit's upper-case names.

use super::expansion::Expansion;
use super::snapshot::{Live, PlayerSnap};
use super::{BUILTIN_AUTHOR, VERSION};

pub const IDENTIFIER: &str = "player";

/// The parameters this expansion answers, as `/papi info` lists them.
const PLACEHOLDERS: &[&str] = &[
    "name",
    "displayname",
    "uuid",
    "x",
    "y",
    "z",
    "world",
    "health",
    "ping",
    "gamemode",
    "level",
];

pub struct PlayerExpansion {
    live: Live,
}

impl PlayerExpansion {
    pub fn new(live: Live) -> PlayerExpansion {
        PlayerExpansion { live }
    }
}

impl Expansion for PlayerExpansion {
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
        PLACEHOLDERS.iter().map(|p| format!("%player_{p}%")).collect()
    }

    fn on_request(&self, player: Option<u128>, params: &str) -> Option<String> {
        let uuid = player?;
        let snap = self.live.lock().unwrap_or_else(|e| e.into_inner());
        let found = snap.player(uuid)?;
        value(found, params)
    }
}

/// The value of one `player_` parameter for a player; `None` for an unknown parameter.
pub fn value(p: &PlayerSnap, params: &str) -> Option<String> {
    Some(match params {
        "name" => p.name.clone(),
        "displayname" => p.nickname.clone().unwrap_or_else(|| p.name.clone()),
        "uuid" => uuid_text(p.uuid),
        "x" => block(p.position[0]).to_string(),
        "y" => block(p.position[1]).to_string(),
        "z" => block(p.position[2]).to_string(),
        "world" => match world_of_dimension(&p.dimension) {
            Some(world) => world.to_owned(),
            None => p.dimension.clone(),
        },
        "health" => format!("{:?}", p.health),
        "ping" => p.latency.to_string(),
        "gamemode" => game_mode_name(p.game_mode)?.to_owned(),
        "level" => p.xp_level.to_string(),
        _ => return None,
    })
}

/// The block a coordinate lies in (Bukkit's `Location.getBlockX` rounds down).
fn block(coordinate: f64) -> i64 {
    coordinate.floor() as i64
}

/// `UUID.toString()`: lower case, 8-4-4-4-12.
pub fn uuid_text(uuid: u128) -> String {
    let hex = format!("{uuid:032x}");
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

/// The world folder name for one of the three default dimensions, as Horizon names them
/// (`world`, `world_nether`, `world_the_end`); `None` for any other dimension.
pub fn world_of_dimension(dimension: &str) -> Option<&'static str> {
    match dimension {
        "minecraft:overworld" => Some("world"),
        "minecraft:the_nether" => Some("world_nether"),
        "minecraft:the_end" => Some("world_the_end"),
        _ => None,
    }
}

/// Bukkit's `GameMode` names.
pub fn game_mode_name(mode: i32) -> Option<&'static str> {
    match mode {
        0 => Some("SURVIVAL"),
        1 => Some("CREATIVE"),
        2 => Some("ADVENTURE"),
        3 => Some("SPECTATOR"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::snapshot::Snapshot;
    use std::sync::{Arc, Mutex};

    const ID: u128 = 0x0123_4567_89ab_cdef_0123_4567_89ab_cdef;

    fn steve() -> PlayerSnap {
        PlayerSnap {
            uuid: ID,
            name: "Steve".into(),
            nickname: None,
            dimension: "minecraft:overworld".into(),
            position: [100.7, 64.0, -0.5],
            game_mode: 1,
            health: 19.5,
            latency: 42,
            xp_level: 7,
        }
    }

    fn live_with(p: PlayerSnap) -> Live {
        let mut snap = Snapshot::default();
        snap.players.push(p);
        Arc::new(Mutex::new(snap))
    }

    #[test]
    fn the_uuid_is_written_as_java_writes_it() {
        assert_eq!(uuid_text(ID), "01234567-89ab-cdef-0123-456789abcdef");
        assert_eq!(uuid_text(0), "00000000-0000-0000-0000-000000000000");
    }

    #[test]
    fn each_parameter_has_its_value() {
        let p = steve();
        assert_eq!(value(&p, "name").unwrap(), "Steve");
        assert_eq!(value(&p, "displayname").unwrap(), "Steve");
        assert_eq!(value(&p, "uuid").unwrap(), "01234567-89ab-cdef-0123-456789abcdef");
        assert_eq!(value(&p, "x").unwrap(), "100");
        assert_eq!(value(&p, "y").unwrap(), "64");
        assert_eq!(value(&p, "z").unwrap(), "-1", "block coordinates round down");
        assert_eq!(value(&p, "world").unwrap(), "world");
        assert_eq!(value(&p, "health").unwrap(), "19.5");
        assert_eq!(value(&p, "ping").unwrap(), "42");
        assert_eq!(value(&p, "gamemode").unwrap(), "CREATIVE");
        assert_eq!(value(&p, "level").unwrap(), "7");
    }

    #[test]
    fn a_whole_health_keeps_its_decimal() {
        let mut p = steve();
        p.health = 20.0;
        assert_eq!(value(&p, "health").unwrap(), "20.0");
    }

    #[test]
    fn the_display_name_is_the_nickname_when_one_is_set() {
        let mut p = steve();
        p.nickname = Some("Shadow".into());
        assert_eq!(value(&p, "displayname").unwrap(), "Shadow");
        assert_eq!(value(&p, "name").unwrap(), "Steve");
    }

    #[test]
    fn the_world_of_the_three_default_dimensions() {
        let mut p = steve();
        for (dim, world) in [
            ("minecraft:overworld", "world"),
            ("minecraft:the_nether", "world_nether"),
            ("minecraft:the_end", "world_the_end"),
        ] {
            p.dimension = dim.into();
            assert_eq!(value(&p, "world").unwrap(), world);
        }
        p.dimension = "mod:custom".into();
        assert_eq!(value(&p, "world").unwrap(), "mod:custom");
    }

    #[test]
    fn game_modes_by_bukkit_name() {
        let mut p = steve();
        let names: Vec<_> = (0..4).map(|m| {
            p.game_mode = m;
            value(&p, "gamemode").unwrap()
        }).collect();
        assert_eq!(names, ["SURVIVAL", "CREATIVE", "ADVENTURE", "SPECTATOR"]);
        p.game_mode = 9;
        assert_eq!(value(&p, "gamemode"), None);
    }

    #[test]
    fn an_unknown_parameter_is_left_alone() {
        assert_eq!(value(&steve(), "food"), None);
        assert_eq!(value(&steve(), ""), None);
    }

    #[test]
    fn the_expansion_needs_a_player_in_the_context() {
        let live = live_with(steve());
        let expansion = PlayerExpansion::new(live);
        assert_eq!(expansion.on_request(None, "name"), None);
        assert_eq!(expansion.on_request(Some(ID + 1), "name"), None);
        assert_eq!(expansion.on_request(Some(ID), "name").unwrap(), "Steve");
    }

    #[test]
    fn the_expansion_lists_its_placeholders() {
        let live = Arc::new(Mutex::new(Snapshot::default()));
        let list = PlayerExpansion::new(live).placeholders();
        assert!(list.contains(&"%player_gamemode%".to_owned()));
        assert_eq!(list.len(), 11);
    }

    #[test]
    fn a_block_is_the_floor_of_the_coordinate() {
        assert_eq!(block(0.0), 0);
        assert_eq!(block(-0.0001), -1);
        assert_eq!(block(-16.0), -16);
    }
}
