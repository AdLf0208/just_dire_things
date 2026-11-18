# Just Dire Things — port notes, slice 1 (the Simple Coal Generator)

Ported from **Just Dire Things 1.5.7** (`Direwolf20-MC/JustDireThings`, **MIT**),
the exact ATM10 jar `justdirethings-1.5.7.jar` (SHA-1
`f0327d7d2f389020063849200a4128ac9dcb52da`, SHA-256
`6e5f7dd7091cc271fee66b0df62bde2250e8b52397b51dd911f79c088eb22d2f`), kept at
`ports/_shared/licence-cache/justdirethings-1.5.7.jar`. Upstream behaviour was
read from tag `v1.5.7` (commit `8390e0effa33cd0e0995265245baf5629b61ef22`) —
analysis only; this is a clean-room Rust rewrite (`FINDINGS.md` lists the files
read and the numbers extracted).

**Licence: MIT → rank 0 (permissive).** Local-only until the maintainer approves
publishing (ARCHITECTURE.md §8). Ported now because `precheck-mod.py
justdirethings` is `DISPATCHABLE` (jar licence MIT, rank 0, core, no unported
dependency, no worldgen) and the mod is the pack's rank-0 energy base.

**Class + tier:** `content` · **`deep`** (`ports/ledger.csv`, mirrored from
`ports/_shared/source-map/atm10-sources.csv`). The tier is declared `deep`
because the mod as a whole has entities, renderers, GUIs and worldgen; **this
slice** is the `machines` shape the pin can express for one family — a block
entity, a declared FE buffer (API 142), a container and a server tick — exactly
what `horizon/docs/modpacks.md` calls machines. Nothing here is a data-only
carry, and nothing here needs a renderer.

**Format (2026-11-18):** this port is now one flat **SDK package** (`.hmod`) —
`horizon-mod.toml` + `Cargo.toml` + `src/lib.rs` + `src/runtime.rs` + `assets/` +
`data/` — after Horizon's mod-API merge (#16941, launcher >= 0.2.70) retired
`.hnmod`; see `## Migration to the SDK format` below and `VALIDATION.md`. The
sections from "Behaviour" to "Embedded content" describe the slice's behaviour;
the hook-injection table and the AE2 embedding paragraph are the **retired**
shape, kept as history.

## Slice chosen, and why

**The Simple Coal Generator (`justdirethings:generatort1`) and the four coals it
burns (`coal_t1..t4`).** It is the smallest machine family in the jar that the pin
expresses end to end: one block, one insert-only FE buffer, one fuel slot, one
tick, no neighbour reads, no GUI needed to prove it, no fluid and no entity. It
is also the pack's first power source, so later slices (the Energy Transmitter,
the Fuel Generator, the pocket generator, item automation) build on it. The jar
inventory behind the choice, and the other families it ships, are in
`FINDINGS.md`.

## Migration to the SDK format

The registry closed PR **#591** on 2026-11-18 with one reason — *republish this
as a native `.hmod` with the SDK from the next Nightly* — so this is a
**resubmission**, not an edit: the manifest, the glue and the packaging are new,
and everything the slice proved about the machine is unchanged.

**SDK and pin:** build `0b14f8acfd97fa2c750e42795249dcc981178ca4` (2026-11-18),
mixins `e59da9094f0c2fdda6c627fe9eff22dec6a906f0`, launcher 0.2.78, 749 surface
items. `horizon-mod.toml`'s `[native]` block is the only pin.

### Files created

| File | What it is |
|---|---|
| `horizon-mod.toml` | the one manifest: `id = "justdirethings"`, `version = "0.1.0"` (the port's, not upstream's 1.5.7), `api = "0.2"`, `side = "both"`, `capabilities = ["registries"]`, `[native]` = game 26.3 + the two commits |
| `Cargo.toml` | one dependency, `horizon-mod-api` from `.horizon/sdk/`; the crate is the mod |
| `src/lib.rs` | the one `#[mod_api::horizon_mod] impl Hooks` (register, the block entity Part, the commands) — the old `support.rs` and its eight `native_*` helper files collapsed into it |
| `src/runtime.rs` | the old `mixins/justdirethings/runtime.rs`, **byte-identical** (`sha256 5e381b0b099569c38066ff4ac51d006963dc90d5fce45f8c4c7749736399db5e`), tests included |
| `assets/justdirethings/**`, `data/**` | the old `content/assets` + `content/data`, unchanged |
| `tools/package.sh` | `cargo horizon-mod build` → copy to the root → `check` → `sha256sum` (replaces `package.py`/`package.sh`) |
| `AGENTS.md` (rewritten), `README.md` §Build, `VALIDATION.md` §The SDK ladder, `CONTENT.md` intro | the port's own docs for the new shape |

