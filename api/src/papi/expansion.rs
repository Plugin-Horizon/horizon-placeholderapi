//! What an expansion is: the source of one identifier's values.
//! Port of `PlaceholderExpansion` / `PlaceholderHook.onRequest` and of the `Relational` interface.

/// A source of placeholder values for one identifier (`player` for `%player_name%`).
///
/// Implementations are shared between threads and are looked up on every request, so keep
/// `on_request` cheap. A value of `None` leaves the placeholder as it was written.
pub trait Expansion: Send + Sync {
    /// The identifier, lower case by convention. The registry lower-cases it on
    /// registration. It may not be empty and may not contain `%`, `{`, `}`, `_` or whitespace.
    fn identifier(&self) -> &str;

    /// Who wrote the expansion, as `/papi info` shows it.
    fn author(&self) -> &str;

    /// The version of the expansion, as `/papi info` shows it.
    fn version(&self) -> &str;

    /// The name `/papi info` shows; the identifier by default (PlaceholderAPI's `getName`).
    fn name(&self) -> &str {
        self.identifier()
    }

    /// The placeholders the expansion offers, as `/papi info` lists them.
    fn placeholders(&self) -> Vec<String> {
        Vec::new()
    }

    /// The value for `params`: everything after the first `_` of the placeholder, empty when
    /// there is nothing after it. `player` is `None` when the text has no player in its context.
    fn on_request(&self, player: Option<u128>, params: &str) -> Option<String>;

    /// Whether the expansion answers `%rel_<identifier>_<params>%` (PlaceholderAPI's
    /// `Relational`). Only those expansions are asked by the relational pass.
    fn is_relational(&self) -> bool {
        false
    }

    /// The value for two players. Asked only when `is_relational` is true.
    fn on_relational(&self, _one: u128, _two: u128, _params: &str) -> Option<String> {
        None
    }
}
