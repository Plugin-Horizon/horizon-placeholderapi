//! The placeholder scanner. Port of PlaceholderAPI's `replacer/CharsReplacer.java`
//! (Copyright (c) 2015 - 2026 PlaceholderAPI Team, GPL-3.0-or-later), with its rules:
//!
//! - A placeholder is the text between a head and the next tail: `%` and `%` for
//!   `%identifier_params%`, `{` and `}` for `{identifier_params}`. Nothing else is special.
//! - The identifier is everything before the first `_` in it, lower-cased for the lookup; the
//!   parameters are everything after that `_` (possibly with more `_`, possibly empty).
//! - A space before the first `_` makes the head literal text and the scan goes on at the next
//!   head, so `%hello world 15%` is not a placeholder. A placeholder without any `_` is literal too.
//! - An unknown identifier, or an expansion that answers `None`, leaves the placeholder as written.
//! - One pass only: a value is never scanned again, so a value may contain `%` or `{`.
//! - There is no escape character. `%%` stays `%%`. Colour codes are not touched.

use std::sync::Arc;

use super::expansion::Expansion;

/// The lookup of an identifier (already lower case) to its expansion.
pub type Lookup<'a> = &'a dyn Fn(&str) -> Option<Arc<dyn Expansion>>;

/// Which delimiters a placeholder is written with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Closure {
    /// `{identifier_params}`
    Bracket,
    /// `%identifier_params%`
    Percent,
}

impl Closure {
    fn head(self) -> char {
        match self {
            Closure::Bracket => '{',
            Closure::Percent => '%',
        }
    }

    fn tail(self) -> char {
        match self {
            Closure::Bracket => '}',
            Closure::Percent => '%',
        }
    }
}

/// Replaces every resolvable placeholder in `text`, as `CharsReplacer.apply` does.
pub fn apply(closure: Closure, text: &str, player: Option<u128>, lookup: Lookup<'_>) -> String {
    let head = closure.head();
    let tail = closure.tail();
    let Some(mut start) = text.find(head) else {
        return text.to_owned();
    };
    let length = text.len();
    let mut out = String::with_capacity(length + length / 8);
    let mut cursor = 0usize;

    'scan: loop {
        // Plain text before the placeholder.
        if start > cursor {
            out.push_str(&text[cursor..start]);
        }

        // No closing tail: the rest is literal.
        let Some(rel) = text[start + 1..].find(tail) else {
            out.push_str(&text[start..]);
            return out;
        };
        let end = start + 1 + rel;

        // The first `_` before the closing tail; a space before it makes the head literal.
        let mut underscore: Option<usize> = None;
        let mut space_first = false;
        for (off, c) in text[start + 1..end].char_indices() {
            let at = start + 1 + off;
            if c == ' ' && underscore.is_none() {
                space_first = true;
                break;
            }
            if c == '_' && underscore.is_none() {
                underscore = Some(at);
            }
        }

        if space_first {
            out.push(head);
            cursor = start + 1;
            match text[cursor..].find(head) {
                Some(rel) => {
                    start = cursor + rel;
                    continue 'scan;
                }
                None => break 'scan,
            }
        }

        match underscore {
            None => {
                // No `_`: the whole thing, tails included, is literal.
                out.push_str(&text[start..=end]);
            }
            Some(us) => {
                let identifier = &text[start + 1..us];
                let params = &text[us + 1..end];
                let replacement = lookup(&identifier.to_lowercase())
                    .and_then(|expansion| expansion.on_request(player, params));
                match replacement {
                    Some(value) => out.push_str(&value),
                    None => {
                        out.push(head);
                        out.push_str(identifier);
                        out.push('_');
                        out.push_str(params);
                        out.push(tail);
                    }
                }
            }
        }
        cursor = end + 1;

        match text[cursor..].find(head) {
            Some(rel) => start = cursor + rel,
            None => break 'scan,
        }
    }

    if cursor < length {
        out.push_str(&text[cursor..]);
    }
    out
}

