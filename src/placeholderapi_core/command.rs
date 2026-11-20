//! The text of `/papi`: argument splitting and the replies, as PlaceholderAPI's `CommandParse`,
//! `CommandList` and `CommandInfo` write them. Replies are `&`-coded lines; the glue shows them.
//!
//! Derived from PlaceholderAPI's commands/impl/local (CommandParse, CommandList, CommandInfo,
//! CommandReload), Copyright (c) 2015 - 2026 PlaceholderAPI Team, GPL-3.0-or-later.
//!
//! The one deliberate change: PlaceholderAPI's info line for an unregistered expansion reads
//! `7cNotRegistered` (a missing `&`). Our info line is always the registered one, so the typo is gone.

use super::expansion::Expansion;

/// Hooks per line in `/papi list`.
pub const LIST_WIDTH: usize = 10;

/// Who a `parse` is for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    /// `me`: the sender. Only a player can be the sender.
    Me,
    /// `--null`: no player in the context.
    Null,
    /// A name, matched against the online players in any case.
    Name(String),
}

/// `parse` arguments: the target and the message, which keeps the spaces it was typed with after
/// the one that separates it from the target.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseArgs {
    pub target: Target,
    pub message: String,
}

pub fn target_of(word: &str) -> Target {
    if word.eq_ignore_ascii_case("me") {
        Target::Me
    } else if word.eq_ignore_ascii_case("--null") {
        Target::Null
    } else {
        Target::Name(word.to_owned())
    }
}

/// Splits the arguments after `parse` (`<target> <message...>`). `None` when the message is missing,
/// which is the usage error.
pub fn parse_args(rest: &str) -> Option<ParseArgs> {
    let (word, message) = rest.split_once(' ')?;
    if message.is_empty() {
        return None;
    }
    Some(ParseArgs {
        target: target_of(word),
        message: message.to_owned(),
    })
}

/// The usage line of `parse`, `bcparse` or `cmdparse` (`alias` is the word typed).
pub fn usage_parse(alias: &str) -> String {
    format!("&cYou must provide a target and message: &b/papi {alias} &7{{target}} &a{{message}}")
}

pub fn usage_parserel() -> String {
    "&cYou must supply two targets, and a message: &b/papi parserel &7{target one} {target two} &a{message}".to_owned()
}

pub fn no_such_player(name: &str) -> String {
    format!("&cFailed to find player: &7{name}")
}

/// `parserel` words its refusal with a white name, not a grey one.
pub fn no_such_player_rel(name: &str) -> String {
    format!("&cFailed to find player: &f{name}")
}

pub fn console_me() -> String {
    "&cYou must be a player to use &7me&c as a target!".to_owned()
}

pub fn usage_info() -> String {
    "&cYou must specify the name of the expansion.".to_owned()
}

pub fn unknown_expansion(identifier: &str) -> String {
    format!("&cThere is no expansion loaded with the identifier: &f{identifier}")
}

pub fn usage_register() -> String {
    "&cYou must specify the name of an expansion file.".to_owned()
}

/// The port has no expansions folder, so no file named to `register` can exist.
pub fn no_such_file(name: &str) -> String {
    format!("&cThe file &f{name}&c doesn't exist!")
}

pub fn unregistered(name: &str, ok: bool) -> String {
    let head = if ok { "&aSuccessfully unregistered expansion: &f" } else { "&cFailed to unregister expansion: &f" };
    format!("{head}{name}")
}

pub fn no_permission() -> String {
    "&cYou do not have permission to do this!".to_owned()
}

/// A word that is no sub command (`search` is already lower case).
pub fn unknown_command(search: &str) -> String {
    format!("&cUnknown command &7{search}")
}

pub fn dump_unavailable() -> String {
    "&cThe dump upload is not available in this port.".to_owned()
}

pub fn ecloud_unavailable() -> String {
    "&cThe eCloud Manager is not available in this port.".to_owned()
}

/// `/papi reload`: how many expansions are registered afterwards.
pub fn reloaded(hooks: usize) -> String {
    format!("{}{hooks} placeholder hook(s) registered!", if hooks == 0 { "&6" } else { "&a" })
}

