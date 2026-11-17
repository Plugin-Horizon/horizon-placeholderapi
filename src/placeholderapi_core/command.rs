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
    } else if word == "--null" {
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

pub fn usage_parse() -> String {
    "&cYou must provide a target and message: &b/papi parse &7{target} &a{message}".to_owned()
}

pub fn no_such_player(name: &str) -> String {
    format!("&cFailed to find player: &7{name}")
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

pub fn usage_root() -> String {
    "&cUsage: &b/papi parse &7<player> <text> &b| list &b| info &7<id> &b| reload".to_owned()
}

pub fn reloaded(hooks: usize) -> String {
    format!("&7PlaceholderAPI reloaded: &f{hooks}&7 placeholder hook(s) active.")
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
        format!("&7Author: &r{}", expansion.author()),
        format!("&7Version: &r{}", expansion.version()),
    ];
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
        assert_eq!(target_of("--NULL"), Target::Name("--NULL".into()));
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
}