/// `PlaceholderAPI.containsPlaceholders`: a `%` and another `%` after it.
pub fn contains_percent(text: &str) -> bool {
    match text.find('%') {
        Some(first) => text[first + 1..].contains('%'),
        None => false,
    }
}

/// `PlaceholderAPI.containsBracketPlaceholders`: a `{` and a `}` after it.
pub fn contains_bracket(text: &str) -> bool {
    match text.find('{') {
        Some(open) => text[open + 1..].contains('}'),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::testing::{echo, func, lookup_of, mock_player, who};

    /// The percent scanner over the mock `player` and `echo`, and the bracket one.
    fn percent(text: &str) -> String {
        let lookup = lookup_of(vec![func("player", mock_player), func("echo", echo), func("who", who)]);
        apply(Closure::Percent, text, None, &lookup)
    }

    fn bracket(text: &str) -> String {
        let lookup = lookup_of(vec![func("player", mock_player), func("echo", echo)]);
        apply(Closure::Bracket, text, None, &lookup)
    }

    #[test]
    fn a_placeholder_with_a_parameter_is_replaced() {
        assert_eq!(percent("%player_name%"), "Sxtanna");
    }

    #[test]
    fn a_placeholder_without_an_underscore_is_literal() {
        assert_eq!(percent("%player%"), "%player%");
    }

    #[test]
    fn an_empty_parameter_is_passed_on_as_empty() {
        assert_eq!(percent("%player_%"), "Empty Argument");
    }

    #[test]
    fn text_around_a_placeholder_is_kept() {
        assert_eq!(percent("My name is %player_name%"), "My name is Sxtanna");
    }

    #[test]
    fn the_large_sentence_of_the_upstream_tests() {
        let text = "My name is %player_name% and my location is (%player_x%, %player_y%, %player_z%), \
                    this placeholder is invalid %server_name%";
        assert_eq!(
            percent(text),
            "My name is Sxtanna and my location is (10, 20, 30), this placeholder is invalid %server_name%"
        );
    }

    #[test]
    fn a_percent_sign_in_plain_text_is_not_a_placeholder() {
        assert_eq!(percent("10% and %hello world 15%"), "10% and %hello world 15%");
    }

    #[test]
    fn adjacent_placeholders_both_resolve() {
        assert_eq!(percent("%player_name%%player_x%"), "Sxtanna10");
    }

    #[test]
    fn a_lone_percent_and_a_double_percent_stay() {
        assert_eq!(percent("100%"), "100%");
        assert_eq!(percent("%%"), "%%");
        assert_eq!(percent("50%% off"), "50%% off");
    }

    #[test]
    fn a_trailing_percent_after_a_placeholder_stays() {
        assert_eq!(percent("%player_name%%"), "Sxtanna%");
    }

    #[test]
    fn only_the_identifier_is_looked_up_in_lower_case() {
        assert_eq!(percent("%PLAYER_name%"), "Sxtanna");
        assert_eq!(percent("%Player_x%"), "10");
        // The parameters keep their case: the mock matches `name`, not `NAME`.
        assert_eq!(percent("%PLAYER_NAME%"), "%PLAYER_NAME%");
    }

    #[test]
    fn an_unresolved_placeholder_keeps_the_case_it_was_written_in() {
        assert_eq!(percent("%Unknown_Thing%"), "%Unknown_Thing%");
        assert_eq!(percent("%player_nothing%"), "%player_nothing%");
    }

    #[test]
    fn a_space_before_the_underscore_makes_the_head_literal() {
        assert_eq!(percent("%player name_x%"), "%player name_x%");
        assert_eq!(percent("%player name_x% %player_name%"), "%player name_x% Sxtanna");
    }

    #[test]
    fn a_space_after_the_underscore_is_part_of_the_parameter() {
        assert_eq!(percent("%echo_a b%"), "[a b]");
    }

    #[test]
    fn everything_after_the_first_underscore_is_the_parameter() {
        assert_eq!(percent("%echo_a_b_c%"), "[a_b_c]");
        assert_eq!(percent("%echo_%"), "[]");
        assert_eq!(percent("%echo__x%"), "[_x]");
    }

    #[test]
    fn an_empty_identifier_is_not_looked_up() {
        assert_eq!(percent("%_x%"), "%_x%");
    }

    #[test]
    fn several_placeholders_in_one_text() {
        assert_eq!(percent("%player_x%,%player_y%,%player_z%"), "10,20,30");
    }

    #[test]
    fn an_unterminated_placeholder_is_literal_to_the_end() {
        assert_eq!(percent("hello %player_name"), "hello %player_name");
    }

    #[test]
    fn text_without_a_head_comes_back_unchanged() {
        assert_eq!(percent("plain text"), "plain text");
        assert_eq!(percent(""), "");
    }

    #[test]
    fn a_value_is_not_scanned_again() {
        fn back(_: Option<u128>, _: &str) -> Option<String> {
            Some("%player_name%".into())
        }
        let lookup = lookup_of(vec![func("loop", back), func("player", mock_player)]);
        assert_eq!(apply(Closure::Percent, "%loop_a%", None, &lookup), "%player_name%");
    }

    #[test]
    fn an_empty_value_is_a_value() {
        fn nothing(_: Option<u128>, _: &str) -> Option<String> {
            Some(String::new())
        }
        let lookup = lookup_of(vec![func("gone", nothing)]);
        assert_eq!(apply(Closure::Percent, "[%gone_x%]", None, &lookup), "[]");
    }

    #[test]
    fn the_player_in_context_reaches_the_expansion() {
        let lookup = lookup_of(vec![func("who", who)]);
        assert_eq!(apply(Closure::Percent, "%who_a%", Some(7), &lookup), "player");
        assert_eq!(apply(Closure::Percent, "%who_a%", None, &lookup), "none");
    }

    #[test]
    fn non_ascii_text_around_placeholders_survives() {
        assert_eq!(percent("ärger %player_name% öhm"), "ärger Sxtanna öhm");
        assert_eq!(percent("ß%echo_é%ß"), "ß[é]ß");
    }

    #[test]
    fn bracket_placeholders_resolve() {
        assert_eq!(bracket("{player_name}"), "Sxtanna");
        assert_eq!(bracket("a {player_x} b {player_y}"), "a 10 b 20");
    }

    #[test]
    fn bracket_text_is_not_a_percent_placeholder_and_the_other_way_round() {
        assert_eq!(bracket("%player_name%"), "%player_name%");
        assert_eq!(percent("{player_name}"), "{player_name}");
    }

    #[test]
    fn a_bracket_with_a_space_before_the_underscore_is_literal() {
        assert_eq!(bracket("{player name_x}"), "{player name_x}");
    }

    #[test]
    fn a_nested_opening_brace_becomes_part_of_the_identifier() {
        assert_eq!(bracket("{a{player_name}"), "{a{player_name}");
    }

    #[test]
    fn a_value_with_percent_signs_is_kept_as_it_is() {
        fn percents(_: Option<u128>, _: &str) -> Option<String> {
            Some("50%".into())
        }
        let lookup = lookup_of(vec![func("pct", percents)]);
        assert_eq!(apply(Closure::Percent, "%pct_x% left", None, &lookup), "50% left");
    }

    #[test]
    fn contains_checks_match_the_upstream_helpers() {
        assert!(contains_percent("%a_b%"));
        assert!(contains_percent("100%%"));
        assert!(!contains_percent("100%"));
        assert!(!contains_percent("none"));
        assert!(contains_bracket("{a}"));
        assert!(!contains_bracket("}{"));
        assert!(!contains_bracket("{a"));
    }
}