/// `/papi version` (and a bare `/papi`).
pub fn version_lines(version: &str) -> Vec<String> {
    vec![
        format!("&b&lPlaceholderAPI &7(&f{version}&7)"),
        "&7Author: &f[HelpChat]".to_owned(),
        "&7PAPI Commands: &b/papi &fhelp".to_owned(),
        "&7eCloud Commands&8: &b/papi &fecloud".to_owned(),
    ]
}

pub fn help_lines(version: &str) -> Vec<String> {
    let mut lines = vec![format!("&b&lPlaceholderAPI &8- &7Help Menu &8- &7(&f{version}&7)"), " ".to_owned()];
    let entries: [(&str, &str); 11] = [
        ("&b/papi &fbcparse &9<me|--null|player name> <message>", "Parse a message with placeholders and broadcast it"),
        ("&b/papi &fcmdparse &9<me|player> <command with placeholders>", "Parse a message with relational placeholders"),
        ("&b/papi &fdump", "Dump all relevant information needed to help debug issues into a paste link."),
        ("&b/papi &finfo &9<placeholder name>", "View information for a specific expansion"),
        ("&b/papi &flist", "List active expansions"),
        ("&b/papi &fparse &9<me|--null|player name> <message>", "Parse a message with placeholders"),
        ("&b/papi &fparserel &9<player one> <player two> <message>", "Parse a message with relational placeholders"),
        ("&b/papi &fregister &9<file name>", "Register an expansion by the name of the file"),
        ("&b/papi &freload", "Reload the config of PAPI"),
        ("&b/papi &funregister &9<expansion name>", "Unregister an expansion by name"),
        ("&b/papi &fversion", "View plugin info/version"),
    ];
    for (usage, what) in entries {
        lines.push(usage.to_owned());
        lines.push(format!("  &7&o{what}"));
    }
    lines
}

pub fn ecloud_help_lines() -> Vec<String> {
    let entries: [(&str, &str); 8] = [
        ("&b/papi &fecloud status", "View status of the eCloud"),
        ("&b/papi &fecloud list <all/{author}/installed> {page}", "List all/author specific available expansions"),
        ("&b/papi &fecloud info <expansion name> {version}", "View information about a specific expansion available on the eCloud"),
        ("&b/papi &fecloud placeholders <expansion name>", "View placeholders for an expansion"),
        ("&b/papi &fecloud download <expansion name> {version}", "Download an expansion from the eCloud"),
        ("&b/papi &fecloud update <expansion name/all>", "Update a specific/all installed expansions"),
        ("&b/papi &fecloud refresh", "Fetch the most up to date list of expansions available."),
        ("&b/papi &fecloud clear", "Clear the expansion cloud cache."),
    ];
    let mut lines = vec!["&b&lPlaceholderAPI &8- &7eCloud Help Menu &8- ".to_owned(), " ".to_owned()];
    for (usage, what) in entries {
        lines.push(usage.to_owned());
        lines.push(format!("  &7&o{what}"));
    }
    lines
}

/// `parserel` arguments: two targets and the message.
pub fn parse_rel_args(rest: &str) -> Option<(String, String, String)> {
    let (one, rest) = rest.split_once(' ')?;
    let (two, message) = rest.split_once(' ')?;
    if message.is_empty() {
        return None;
    }
    Some((one.to_owned(), two.to_owned(), message.to_owned()))
}

/// The sub commands, and the aliases of `parse`, in the order the router lists them.
pub const LABELS: [&str; 13] = [
    "help", "info", "list", "dump", "ecloud", "parse", "bcparse", "cmdparse", "parserel", "reload",
    "version", "register", "unregister",
];

/// The sub commands of `ecloud`.
pub const ECLOUD_LABELS: [&str; 8] =
    ["clear", "status", "update", "refresh", "download", "info", "list", "placeholders"];

