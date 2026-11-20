//! horizon-placeholderapi as an arismoko native trait mod (`mod_api`). The rules are the unchanged
//! `placeholderapi-core` crate (`core/`, the former `api/src/papi/`); this file keeps the registry
//! and the player and server snapshot in statics, maps `/papi` to arismoko's typed command hooks,
//! and publishes the functions other mods call (PORT_NOTES.md, "Cross-mod interface").
//!
//! Derived from PlaceholderAPI (https://github.com/PlaceholderAPI/PlaceholderAPI), GPL-3.0-or-later,
//! Copyright (c) 2015 - 2026 PlaceholderAPI Team.


pub mod placeholderapi_core;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex, MutexGuard};

use mod_api::{
    Command, CommandArgumentValue, CommandSource, Flow, Host, Hooks, NodeDefault,
    PermissionGatherNodesEvent, PermissionLevel, PermissionNode, PermissionValueType,
    PlayerCommandEvent, PlayerRef, RegisterCommandsEvent, RunModCommandEvent, ServerHost,
    ServerTickEvent, ServerTickPhase, Suggestion, TabCompleteEvent, Text,
};
use crate::placeholderapi_core::command::{self, ParseArgs, Target};
use crate::placeholderapi_core::config::{self, Config};
use crate::placeholderapi_core::player::PlayerExpansion;
use crate::placeholderapi_core::registry::Registry;
use crate::placeholderapi_core::relational;
use crate::placeholderapi_core::replacer::{self, Closure};
use crate::placeholderapi_core::server::{Clock, ServerExpansion};
use crate::placeholderapi_core::snapshot::{Live, PlayerSnap, Snapshot};
use crate::placeholderapi_core::text;

pub use crate::placeholderapi_core::expansion::Expansion;

/// The roots of `/papi`: PlaceholderAPI's command and its name.
const ROOTS: [&str; 2] = ["papi", "placeholderapi"];

/// The usages below each root: every sub command takes the rest of the line as `[args:text]`, which
/// the core parses as before. A word that is no sub command is `<unknown:text>`, answered as PlaceholderAPI does.
const UNKNOWN_USAGE: &str = "<unknown:text>";

/// The PlaceholderAPI version this port follows (`/papi version`, `/papi help`).
const PAPI_VERSION: &str = "2.12.3";

/// The operator level the nodes default to: PlaceholderAPI's `op` default.
// PlaceholderAPI's per-subcommand nodes (`placeholderapi.parse` and so on) are declared for the permission
// mod and checked here, not through `with_node`: the engine gate asks one node, and a grant of the parent
// `placeholderapi.admin` has to open every sub command it covers. The usages are open (level 0) and
// `player_command` / `run_mod_command` refuse with PAPI's own line. The nodes `placeholderapi.*` and
// `placeholderapi.ecloud.*` cannot be declared (a node name has no `*`).
const OP_LEVEL: i32 = 1;

/// `placeholderapi.admin` defaults to the highest level, so a deny of one sub command still holds for
/// the ops below it; a grant of the node itself opens everything it covers.
const ADMIN_LEVEL: i32 = 4;

/// The white of a line with no colour of its own.
const WHITE: u32 = 0xFFFFFF;

/// The snapshot the built-in expansions read, refreshed every tick.
static LIVE: LazyLock<Live> = LazyLock::new(|| Arc::new(Mutex::new(Snapshot::default())));

/// Every expansion by identifier; the built-ins are registered on first use.
static REGISTRY: LazyLock<Mutex<Registry>> = LazyLock::new(|| {
    let mut registry = Registry::new();
    for expansion in builtins() {
        registry
            .register(expansion)
            .expect("the built-in identifiers are valid and unique");
    }
    Mutex::new(registry)
});

/// The `player` and `server` expansions (`/papi reload` brings back one that was unregistered).
fn builtins() -> [Arc<dyn Expansion>; 2] {
    let live: Live = LIVE.clone();
    let clock: Clock = Box::new(clock_now);
    [Arc::new(PlayerExpansion::new(live.clone())), Arc::new(ServerExpansion::new(live, clock))]
}

