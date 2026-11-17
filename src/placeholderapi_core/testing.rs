//! Test doubles shared by the unit tests: expansions built from functions, and a lookup over a
//! registry, as the replacer is given it.

use std::sync::Arc;

use super::expansion::Expansion;
use super::registry::Registry;

pub type Answer = fn(Option<u128>, &str) -> Option<String>;
pub type RelationalAnswer = fn(u128, u128, &str) -> Option<String>;

/// An expansion whose answers are functions.
pub struct FnExpansion {
    pub id: &'static str,
    pub answer: Answer,
    pub relational: Option<RelationalAnswer>,
}

impl Expansion for FnExpansion {
    fn identifier(&self) -> &str {
        self.id
    }

    fn author(&self) -> &str {
        "test"
    }

    fn version(&self) -> &str {
        "1"
    }

    fn on_request(&self, player: Option<u128>, params: &str) -> Option<String> {
        (self.answer)(player, params)
    }

    fn is_relational(&self) -> bool {
        self.relational.is_some()
    }

    fn on_relational(&self, one: u128, two: u128, params: &str) -> Option<String> {
        self.relational.and_then(|answer| answer(one, two, params))
    }
}

pub fn func(id: &'static str, answer: Answer) -> Arc<dyn Expansion> {
    Arc::new(FnExpansion { id, answer, relational: None })
}

/// The mock of PlaceholderAPI's `ReplacerUnitTester`: `player` with `name`, `x`, `y`, `z` and an
/// empty argument, and nothing else.
pub fn mock_player(_player: Option<u128>, params: &str) -> Option<String> {
    let first = params.split('_').next().unwrap_or("");
    match first {
        "name" => Some("Sxtanna".into()),
        "x" => Some("10".into()),
        "y" => Some("20".into()),
        "z" => Some("30".into()),
        "" => Some("Empty Argument".into()),
        _ => None,
    }
}

/// Echoes the parameters in brackets, so a test can see exactly what the parser split off.
pub fn echo(_player: Option<u128>, params: &str) -> Option<String> {
    Some(format!("[{params}]"))
}

/// Says whether a player was in the context.
pub fn who(player: Option<u128>, _params: &str) -> Option<String> {
    Some(match player {
        Some(_) => "player".into(),
        None => "none".into(),
    })
}

/// A lookup over the given expansions, the way the registry gives them to the replacer.
pub fn lookup_of(expansions: Vec<Arc<dyn Expansion>>) -> impl Fn(&str) -> Option<Arc<dyn Expansion>> {
    let mut registry = Registry::new();
    for expansion in expansions {
        registry.register(expansion).expect("test identifiers are valid");
    }
    move |identifier: &str| registry.get(identifier)
}
