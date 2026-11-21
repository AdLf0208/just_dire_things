# Just Dire Things — port notes

**Slice 1** (the Simple Coal Generator) shipped as `0.1.0`; **slice 2** (the four
redstone modes) is `0.2.3` — see `## Slice 2 — the redstone control` below.

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

## Slice 2 — the redstone control (the four modes)

**Gap line it comes from** (`notes.md` §Gaps 12, quoted as it stood after slice 1):
*"**Redstone modes** beyond the `IGNORED` default, the comparator output and the
`isRedstoneConductor = false` property: no content property for the last one, no hook
declared for the second."*

**Why this family, and why it fits the tier.** `RedstoneControlledBE` is the **base-class
behaviour every later Just Dire Things machine inherits**, so it lays the foundation for the
`machines` shape inside the `deep` tier instead of adding a one-off. It is fully expressible at
this pin — the signal is one read (`LevelRef::has_neighbor_signal`, the same call
`actuallyadditions` makes), the state is three saved keys and the gate is one closure — and it is
provable with **vanilla blocks only** (a redstone block placed and removed), so no renderer, no
menu and no new capability. Plan: `PLAN-slice2.md`.

Two corrections the artefact forced on the old gap text:

1. **Upstream has no comparator output.** A `grep` over the 1.5.7 sources finds no
   `hasAnalogOutputSignal`/`getAnalogOutputSignal` override on `BaseMachineBlock` or any machine
   block entity, so the port must not invent one; that clause is **withdrawn** with that evidence.
2. **The mode set is four, not "beyond IGNORED"**: `MiscHelpers.RedstoneMode`
   (`IGNORED, LOW, HIGH, PULSE`, in that order, the ordinal being the saved value) and the jar's
   own GUI button textures `redstone{ignore,low,high,pulse}.png` agree.

**Upstream files read** (analysis only, not translated) — tag `v1.5.7`, commit
`8390e0effa33cd0e0995265245baf5629b61ef22`: `util/MiscHelpers.java` (`RedstoneMode`),
`util/interfacehelpers/RedstoneControlData.java`, `common/blockentities/basebe/RedstoneControlledBE.java`
(`evaluateRedstone`, `isActiveRedstone`, save/load), `common/blockentities/basebe/BaseMachineBE.java`
(`tickServer` order, `canRun`), `common/blocks/baseblocks/BaseMachineBlock.java`
(`neighborChanged` clears the latch), `common/blockentities/GeneratorT1BE.java` (`doGenerate`'s two
`isActiveRedstone()` calls).

| | Upstream | This port |
|---|---|---|
| modes | `IGNORED, LOW, HIGH, PULSE` (ordinal saved) | same, `RedstoneMode` (`runtime.rs`) |
| sample | `evaluateRedstone`: record the signal; latch one pulse on a **rising** edge in `PULSE` | `RedstoneControl::sample` |
| gate | `isActiveRedstone`, asked to **start** a burn and again for the boundary top-up | `RedstoneControl::active`, a closure so `PULSE` can *spend* its latch |
| in-progress burn | the gate is not consulted: a burn that started runs to its end | same, unit-tested (`high_mode_keeps_an_in_progress_burn…`) |
| saved state | `redstoneMode`, `pulsed`, `receivingRedstone` | the same three keys in the block entity's compound |
| control | a GUI button | `/justdirethings mode <mode:word> <pos>` (operator, in reach) — a harness control, as `/justdirethings set` is |

### The defect the in-game leg found, and the fix

The first in-game run of this slice failed at the **second** rising edge of `PULSE`
(`e2e-slice2-launcher-attempt1.log`): one edge bought a burn, a held signal bought none, but
after the block was removed and replaced nothing burned again. The cause is the tick's
`BlockEntityTickReply::Unchanged` guard: it discarded the payload whenever nothing was produced
and no fuel was pulled, which **also discarded the recorded falling edge**. `receiving` stayed
`true` in the saved state, so the next rising edge was not a rising edge.

- Rule now in `runtime::persists(produced, pulled, before, after)`: keep the payload when
  something burnt, a fuel item was pulled, **or the redstone control changed**. The glue replies
  `Unchanged` only when none of the three holds.
- Red first: `a_falling_edge_is_kept_even_when_nothing_burned` failed at the
  `persists(0, 0, before, after)` assertion with *"a falling edge with no burn must still be
  written back"* (`../_shared/evidence/justdirethings-slice2/red-falling-edge.log`), then the
  clause `before != after` made it green (26 tests).
- Proven again in game: the same scenario now reads `3000/1000000 FE … stick x2` after the second
  edge — one stick's burn per edge, two edges, two sticks.

This is a port-side bug rather than a mod defect (the Java tick writes its state unconditionally),
so it is recorded as a defect found by the harness and fixed, not as an upstream report.

### Coverage (ruling 2026-11-20)