/// `plugins/PlaceholderAPI/config.yml`, as the original has it; read with server.properties and by `/papi reload`.
static CONFIG: LazyLock<Mutex<Config>> = LazyLock::new(|| Mutex::new(Config::default()));

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

/// The same for the lines of a list, for two players (`%rel_...%`).
pub fn set_relational_placeholders_all(one: u128, two: u128, lines: &[String]) -> Vec<String> {
    lines.iter().map(|line| set_relational_placeholders(one, two, line)).collect()
}

/// Whether `text` has a `{` and a later `}`, that is, could hold a bracket placeholder.
pub fn contains_bracket_placeholders(text: &str) -> bool {
    replacer::contains_bracket(text)
}

/// `boolean.true` of config.yml: what expansions print for a true value (PlaceholderAPI's `booleanTrue()`).
pub fn boolean_true() -> String {
    config().boolean_true.clone()
}

/// `boolean.false` of config.yml (`booleanFalse()`).
pub fn boolean_false() -> String {
    config().boolean_false.clone()
}

/// `date_format` of config.yml, a Java `SimpleDateFormat` pattern (`getDateFormat()`).
pub fn date_format() -> String {
    config().date_format.clone()
}

/// `debug` of config.yml.
pub fn debug() -> bool {
    config().debug
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

fn config() -> MutexGuard<'static, Config> {
    CONFIG.lock().unwrap_or_else(|e| e.into_inner())
}

/// Reads `plugins/PlaceholderAPI/config.yml` under the server folder; writes the default file when none exists.
fn read_config(server_dir: &std::path::Path) {
    let dir = server_dir.join("plugins").join("PlaceholderAPI");
    let file = dir.join("config.yml");
    match std::fs::read_to_string(&file) {
        Ok(text) => *config() = Config::parse(&text),
        Err(_) => {
            *config() = Config::default();
            if std::fs::create_dir_all(&dir).is_ok() {
                let _ = std::fs::write(&file, config::DEFAULT_FILE);
                *config() = Config::parse(config::DEFAULT_FILE);
            }
        }
    }
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
        // HMOD-GAP(nickname): Horizon main can set a nickname (`set_nickname`) but has no getter; %player_nickname% stays empty.
        nickname: None,
        dimension: p
            .dimension(host)
            .map(|d| d.to_string())
            .unwrap_or_else(|| "minecraft:overworld".to_owned()),
        position: entity.position(host).unwrap_or([0.0, 0.0, 0.0]),
        game_mode: p.game_mode(host).map_or(0, |mode| mode.id()),
        health: host.living(entity).and_then(|l| l.health(host)).unwrap_or(0.0),
        // HMOD-GAP(latency): Horizon main has no player latency query; %player_ping% reads 0.
        latency: 0,
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
    read_config(&server_dir);
    if let Ok(text) = std::fs::read_to_string(server_dir.join("server.properties")) {
        live().set_properties(&text);
    }
    PROPERTIES_READ.store(true, Ordering::Relaxed);
    true
}

/// One line of an answer: a `&`-coded line, a placeholder value shown as it is (PlaceholderAPI
/// sends `parse` output with `sendMessage`, which does not colour it), or one broadcast to everybody.
enum Reply {
    Coded(String),
    Raw(String),
    Broadcast(String),
}

fn coded(lines: Vec<String>) -> Vec<Reply> {
    lines.into_iter().map(Reply::Coded).collect()
}

/// What a source is, for `me` and `cmdparse`.
enum Sender {
    Player { uuid: u128, entity: mod_api::EntityRef, level: PermissionLevel },
    Console,
    Other,
}

impl Sender {
    fn uuid(&self) -> Option<u128> {
        match self {
            Sender::Player { uuid, .. } => Some(*uuid),
            _ => None,
        }
    }
}

/// Whether the player holds `name`, or `placeholderapi.admin` when the admin node covers it.
/// An error is a refusal.
fn has_node(host: &mut dyn ServerHost, uuid: u128, name: &str) -> bool {
    let node = PermissionNode::new(name, PermissionValueType::Boolean);
    host.player(uuid)
        .is_some_and(|p| p.permission_bool(&mut *host, &node).unwrap_or(false))
}

