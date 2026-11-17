//! The expansion registry: identifier to expansion. Port of `LocalExpansionManager` (the part that
//! keeps identifiers unique and lower case), GPL-3.0-or-later, PlaceholderAPI Team.
//!
//! Differences: PlaceholderAPI lets an internal expansion be replaced by an external one. Here an
//! identifier is taken once, by whoever registered it first, and must be unregistered before
//! someone else may use it. An identifier must not be empty or contain `%`, `{`, `}`, `_` or
//! whitespace, since the parser could never reach it.

use std::collections::BTreeMap;
use std::sync::Arc;

use super::expansion::Expansion;

/// The identifiers, sorted, each with its expansion.
#[derive(Default)]
pub struct Registry {
    expansions: BTreeMap<String, Arc<dyn Expansion>>,
}

/// Why an identifier cannot be registered, as one line of text.
pub fn check_identifier(identifier: &str) -> Result<(), String> {
    if identifier.is_empty() {
        return Err("the identifier is empty".to_owned());
    }
    if let Some(bad) = identifier
        .chars()
        .find(|c| matches!(c, '%' | '{' | '}' | '_') || c.is_whitespace())
    {
        return Err(format!("the identifier {identifier} contains {bad:?}"));
    }
    Ok(())
}

impl Registry {
    pub fn new() -> Registry {
        Registry::default()
    }

    /// Adds an expansion under its lower-cased identifier.
    pub fn register(&mut self, expansion: Arc<dyn Expansion>) -> Result<(), String> {
        let key = expansion.identifier().to_lowercase();
        check_identifier(&key)?;
        if self.expansions.contains_key(&key) {
            return Err(format!("the identifier {key} is already in use"));
        }
        self.expansions.insert(key, expansion);
        Ok(())
    }

    /// Removes the expansion of `identifier`; false when none was registered under it.
    pub fn unregister(&mut self, identifier: &str) -> bool {
        self.expansions.remove(&identifier.to_lowercase()).is_some()
    }

    /// The expansion of `identifier` (any case).
    pub fn get(&self, identifier: &str) -> Option<Arc<dyn Expansion>> {
        self.expansions.get(&identifier.to_lowercase()).cloned()
    }

    /// The identifiers in sorted order, as `/papi list` shows them.
    pub fn identifiers(&self) -> Vec<String> {
        self.expansions.keys().cloned().collect()
    }

    pub fn len(&self) -> usize {
        self.expansions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.expansions.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::testing::func;

    fn named(id: &'static str) -> Arc<dyn Expansion> {
        func(id, |_, _| None)
    }

    #[test]
    fn an_expansion_is_found_in_any_case() {
        let mut r = Registry::new();
        r.register(named("Coins")).unwrap();
        assert!(r.get("coins").is_some());
        assert!(r.get("COINS").is_some());
        assert_eq!(r.identifiers(), vec!["coins".to_owned()]);
    }

    #[test]
    fn identifiers_come_back_sorted() {
        let mut r = Registry::new();
        for id in ["server", "player", "coins"] {
            r.register(named(id)).unwrap();
        }
        assert_eq!(r.identifiers(), vec!["coins", "player", "server"]);
        assert_eq!(r.len(), 3);
    }

    #[test]
    fn an_identifier_is_taken_once() {
        let mut r = Registry::new();
        r.register(named("dup")).unwrap();
        let err = r.register(named("DUP")).unwrap_err();
        assert!(err.contains("already in use"), "{err}");
        assert_eq!(r.len(), 1);
    }

    #[test]
    fn an_identifier_can_be_taken_again_after_it_is_unregistered() {
        let mut r = Registry::new();
        r.register(named("tmp")).unwrap();
        assert!(r.unregister("TMP"));
        assert!(!r.unregister("tmp"));
        r.register(named("tmp")).unwrap();
        assert!(r.get("tmp").is_some());
    }

    #[test]
    fn identifiers_that_the_parser_could_not_reach_are_refused() {
        let r = Registry::new();
        for bad in ["", "a_b", "a%", "%a", "{a", "a}", "a b", "a\tb"] {
            assert!(check_identifier(bad).is_err(), "{bad:?} should be refused");
        }
        assert!(r.is_empty());
    }

    #[test]
    fn the_refusal_names_the_character() {
        let err = check_identifier("a_b").unwrap_err();
        assert!(err.contains("'_'"), "{err}");
        assert!(check_identifier("ok-id.1").is_ok());
    }
}
