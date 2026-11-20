Parity: 88% now / 88% with our PRs / 90% with engine changes / 90% with new SDK (101/115 items now, 103/115 with the wildcard branch; method: audit + PARITY.md estimate; items = sub commands, nodes, config keys, messages, features)

## Round 100 (blind, not compiled; owner target 100 %)
Walked the whole original (router, every command class, plugin.yml nodes, config.yml, ServerLoadEventListener, the
parse/tab code) against the port. Commits on `hmod-format`:
- `2477b7f` info: `&7Status: &r&aRegistered` as the original writes it (the label was grey).
- `b7e13be` `/papi version` and help show `2.12.4-DEV`, the plugin version of the compared commit (was 2.12.3); `/papi register`
  names the file by its last path segment (`command::base_name`, Java `File.getName`).
- `72ffc39` wildcard nodes `placeholderapi.*` and `placeholderapi.ecloud.*` declared and granted behind feature
  `horizon_wildcard_nodes` (engine final-`*` rule, horizon issue #18818, branch `api-permission_prefix_and_wildcard`, off by default).
  `command::parents_of` lists the plugin.yml parents of a node; `holds` asks them by name (the engine does not expand a wildcard).
- `4969700` console: the two load lines PlaceholderAPI logs on server load, printed at the first tick with a world open.

Checklist round 100 (x = matches the original, ~ = differs on purpose, - = API-blocked):
- [x] `/papi` bare = version, without a permission check
- [x] version: 4 lines, author `[HelpChat]`, `2.12.4-DEV`
- [x] help: header, space line, 11 entries with the original's usage and descriptions
- [x] info: name, status, author/version (omitted when empty), `Requires plugin`, placeholder list; unknown id and usage lines
- [x] list: header with count, 10 per line, empty answer
- [x] parse / bcparse / cmdparse / parserel: usage texts, `me` and `--null` (any case), `Failed to find player` (grey for parse, white for parserel), console `me` refusal, raw vs broadcast output, message keeps its inner spaces
- [x] cmdparse: a player runs the line as the player (own level), console as console
- [x] reload: re-reads config and server.properties, counts the registered hooks (`N placeholder hook(s) registered!`)
- [x] unregister: match by name (any case), the two result lines, usage
- [x] unknown sub command: `Unknown command <word>` (lower case)
- [x] no permission: PlaceholderAPI's line with `&c`
- [x] tab completion: sub names by node, info/unregister ids, parse targets and `%placeholder%` after `%id_`, parserel names, ecloud subs
- [x] permission nodes: all 20 non-wildcard nodes of plugin.yml, admin children, `placeholderapi.dump` not a child (as plugin.yml)
- [x] wildcards `placeholderapi.*` and `placeholderapi.ecloud.*`: feature `horizon_wildcard_nodes`, declared and checked by name
- [x] config.yml: all nine keys read, default file written when missing, `/papi reload` reads it again
- [x] console load lines (`ServerLoadEventListener`)
- [~] `/papi dump`: "not available". The original posts the server's plugin and server list to a third-party paste site; the port keeps that data on the server (rule 7)
- [~] `/papi ecloud` (download, list, update, refresh): menus and refusals as the original, the actions say "not available in this port" (no network, no expansion jars)
- [~] `/papi register`: refusal for a missing file as the original; there is no expansions folder to load from
- [~] `check_updates`, `cloud_enabled`, `cloud_sorting`, `detect_malicious_expansions`, `use_adventure_provided_replacer`: read and kept, no effect (no update check, no cloud, no folder, Paper-only replacer)
- [~] `/papi reload` keeps other mods' expansions (the original unloads all jar expansions; ours are code, see README)
- [~] `reload` count: the port counts every registered hook (the built-in `player` and `server` included), the original counts jar expansions only
- [~] ExpansionsLoaded / Register / Unregister events and the `getBracketPlaceholderPattern` family: Java plugin events and regex getters, no Horizon surface
- [-] nickname getter for `%player_nickname%` / `%player_displayname%`: no engine API (no getter anywhere; GAPS_UPSTREAM line added)
- [-] `%player_ping%`: no engine API on main, the branch or the merged list (`PlayerRef::ping` is asked by skript in GAPS_UPSTREAM)
- [-] `/papi parse <offline name>`: name lookup is `profile_by_name` (merged #17923, user cache) but not in our pin; the offline player's placeholder values have no data either, so it stays open
- [-] `cmdparse` from a command block or function: no source to run the line as; running it as console would raise the command block's rights, so it is refused on purpose
- [-] live values: the API functions take no host, so values come from the snapshot taken at each tick start (at most one tick, 50 ms, old right after a join)
- [-] eCloud and expansion jars: design choice (no network, no class loading)

Parity line: 88 % now (101/115) -> 90 % with the wildcard branch (103/115, +2 nodes). The offline lookup moves
with the pin bump, not counted until the placeholder values of offline players exist. Nothing moves for our open
upstream PRs in HOOKS_DONE.md (none touch placeholders).

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
