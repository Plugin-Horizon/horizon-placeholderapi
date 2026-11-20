Parity: before ~75% -> after ~84% (~97/~115 items; method: audit + PARITY.md estimate; items = sub commands, nodes, config keys, messages, features)

## Round 3 (blind, not compiled)
- Added `plugins/PlaceholderAPI/config.yml` (`placeholderapi_core/config.rs`): all nine original keys are read, the default file is
  written when missing, `/papi reload` reads it again. Public API: `boolean_true()`, `boolean_false()`, `date_format()`, `debug()`.
- `/papi info` prints `Requires plugin: ...` for an expansion that names one (`Expansion::required_plugin`, default none).
- API: `contains_bracket_placeholders`, `set_relational_placeholders_all`.
- Open: offline player lookup for `parse`, wildcard nodes (a grant of `placeholderapi.*` works through the permission provider, the
  node itself cannot be declared), eCloud and expansion jars (design choice), `dump` upload, values read live instead of per tick (no
  host in the API functions), nickname and ping getters.

# hmod port: placeholderapi
Round 1
- Fixed:
  - `src/lib.rs` `usage_node`: returns a typed `PermissionNode` (Boolean) instead of a string; new `SUBS` list.
  - `src/lib.rs` `permission_gather_nodes` (new hook): declares the five `placeholderapi.*` nodes.
  - `src/lib.rs` `register_commands`: `with_node(&node)` with the typed node.
  - `src/lib.rs` `player_command` / `run_mod_command`: `permission_bool(...).unwrap_or(false)` replaces `permission_value` and `permission_level` fallback. Any error denies.
  - `src/lib.rs` `player_snap`: `nickname` and `latency` have no Horizon main counterpart; set to `None` / `0` with `HMOD-GAP` comments.
  - Imports: `NodeDefault`, `PermissionGatherNodesEvent`, `PermissionLevel`, `PermissionNode`, `PermissionValueType` added.
- Permission nodes: `placeholderapi.parse`, `placeholderapi.list`, `placeholderapi.info`, `placeholderapi.reload`, `placeholderapi.version` -> Level 1 (`OP_LEVEL`), declared in `src/lib.rs` `permission_gather_nodes`.
- Gaps:
  - HMOD-GAP(nickname): Horizon main has `set_nickname` but no getter. `%player_nickname%` (and the nickname part of the snapshot) reads empty.
  - HMOD-GAP(latency): Horizon main has no player latency query. `%player_ping%` reads 0.
- Unsure:
  - `run_mod_command` deny check: the old code denied only an explicit deny (`== Some(false)`). The typed query has no "unset" state, so any non-grant now denies. The engine gate uses the same node, so the two checks agree.
  - Node default level 1 mirrors the old `OP_LEVEL` fallback; with no permission provider the result should match the old `level >= 1` rule.
  - `use mod_api::Host` is still imported and now unused (warning only, left alone).

Round 2 (parity audit)
- Fixed: `/papi` bare = version; unknown word answers `Unknown command <word>`; no-permission line is PlaceholderAPI's (the literal `<red>` is gone); `version` and `reload` print the original lines; `--null` is case-insensitive; the player snapshot is refreshed before each command; `info` omits an empty author or version.
- Added: `help`, `bcparse`, `cmdparse`, `parserel`, `unregister`, `register`, `dump`, `ecloud` (help menu, unknown and unavailable answers), tab completion (`tab_complete`), the missing nodes and `placeholderapi.admin` as a parent (typed nodes; checked together with the sub command's node).
- Permission nodes: all of plugin.yml except the two wildcards, default level 1 (`placeholderapi.admin`: level 4), declared in `src/lib.rs` `permission_gather_nodes` from `command::NODES`. Sub commands are open usages (level 0) gated in `player_command` / `run_mod_command`, so a parent grant works.
- Gaps: HMOD-GAP(cmdparse-source) a command block or function source cannot run the parsed command. Wildcard nodes `placeholderapi.*` / `placeholderapi.ecloud.*` cannot be typed nodes. No offline player lookup for `parse`. No eCloud, expansions folder, `config.yml` or paste upload.
- Unsure: `entity.run_command(host, line, level)` is the generated forwarder of `entity_run_command`; the `<unknown:text>` catch-all usage next to the literal sub commands; the end of `TabCompleteEvent::range` is taken as the end of the typed word.