fn holds(host: &mut dyn ServerHost, uuid: u128, name: &str) -> bool {
    has_node(host, uuid, name) || (command::admin_covers(name) && has_node(host, uuid, "placeholderapi.admin"))
}

/// Whether the player may run `/papi <args>`: the sub command's node, and for `ecloud` the node
/// of its own sub command. A word that is no sub command needs nothing.
fn permitted(host: &mut dyn ServerHost, uuid: u128, args: &str) -> bool {
    let mut words = args.split_whitespace();
    let sub = words.next().unwrap_or_default().to_lowercase();
    let Some(node) = command::node_of(&sub) else {
        return true;
    };
    if !holds(host, uuid, node) {
        return false;
    }
    if sub == "ecloud"
        && let Some(node) = words.next().and_then(|w| command::ecloud_node_of(&w.to_lowercase()))
    {
        return holds(host, uuid, node);
    }
    true
}

/// `/papi <args>` for a source. `args` is the line after the root.
fn papi_command(host: &mut dyn ServerHost, sender: &Sender, args: &str) -> Vec<Reply> {
    let (sub, rest) = args.split_once(' ').unwrap_or((args, ""));
    let sub = sub.to_lowercase();
    match sub.as_str() {
        "" | "version" => coded(command::version_lines(PAPI_VERSION)),
        "help" => coded(command::help_lines(PAPI_VERSION)),
        "parse" => parse(sender, rest, "parse"),
        "bcparse" => parse(sender, rest, "bcparse"),
        "cmdparse" => cmdparse(host, sender, rest),
        "parserel" => parserel(sender, rest),
        "list" => coded(command::list_lines(&identifiers())),
        "info" => info(rest.trim()),
        "reload" => {
            read_properties(host);
            for expansion in builtins() {
                let _ = registry().register(expansion);
            }
            let hooks = registry().len();
            coded(vec![command::reloaded(hooks)])
        }
        "unregister" => unregister_command(rest.trim()),
        "register" => match rest.split_whitespace().next() {
            None => coded(vec![command::usage_register()]),
            Some(file) => coded(vec![command::no_such_file(file)]),
        },
        "dump" => coded(vec![command::dump_unavailable()]),
        "ecloud" => ecloud(rest),
        other => coded(vec![command::unknown_command(other)]),
    }
}

/// `/papi ecloud`: the help menu and the refusals PlaceholderAPI gives; there is no cloud to ask.
fn ecloud(rest: &str) -> Vec<Reply> {
    let Some(search) = rest.split_whitespace().next() else {
        return coded(command::ecloud_help_lines());
    };
    let search = search.to_lowercase();
    if !command::ECLOUD_LABELS.contains(&search.as_str()) {
        return coded(vec![format!("&cUnknown command &7ecloud {search}")]);
    }
    coded(vec![command::ecloud_unavailable()])
}

fn unregister_command(name: &str) -> Vec<Reply> {
    if name.is_empty() {
        return coded(vec![command::usage_info()]);
    }
    let mut registry = registry();
    let found = registry
        .identifiers()
        .into_iter()
        .filter_map(|id| registry.get(&id))
        .find(|e| e.name().eq_ignore_ascii_case(name));
    let Some(expansion) = found else {
        return coded(vec![command::unknown_expansion(name)]);
    };
    let ok = registry.unregister(expansion.identifier());
    coded(vec![command::unregistered(expansion.name(), ok)])
}

/// A `parse` target: `me`, `--null` or an online player's name. `Err` is the refusal to show.
fn resolve(sender: &Sender, target: Target) -> Result<Option<u128>, Reply> {
    match target {
        Target::Null => Ok(None),
        Target::Me => match sender.uuid() {
            Some(uuid) => Ok(Some(uuid)),
            None => Err(Reply::Coded(command::console_me())),
        },
        Target::Name(name) => {
            let found = live().find_by_name(&name).map(|p| p.uuid);
            match found {
                Some(uuid) => Ok(Some(uuid)),
                None => Err(Reply::Coded(command::no_such_player(&name))),
            }
        }
    }
}

