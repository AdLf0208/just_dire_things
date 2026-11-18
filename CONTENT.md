# Content — Just Dire Things slice 1

The content half of the flat SDK package: `assets/` and `data/` at the package
root (the same directory as `horizon-mod.toml` and `src/`), carried for the
Simple Coal Generator and the four coals. `justdirethings.hmod` in the root is
the built package. Rebuild the sources with:
`python3 tools/assemble_content.py` (carries from
`ports/_shared/licence-cache/justdirethings-1.5.7.jar`), then package with
`tools/package.sh` inside a build slot.

## Carried byte-for-byte from the jar

| File | Why |
|---|---|
| `assets/justdirethings/blockstates/generatort1.json` | the jar's single `""` variant |
| `assets/justdirethings/models/block/generatort1.json` | the Blockbench model (3 elements, 3 textures) |
| `assets/justdirethings/models/item/generatort1.json` | the block item model |
| `assets/justdirethings/models/item/coal_t1..t4.json` | the four fuel item models |
| `assets/justdirethings/textures/block/generatort1_{side,top,bottom}.png` | the machine's textures |
| `assets/justdirethings/textures/item/coal_t1..t4.png` | the four fuels' textures |

## Written (1.21.1 → 26.3 shapes)

| File | What it is, and why it is not a raw carry |
|---|---|
| `assets/justdirethings/items/{generatort1,coal_t1..t4}.json` | 26.3 item definitions (`minecraft:model`), which 1.21.1 does not have |
| `assets/justdirethings/lang/en_us.json` | the six keys the slice uses; 1.21.1's `itemGroup.DeferredHolder{…}` key is unusable, so `itemGroup.justdirethings` |
| `data/justdirethings/horizon/block/generatort1.json` | the template declaration: `minecraft:iron_block` (no state properties, full cube), hardness 2.0, blast resistance 2.0, iron sounds, no correct-tool requirement, an item |
| `data/justdirethings/horizon/item/coal_t1..t4.json` | `minecraft:coal` plus the jar's `data/neoforge/data_maps/item/furnace_fuels.json` values now written as each item's `minecraft:cooking_fuel` component (`burn_time`, `speed_multiplier`) |
| `data/justdirethings/loot_table/blocks/generatort1.json` | the jar's table; 1.21.1 already uses the single `condition` object 26.3 wants |
| `data/justdirethings/recipe/generatort1.json` | the jar's shaped recipe with 26.3 ingredient strings; **it does not match yet** because `justdirethings:ferricore_ingot` is a later slice |
| `data/justdirethings/horizon/creative_tab/machines.json` | upstream's tab is code; one tab with the block and the four coals |
| `data/minecraft/tags/block/mineable/pickaxe.json` | only the slice's block, not the jar's 90 entries |

## Fuel data (the reason the coals are carried)

| Item | `burn_time` | `speed_multiplier` | FE/t | Ticks | Total FE |
|---|---:|---:|---:|---:|---:|
| `minecraft:coal` | 1,600 | 1.0 | 60 | 400 | 24,000 |
| `justdirethings:coal_t1` | 4,800 | 2.0 | 120 | 600 | 72,000 |
| `justdirethings:coal_t2` | 14,400 | 4.0 | 240 | 900 | 216,000 |
| `justdirethings:coal_t3` | 43,200 | 8.0 | 480 | 1,350 | 648,000 |
| `justdirethings:coal_t4` | 129,600 | 16.0 | 960 | 2,025 | 1,944,000 |

Every tier is 15 FE per burn tick, as upstream's config says; the tiers differ in
how fast they give it.

## Not carried (with the reason)

- `data/neoforge/data_maps/item/furnace_fuels.json`: 26.3 expresses the same
  values as an item component, so the file is not needed and NeoForge's
  `neoforge:` data-map registry is not one Horizon reads.
- `data/justdirethings/recipe/generatort1_nbtclear.json` and the
  `energytransmitter`/`generatorfluidt1`/`pocket_generator` recipes: later
  slices' items.
- The `Data/forge/loot_modifiers/global_loot_modifiers.old` file is dead
  (`.old`) upstream.
- The Patchouli book, the GUI textures and 400+ item models: not part of this
  slice.