### Files deleted, and why

| Deleted | Why |
|---|---|
| `mixins/justdirethings/` (`horizon-mixin.toml`, `runtime.rs`, `support.rs`, `block_entity.rs`, `ticks.rs`, `container.rs`, `energy.rs`, `tick.rs`, `register.rs`, `perform.rs`, `embedded_content.rs`, `data/justdirethings-content.hmod`) | the plan/injection schema and per-hook helpers are not part of any new package; the one behaviour still needed is a `BlockEntityPart` in `lib.rs` |
| `content/` (`horizon-mod.toml`, `assets/`, `data/`, `justdirethings.hmod`, `target/`) | the content moved to the package root; a second manifest and a second package would be a duplicate |
| `justdirethings.hnmod` | `.hnmod` is retired; `justdirethings.hmod` replaces it |
| `compatibility.json` | it recorded the old pin for the staging ladder; `[native]` is the pin now |
| `rust-toolchain.toml` | the SDK pins its own toolchain (`sdk.json` → rust 1.99.1); a port-level toolchain file is not part of the format |
| `tools/{horizon.py,package.py,test_stage.py,test_template.py,add-notices.py,hmod-load.sh}` | they drive the retired staged `.hnmod` ladder (`git archive` of `1d63e0a`, mixins plan/apply, `.hnmod` packing); `tools/e2e.sh` stays as the scenario record |
| `tools/audit/` (the `native-mod-source-audit` crate) | it audited the injected `.rs` files, which no longer exist — and a nested cargo package makes `cargo horizon-mod` refuse the port: `error: unexpected package path: tools/audit/Cargo.toml` |

### Hook mapping — every old injection, and where it went

