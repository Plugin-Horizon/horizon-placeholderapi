//! The engine-free rules of PlaceholderAPI: placeholder parsing, the expansion registry, the
//! built-in `player` and `server` expansions, time formats, `/papi` replies and `&` colour codes.
//!
//! Derived from PlaceholderAPI (https://github.com/PlaceholderAPI/PlaceholderAPI),
//! Copyright (c) 2015 - 2026 PlaceholderAPI Team, GPL-3.0-or-later. The parsing follows
//! `replacer/CharsReplacer.java` and `PlaceholderAPI.java`; see README.md for what differs.
//!
//! Nothing here touches the engine. `core/` builds this directory as a crate of its own
//! (`cd core && cargo test`); the trait mod's glue is `api/src/lib.rs`.

pub mod command;
pub mod expansion;
pub mod player;
pub mod registry;
pub mod relational;
pub mod replacer;
pub mod server;
pub mod snapshot;
pub mod text;
pub mod timefmt;

#[cfg(test)]
pub mod testing;

/// The version the built-in expansions report (the mod's `[mod] version`).
pub const VERSION: &str = "0.1.0";

/// Who the built-in expansions name as their author.
pub const BUILTIN_AUTHOR: &str = "Plugin-Horizon contributors";
