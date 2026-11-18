# Just Dire Things — findings (slice 1)

Analysis for the Horizon port of **Just Dire Things 1.5.7** (Minecraft Java
1.21.1 / NeoForge, ATM10's jar). Upstream behaviour was extracted from the
public repository at the release tag; no Java code is translated, and the port
is a clean-room Rust rewrite against the pinned Horizon APIs.

## Provenance

| | |
|---|---|
| Jar | `ports/_shared/licence-cache/justdirethings-1.5.7.jar` (kept as evidence) |
| Size / SHA-1 / SHA-256 | 2,853,190 B · `f0327d7d2f389020063849200a4128ac9dcb52da` · `6e5f7dd7091cc271fee66b0df62bde2250e8b52397b51dd911f79c088eb22d2f` |
| Mod id / version | `justdirethings` 1.5.7 (`META-INF/neoforge.mods.toml`) |
| Licence (jar) | **MIT** — `neoforge.mods.toml` `license="MIT"`; **rank 0** (permissive). Local-only until publishing is approved (ARCHITECTURE.md §8). |
| Source repo | <https://github.com/Direwolf20-MC/JustDireThings>, tag `v1.5.7`, commit `8390e0effa33cd0e0995265245baf5629b61ef22` (2026-01-15). Shallow clone in `ports/_shared/upstream/justdirethings/` (analysis only, deleted after this record). |
| Author | Direwolf20 |

Files read: `common/blocks/GeneratorT1.java`, `common/blocks/baseblocks/BaseMachineBlock.java`,
`common/blockentities/GeneratorT1BE.java`, `common/blockentities/basebe/BaseMachineBE.java`,
`common/blockentities/basebe/RedstoneControlledBE.java`, `common/capabilities/GeneratorItemHandler.java`,
`common/capabilities/MachineEnergyStorage.java`, `common/capabilities/EnergyStorageNoReceive.java`,
`common/items/resources/Coal_T1..4.java`, `common/blocks/resources/CoalBlock_T1..4.java`,
`setup/Config.java` (`generatorT1Config`), `setup/Registration.java` (the generator/energy entries),
`util/interfacehelpers/RedstoneControlData.java`, `client/screens/GeneratorT1Screen.java`.
Jar data read: `assets/justdirethings/{blockstates,models,textures,lang}`, `data/justdirethings/{recipe,loot_table}`,
`data/neoforge/data_maps/item/furnace_fuels.json`, `META-INF/neoforge.mods.toml`.

## The families the jar ships (block states + lang, not names guessed)

| Family | What the jar has |
|---|---|
| **Energy generation** | `generatort1` (Simple Coal Generator), `generatorfluidt1` (Simple Fuel Generator), item `pocket_generator` |
| **Energy transfer/storage** | `energytransmitter` (Energy Transmitter) |
| **Item automation** | `itemcollector`, `sensort1/t2`, `clickert1/t2`, `droppert1/t2`, `blockplacert1/t2`, `blockbreakert1/t2`, `blockswappert1/t2`, `fluidcollectort1/t2`, `fluidplacert1/t2` |
| **Fuels** | items `coal_t1..t4`, blocks `coalblock_t1..t4`, block `charcoal`, ores `raw_coal_t1..t4_ore` |
| **Goo / farming** | `gooblock_tier1..4`, `goosoil_tier1..4`, `goopatternblock`, `goo` fluids |
| **Time / paradox** | `time_crystal_*` (block, budding, four cluster sizes), `time_fluid`, `paradoxmachine`, `eclipsegateblock`, `eclipsealloy_block` |
| **Storage / misc** | `experienceholder`, `inventory_holder`, `playeraccessor` |
| **Tools, armour, upgrades** | the ferricore / blazegold / celestigem / eclipsealloy item sets (abilities are components) |
| **Guide** | Patchouli book (`patchouli_books/justdirethingsbook`, 337 files) |

## The slice chosen: the Simple Coal Generator (`justdirethings:generatort1`)

**Why this one.** It is the smallest *machine* family the pin can express end to
end: one block, one fuel slot, one FE buffer, one server tick and no neighbour
reads. It is the pack's first power source, so later slices (the Energy
Transmitter, the Fuel Generator, the pocket generator) build on it. Two
measured platform facts decided the boundary:

1. A machine's own buffer and its own fuel slot are fully expressible at this
   pin (`block_entity`, `block_entity_ticks`, `block_entity_container`,
   `block_entity_energy` API 142, `block_entity_tick`).
