# What Horizon is missing for this mod

Checked against Horizon 1453e0e61 (the pin in `horizon-mixin.toml`). Each line: what the mod would need,
which feature it serves, and what the mod does today. Nothing here blocks building; each item is a
better answer than the workaround.

## Helpers

- [ ] `Host::max_players() -> usize` and `Host::motd() -> String`. Horizon has `native_motd` (server.properties'
  motd) and a management `max_players` value, but neither is a Host helper. Needed for `%server_max_players%`
  and `%server_motd%`. Today: `api/src/lib.rs` `read_properties` reads `server.properties` from the parent of
  the world folder, on the first tick with a world and on `/papi reload`. A change of `max-players` shows only
  after a reload.
- [ ] A Host helper for a dimension's world folder name (`native_world_of_dimension`, which exists on the
  game but is not a Host helper). Needed for `%player_world%` on a world that is not one of the three defaults.
  Today: `api/src/papi/player.rs` `world_of_dimension` maps the three defaults as Horizon names them and
  returns the dimension id for anything else.
- [ ] A Horizon server name (for `%server_name%`). Today: the constant `Horizon`.
- [ ] `native_profile_by_name(name) -> Option<(u128, String)>` (the user cache). Needed for `/papi parse <offline
  player>`: PlaceholderAPI resolves offline players and gives player expansions an offline player. Today: only players online at the last tick can be named; the rest
  get the "Failed to find player" reply.
- [ ] A player's max health and food level (`native_player_max_health`, `native_player_food`), for PlaceholderAPI's
  `max_health` and `food` style placeholders. Not in the port's list; `NativeServerPlayer` has health and
  experience but not these.

## Hooks

- [ ] A player-joined and player-left event that the mod can read before the next tick. Needed only for
  exactness: the snapshot is up to one tick old (50 ms). Today: `tick` refreshes it.

## Other mods

- [ ] Nothing. The registry lives in this mod's statics, so other mods need only the bundle dependency
  (`[dependencies] placeholderapi = { version = ">=0", optional = true }`). Two notes for the main session:
  - The registry is per process, not per world, and is not saved: each start the other mods register again.
  - Unregistering an expansion does not unload its code; the owning mod must call `placeholderapi::unregister`
    when it turns off (Options > Mods).

## Not from Horizon

- The eCloud expansions' exact formats (`%player_x%` as block or exact coordinates, `%server_tps%` decimals)
  are not in the PlaceholderAPI repository and could not be checked here (no network). README.md states the
  formats the port uses.
- The eCloud download and cloud list are out of scope (no network).