/// Every node plugin.yml declares that a mod can check (the two wildcards cannot be typed nodes).
pub const NODES: [&str; 20] = [
    "placeholderapi.admin",
    "placeholderapi.help",
    "placeholderapi.info",
    "placeholderapi.list",
    "placeholderapi.dump",
    "placeholderapi.ecloud",
    "placeholderapi.parse",
    "placeholderapi.reload",
    "placeholderapi.version",
    "placeholderapi.register",
    "placeholderapi.unregister",
    "placeholderapi.updatenotify",
    "placeholderapi.ecloud.info",
    "placeholderapi.ecloud.list",
    "placeholderapi.ecloud.clear",
    "placeholderapi.ecloud.status",
    "placeholderapi.ecloud.update",
    "placeholderapi.ecloud.refresh",
    "placeholderapi.ecloud.download",
    "placeholderapi.ecloud.placeholders",
];

/// The node a sub command asks (`parse` and its aliases share one).
pub fn node_of(label: &str) -> Option<&'static str> {
    Some(match label {
        "help" => "placeholderapi.help",
        "info" => "placeholderapi.info",
        "list" => "placeholderapi.list",
        "dump" => "placeholderapi.dump",
        "ecloud" => "placeholderapi.ecloud",
        "parse" | "bcparse" | "cmdparse" | "parserel" => "placeholderapi.parse",
        "reload" => "placeholderapi.reload",
        "version" => "placeholderapi.version",
        "register" => "placeholderapi.register",
        "unregister" => "placeholderapi.unregister",
        _ => return None,
    })
}

/// The node of an `ecloud` sub command.
pub fn ecloud_node_of(label: &str) -> Option<&'static str> {
    NODES.iter().copied().find(|node| {
        node.strip_prefix("placeholderapi.ecloud.") == Some(label)
    })
}

/// Whether `placeholderapi.admin` grants `node` (its children in plugin.yml).
pub fn admin_covers(node: &str) -> bool {
    matches!(
        node,
        "placeholderapi.help"
            | "placeholderapi.info"
            | "placeholderapi.list"
            | "placeholderapi.parse"
            | "placeholderapi.reload"
            | "placeholderapi.version"
            | "placeholderapi.register"
            | "placeholderapi.unregister"
            | "placeholderapi.updatenotify"
    )
}

/// What the tab completion may look at.
pub struct Completer<'a> {
    pub is_player: bool,
    /// Whether the sender holds a node.
    pub allowed: &'a dyn Fn(&str) -> bool,
    pub online: &'a [String],
    pub identifiers: &'a [String],
    /// The placeholders an expansion lists, by identifier (empty for none).
    pub placeholders: &'a dyn Fn(&str) -> Vec<String>,
}

fn starting_with<'a>(
    possible: impl IntoIterator<Item = &'a str>,
    typed: &str,
) -> Vec<String> {
    let typed = typed.to_lowercase();
    possible
        .into_iter()
        .filter(|s| s.to_lowercase().starts_with(&typed))
        .map(str::to_owned)
        .collect()
}

/// The completions after `/papi`, as PlaceholderAPI's router and commands give them. `args` is
/// what follows the root; its last element is the word being typed (empty after a space).
pub fn complete(args: &[&str], c: &Completer<'_>) -> Vec<String> {
    let Some((&first, params)) = args.split_first() else {
        return Vec::new();
    };
    let permitted = |label: &str| node_of(label).is_none_or(|node| (c.allowed)(node));
    if params.is_empty() {
        return starting_with(LABELS.iter().copied().filter(|l| permitted(l)), first);
    }
    let label = first.to_lowercase();
    if !permitted(&label) {
        return Vec::new();
    }
    let last = params[params.len() - 1];
    match label.as_str() {
        "info" | "unregister" if params.len() <= 1 => {
            starting_with(c.identifiers.iter().map(String::as_str), last)
        }
        "parse" | "bcparse" | "cmdparse" => {
            if params.len() <= 1 {
                let mut out = Vec::new();
                if c.is_player && "me".starts_with(&last.to_lowercase()) {
                    out.push("me".to_owned());
                }
                if "--null".starts_with(&last.to_lowercase()) {
                    out.push("--null".to_owned());
                }
                out.extend(starting_with(c.online.iter().map(String::as_str), last));
                return out;
            }
            if !last.starts_with('%') || last.ends_with('%') {
                return Vec::new();
            }
            let Some(index) = last.find('_') else {
                return Vec::new();
            };
            let placeholders = (c.placeholders)(&last[1..index]);
            starting_with(placeholders.iter().map(String::as_str), last)
        }
        "parserel" if params.len() <= 2 => starting_with(c.online.iter().map(String::as_str), last),
        "ecloud" if params.len() <= 1 => starting_with(
            ECLOUD_LABELS
                .iter()
                .copied()
                .filter(|l| ecloud_node_of(l).is_none_or(|node| (c.allowed)(node))),
            last,
        ),
        _ => Vec::new(),
    }
}