| Retired hook (API) | SDK equivalent used here |
|---|---|
| `block_entity` (15) — "the generator has a block entity" | `register`: `r.block_entity_type("generatort1").blocks(&["justdirethings:generatort1"]).build()`. No `BlockEntityPart::block_entity` is needed: a declared type starts every entity with an empty compound, which is exactly what the old helper answered |
| `block_entity_ticks` (15) | `BlockEntityPart::block_entity_ticks` sets `event.ticks = true` |
| `block_entity_container` (16) | `DefaultCapabilitiesRegistrar::block_container(&block, ModContainer { slots: 1, every face reaches slot 0, inputs/outputs: [0], menu: None })` |
| `block_entity_energy` (142) | `DefaultCapabilitiesRegistrar::block_energy(&block, ModEnergyBuffer { capacity 1_000_000, max_insert 0, max_extract = capacity, insert_faces false, extract_faces true })` |
| `block_entity_tick` (17) | `BlockEntityPart::block_entity_tick(&self, host, event)`: `event.entity.nbt` and `event.entity.items` are the old `ModBlockEntity` fields; `*event.reply = BlockEntityTickReply::Unchanged` when nothing burnt and nothing was pulled — the old `mark_modified` guard, expressed as the payload reply instead of a chunk write |
| `register_commands` (8) | `Hooks::register_commands` pushes `Command::new("justdirethings info <pos:block_pos>")` and `Command::new("justdirethings set <amount:integer> <pos:block_pos>")` |
| `perform_command` (8) | `Hooks::run_mod_command` + `CommandInvocation::{literals, argument}`, and `host.commands().feedback(Text::literal(..))` for the line; `Flow::Cancel` marks it handled |
| `game.res.providers().burn_time/speed` | `host.item_burn_time(item)` for the burn time (the engine resolves the component/data map); the **speed multiplier** from the item's declared `minecraft:cooking_fuel` component via `host.item_default_component` — see the gap list |
| `game.world.block_entities` / `native_with_mod_block_entity` (the commands) | `host.block_entity(level, pos)` → `entity.data(host)` (read), `entity.item_slot(host, 0)` (the fuel stack), `entity.merge_data(host, data)` (the `set` write, which is `/data merge block`'s path: load, `setChanged`, `sendBlockUpdated`) |
| `content_root` HEAD injection + `native_data!` (AE2 pattern) | **not needed**: one flat package carries `assets/` and `data/` itself, which is the point of the merge. `embedded_content.rs` and the content copy inside `data/` are gone |
| module `crate::game::justdirethings` (runtime) | `mod runtime;` — same file, now `crate::runtime` |
| module `crate::game::justdirethings_state` (support) | private functions in `src/lib.rs` |

### Manifest mapping

| `horizon-mixin.toml` / `horizon-native.json` | `horizon-mod.toml` |
|---|---|
| `[mod] id/version/priority = 100` | `id = "justdirethings"`, `version = "0.1.0"` (priority has no counterpart: ordering is per-hook `#[listener(priority = ..)]`) |
| `[compatibility] horizon_revision` | `[native] horizon` |
| `horizon-native.json` `game`/`mixins`/`schema` | `[native] game`/`mixins`; the schema is the SDK's (`api = "0.2"`) |
| content manifest (`id`/`name`/`version = 1.5.7`/`side`/`capabilities`) | the same fields in the one manifest, `version = 0.1.0` |
| `[[module]]` ×2 | `src/lib.rs` + `src/runtime.rs` |
| `[[injection]]` ×7 + the raw `content_root` target | `register` declarations, the `BlockEntityPart`, the two command hooks |

### Build and check

```sh
cargo test                                     # runtime: 11 passed
cargo fmt -- --check                           # clean (not --all: the SDK's own build.rs)
cargo clippy --all-targets -- -D warnings       # 0 warnings
cargo horizon-mod build                        # target/horizon-mod/justdirethings.hmod
cargo horizon-mod check justdirethings.hmod     # native-mod, format native-1/0b14f8a…/e59da90…
cargo horizon-mod check --uses justdirethings.hmod
cargo horizon-mod fits justdirethings.hmod      # "0b14f8a  made for"
tools/package.sh                               # build + deliver to the root + hash
```

### Traps (each cost a run here)

1. **`cargo horizon-mod build` does not prove the code compiles.** It packs; the
   first run printed `Built ./target/horizon-mod/justdirethings.hmod` while
   `cargo test` then failed with five errors. The compile gate is `cargo test`
   (or `cargo clippy`).
2. **A nested cargo package is refused.** `tools/audit/Cargo.toml` gave
   `error: unexpected package path: tools/audit/Cargo.toml` before anything
   built. It must not come back.
3. **`cargo fmt --all -- --check` fails on the fetched SDK**, not on this port:
   `.horizon/sdk/api/crates/*/build.rs` are the SDK's own files. Use
   `cargo fmt -- --check`.
4. **Most host reads are handle methods, not `host.*`.** `host.level_block(..)`,
   `host.level_game_time(..)` and `host.dimension_key_id(..)` do not exist on
   `dyn ServerHost`; they are `level.block(host, pos)`, `level.game_time(host)`
   and `dimension.id(host)`. `cargo horizon-mod hooks <text>` searches handles
   and methods too, offline.
5. **A `BlockEntityPart` must also be a `BlockPart`** (`trait BlockEntityPart:
   BlockPart`): an empty `#[horizon_part] impl BlockPart` is required even when
   the block itself adds nothing.
6. **`Compound` is keyed by `JavaString`, which the API does not export.** Reads
   compare UTF-16 units (`name.0 == key.encode_utf16().collect::<Vec<u16>>()`);
   writes use `nbt.insert(key.into(), Tag::Int(value))`.
7. **`Command` is `#[non_exhaustive]` and not struct-constructible** — build it
   with `Command::new(usage)` / `Command::op(usage, level)`.
8. **The delivered `.hmod` may live in the port root.** Verified: with
   `justdirethings.hmod` present, `cargo horizon-mod build` packs 38 entries and
   the same SHA-256 (the root file is not packed into itself).
9. **The SDK is fetched per clone** into `.horizon/sdk/` by the first
   `cargo horizon-mod` command (network), and is never packed; `.gitignore` now
   ignores `/.horizon`.
10. **The in-game leg could not run.** `horizon-launcher` is not installed here
    and the published shared binaries are for the retired `1d63e0a5` pin, so
    `tools/e2e.sh` (which needs a pin-matched `horizon-ctl` and game) was not
    executed against `0b14f8a`. The scenario and its PASS lines stay in
    §In-game transcript; re-run it, with a ctl for the SDK build, before the next
    status claim that rests on gameplay.

## Behaviour (upstream → this port)

| | Upstream | This port |
|---|---|---|
| buffer | `EnergyStorageNoReceive` of `GENERATOR_T1_MAX_FE` = **1,000,000 FE**; `canReceive()` false | same, `block_entity_energy` (API 142): capacity 1,000,000, `max_insert` 0, `max_extract` 1,000,000, every face drains, none fills |
| fuel value | `stack.getBurnTime(SMELTING)` and the item's own speed multiplier (JDT coals: code multiplier 2/4/8/16; NeoForge `furnace_fuels` data map) | the engine's `minecraft:cooking_fuel` (`Providers::burn_time` / `Providers::speed`); the JDT values are carried as the items' components, so vanilla and modded fuels both work |
| burn | `feRemaining = burnTime × 15`; `maxBurn = floor(burnTime / (4 × fuelMultiplier))`; `fePerTick = 15 × 4 × fuelMultiplier` | same (`runtime::Generator`), unit-tested |
| rate | config `GENERATOR_T1_FE_PER_FUEL_TICK` 15, `GENERATOR_T1_BURN_SPEED_MULTIPLIER` 4 | same constants |
| coal | 1,600 ticks × 1.0 → 60 FE/t, 400 ticks, **24,000 FE** | same, pinned by a unit test and measured in-game |
| Primal Coal | 4,800 ticks × 2.0 → 120 FE/t, 600 ticks, **72,000 FE** | same, measured in-game from the carried item's component |
| full buffer | fuel is not burnt (`insertEnergy(fePerTick, simulate) == 0`); a burn in progress pauses | same; a full buffer holds its burn still, then resumes when drained |
| fuel boundary | when a burn ends mid-tick the next item loads and the tick is topped up | same, unit-tested (`output_continues_across_a_fuel_boundary_in_one_tick`) |
| slot | `GeneratorItemHandler`: one slot, any item with burn time > 0 | one container slot; the tick refuses what does not burn (no per-slot `may_place` for native menus — see the gaps) |
| consumption | one item per burn; a crafting remainder replaces the last item | same, except a stack keeps its remaining items (deviation 4 in `FINDINGS.md`) |
| redstone | `RedstoneMode.IGNORED` default, so always on | always on; the mode GUI is a gap |
| saved state | `energy`, `feRemaining`, `burnRemaining`, `maxBurn`, `fuelBurnMultiplier` | the same keys in the block entity's compound |
| close | right-click opens `GeneratorT1Screen` (FE bar + burn bar) | no GUI; the harness command reads the same fields |

### Hooks — the retired `.hnmod` injections (historical; the mapping is in §Migration)

| Hook (API) | Helper | What it does |
|---|---|---|
| `block_entity` (15) | `block_entity.rs` | the generator gets a block entity |
| `block_entity_ticks` (15) | `ticks.rs` | it ticks on the server |
| `block_entity_container` (16) | `container.rs` | the one fuel slot, every face |
| `block_entity_energy` (142) | `energy.rs` | the insert-only 1,000,000 FE buffer |
| `block_entity_tick` (17) | `tick.rs` | burn, store, persist |
| `register_commands` (8) | `register.rs` | `/justdirethings info\|set` |
| `perform_command` (8) | `perform.rs` | their bodies |
| `content_root` (raw target, `native_data!` API 41) | `embedded_content.rs` | writes the bundled `.hmod` out and appends it to the dev-mod list |

7 hook injections + 1 raw `content_root` target, 2 modules, **0 unstamped hooks**.

### Commands (harness controls)

- `/justdirethings info <x> <y> <z>` — `x/1000000 FE`, `burning`, `fe` left of the
  burn's worth, `burn` ticks left / `maxBurn`, `multiplier`, the fuel stack, and
  `t=` (world game time, so a reply can be told from a stale one).
- `/justdirethings set <amount> <x> <y> <z>` — sets the buffer, clamped to the
  capacity. A harness control (precedent `/powah set`, `/charginggadgets
  energy`), not upstream gameplay; upstream's own controls are its GUI.

Typed through `ctl chat "/justdirethings …"`, because `ctl command` runs the
console path and bypasses `perform_command`.

## Content (`.hmod`)

`content/` is assembled by [`tools/assemble_content.py`](tools/assemble_content.py);
[`CONTENT.md`](CONTENT.md) lists every carried and every written file and why.
30 entries: the blockstate, the block/item models, three block textures, four
coal item models + textures, 26.3 item definitions, the `en_us` subset, the
block declaration (`minecraft:iron_block` template, hardness 2.0), the four
coal declarations with their `minecraft:cooking_fuel` values, the loot table (the
jar's, already in 26.3's singular-condition shape), the recipe (it does not match
yet: `ferricore_ingot` is a later slice), the creative tab and the pickaxe tag.

The built file is also copied to
`mixins/justdirethings/data/justdirethings-content.hmod`, so the `.hnmod` carries
it (see the next section).

## Embedded content (AE2 pattern) — retired with `.hnmod`

The registry stores **one file per project** (playbook §1.0), so a port with both
halves could not be listed faithfully: an `.hnmod` is code-only and an `.hmod`
is content. This bundle now carries its content, exactly as the merged AE2 entry
does:

- **`mixins/justdirethings/data/justdirethings-content.hmod`** is the built
  `content/justdirethings.hmod`, byte for byte (10,975 B,
  `fd49d5f5…b93ff`). `tools/package.py` packs the bundle's top-level
  `.rs`/`.toml` plus everything under `data/` as `mixins/<id>/data/…` (so
  `native_data!` compiles it in), with fixed zip timestamps: two consecutive
  runs give the same SHA-256. When the plan declares a `content_root` injection
  it **requires** `content/<id>.hmod` and a byte-identical `data/<id>-content.hmod`
  and refuses the bundle otherwise, so a rebuilt content mod can never ship stale
  inside it. `tools/package.sh` re-syncs the copy after every `.hmod` rebuild.
- **`mixins/justdirethings/embedded_content.rs`** with the `embedded-content`
  `[[injection]]` (`target = "horizon_client::game::mods::content_root"`,
  `kind = "HEAD"`, `cancellable = true`, bindings `root`/`enabled`/`dev`)
  writes the file to `.horizon-cache/justdirethings-embedded-content.hmod` when
  the copy on disk differs, appends it to the dev-mod list and returns its
  `content_root`. The helper calls `content_root` once more and stops at its own
  `dev.contains(&file)` guard, so the second call runs the vanilla body — the
  same one-level recursion AE2 uses. An unwritable cache logs a warning and
  `Continue`s (no content) instead of failing the game.
- The file goes to `std::env::current_dir()/.horizon-cache/`; the game only
  `set_current_dir`s for `--game-dir`, which this port's e2e never passes, so the
  cache is the e2e scratch root. The control leg blocks **that** path (a plain
  file), not a guessed one, so it cannot pass vacuously.

Verified in game with **only the bundle** (no `--dev-mod`): the whole generator
scenario passes, the file the helper wrote is byte-identical to
`content/justdirethings.hmod`, and the control (same binary, `.horizon-cache`
unusable) leaves both ids unknown — `justdirethings:coal_t1` never arrives in
the inventory and `setblock … justdirethings:generatort1` leaves
`minecraft:air`. `SELFTEST=1` still fails the flipped no-fuel assertion.

This makes the `.hnmod` the single listable artifact of this port; it does not by
itself publish it (licence approval is still required, ARCHITECTURE.md §8).

Skills for this slice: superpowers/using-superpowers, verification-before-completion;
BeautifulRust/rust-best-practices (fmt, clippy `-D warnings`, tests).

### In-game transcript (quoted from `e2e-embedded.log`, one file only)

```
== control: the same game with no embedded content file ==
  without the embedded file: no coal item, the block stayed minecraft:air
  launch: no --dev-mod; the bundle carries its own content
  embedded content: 10975 B at /tmp/opencode/justdirethings-home.dUm59Z/.horizon-cache/justdirethings-embedded-content.hmod, identical to content/justdirethings.hmod
  block(0,100,0) = justdirethings:generatort1
  block(4,100,0) = justdirethings:generatort1
  Simple Coal Generator at 0,100,0: 660/1000000 FE (burning true, fe 23340 of 24000, burn 390/400, multiplier 1, fuel minecraft:coal x4, t=227)
  Simple Coal Generator at 4,100,0: 1320/1000000 FE (burning true, fe 70680 of 72000, burn 590/600, multiplier 2, fuel justdirethings:coal_t1 x4, t=245)
PASS: an unfuelled generator made no FE, coal burned at 60 FE/t for 24,000 FE, the carried Primal Coal burned at 120 FE/t for 72,000 FE, a full buffer burned nothing until drained, and the capacity clamped a larger set
EXIT:0
```

## Pin facts that shaped the slice

1. **A block's own buffer and own slot are enough.** API 142 keeps the amount as
   the int `energy` in the block entity's compound, so a tick can read and write
   it without touching a neighbour.
2. **Block-to-block transfer does exist at this pin** (`native_move_energy`,
   `game/machines/native_energy.rs`, used by `powah` at the same pin) — the
   brief's and ARCHITECTURE §3's "block-to-block FE does not exist" row is stale
   for API 142. It changes nothing here: upstream's generator is insert-only and
   its only sink in this family is the Energy Transmitter (slice 2), so this
   slice still stores into its own buffer. Recorded rather than exploited.
3. **Fuel values are item components in 26.3**, not a NeoForge data map, so the
   four coals carry their own `cooking_fuel` and no code-side table exists.
4. **Native menus have no per-slot may_place.** `menu_slot_rules` (API
   130) reaches content menu types only, so the tick refuses what it cannot burn
   instead of the slot refusing the insert.

## Gaps (the rest of the mod)

1. **Push-out to neighbours** (`providePowerAdjacent`, `fePerTick × 10` per side).
   API 142 gives `native_move_energy`; the sink is the Energy Transmitter.
2. **No GUI/menu.** Upstream opens `GeneratorT1Screen`; this slice has no menu
   type, so right-click does nothing and the fields are read by command.
3. **The Fuel Generator** (`generatorfluidt1`): 5,000,000 FE buffer, 5,000 FE/t,
   tiered fuel fluids (`fuel_tier2/3/4_fe_per_mb` 450/1300/4000). Needs
   `block_entity_fluids` tanks and a fluid capability.
4. **The Energy Transmitter** (`energytransmitter`): the network, per-side
   settings, its GUI and `native_move_energy` moves.
5. **The pocket generator** (item FE): `item_energy_buffer` (API 307) plus an
   item tick/`use` path and its screen.
6. **Item automation**: `itemcollector`, `sensort1/t2`, `clickert1/t2`,
   `droppert1/t2`, `blockplacert1/t2`, `blockbreakert1/t2`, `blockswappert1/t2`,
   `fluidcollectort1/t2`, `fluidplacert1/t2`. They need item entities
   (`native_item_entities` is read-only; `native_claim_items` exists), a fake
   player (`native_use_as_player`) and block breaking
   (`native_machine_destroy_block`).
7. **Goo and farming** (`gooblock_tier1..4`, `goosoil_tier1..4`,
   `goopatternblock`, goo fluids): random ticks and fluid goo.
8. **Time and paradox** (`time_crystal_*`, `time_fluid`, `paradoxmachine`,
   `eclipsegateblock`): accelerating another block entity has no helper
   ("tick another block entity" is a recorded API gap).
9. **Tools, armour and upgrades** (ferricore / blazegold / celestigem /
   eclipsealloy, abilities): per-stack tool stats are pending and there is no
   attribute API (flight, reach, gravity).
10. **Ores and the coal blocks** (`raw_*_ore`, `coalblock_t1..4`, `charcoal`):
    worldgen sorts last under the 2026-11-09 ruling; the blocks' fuel values are
    known (48,000 / 144,000 / 432,000 / 1,296,000 ticks) but not carried.
11. **Break behaviour**: upstream drops the slot's contents and saves the
    machine's settings into the block item; this slice loses both on break.
12. **Redstone modes** beyond the `IGNORED` default, the comparator output and
    the `isRedstoneConductor = false` property: no content property for the
    last one, no hook declared for the second.
13. **Crafting**: the carried recipe needs `justdirethings:ferricore_ingot`.
14. **A non-integer fuel speed multiplier is rounded** (upstream kept an int;
    26.3's component is a float). Vanilla and all four JDT coals are integers.

### SDK-era gaps and deviations (2026-11-18)

15. **No resolved `cooking_fuel` speed multiplier in the SDK.** `ItemsHost` has
    `item_burn_time` (the burn time, provider/data-map resolved) but nothing that
    answers the *speed multiplier* after resolution; the old API's
    `Providers::speed` has no successor. The port reads the item's **declared**
    `minecraft:cooking_fuel` component (`host.item_default_component`) and takes
    `speed_multiplier` from it, defaulting to 1.0. What that covers: its own four
    coals (they declare `4800 / 2.0` and so on) and any item that declares a
    literal number. What it does not: a fuel whose speed a data pack injects as a
    provider, which reads as 1.0 — it still yields its full `burn_time x 15` FE,
    only over more ticks at 60 FE/t. The runtime is unchanged either way.
    **Issue shape:** *"Expose the resolved `minecraft:cooking_fuel` as one read
    (burn time and speed multiplier), instead of only `item_burn_time`."*
16. **The in-game leg was not re-run at the SDK build.** The package is checked,
    fits and reproducible, and the runtime's 11 tests pass, but no
    `horizon-ctl`/game exists here for `0b14f8a` (the shared pair is the retired
    `1d63e0a5`, and `horizon-launcher` is not installed), so the slice's gameplay
    rests on the retired bundle's proven run (§In-game transcript) plus the code
    mapping above. Re-run `tools/e2e.sh` with a ctl for the SDK build before any
    claim that needs a fresh in-game transcript.
17. **No `block_entity` initial-data hook.** Not a gap in behaviour: a declared
    block entity type starts with an empty compound, so the old helper's
    `Some(Compound::new())` has no successor and needs none.

Skills for this migration: superpowers/using-superpowers, brainstorming, writing-plans, systematic-debugging, test-driven-development, verification-before-completion; BeautifulRust/rust-best-practices

## Verification

**Current ladder at the SDK build (`0b14f8a`): `VALIDATION.md` §The SDK ladder** —
`cargo test` 11 passed, clippy `-D warnings` clean, `cargo horizon-mod build`
reproducible (SHA-256 `4d1c5f14…58005`, 31,298 B, 38 entries), `cargo horizon-mod
check` and `fits` green. Evidence: `../_shared/evidence/justdirethings/sdk-*.log`.
What follows is the record of the retired `.hnmod` build on which the slice's
gameplay was proved.

Pin **`1d63e0a5a37a7a708eb0b32b49bf0dfeb28ed8a9`**, mixins
**`5a64af938ec86b00d53d16a408aabc0038fb3e57`** (schema 6); checkouts
`ports/_shared/pins/horizon-1d63e0a` and `.../horizon-mixins-5a64af9`. Every
build/test/e2e ran through `ports/tools/build-slot.sh` with `CARGO_INCREMENTAL=0`,
never a bare cargo and never `--target-dir`; the e2e used the port's own fast game
and the pin-matched shared `horizon-ctl`
(`ports/_shared/bin/horizon-1d63e0a5/horizon-ctl`).

Skills: superpowers/using-superpowers, brainstorming, writing-plans, test-driven-development, systematic-debugging, verification-before-completion; BeautifulRust/rust-best-practices, rust-port

### Ladder

| Step | Command | Result |
|---|---|---|
| format | `cargo fmt --check` | clean |
| runtime tests | `cargo test --locked --offline` | **11 passed**, 0 failed; TDD: the same tests were watched failing first (**6 failed, 5 passed** on the stubs; the 5 are constant/default/emptiness assertions) |
| clippy | `cargo clippy --all-targets --locked --offline -- -D warnings` | clean |
| staged check | `python3 tools/horizon.py check --horizon …/pins/horizon-1d63e0a` | `cargo_exit: 0`, **0 warnings** |
| embedded tests | `python3 tools/horizon.py test …` | **11 passed; 0 failed; 1240 filtered out** |
| fast build | `python3 tools/horizon.py build …` | `cargo_exit: 0`, **0 warnings** |
| e2e, one file only (no `--dev-mod`) | `build-slot.sh e2e --job jdt-embed-e2e --port ports/justdirethings -- tools/e2e.sh` | **PASS**, `EXIT:0`; the control leg (same binary, `.horizon-cache` a plain file) leaves the ids unknown |
| embed fail-capability | `SELFTEST=1 tools/e2e.sh` | **EXIT:1** at the flipped no-fuel assertion |
| content `.hmod` | `tools/package.sh` (`cargo horizon-mod build` + `check`) | **10,975 B**, SHA-256 `fd49d5f5…b93ff`, **30 entries**, content only, API 0.2, `side = both` |
| native `.hnmod` (content embedded) | `tools/package.sh` | **18,322 B**, SHA-256 `31d5f91f…cce7`, **13 entries** incl. `mixins/justdirethings/data/justdirethings-content.hmod` (10,975 B, the built `.hmod`) |
| `cargo horizon-mod check --kind native-mod` | `check-native.log` | exit 0, `justdirethings 0.1.0 (native-mod, format native-1/1d63e0a5…/5a64af93…, for 26.3)` |
| reproducible | `tools/package.py` run twice | same bytes (`REPRODUCIBLE`, `31d5f91f…`), and the `.hmod` rebuilt on a second machine kept `fd49d5f5…` |
| load-check | `tools/hmod-load.sh` (shared unmodded 1d63e0a5 game) | **PASS** with `--dev-mod <the .hmod>` (block + 5 items); without it the same ids are **unknown** (`Unknown block type 'justdirethings:generatort1'`, block stays bedrock) |
| e2e-log checker | `python3 ports/tools/check-e2e-logs.py justdirethings --strict` | `logs=4 ok=4 missing-exit=0 bad-exit=0`, exit 0 |
| upstream gate | `python3 ports/tools/upstream_ready.py justdirethings` | `READY … MIT`, exit 0 |

Evidence: `ports/_shared/evidence/justdirethings/` — the SDK migration's
`sdk-{build-1,build-2,build-3-with-root-file,build-final,build-final-2,check,
check-uses,fits,fmt,runtime-test,runtime-identical,clippy}.log`, and the retired
build's `e2e-embedded.log`,
`e2e-embedded-selftest.log`, the pre-embedding pair
(`e2e-pre-embedding{,-selftest}.log`), `hmod-load.log`, `cargo-test.log`,
`clippy.log`, `fmt*.log`, `horizon-{check,test,build}.log`, their
`*-cargo.log` output, `package.log` and `check-native.log`.

### In-game scenario (quoted from `e2e-pre-embedding.log`; the same scenario now runs from one file)

```
  arena: y=100.0 floor(0,99,3)=minecraft:stone feet(0,100,3)=minecraft:air
  block(0,100,0) = justdirethings:generatort1
  block(4,100,0) = justdirethings:generatort1
  generator: Simple Coal Generator at 0,100,0: 0/1000000 FE (burning false, fe 0 of 0, burn 0/0, multiplier 1, fuel empty, t=127)
  after 40 ticks: Simple Coal Generator at 0,100,0: 0/1000000 FE (burning false, fe 0 of 0, burn 0/0, multiplier 1, fuel empty, t=177)
  neighbour: Simple Coal Generator at 4,100,0: 0/1000000 FE (burning false, fe 0 of 0, burn 0/0, multiplier 1, fuel empty, t=185)
  has(0,100,0) minecraft:coal x4: ok
  Simple Coal Generator at 0,100,0: 600/1000000 FE (burning true, fe 23400 of 24000, burn 391/400, multiplier 1, fuel minecraft:coal x4, t=202)
  has(4,100,0) justdirethings:coal_t1 x4: ok
  Simple Coal Generator at 4,100,0: 1200/1000000 FE (burning true, fe 70800 of 72000, burn 591/600, multiplier 2, fuel justdirethings:coal_t1 x4, t=219)
  set(0,100,0) = Simple Coal Generator at 0,100,0: 1000000/1000000 FE (burning true, fe 21960 of 24000, burn 365/400, multiplier 1, fuel minecraft:coal x4, t=228)
  full buffer after 40 ticks: Simple Coal Generator at 0,100,0: 1000000/1000000 FE (burning true, fe 21960 of 24000, burn 308/400, multiplier 1, fuel minecraft:coal x4, t=285)
  after draining: Simple Coal Generator at 0,100,0: 660/1000000 FE (burning true, fe 21300 of 24000, burn 289/400, multiplier 1, fuel minecraft:coal x4, t=304)
  after 40 more ticks: Simple Coal Generator at 0,100,0: 3600/1000000 FE (burning true, fe 18360 of 24000, burn 240/400, multiplier 1, fuel minecraft:coal x4, t=353)
  set 2000000 -> Simple Coal Generator at 0,100,0: 1000000/1000000 FE (burning true, fe 17940 of 24000, burn 222/400, multiplier 1, fuel minecraft:coal x4, t=371)
PASS: an unfuelled generator made no FE, coal burned at 60 FE/t for 24,000 FE, the carried Primal Coal burned at 120 FE/t for 72,000 FE, a full buffer burned nothing until drained, and the capacity clamped a larger set
EXIT:0
```

The run's controls: a generator with **no fuel** stores 0 FE for 40 ticks and is
not burning (and its neighbour, also unfuelled, stays at 0); a **full buffer**
holds both its 1,000,000 FE and the burn's `fe`/`burn` counters for 40 ticks,
then resumes when drained (660 FE, 40 ticks later 3,600); and the **carried**
Primal Coal reports a different worth (72,000 vs 24,000) and multiplier (2 vs 1),
so the content's fuel data is what the runtime reads. `SELFTEST=1` flips the
no-fuel assertion and the run fails there (`EXIT:1`), so the scenario can fail.
The exact per-tick rate (60 FE/t) is pinned by the unit tests, not by the e2e: a
chat round trip here is far longer than a tick.

## Review repairs (PR #591)

- *"Fix the original-project link (mods/justdirethings.toml:6)"* — the entry
  links `https://github.com/Direwolf20-MC/JustDireThings` and credits
  Direwolf20; the description is rewritten complete (it previously carried
  wrapped lines as bullets and stopped mid-sentence).
- Staging is the orchestrator's (horizon-mods#686).
- `mixins/justdirethings/LICENSE` and `NOTICE.md` travel inside the artifact.

### Verification after the repairs

- Artifact: `justdirethings.hnmod`, 21,106 B, SHA-256 `a6034d0a1a6c50d9b27973b77968eeb91c0a24c4798d2cfa21c76d01d55fd368` (rebuilt twice, identical bytes; `tools/package.py`; the embedded content equals `content/justdirethings.hmod`).
- `python3 ports/tools/upstream_ready.py justdirethings` and `check-skills.py --notes ports/justdirethings`: exit 0.