2. Upstream's generator is an **insert-only** buffer (`EnergyStorageNoReceive`):
   it makes FE into itself and pushes to neighbours; nothing may fill it. The
   brief's measured limit — own-buffer generation only — is exactly this
   machine's shape. The push half needs an FE sink; Just Dire Things' only sink
   in this family is the Energy Transmitter, which is the *next* slice, so the
   push is recorded as a gap rather than half-built against nothing.

The **Fuel Generator** (`generatorfluidt1`) is the same family but needs fluid
tanks and a fluid capability; the **Energy Transmitter** is a network; the item
automation blocks need item-entity and fake-player paths. All are gaps below.

## Behaviour extracted (upstream → this port)

Numbers are upstream's config **defaults** (`Config.generatorT1Config`):

| Value | Upstream | Effect |
|---|---|---|
| `GENERATOR_T1_MAX_FE` | 1,000,000 | buffer capacity |
| `GENERATOR_T1_FE_PER_FUEL_TICK` | 15 | FE made per *fuel tick* of burn time |
| `GENERATOR_T1_BURN_SPEED_MULTIPLIER` | 4 | burn time divided by this, output multiplied by it |
| `GENERATOR_T1_FE_PER_TICK` | 1,000 | only the push-out cap (`×10` per neighbour); not the buffer's fill rate |
| storage class | `EnergyStorageNoReceive` | the buffer cannot be filled from outside (`canReceive()` false) |
| redstone default | `RedstoneMode.IGNORED` | the generator always runs; no redstone gate in this slice |

Derived behaviour (all unit-tested in `runtime.rs`):

- A fuel item is burned only when no FE is left in the current burn **and** the
  buffer has room for at least one FE.
- `fe_remaining = burn_time × 15`; `burn_speed = 4 × fuel_speed_multiplier`;
  `fe_per_tick = 15 × burn_speed`; `max_burn = floor(burn_time / burn_speed)`;
  `burn_remaining = max_burn + 1`.
- Each tick inserts `min(fe_per_tick, fe_remaining)`, limited by the buffer's
  room. Half-way through a burn, a second fuel item is loaded in the same tick
  (up to two items per tick), so output is continuous across fuel boundaries.
- The fuel item's **speed multiplier** comes from the engine's
  `minecraft:cooking_fuel.speed_multiplier` (`Providers::speed`), and its burn
  time from `minecraft:cooking_fuel.burn_time` (`Providers::burn_time`); vanilla
  coal is 1,600 ticks × 1.0, JDT's own Primal Coal (`coal_t1`) 4,800 × 2.0.
  This replaces NeoForge's `data/neoforge/data_maps/item/furnace_fuels.json`
  (carried into the items' 26.3 components) and the code-side
  `Coal_T1/CoalBlock_T1.getBurnSpeedMultiplier()`.
- The slot takes any item whose burn time is above zero
  (`GeneratorItemHandler.isItemValid`), one stack, consumed one at a time; a
  crafting remainder replaces the last item (a lava bucket leaves a bucket).
- The machine drops its slot's contents when broken and saves its settings in
  the block item (`BaseMachineBlock.onRemove` / `getDrops`).
- State keys upstream saves: `burnRemaining`, `maxBurn`, `feRemaining`,
  `fuelBurnMultiplier`, plus the energy buffer's `energy` int.
- No block state properties at all (the jar's blockstate has a single `""`
  variant), a full-cube shape, `strength(2.0)`, metal sounds, and
  `isRedstoneConductor = false`.

## Deliberate differences

1. **No push-out to neighbours** (gap 1): upstream pushes `fe_per_tick × 10`
   into each adjacent FE block. This slice stores into its own buffer only; the
   brief's measured shape. Nothing else can take FE in this mod's first tier.
2. **No GUI or menu.** Upstream right-click opens `GeneratorT1Screen` (FE and
   burn bar). The container slot is reachable by automation and by the harness.
3. **The buffer's surplus is kept.** Upstream decrements `feRemaining` by the
   *requested* amount even when the buffer only had room for part of it, so a
   nearly-full buffer destroys the rest of a burn. The port inserts what fits
   and keeps the remainder, so one coal always yields its 24,000 FE.
4. **A stack keeps the rest of its items** when a fuel with a crafting remainder
   is consumed; upstream replaces the whole stack with the remainder.
5. **Settings/contents on break** are not carried into the block item, and a
   broken generator loses its FE and fuel (upstream saves/restores them).
