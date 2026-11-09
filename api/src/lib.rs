//! horizon-placeholderapi: PlaceholderAPI's placeholders for Horizon servers, as a trait mod
//! (docs/native-mods.md, "Trait mods").
//!
//! The rules live in `papi` (engine-free, tested through `core/`). This file keeps the registry
//! and the player and server snapshot in statics, maps Horizon's hooks to the `/papi` command, and
//! publishes the functions other mods call. It was written against Horizon b13dbf068 without an
//! engine build (STATUS.md).
//!
//! Derived from PlaceholderAPI (https://github.com/PlaceholderAPI/PlaceholderAPI), GPL-3.0-or-later,
//! Copyright (c) 2015 - 2026 PlaceholderAPI Team.

#[allow(dead_code)]
mod papi;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex, MutexGuard};

use horizon_client::mod_api::{
    Flow, Hooks, Host, NativeCommand, NativeServerPlayer, PerformCommandEvent,
    PerformConsoleCommandEvent, RegisterCommandsEvent, Text, TickEvent,
};
use papi::command::{self, ParseArgs, Target};
use papi::player::PlayerExpansion;
use papi::registry::Registry;
use papi::replacer::{self, Closure};
use papi::server::{Clock, ServerExpansion};
use papi::snapshot::{Live, PlayerSnap, Snapshot};
use papi::text;

pub use papi::expansion::Expansion;

/// The roots of `/papi`: PlaceholderAPI's command and its name.
const ROOTS: [&str; 2] = ["papi", "placeholderapi"];

/// The usages of the root and the permission node each one asks (PlaceholderAPI's plugin.yml).
const USAGES: &[(&str, &str)] = &[
    ("parse <target:player_name> <text:text>", "placeholderapi.parse"),
    ("list", "placeholderapi.list"),
    ("info <id:word>", "placeholderapi.info"),
    ("reload", "placeholderapi.reload"),
];

/// The operator level a usage needs without a permissions mod: PlaceholderAPI's `op` default.
const OP_LEVEL: u8 = 1;

/// The white of a line with no colour of its own.
const WHITE: u32 = 0xFFFFFF;

/// The snapshot the built-in expansions read, refreshed every tick.
static LIVE: LazyLock<Live> = LazyLock::new(|| Arc::new(Mutex::new(Snapshot::default())));

/// Every expansion by identifier; the built-ins are registered on first use.
static REGISTRY: LazyLock<Mutex<Registry>> = LazyLock::new(|| {
    let live: Live = LIVE.clone();
    let clock: Clock = Box::new(clock_now);
    let mut registry = Registry::new();
    let builtins: [Arc<dyn Expansion>; 2] = [
        Arc::new(PlayerExpansion::new(live.clone())),
        Arc::new(ServerExpansion::new(live, clock)),
    ];
    for expansion in builtins {
        registry
            .register(expansion)
            .expect("the built-in identifiers are valid and unique");
    }
    Mutex::new(registry)
});

/// Whether server.properties has been read (it is read on the first tick the world is known).
static PROPERTIES_READ: AtomicBool = AtomicBool::new(false);

pub fn hooks() -> Box<dyn Hooks> {
    Box::new(PlaceholderApi)
}

struct PlaceholderApi;

// ---- The API other mods call ------------------------------------------------------------------

/// Replaces every `%identifier_params%` in `text` for `player` (`None`: there is no player).
/// Unknown placeholders stay as they were written. There is no escape character.
pub fn set_placeholders(player: Option<u128>, text: &str) -> String {
    replacer::apply(Closure::Percent, text, player, &lookup)
}

/// The same for the lines of a list, such as a tab list header and footer.
pub fn set_placeholders_all(player: Option<u128>, lines: &[String]) -> Vec<String> {
    lines.iter().map(|line| set_placeholders(player, line)).collect()
}

/// Replaces every `{identifier_params}` in `text` for `player`.
pub fn set_bracket_placeholders(player: Option<u128>, text: &str) -> String {
    replacer::apply(Closure::Bracket, text, player, &lookup)
}

/// Replaces every `%rel_identifier_params%` in `text` for two players.
pub fn set_relational_placeholders(one: u128, two: u128, text: &str) -> String {
    papi::relational::apply(text, one, two, &lookup)
}

/// Whether `text` has two `%` signs, that is, could hold a placeholder.
pub fn contains_placeholders(text: &str) -> bool {
    replacer::contains_percent(text)
}

/// Adds an expansion. Its identifier must be new (lower case, no `%`, `{`, `}`, `_` or whitespace).
pub fn register(expansion: Box<dyn Expansion>) -> Result<(), String> {
    registry().register(Arc::from(expansion))
}

/// Removes the expansion of `identifier`; false when none was registered under it.
pub fn unregister(identifier: &str) -> bool {
    registry().unregister(identifier)
}

pub fn is_registered(identifier: &str) -> bool {
    lookup(identifier).is_some()
}

/// The identifiers of the registered expansions, sorted.
pub fn identifiers() -> Vec<String> {
    registry().identifiers()
}

/// A `&`-coded string as Horizon text, for a mod that wants colours on a placeholder's value.
/// Placeholder values are never coloured by the expansion mechanism itself.
pub fn colored_text(s: &str) -> Text {
    text::segments(s, WHITE)
        .into_iter()
        .fold(Text::empty(), |all, seg| all.append(Text::colored(seg.text, seg.rgb)))
}

// ---- The glue ---------------------------------------------------------------------------------

fn live() -> MutexGuard<'static, Snapshot> {
    LIVE.lock().unwrap_or_else(|e| e.into_inner())
}

fn registry() -> MutexGuard<'static, Registry> {
    REGISTRY.lock().unwrap_or_else(|e| e.into_inner())
}