| Family (gap line) | Status | Where it is proven |
|---|---|---|
| `IGNORED` (default) runs whatever the signal | **tested** | unit `ignored_runs_whatever_the_signal`, `ignored_still_tops_up_the_tick_a_burn_ends_in`; in game: the whole slice-1 phase |
| `LOW` runs unpowered, stops while powered | **tested** | unit `low_runs_unpowered_high_runs_powered`; in game: frozen buffer for 40 ticks, resumes after the block is removed |
| `HIGH` runs only while powered | **tested** | same unit test; in game: 0 FE for 40 ticks unpowered, then a burn with a redstone block beside it |
| `PULSE` one burn per rising edge, none for a held signal | **tested** | units `pulse_runs_once_per_rising_edge_and_ignores_a_held_signal`, `a_pulse_starts_one_burn_that_then_runs_its_course`, `no_new_burn_starts_without_the_next_edge`, `a_pulse_does_not_top_up_the_tick_a_burn_ends_in`; in game: 1,500 FE per edge, `stick x4 → x3 → x2` |
| the saved mode round-trips, an unknown value loads as `IGNORED` | **tested** | units `the_four_modes_are_upstreams_and_round_trip`, `an_unknown_saved_mode_reads_as_ignored` |
| the mode control is an operator command | **tested (unit only)** | `the_redstone_mode_control_is_operator_only`, `a_player_below_level_two_may_not_administer`; a LAN guest on this pin **joins with command permission**, so the level-0 path cannot be produced in this harness (see below) |
| the mode control obeys the distance guard | **tested** | in game: a guest 40 blocks away is refused (`That generator is out of reach.`) for both `set` and `mode`, and the same guest one block away lands both |
| comparator output | **not ported — withdrawn** | upstream has none (grep evidence above) |
| `isRedstoneConductor = false` | **not ported** (gap 12) | no content property for it at this pin |

**Measured, not assumed:** a LAN guest (`ctl lan open 0 commands`) on `0a875da` joins with a
permission level at or above `Gamemasters` — `deop Guest` does not change it — so the operator
gate's *refusal* cannot be exercised by a second player here. It stays covered by the unit tests
plus the declared `Command::op(..., 2)`, and the reach guard is the in-game proof that another
player cannot write to a machine they are not standing at.

### In-game transcript (quoted from `e2e-slice2-launcher.log`)

```
  default mode: Simple Coal Generator at 12,100,0: 0/1000000 FE (burning false, fe 0 of 0, burn 0/0, multiplier 1, mode ignored, fuel empty, t=385)
  mode(12,100,0) = Simple Coal Generator at 12,100,0: 0/1000000 FE (burning false, fe 0 of 0, burn 0/0, multiplier 1, mode high, fuel empty, t=400)
  HIGH, unpowered after 40 ticks: Simple Coal Generator at 12,100,0: 0/1000000 FE (burning false, fe 0 of 0, burn 0/0, multiplier 1, mode high, fuel minecraft:stick x4, t=470)
  HIGH, powered: Simple Coal Generator at 12,100,0: 3000/1000000 FE (burning true, fe 1500 of 1500, burn 26/25, multiplier 1, mode high, fuel minecraft:stick x1, t=527)
  LOW, powered, 40 ticks later: Simple Coal Generator at 12,100,0: 4500/1000000 FE (burning false, fe 0 of 1500, burn 1/25, multiplier 1, mode low, fuel minecraft:stick x1, t=686)
  LOW, unpowered: Simple Coal Generator at 12,100,0: 6000/1000000 FE (burning false, fe 0 of 0, burn 0/0, multiplier 1, mode low, fuel empty, t=737)
  PULSE, no edge yet: Simple Coal Generator at 16,100,0: 0/1000000 FE (burning false, fe 0 of 0, burn 0/0, multiplier 1, mode pulse, fuel minecraft:stick x4, t=822)
  PULSE, rising edge: Simple Coal Generator at 16,100,0: 1140/1000000 FE (burning true, fe 360 of 1500, burn 7/25, multiplier 1, mode pulse, fuel minecraft:stick x3, t=848)
  PULSE, signal still on after 60 ticks: Simple Coal Generator at 16,100,0: 1500/1000000 FE (burning false, fe 0 of 1500, burn 1/25, multiplier 1, mode pulse, fuel minecraft:stick x3, t=920)
  PULSE, second edge: Simple Coal Generator at 16,100,0: 3000/1000000 FE (burning false, fe 0 of 1500, burn 1/25, multiplier 1, mode pulse, fuel minecraft:stick x2, t=988)
  guest position after the host's /tp: 40.5,95.936654946153,40.5
  guest set reply (40 blocks away): That generator is out of reach. [Player92307: Teleported Guest to 40.500000, 100.000000, 40.500000]
  guest mode reply (40 blocks away): That generator is out of reach. That generator is out of reach. …
  guest's target mode after the distant mode: Simple Coal Generator at 8,100,0: 0/1000000 FE (… mode ignored, fuel empty, t=1322)
  guest set reply (one block away): Energy set to 123 FE. …
  guest's target after the in-reach set: Simple Coal Generator at 8,100,0: 123/1000000 FE (… mode ignored, fuel empty, t=1383)
  guest mode reply (one block away): Redstone mode set to high. …
  guest's target after the in-reach mode: Simple Coal Generator at 8,100,0: 0/1000000 FE (… mode high, fuel empty, t=1464)
PASS: … HIGH ran only while powered and LOW only while unpowered; PULSE bought exactly one stick's burn per rising edge and a held signal bought none; and a guest 40 blocks away changed neither the buffer nor the mode while the same guest one block away changed both, so the refusal is distance
EXIT:0
```

