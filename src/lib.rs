//! horizon-placeholderapi as an arismoko native trait mod (`mod_api`). The rules are the unchanged
//! `placeholderapi-core` crate (`core/`, the former `api/src/papi/`); this file keeps the registry
//! and the player and server snapshot in statics, maps `/papi` to arismoko's typed command hooks,
//! and publishes the functions other mods call (PORT_NOTES.md, "Cross-mod interface").
//!
//! Derived from PlaceholderAPI (https://github.com/PlaceholderAPI/PlaceholderAPI), GPL-3.0-or-later,
//! Copyright (c) 2015 - 2026 PlaceholderAPI Team.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex, MutexGuard};

use mod_api::{
    Command, CommandArgumentValue, CommandSource, Flow, Host, Hooks, PlayerCommandEvent, PlayerRef,
    RegisterCommandsEvent, RunModCommandEvent, ServerHost, ServerTickEvent, ServerTickPhase, Text,
};
use placeholderapi_core::command::{self, ParseArgs, Target};
use placeholderapi_core::player::PlayerExpansion;
use placeholderapi_core::registry::Registry;
use placeholderapi_core::relational;
use placeholderapi_core::replacer::{self, Closure};
use placeholderapi_core::server::{Clock, ServerExpansion};
use placeholderapi_core::snapshot::{Live, PlayerSnap, Snapshot};
use placeholderapi_core::text;

pub use placeholderapi_core::expansion::Expansion;

/// The roots of `/papi`: PlaceholderAPI's command and its name.
const ROOTS: [&str; 2] = ["papi", "placeholderapi"];

/// The usages below each root. `[args:text]` is the rest of the line, parsed by the core as before.
const USAGES: &[&str] = &["parse [args:text]", "list", "info [id:word]", "reload", "version"];

/// The PlaceholderAPI version this port follows, and the port's own version (`/papi version`).
const PAPI_VERSION: &str = "2.12.3";
const PORT_VERSION: &str = "0.1.0";

/// The operator level of every usage: PlaceholderAPI's `op` default.
// PlaceholderAPI's per-subcommand nodes (placeholderapi.parse/.list/.info/.reload) gate each usage
// (`with_node`): a grant opens one to a non-operator, a deny closes it; the level decides when no
// permissions mod answers. `run_mod_command` still refuses an explicit deny with PAPI's own line.
const OP_LEVEL: u8 = 1;

/// The node of each `/papi` subcommand, for the deny check in `run_mod_command`.
fn usage_node(sub: &str) -> Option<&'static str> {
    match sub {
        "parse" => Some("placeholderapi.parse"),
        "list" => Some("placeholderapi.list"),
        "info" => Some("placeholderapi.info"),
        "reload" => Some("placeholderapi.reload"),
        "version" => Some("placeholderapi.version"),
        _ => None,
    }
}

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

#[derive(Default)]
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
    relational::apply(text, one, two, &lookup)
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
        .fold(Text::literal(""), |all, seg| all.append(paint(&seg.text, seg.rgb)))
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

/// A text component with one colour (`#RRGGBB` in arismoko's style).
fn paint(s: &str, rgb: u32) -> Text {
    let mut line = Text::literal(s);
    line.style.color = Some(format!("#{rgb:06X}"));
    line
}

fn player_snap(host: &dyn ServerHost, p: PlayerRef) -> PlayerSnap {
    let entity = p.entity();
    PlayerSnap {
        uuid: p.uuid(),
        name: p.name(host).unwrap_or_default(),
        // `nickname` is None outside a read failure; a nickname a mod set shows here.
        nickname: p.nickname(host).flatten(),
        dimension: p
            .dimension(host)
            .map(|d| d.to_string())
            .unwrap_or_else(|| "minecraft:overworld".to_owned()),
        position: entity.position(host).unwrap_or([0.0, 0.0, 0.0]),
        game_mode: p.game_mode(host).map_or(0, |mode| mode.id()),
        health: host.living(entity).and_then(|l| l.health(host)).unwrap_or(0.0),
        latency: p.latency(host).unwrap_or(0),
        xp_level: p.experience(host).map_or(0, |e| e.level),
    }
}

