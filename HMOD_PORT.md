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