fn parse(sender: &Sender, rest: &str, alias: &str) -> Vec<Reply> {
    let Some(ParseArgs { target, message }) = command::parse_args(rest) else {
        return coded(vec![command::usage_parse(alias)]);
    };
    let uuid = match resolve(sender, target) {
        Ok(uuid) => uuid,
        Err(reply) => return vec![reply],
    };
    let text = set_placeholders(uuid, &message);
    vec![if alias == "bcparse" { Reply::Broadcast(text) } else { Reply::Raw(text) }]
}

/// `cmdparse`: the parsed text is run as a command by the sender, with the sender's own level.
fn cmdparse(host: &mut dyn ServerHost, sender: &Sender, rest: &str) -> Vec<Reply> {
    let Some(ParseArgs { target, message }) = command::parse_args(rest) else {
        return coded(vec![command::usage_parse("cmdparse")]);
    };
    let uuid = match resolve(sender, target) {
        Ok(uuid) => uuid,
        Err(reply) => return vec![reply],
    };
    let line = set_placeholders(uuid, &message);
    match sender {
        Sender::Player { entity, level, .. } => {
            let _ = entity.run_command(&mut *host, &line, *level);
        }
        Sender::Console => {
            let _ = host.commands().run_command(&line);
        }
        // HMOD-GAP(cmdparse-source): a command block or function has no entity to run it as.
        Sender::Other => {}
    }
    Vec::new()
}