/// Reads the world folder's server.properties (the server folder is its parent). Returns false
/// while no world is open.
fn read_properties(host: &mut dyn ServerHost) -> bool {
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

/// `/papi <args>` for a player (`sender`) or the console (`None`). `args` is the line after the root.
fn papi_command(host: &mut dyn ServerHost, sender: Option<u128>, args: &str) -> Vec<Reply> {
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
        "version" => coded(vec![format!(
            "&aPlaceholderAPI &7version: &f{PAPI_VERSION} &7(Horizon port {PORT_VERSION})"
        )]),
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

/// Answers one line to the command's source: a player, or the console (arismoko's feedback goes
/// to whichever it is, with the silence rules of the source).
fn send(host: &mut dyn ServerHost, reply: Reply) {
    let line = match reply {
        Reply::Coded(s) => colored_text(&s),
        Reply::Raw(s) => paint(&s, WHITE),
    };
    let _ = host.commands().feedback(line);
}

#[mod_api::horizon_mod]
impl Hooks for PlaceholderApi {
    fn register_commands(&self, _: &mut dyn ServerHost, event: &mut RegisterCommandsEvent<'_>) {
        for root in ROOTS {
            // The bare root (`/papi`) has no permission in plugin.yml: everybody gets the usage line.
            event.commands.push(Command::new(root));
            for usage in USAGES {
                let command = Command::op(&format!("{root} {usage}"), OP_LEVEL);
                let sub = usage.split(' ').next().unwrap_or_default();
                event.commands.push(match usage_node(sub) {
                    Some(node) => command.with_node(node),
                    None => command,
                });
            }
        }
    }

    // Before the engine looks up the typed line: a player without the subcommand's node gets the refusal
    // line, not "Unknown or incomplete command" (the hidden literal). Ops and granted players pass.
    fn player_command(&self, host: &mut dyn ServerHost, event: &mut PlayerCommandEvent<'_>) -> Flow {
        let mut words = event.command.split_whitespace();
        let root = words.next().unwrap_or_default().to_ascii_lowercase();
        let sub = words.next().unwrap_or_default().to_ascii_lowercase();
        if !ROOTS.contains(&root.as_str()) {
            return Flow::Continue;
        }
        let Some(node) = usage_node(&sub) else {
            return Flow::Continue;
        };
        let allowed = host.player(event.player).is_some_and(|p| {
            let level = p.permission_level(&*host).unwrap_or(0);
            p.permission_value(&*host, node).unwrap_or(level >= OP_LEVEL)
        });
        if allowed {
            return Flow::Continue;
        }
        event.replies.push(colored_text("&cYou do not have permission for this /papi command."));
        event.cancelled = true;
        Flow::Cancel
    }

    fn run_mod_command(&self, host: &mut dyn ServerHost, event: &mut RunModCommandEvent<'_>) -> Flow {
        let literals = &event.command.literals;
        let Some(root) = literals.first() else {
            return Flow::Continue;
        };
        if !ROOTS.contains(&root.as_str()) {
            return Flow::Continue;
        }
        // The subcommand word and its one optional argument, joined back into the line the core
        // parses (`parse <target> <message>`, `info <id>`, `list`, `reload`).
        let sub = literals.get(1).cloned().unwrap_or_default();
        if let CommandSource::Player { player, .. } = &event.source
            && usage_node(&sub).is_some_and(|node| player.permission_value(&*host, node) == Some(false))
        {
            send(host, Reply::Coded("<red>You do not have permission for this /papi command.".to_owned()));
            return Flow::Cancel;
        }
        let tail = ["args", "id"]
            .into_iter()
            .find_map(|name| match event.command.argument(name) {
                Some(CommandArgumentValue::String(s)) => Some(s.clone()),
                _ => None,
            })
            .unwrap_or_default();
        let args = if tail.is_empty() { sub } else { format!("{sub} {tail}") };
        let sender = match &event.source {
            CommandSource::Player { player, .. } => Some(player.uuid()),
            _ => None,
        };
        for reply in papi_command(host, sender, &args) {
            send(host, reply);
        }
        Flow::Cancel
    }

    fn server_tick(&self, host: &mut dyn ServerHost, event: &mut ServerTickEvent<'_>) {
        if !matches!(event.phase, ServerTickPhase::ServerTickStart { world_running: true, .. }) {
            return;
        }
        if !PROPERTIES_READ.load(Ordering::Relaxed) {
            read_properties(host);
        }
        let handles = host.players().server_players();
        let players: Vec<PlayerSnap> = handles.into_iter().map(|p| player_snap(&*host, p)).collect();
        let (rate, mspt) = host.tick_stats();
        live().update(players, rate, mspt);
    }
}
