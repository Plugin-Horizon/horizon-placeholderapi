# horizon-placeholderapi

> **Unofficial Rust port of PlaceholderAPI for the Horizon server (https://github.com/horizon-rust/horizon). Not affiliated with or endorsed by the PlaceholderAPI authors.**

PlaceholderAPI's placeholders for [Horizon](https://github.com/horizon-rust/horizon) servers, as a
native trait mod. Other native mods (a tab list, a chat format, a scoreboard) call it to turn
`%player_name%`, `{server_tps}` and the like into values, and other mods add their own expansions
(`%coins_balance%`) by registering them.

**Derived from PlaceholderAPI (https://github.com/PlaceholderAPI/PlaceholderAPI), GPL-3.0.** The
placeholder scanner, the relational pass, the expansion registry and the `/papi` replies follow
PlaceholderAPI's source (`CharsReplacer`, `PlaceholderAPI`, `LocalExpansionManager`, the local
commands). The port is therefore a derived work under the **GNU General Public License, version 3 or
(at your option) any later version** (see LICENSE). The copyright holders are in NOTICE.

**State: the glue builds against Horizon `1453e0e61` (`cargo build`, no changes needed); the 96 rule tests in
`core/` pass. The glue has not run inside a Horizon server yet (see STATUS.md and GAPS.md).**

Upstream: PlaceholderAPI (https://github.com/PlaceholderAPI/PlaceholderAPI), commit `27ec162393ae4bf64f8acf9d991d2a0c14b1601a`, GPL-3.0. Horizon: `1453e0e614af4b2d04dd0d228a94162b5d459910`
(pinned in `horizon-mixin.toml` and `api/Cargo.toml`).

## What it does

- `%identifier_params%` and `{identifier_params}` placeholders, for a player or for no player.
- `%rel_identifier_params%` placeholders about two players.
- An expansion registry that other mods register into: an identifier, an author, a version and an
  `on_request(player, params)` answer. Unregistering is possible; identifiers are unique.
- Built-in `player` and `server` placeholders (see below), read from a snapshot the mod takes every tick.
- Commands: `/papi parse <player> <text>`, `/papi list`, `/papi info <id>`, `/papi reload`, with
  PlaceholderAPI's permission nodes.

**Not in this port:** downloading expansions from the PlaceholderAPI eCloud (`/papi ecloud`, the
cloud list and updates). The mod makes no network connection and has no cloud code. Expansions from
PlaceholderAPI's own jar folder (`plugins/PlaceholderAPI/expansions`) are not loaded either; the
other mods register theirs in code. Also not ported: `/papi parserel`, `bcparse`, `cmdparse`,
`register`, `unregister`, `version` and `help` (see STATUS.md), and Adventure component replacement
(`use_adventure_provided_replacer`), which is Paper-only.

## For other mods

Declare this mod as an optional dependency of your bundle (`[dependencies] placeholderapi = { version = ">=0", optional = true }`)
and use the functions of its `[api]` package, behind `#[cfg(feature = "placeholderapi")]`
(docs/native-mods.md, "Dependencies between trait mods"):

```rust
use placeholderapi::{Expansion, set_placeholders, set_bracket_placeholders, set_relational_placeholders};

// Replace placeholders in a tab list line for one player (None when there is no player).
let line = set_placeholders(Some(uuid), "&7[%player_gamemode%] %player_name%");

// The same with {braces}, and the relational form for two players.
let title = set_bracket_placeholders(None, "Server {server_name}");
let duel = set_relational_placeholders(one, two, "%rel_coins_richer%");

// An expansion of your own.
struct Coins;
impl Expansion for Coins {
    fn identifier(&self) -> &str { "coins" }
    fn author(&self) -> &str { "Your name" }
    fn version(&self) -> &str { "1.0" }
    fn placeholders(&self) -> Vec<String> { vec!["%coins_balance%".into()] }
    fn on_request(&self, player: Option<u128>, params: &str) -> Option<String> {
        match (player, params) {
            (Some(uuid), "balance") => Some(balance_of(uuid).to_string()),
            _ => None,               // unknown: the placeholder stays as it was written
        }
    }
}
placeholderapi::register(Box::new(Coins))?;   // an identifier is taken once
```

Also: `set_placeholders_all` (a list of lines), `colored_text` (a `&`-coded string as Horizon text,
for a mod that wants colours; the placeholder values themselves are never coloured),
`contains_placeholders`, `is_registered`, `unregister`, `identifiers`.

## Parsing rules

These are PlaceholderAPI's, as `CharsReplacer` applies them:

- The head (`%` or `{`) starts a placeholder; the next tail (`%` or `}`) ends it. Text in between is
  the placeholder. Nothing is nested.
- The identifier is what comes before the first `_`, lower-cased for the lookup. The parameters are
  everything after that `_`, with any further `_` and possibly nothing (`%player_%`).
- A space before the first `_` makes the head plain text (`%hello world 15%`). A placeholder with no
  `_` at all is plain text (`%player%`).
- An identifier nobody registered, or an expansion that answers nothing, leaves the placeholder as it
  was written, case included (`%Unknown_Thing%`).
- One pass: a value is not scanned again, and a value may contain `%` or `{`.
- **Escape:** PlaceholderAPI has no escape character. `%%` stays `%%` and a lone `%` stays. A
  placeholder whose identifier is registered and whose expansion answers is always replaced, so
  there is no way to write one that comes out literally; one that nothing answers stays as written.
- **Colour:** PlaceholderAPI's `setPlaceholders` does not colour anything; its `colorize` argument is
  ignored in the current source. `&` codes in a value or in the text come back as they were. The
  `/papi` replies are `&`-coded lines (PlaceholderAPI's `Msg.color`); the `parse` output is sent as
  it is, as Bukkit's `sendMessage` would send it.

## Built-in placeholders

PlaceholderAPI itself ships none; its eCloud expansions named `player` and `server` do, and their
exact output formats are not in the PlaceholderAPI repository. The formats here follow the names:

| Placeholder | Value |
|---|---|
| `%player_name%`, `%player_displayname%` | the name; the display name is the nickname a mod set, if any |
| `%player_uuid%` | the UUID, lower case, 8-4-4-4-12 |
| `%player_x%`, `%player_y%`, `%player_z%` | the block coordinates (rounded down) |
| `%player_world%` | `world`, `world_nether`, `world_the_end` (other dimensions: their id) |
| `%player_health%` | the health as a decimal, `20.0`, `19.5` |
| `%player_ping%` | the round-trip time in milliseconds |
| `%player_gamemode%` | `SURVIVAL`, `CREATIVE`, `ADVENTURE`, `SPECTATOR` |
| `%player_level%` | the experience level |
| `%server_name%` | `Horizon` |
| `%server_online%` | the number of players online |
| `%server_max_players%` | `max-players` from server.properties (default 20) |
| `%server_motd%` | `motd` from server.properties, `&` codes as written |
| `%server_tps%` | ticks a second, two decimals (`20.00`; lower when the ticks take over 50 ms) |
| `%server_time_<pattern>%` | the local time in a `SimpleDateFormat`-style pattern (below) |

The `player` values need a player in the context; `server` values never do. Without a player the
`player` placeholders stay as written.

**Time patterns:** `y` (`yy` for two digits), `M` (`M`, `MM`, `MMM`, `MMMM`), `d`, `H`, `h` (1 to 12),
`m`, `s`, `S` (milliseconds), `a` (AM/PM), `E` (`EEE`, `EEEE`); `'...'` is literal text. Any other
letter makes the placeholder stay as written. The time zone is the server's local time zone.

## Commands

| Command | Permission (default: operators, level 1) | Does |
|---|---|---|
| `/papi parse <player> <text>` | `placeholderapi.parse` | parses `<text>` for the player; `me` is the sender, `--null` no player |
| `/papi list` | `placeholderapi.list` | the identifiers, ten to a line |
| `/papi info <id>` | `placeholderapi.info` | the expansion's name, author, version and placeholders |
| `/papi reload` | `placeholderapi.reload` | re-reads `max-players` and `motd` from server.properties |

`placeholderapi` is a second name for the command. With a permissions mod the nodes decide; without
one, operators of level 1 and up may use the commands. `parse` only finds players who are online (see
GAPS.md). `/papi reload` does not unload or reload other mods' expansions: they are code, and they
stay until their mod unregisters them.

## Layout and build

- `api/` — the trait mod (`[api]` in `horizon-mixin.toml`): `src/lib.rs` keeps the registry and the
  snapshot in statics, maps Horizon's hooks to `/papi` and publishes the functions above.
- `api/src/papi/` — the rules, with no Horizon types: the scanner, the relational pass, the registry,
  the built-ins, time patterns, text codes and the replies.
- `core/` — builds `api/src/papi/` as a crate of its own: `cd core && cargo test`.

Build the mod as any trait mod: put this folder into the launcher's mods and build the game
(docs/native-mods.md, "Trait mods"). It needs no injections.

## Licence and origin

GPL-3.0-or-later (LICENSE). NOTICE names the PlaceholderAPI authors whose code the port follows and
the contributors who changed it for Horizon. The upstream commit the port follows is in STATUS.md.

## Contributing

Pull requests and improvements are always welcome! Open an issue or a PR; direct pushes to main are
disabled, every change goes through review. See CONTRIBUTING.md.
