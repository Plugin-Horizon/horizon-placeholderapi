//! `%rel_<identifier>_<params>%`: placeholders about two players. Port of
//! `PlaceholderAPI.setRelationalPlaceholders` (GPL-3.0-or-later, PlaceholderAPI Team).
//!
//! Matches are found in the text as it was given, as the pattern `%rel_([^%]+)%` finds them.
//! An identifier with no `_` (or an empty one) is skipped, and so is an expansion that does not
//! answer relational requests. A value replaces every occurrence of its placeholder.

use super::replacer::Lookup;

const PREFIX: &str = "%rel_";

/// The `%rel_...%` matches of `text` as (start, end) byte ranges, end exclusive, left to right.
/// A match needs at least one character between `%rel_` and the closing `%`.
fn matches(text: &str) -> Vec<(usize, usize)> {
    let mut found = Vec::new();
    let mut from = 0usize;
    while let Some(rel) = text[from..].find(PREFIX) {
        let at = from + rel;
        let body = at + PREFIX.len();
        match text[body..].find('%') {
            Some(close) if close > 0 => {
                let end = body + close + 1;
                found.push((at, end));
                from = end;
            }
            _ => {
                // No match here; the next `%rel_` may start one character later.
                from = at + 1;
            }
        }
        if from >= text.len() {
            break;
        }
    }
    found
}

/// Replaces the relational placeholders of `text` for the two players, as
/// `setRelationalPlaceholders` does.
pub fn apply(text: &str, one: u128, two: u128, lookup: Lookup<'_>) -> String {
    let mut current = text.to_owned();
    for (start, end) in matches(text) {
        let literal = &text[start..end];
        let format = &literal[PREFIX.len()..literal.len() - 1];
        let Some(underscore) = format.find('_') else {
            continue;
        };
        if underscore == 0 {
            continue;
        }
        let identifier = format[..underscore].to_lowercase();
        let params = &format[underscore + 1..];
        let Some(expansion) = lookup(&identifier) else {
            continue;
        };
        if !expansion.is_relational() {
            continue;
        }
        if let Some(value) = expansion.on_relational(one, two, params) {
            current = current.replace(literal, &value);
        }
    }
    current
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::expansion::Expansion;
    use super::super::testing::{lookup_of, FnExpansion};
    use std::sync::Arc;

    fn pair(one: u128, two: u128, params: &str) -> Option<String> {
        Some(format!("{one}:{two}:{params}"))
    }

    fn relational(id: &'static str) -> Arc<dyn Expansion> {
        Arc::new(FnExpansion {
            id,
            answer: |_, _| None,
            relational: Some(pair),
        })
    }

    fn plain(id: &'static str) -> Arc<dyn Expansion> {
        Arc::new(FnExpansion {
            id,
            answer: |_, _| Some("single".into()),
            relational: None,
        })
    }

    fn run(text: &str) -> String {
        let lookup = lookup_of(vec![relational("fac"), plain("solo")]);
        apply(text, 1, 2, &lookup)
    }

    #[test]
    fn a_relational_placeholder_gets_both_players_and_its_params() {
        assert_eq!(run("%rel_fac_x%"), "1:2:x");
    }

    #[test]
    fn the_parameters_keep_their_underscores() {
        assert_eq!(run("%rel_fac_a_b%"), "1:2:a_b");
    }

    #[test]
    fn every_occurrence_of_the_same_placeholder_is_replaced() {
        assert_eq!(run("%rel_fac_a% and %rel_fac_a%"), "1:2:a and 1:2:a");
    }

    #[test]
    fn adjacent_relational_placeholders_are_both_replaced() {
        assert_eq!(run("%rel_fac_a%%rel_fac_b%"), "1:2:a1:2:b");
    }

    #[test]
    fn an_expansion_that_is_not_relational_is_skipped() {
        assert_eq!(run("%rel_solo_x%"), "%rel_solo_x%");
    }

    #[test]
    fn an_identifier_without_a_parameter_is_skipped() {
        assert_eq!(run("%rel_fac%"), "%rel_fac%");
        assert_eq!(run("%rel__x%"), "%rel__x%");
    }

    #[test]
    fn an_empty_body_is_no_match() {
        assert_eq!(run("%rel_%"), "%rel_%");
    }

    #[test]
    fn an_unknown_identifier_is_left_alone() {
        assert_eq!(run("%rel_nope_x%"), "%rel_nope_x%");
    }

    #[test]
    fn the_identifier_is_looked_up_in_lower_case() {
        assert_eq!(run("%rel_FAC_x%"), "1:2:x");
    }

    #[test]
    fn plain_text_around_it_is_kept() {
        assert_eq!(run("you vs them: %rel_fac_x%!"), "you vs them: 1:2:x!");
    }

    #[test]
    fn an_unterminated_match_is_left_alone() {
        assert_eq!(run("%rel_fac_x"), "%rel_fac_x");
    }

    #[test]
    fn a_relational_placeholder_is_not_a_normal_one() {
        // The relational pass only; `%fac_x%` is not relational and needs the normal pass.
        let lookup = lookup_of(vec![relational("fac")]);
        assert_eq!(apply("%fac_x%", 1, 2, &lookup), "%fac_x%");
    }
}