/// `/papi list`: a header with the count, then the identifiers, `LIST_WIDTH` to a line.
pub fn list_lines(identifiers: &[String]) -> Vec<String> {
    if identifiers.is_empty() {
        return vec!["&cThere are no placeholder hooks active!".to_owned()];
    }
    let mut lines = vec![format!(
        "&7A total of &f{}&7 placeholder hook(s) are active: &a",
        identifiers.len()
    )];
    for chunk in identifiers.chunks(LIST_WIDTH) {
        lines.push(chunk.join("&7, &a"));
    }
    lines
}

/// `/papi info <id>`: the expansion's name, status, author, version and placeholders.
pub fn info_lines(expansion: &dyn Expansion) -> Vec<String> {
    let mut lines = vec![
        format!("&7Placeholder expansion info for: &r{}", expansion.name()),
        "&7Status: &aRegistered".to_owned(),
    ];
    if !expansion.author().is_empty() {
        lines.push(format!("&7Author: &r{}", expansion.author()));
    }
    if !expansion.version().is_empty() {
        lines.push(format!("&7Version: &r{}", expansion.version()));
    }
    if let Some(required) = expansion.required_plugin() {
        lines.push(format!("&7Requires plugin: &r{required}"));
    }
    let placeholders = expansion.placeholders();
    if !placeholders.is_empty() {
        lines.push("&8&m-- &7Placeholders &8&m--&r".to_owned());
        lines.extend(placeholders);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_words() {
        assert_eq!(target_of("ME"), Target::Me);
        assert_eq!(target_of("--null"), Target::Null);
        assert_eq!(target_of("--NULL"), Target::Null);
        assert_eq!(target_of("Steve"), Target::Name("Steve".into()));
    }

    #[test]
    fn message_keeps_its_inner_spaces() {
        let args = parse_args("me a   b  c").unwrap();
        assert_eq!(args.message, "a   b  c");
    }

    #[test]
    fn the_space_after_the_target_is_the_separator_only() {
        let args = parse_args("me  two spaces").unwrap();
        assert_eq!(args.message, " two spaces");
    }

    #[test]
    fn a_missing_message_is_a_usage_error() {
        assert_eq!(parse_args("me"), None);
        assert_eq!(parse_args("me "), None);
        assert_eq!(parse_args(""), None);
    }

    #[test]
    fn list_shows_ten_to_a_line() {
        let ids: Vec<String> = (0..12).map(|i| format!("e{i:02}")).collect();
        let lines = list_lines(&ids);
        assert_eq!(lines[0], "&7A total of &f12&7 placeholder hook(s) are active: &a");
        assert_eq!(lines.len(), 3);
        assert!(lines[1].starts_with("e00&7, &ae01"));
        assert!(lines[1].ends_with("&7, &ae09"));
        assert_eq!(lines[2], "e10&7, &ae11");
    }

    #[test]
    fn an_empty_list_says_so() {
        assert_eq!(list_lines(&[]), vec!["&cThere are no placeholder hooks active!"]);
    }

    fn ctx<'a>(
        allowed: &'a dyn Fn(&str) -> bool,
        online: &'a [String],
        ids: &'a [String],
        placeholders: &'a dyn Fn(&str) -> Vec<String>,
    ) -> Completer<'a> {
        Completer { is_player: true, allowed, online, identifiers: ids, placeholders }
    }

    #[test]
    fn null_target_is_case_insensitive() {
        assert_eq!(parse_args("--NuLl hi").unwrap().target, Target::Null);
    }

    #[test]
    fn usage_names_the_alias_typed() {
        assert_eq!(
            usage_parse("bcparse"),
            "&cYou must provide a target and message: &b/papi bcparse &7{target} &a{message}"
        );
    }

    #[test]
    fn parserel_needs_two_targets_and_a_message() {
        assert_eq!(
            parse_rel_args("a b hello  there"),
            Some(("a".into(), "b".into(), "hello  there".into()))
        );
        assert_eq!(parse_rel_args("a b"), None);
        assert_eq!(parse_rel_args("a b "), None);
        assert_eq!(parse_rel_args("a"), None);
    }

    #[test]
    fn version_lines_match_the_original() {
        let lines = version_lines("2.12.3");
        assert_eq!(lines[0], "&b&lPlaceholderAPI &7(&f2.12.3&7)");
        assert_eq!(lines[1], "&7Author: &f[HelpChat]");
        assert_eq!(lines[2], "&7PAPI Commands: &b/papi &fhelp");
        assert_eq!(lines[3], "&7eCloud Commands&8: &b/papi &fecloud");
    }

    #[test]
    fn help_menu_has_a_header_and_every_command() {
        let lines = help_lines("2.12.3");
        assert_eq!(lines[0], "&b&lPlaceholderAPI &8- &7Help Menu &8- &7(&f2.12.3&7)");
        assert_eq!(lines.len(), 2 + 11 * 2);
        assert_eq!(lines[2], "&b/papi &fbcparse &9<me|--null|player name> <message>");
        assert_eq!(lines[3], "  &7&oParse a message with placeholders and broadcast it");
    }

    #[test]
    fn reload_message_counts_the_hooks() {
        assert_eq!(reloaded(2), "&a2 placeholder hook(s) registered!");
        assert_eq!(reloaded(0), "&60 placeholder hook(s) registered!");
    }

    #[test]
    fn the_aliases_of_parse_share_its_node() {
        for label in ["parse", "bcparse", "cmdparse", "parserel"] {
            assert_eq!(node_of(label), Some("placeholderapi.parse"));
        }
        assert_eq!(node_of("unregister"), Some("placeholderapi.unregister"));
        assert_eq!(node_of("nope"), None);
        assert_eq!(ecloud_node_of("download"), Some("placeholderapi.ecloud.download"));
        assert_eq!(ecloud_node_of("nope"), None);
    }

    #[test]
    fn admin_covers_its_children_only() {
        assert!(admin_covers("placeholderapi.parse"));
        assert!(admin_covers("placeholderapi.updatenotify"));
        assert!(!admin_covers("placeholderapi.dump"));
        assert!(!admin_covers("placeholderapi.ecloud"));
    }

    #[test]
    fn sub_commands_are_offered_by_permission() {
        let allowed = |n: &str| n == "placeholderapi.parse" || n == "placeholderapi.list";
        let none = |_: &str| -> Vec<String> { Vec::new() };
        let c = ctx(&allowed, &[], &[], &none);
        assert_eq!(complete(&["p"], &c), vec!["parse", "parserel"]);
        assert_eq!(complete(&[""], &c), vec!["list", "parse", "bcparse", "cmdparse", "parserel"]);
        assert_eq!(complete(&["PA"], &c), vec!["parse", "parserel"]);
    }

    #[test]
    fn info_and_unregister_complete_identifiers() {
        let all = |_: &str| true;
        let none = |_: &str| -> Vec<String> { Vec::new() };
        let ids = vec!["player".to_owned(), "server".to_owned()];
        let c = ctx(&all, &[], &ids, &none);
        assert_eq!(complete(&["info", "S"], &c), vec!["server"]);
        assert_eq!(complete(&["unregister", ""], &c), vec!["player", "server"]);
        assert!(complete(&["info", "server", ""], &c).is_empty());
    }

    #[test]
    fn parse_completes_me_null_and_names() {
        let all = |_: &str| true;
        let none = |_: &str| -> Vec<String> { Vec::new() };
        let names = vec!["Alex".to_owned(), "Bob".to_owned()];
        let c = ctx(&all, &names, &[], &none);
        assert_eq!(complete(&["parse", ""], &c), vec!["me", "--null", "Alex", "Bob"]);
        assert_eq!(complete(&["parse", "-"], &c), vec!["--null"]);
        assert_eq!(complete(&["parse", "m"], &c), vec!["me"]);
        let console = Completer { is_player: false, ..ctx(&all, &names, &[], &none) };
        assert_eq!(complete(&["bcparse", ""], &console), vec!["--null", "Alex", "Bob"]);
    }

    #[test]
    fn parse_completes_the_placeholders_of_a_half_typed_one() {
        let all = |_: &str| true;
        let lookup = |id: &str| -> Vec<String> {
            if id == "player" { vec!["%player_name%".to_owned(), "%player_ping%".to_owned(), "%server_tps%".to_owned()] } else { Vec::new() }
        };
        let c = ctx(&all, &[], &[], &lookup);
        assert_eq!(complete(&["parse", "me", "%player_n"], &c), vec!["%player_name%"]);
        assert!(complete(&["parse", "me", "%player_name%"], &c).is_empty());
        assert!(complete(&["parse", "me", "%player"], &c).is_empty());
        assert!(complete(&["parse", "me", "hello"], &c).is_empty());
        assert!(complete(&["parse", "me", "%nope_x"], &c).is_empty());
    }

    #[test]
    fn parserel_completes_names_for_two_arguments() {
        let all = |_: &str| true;
        let none = |_: &str| -> Vec<String> { Vec::new() };
        let names = vec!["Alex".to_owned()];
        let c = ctx(&all, &names, &[], &none);
        assert_eq!(complete(&["parserel", "a"], &c), vec!["Alex"]);
        assert_eq!(complete(&["parserel", "Alex", ""], &c), vec!["Alex"]);
        assert!(complete(&["parserel", "Alex", "Alex", ""], &c).is_empty());
    }

    #[test]
    fn a_sub_command_without_its_node_completes_nothing() {
        let deny = |_: &str| false;
        let none = |_: &str| -> Vec<String> { Vec::new() };
        let ids = vec!["player".to_owned()];
        let c = ctx(&deny, &[], &ids, &none);
        assert!(complete(&["info", ""], &c).is_empty());
        assert!(complete(&[""], &c).is_empty());
    }

    #[test]
    fn ecloud_completes_its_sub_commands() {
        let allowed = |n: &str| n == "placeholderapi.ecloud" || n == "placeholderapi.ecloud.status" || n == "placeholderapi.ecloud.list";
        let none = |_: &str| -> Vec<String> { Vec::new() };
        let c = ctx(&allowed, &[], &[], &none);
        assert_eq!(complete(&["ecloud", "s"], &c), vec!["status"]);
        assert_eq!(complete(&["ecloud", ""], &c), vec!["status", "list"]);
    }

    #[test]
    fn info_omits_empty_author_and_version() {
        struct Bare;
        impl Expansion for Bare {
            fn identifier(&self) -> &str { "bare" }
            fn author(&self) -> &str { "" }
            fn version(&self) -> &str { "" }
            fn on_request(&self, _: Option<u128>, _: &str) -> Option<String> { None }
        }
        assert_eq!(
            info_lines(&Bare),
            vec!["&7Placeholder expansion info for: &rbare", "&7Status: &aRegistered"]
        );
    }

    #[test]
    fn unregister_messages() {
        assert_eq!(unregistered("coins", true), "&aSuccessfully unregistered expansion: &fcoins");
        assert_eq!(unregistered("coins", false), "&cFailed to unregister expansion: &fcoins");
        assert_eq!(unknown_command("foo"), "&cUnknown command &7foo");
        assert_eq!(no_permission(), "&cYou do not have permission to do this!");
    }
}