/// `parserel`: both targets must be online players (`me` is the sender).
fn parserel(sender: &Sender, rest: &str) -> Vec<Reply> {
    let Some((one, two, message)) = command::parse_rel_args(rest) else {
        return coded(vec![command::usage_parserel()]);
    };
    let mut ends = [0u128; 2];
    for (slot, word) in ends.iter_mut().zip([&one, &two]) {
        let uuid = if word.eq_ignore_ascii_case("me") {
            match sender.uuid() {
                Some(uuid) => uuid,
                None => return coded(vec![command::console_me()]),
            }
        } else {
            match live().find_by_name(word).map(|p| p.uuid) {
                Some(uuid) => uuid,
                None => return coded(vec![command::no_such_player_rel(word)]),
            }
        };
        *slot = uuid;
    }
    vec![Reply::Raw(set_relational_placeholders(ends[0], ends[1], &message))]
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

/// Rebuilds the player snapshot the built-in expansions read.
fn refresh(host: &mut dyn ServerHost) {
    let handles = host.players().server_players();
    let players: Vec<PlayerSnap> = handles.into_iter().map(|p| player_snap(&*host, p)).collect();
    let (rate, mspt) = host.tick_stats();
    live().update(players, rate, mspt);
}

/// Answers one line to the command's source: a player, or the console (arismoko's feedback goes
/// to whichever it is, with the silence rules of the source).
fn send(host: &mut dyn ServerHost, reply: Reply) {
    let line = match reply {
        Reply::Coded(s) => colored_text(&s),
        Reply::Raw(s) => paint(&s, WHITE),
        Reply::Broadcast(s) => {
            let _ = host.messages().broadcast(paint(&s, WHITE));
            return;
        }
    };
    let _ = host.commands().feedback(line);
}

#[mod_api::horizon_mod]
impl Hooks for PlaceholderApi {
    fn permission_gather_nodes(&self, event: &mut PermissionGatherNodesEvent<'_>) {
        for name in command::NODES {
            let level = if name == "placeholderapi.admin" { ADMIN_LEVEL } else { OP_LEVEL };
            let node = PermissionNode::new(name, PermissionValueType::Boolean);
            let _ = event.nodes.add(node, NodeDefault::Level(PermissionLevel::from_level(level)));
        }
    }

    fn register_commands(&self, _: &mut dyn ServerHost, event: &mut RegisterCommandsEvent<'_>) {
        for root in ROOTS {
            // The bare root (`/papi`) has no permission in plugin.yml: everybody gets the version.
            event.commands.push(Command::new(root));
            for label in command::LABELS {
                event.commands.push(Command::new(&format!("{root} {label} [args:text]")));
            }
            event.commands.push(Command::new(&format!("{root} {UNKNOWN_USAGE}")));
        }
    }

    // Before the engine looks up the typed line: a player without the subcommand's node gets the refusal
    // line. Ops and granted players pass.
    fn player_command(&self, host: &mut dyn ServerHost, event: &mut PlayerCommandEvent<'_>) -> Flow {
        let line = event.command.trim_start();
        let (root, rest) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
        if !ROOTS.contains(&root.to_ascii_lowercase().as_str()) {
            return Flow::Continue;
        }
        if permitted(host, event.player, rest.trim_start()) {
            return Flow::Continue;
        }
        event.replies.push(colored_text(&command::no_permission()));
        event.cancelled = true;
        Flow::Cancel
    }

    fn tab_complete(&self, host: &mut dyn ServerHost, event: &mut TabCompleteEvent<'_>) {
        let line = event.command.trim_start_matches('/');
        let mut words: Vec<&str> = line.split(' ').collect();
        if words.len() < 2 || !ROOTS.contains(&words.remove(0).to_ascii_lowercase().as_str()) {
            return;
        }
        let last = words[words.len() - 1];
        let end = event.range[1];
        let range = (end.saturating_sub(last.len()), end);
        let uuid = event.player;
        let granted: Vec<&str> =
            command::NODES.iter().copied().filter(|node| holds(&mut *host, uuid, node)).collect();
        let allowed = |node: &str| granted.contains(&node);
        let names: Vec<String> = live().players.iter().map(|p| p.name.clone()).collect();
        let ids = identifiers();
        let placeholders = |id: &str| lookup(id).map(|e| e.placeholders()).unwrap_or_default();
        let completer = command::Completer {
            is_player: true,
            allowed: &allowed,
            online: &names,
            identifiers: &ids,
            placeholders: &placeholders,
        };
        let offered = command::complete(&words, &completer);
        if words.len() == 1 {
            // The engine lists every sub command; keep only the ones this player may run.
            event.suggestions.retain(|s| {
                !command::LABELS.contains(&s.text.to_lowercase().as_str()) || offered.contains(&s.text)
            });
        }
        for text in offered {
            if !event.suggestions.iter().any(|s| s.text == text) {
                event.suggestions.push(Suggestion::new(range, text));
            }
        }
    }

    fn run_mod_command(&self, host: &mut dyn ServerHost, event: &mut RunModCommandEvent<'_>) -> Flow {
        let literals = &event.command.literals;
        let Some(root) = literals.first() else {
            return Flow::Continue;
        };
        if !ROOTS.contains(&root.as_str()) {
            return Flow::Continue;
        }
        // The subcommand word and its rest of the line, joined back into the line the core parses
        // (`parse <target> <message>`, `info <id>`, `list`, `reload`); a word that is no sub command
        // arrives whole as `unknown`.
        let sub = literals.get(1).cloned().unwrap_or_default();
        let tail = ["args", "unknown"]
            .into_iter()
            .find_map(|name| match event.command.argument(name) {
                Some(CommandArgumentValue::String(s)) => Some(s.clone()),
                _ => None,
            })
            .unwrap_or_default();
        let args = match (sub.is_empty(), tail.is_empty()) {
            (_, true) => sub,
            (true, false) => tail,
            (false, false) => format!("{sub} {tail}"),
        };
        let sender = match &event.source {
            CommandSource::Player { player, permission } => {
                Sender::Player { uuid: player.uuid(), entity: player.entity(), level: *permission }
            }
            CommandSource::Console { .. } => Sender::Console,
            _ => Sender::Other,
        };
        if let Sender::Player { uuid, .. } = &sender
            && !permitted(host, *uuid, &args)
        {
            send(host, Reply::Coded(command::no_permission()));
            return Flow::Cancel;
        }
        refresh(host);
        for reply in papi_command(host, &sender, &args) {
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
        refresh(host);
    }
}