The legs around it: `e2e-slice2-control.log` (the same Nightly without the package: the block id
is unknown, `FAIL` + `EXIT:1`), `e2e-slice2-selftest.log` (`SELFTEST=1` flips the no-fuel
assertion, `FAIL` + `EXIT:1`) and `e2e-slice2-selftest-redstone.log` (`SELFTEST=redstone` flips
the unpowered-`HIGH` assertion, `FAIL` + `EXIT:1`). `check-e2e-logs.py justdirethings-e2e
--strict` reads all seven logs: `ok=7 missing-exit=0 bad-exit=0`.

Skills: superpowers/using-superpowers, brainstorming, writing-plans, test-driven-development, systematic-debugging, verification-before-completion; BeautifulRust/rust-best-practices

## Migration to the SDK format

The registry closed PR **#591** on 2026-11-18 with one reason — *republish this
as a native `.hmod` with the SDK from the next Nightly* — so this is a
**resubmission**, not an edit: the manifest, the glue and the packaging are new,
and everything the slice proved about the machine is unchanged.

**SDK and pin:** build `0b14f8acfd97fa2c750e42795249dcc981178ca4` (2026-11-18),
mixins `e59da9094f0c2fdda6c627fe9eff22dec6a906f0`, launcher 0.2.78, 749 surface
items. `horizon-mod.toml`'s `[native]` block is the only pin. **The campaign pin moved
on to `0a875da91dbb0fdcb7ba8c42c11341aa229af51a` (Nightly 2026-11-20, launcher 0.2.92)
for slice 2**, which is what `0.2.3` declares and what both `fits` lines are quoted
against.

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
12. **`isRedstoneConductor = false`** — no content property for it, so the block still
    conducts. (The **four redstone modes** this line used to name are **done** in slice 2; the
    "comparator output" clause was **withdrawn** — upstream has no such override. See
    §Slice 2.)
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

**Current ladder (`0.2.3`, pin `0a875da`): `VALIDATION.md` §Slice 2** — `cargo test` 26
passed, clippy `-D warnings` clean, two `cargo horizon-mod build` runs `cmp`-identical
(SHA-256 `8c9be198…a3cd`, 36,240 B, 37 entries), `check` green, `fits` both ways
(`0a875da made for`, `0b14f8a fits`), `verify-sdk-package.py` 1/1, the in-game leg
`PASS`+`EXIT:0` with the control and both selftests at `EXIT:1`, and
`check-e2e-logs.py … --strict` at `ok=7`. Evidence: `../_shared/evidence/justdirethings/`
and `../_shared/evidence/justdirethings-e2e/`. What follows is the slice-1 record at the
SDK pin `0b14f8a` and, below it, the retired `.hnmod` build on which the gameplay was
first proved.

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

## Review fixes (2026-11-18, PR #1173)

The registry reviewer found the harness control `/justdirethings set` at level 0: it wrote the
saved energy of any loaded `justdirethings:generatort1` in the command's dimension, with no
operator, proximity or ownership check, so a player could mint or erase another player's FE.

- **Fix.** The usage is now `Command::op("justdirethings set <amount:integer> <pos:block_pos>", 2)`
  (`declared_commands`), and the handler repeats the check (`may_administer`) so a source that
  bypassed a usage's level is still refused. A player source must also be within six blocks of the
  generator (`within_reach`); the console, command blocks and functions keep acting anywhere.
  `info` stays a public read.
- **Tests.** `the_energy_set_control_is_operator_only` (level 2, `info` level 0),
  `a_player_below_level_two_may_not_administer`, `a_player_acts_only_within_reach`.
- **In-game.** A LAN guest (level 0) runs `/justdirethings set 123 8 100 0`; the reply is
  `That generator is out of reach.` and the readout stays `0/1000000 FE`. Evidence:
  `../_shared/evidence/justdirethings-e2e/e2e-launcher.log` (`PASS` + `EXIT:0`) and
  `e2e-launcher-control.log` (`EXIT:1`); `check-e2e-logs.py justdirethings-e2e --strict` green.
- **Package.** `justdirethings.hmod`, **0.2.1**, 30,684 B, sha256
  `62c233cb7112e60a66c150dd6472dd079600af0ee92eb4701dc53e61f52104df`; two builds byte-identical,
  `check` and `fits --build 0b14f8a` exit 0.

Skills: superpowers/using-superpowers, brainstorming, writing-plans, test-driven-development, systematic-debugging, verification-before-completion; BeautifulRust/rust-best-practices, rust-port, rust-review