fn lookup(identifier: &str) -> Option<Arc<dyn Expansion>> {
    registry().get(identifier)
}

/// The local time: milliseconds since the epoch and the offset east of UTC in seconds.
fn clock_now() -> (i64, i32) {
    let now = chrono::Local::now();
    (now.timestamp_millis(), now.offset().local_minus_utc())
}

fn player_snap(p: &NativeServerPlayer) -> PlayerSnap {
    PlayerSnap {
        uuid: p.uuid,
        name: p.name.clone(),
        nickname: p.nickname.clone(),
        dimension: p.dimension.clone(),
        position: p.position,
        game_mode: p.game_mode,
        health: p.health,
        latency: p.latency,
        xp_level: p.xp_level,
    }
}

/// Reads the world folder's server.properties (the server folder is its parent). Returns false
/// while no world is open.
fn read_properties(host: &mut dyn Host) -> bool {
    let Some(world) = host.world_dir() else {
        return false;
    };
    let server_dir = match world.parent() {
        Some(dir) if !dir.as_os_str().is_empty() => dir.to_path_buf(),
        _ => PathBuf::from("."),
    };
    if let Ok(text) = std::fs::read_to_string(server_dir.join("server.properties")) {
        live().set_properties(&text);
    }
    PROPERTIES_READ.store(true, Ordering::Relaxed);
    true
}

/// One line of an answer: a `&`-coded line, or a placeholder value shown as it is (PlaceholderAPI
/// sends `parse` output with `sendMessage`, which does not colour it).
enum Reply {
    Coded(String),
    Raw(String),
}

fn coded(lines: Vec<String>) -> Vec<Reply> {
    lines.into_iter().map(Reply::Coded).collect()
}

/// `/papi <args>` for a player (`sender`) or the console (`None`).
fn papi_command(host: &mut dyn Host, sender: Option<u128>, args: &str) -> Vec<Reply> {
    let (sub, rest) = args.split_once(' ').unwrap_or((args, ""));
    match sub {
        "parse" => parse(sender, rest),
        "list" => coded(command::list_lines(&identifiers())),
        "info" => info(rest.trim()),
        "reload" => {
            read_properties(host);
            let hooks = registry().len();
            coded(vec![command::reloaded(hooks)])
        }
        _ => coded(vec![command::usage_root()]),
    }
}

fn parse(sender: Option<u128>, rest: &str) -> Vec<Reply> {
    let Some(ParseArgs { target, message }) = command::parse_args(rest) else {
        return coded(vec![command::usage_parse()]);
    };
    let uuid = match target {
        Target::Null => None,
        Target::Me => match sender {
            Some(uuid) => Some(uuid),
            None => return coded(vec![command::console_me()]),
        },
        Target::Name(name) => {
            let found = live().find_by_name(&name).map(|p| p.uuid);
            match found {
                Some(uuid) => Some(uuid),
                None => return coded(vec![command::no_such_player(&name)]),
            }
        }
    };
    vec![Reply::Raw(set_placeholders(uuid, &message))]
}

fn info(identifier: &str) -> Vec<Reply> {
    if identifier.is_empty() {
        return coded(vec![command::usage_info()]);
    }
    match lookup(identifier) {
        Some(expansion) => coded(command::info_lines(&*expansion)),
        None => coded(vec![command::unknown_expansion(identifier)]),
    }
}

/// Sends one answer line to a player.
fn send(host: &mut dyn Host, uuid: u128, reply: Reply) {
    let line = match reply {
        Reply::Coded(s) => colored_text(&s),
        Reply::Raw(s) => Text::colored(s, WHITE),
    };
    host.tell(uuid, line);
}

impl Hooks for PlaceholderApi {
    fn register_commands(&self, _host: &mut dyn Host, event: &mut RegisterCommandsEvent<'_>) {
        // One usage per subcommand, each with its permission node. The usages share the words
        // of `papi`, so no bare root is needed: a root shows when one of its usages is allowed.
        for root in ROOTS {
            for &(usage, node) in USAGES {
                event
                    .commands
                    .push(NativeCommand::op(&format!("{root} {usage}"), OP_LEVEL).permission(node));
            }
        }
    }

    fn perform_command(&self, host: &mut dyn Host, event: &mut PerformCommandEvent<'_>) -> Flow {
        let (root, args) = event.command.split_once(' ').unwrap_or((event.command, ""));
        if !ROOTS.contains(&root) {
            return Flow::Continue;
        }
        let sender = event.player.uuid;
        for reply in papi_command(host, Some(sender), args) {
            send(host, sender, reply);
        }
        Flow::Cancel
    }

    fn perform_console_command(
        &self,
        host: &mut dyn Host,
        event: &mut PerformConsoleCommandEvent<'_>,
    ) -> Flow {
        let cmd = event.command.trim_start_matches('/');
        let (root, args) = cmd.split_once(' ').unwrap_or((cmd, ""));
        if !ROOTS.contains(&root) {
            return Flow::Continue;
        }
        for reply in papi_command(host, None, args) {
            match reply {
                Reply::Coded(s) => eprintln!("[placeholderapi] {}", text::plain(&s)),
                Reply::Raw(s) => eprintln!("[placeholderapi] {s}"),
            }
        }
        Flow::Cancel
    }

    fn tick(&self, host: &mut dyn Host, _event: &mut TickEvent) {
        if !PROPERTIES_READ.load(Ordering::Relaxed) {
            read_properties(host);
        }
        let players: Vec<PlayerSnap> = host.server_players().iter().map(player_snap).collect();
        let (rate, mspt) = host.tick_stats();
        live().update(players, rate, mspt);
    }
}
