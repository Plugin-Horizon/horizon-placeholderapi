# Status (2026-11-09, written blind: no engine build yet)

Legend: **done** = written and covered by tests in `core/`; **glue** = written in `api/src/lib.rs`, type-checked
and run against a stub of Horizon's mod API, not compiled against the engine; **partly** = works with a
stated limit; **not ported** = left out on purpose (README.md says why).

Upstream: PlaceholderAPI at commit `27ec162` (shallow clone, GPL-3.0). Horizon pin: `b13dbf068`, which has
the helpers this mod calls (the `has_permission` helper and the command permission nodes among them).

## Verification

- `cd core && cargo test`: **96 tests, all green**; `cargo clippy --all-targets` clean.
- `api/` type-checked with `cargo check` against a stub `horizon_client::mod_api` typed from the pinned
  source, with chrono 0.4.45 from the offline cache; clippy clean on the same build.
- End-to-end checks of the glue on the stub Host (12 tests, not in this repo): command registration with
  permissions, `/papi parse` for `me`, a name, `--null`, the usage and the not-found replies, the console
  path, `list`, `info`, `reload` reading server.properties from the world's parent folder, the tick
  snapshot, bracket and relational forms through the API, an external expansion registered, used,
  relationally answered and unregistered, colour segments and line lists.

None of this runs the glue inside a Horizon server. The stub checks types and flow, not the engine's
behaviour.

## Parsing and registry (core)

| Feature | Status |
|---|---|
| `%identifier_params%` scanner, one pass, lower-cased identifier, parameters kept, unresolved kept as written | done |
| `{identifier_params}` bracket form (same scanner, `}` tail) | done |
| Space before the first `_` makes the head literal; no `_` makes the placeholder literal | done |
| Unterminated placeholder, `%%`, lone `%`, non-ASCII text | done |
| Relational `%rel_identifier_params%` for two players; every occurrence replaced; identifier with no `_` skipped | done |
| `containsPlaceholders` / bracket check | done |
| Registry: lower-cased unique identifiers, identifier validation (no `%`, `{`, `}`, `_`, whitespace), sorted list, unregister | done |
| Escape handling | **none in PlaceholderAPI**: `%%` stays, nothing to escape (README.md, "Parsing rules") |
| `&` colour codes (`Msg.color` rules) for replies, `colored_text` for mods | done |

## Built-in expansions (core, read from the snapshot)

| Feature | Status |
|---|---|
| `player`: name, displayname, uuid, x/y/z (block), world, health, ping, gamemode, level | done (formats are ours, README.md) |
| `server`: name, online, max_players, motd, tps (two decimals), `time_<pattern>` | done (formats are ours, README.md) |
| Time patterns: y, M, d, H, h, m, s, S, a, E, quotes; other letters invalid | done |
| Snapshot: TPS = min(tick rate, 1000 / ms per tick); server.properties `max-players`, `motd` | done |
| `player_health` decimal style, `player_gamemode` Bukkit names | done |

## Commands and glue

| Feature | Status |
|---|---|
| `/papi parse <player> <text>` (`me`, `--null`; the message keeps its inner spaces; output sent raw) | glue |
| `/papi list` (ten per line), `/papi info <id>`, `/papi reload` | glue |
| Permission nodes `placeholderapi.parse`, `.list`, `.info` and `.reload`, operator level 1 without a permissions mod (PlaceholderAPI's `op` default) | glue |
| `papi` and `placeholderapi` roots | glue |
| Snapshot every tick (players, tick rate, ms per tick); server.properties on the first tick with a world and on reload | glue |
| Public API: `set_placeholders`, `set_placeholders_all`, `set_bracket_placeholders`, `set_relational_placeholders`, `register`, `unregister`, `is_registered`, `identifiers`, `contains_placeholders`, `colored_text` | glue |
| eCloud download and cloud list/update | **not ported** (no network, out of scope) |
| `/papi parserel`, `bcparse`, `cmdparse`, `register`, `unregister`, `version`, `help`, `ecloud` | **not ported** (command set of the brief; the API functions cover parserel and register) |
| Expansions loaded from the PlaceholderAPI jar folder | **not ported** (expansions are registered in code) |
| Config file (`config.yml` keys) | **not ported**: the keys used by the port are read from the server |

## Test first, once a build is possible

1. Build `api/` against the pinned engine (`horizon-mixins apply`, then the launcher's native build). Fix
   what the stub may have wrong. The UNVERIFIED spots, all in `api/src/lib.rs`:
   - `Hooks` method signatures: `register_commands`, `perform_command`, `perform_console_command`, `tick`,
     and the event fields `event.commands`, `event.player`, `event.command`.
   - `Host` method signatures: `server_players() -> Vec<NativeServerPlayer>`, `tick_stats() -> (f32, f64)`,
     `world_dir() -> Option<PathBuf>`, `tell(uuid, Text)`.
   - `NativeCommand::op(usage, level).permission(node)` and the usage kinds `player_name`, `text`, `word`.
   - `Text::colored(String, u32)`, `Text::empty()`, `append`.
2. `/papi parse me %player_name% %player_health% &a` in a test world; a second player's name with `parse`;
   `--null`; `/papi list`, `/papi info player`, `/papi reload` after editing `max-players` (restart not needed).
3. A second mod registering an expansion: `%coins_balance%` in a tab list, the same mod unloaded and the
   placeholder then left as written.
4. Clock: `%server_time_HH:mm%` against the server's `date` output in the same time zone.
