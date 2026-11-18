# Just Dire Things — a Horizon port

**Just Dire Things** is a Minecraft mod. Just some things Direwolf20 wants to have. This repository is a **port of it to [Horizon](https://github.com/horizon-rust/horizon)**, the Rust engine for Minecraft Java 26.3. It carries the complete source of the port plus the content files it ships, and installs as a single `.hmod` package.

Upstream: [Direwolf20-MC/JustDireThings](https://github.com/Direwolf20-MC/JustDireThings) — licence **MIT**.

| | |
|---|---|
| Upstream mod | Just Dire Things ([source](https://github.com/Direwolf20-MC/JustDireThings)) |
| Ported to | Horizon 0b14f8a, Minecraft Java 26.3, mod API `0.2` |
| Package | `justdirethings.hmod` (install it with the Horizon launcher) |
| Licence | **MIT** — the upstream mod's licence; see [Licence and credit](#licence-and-credit) |

## What this port implements

### What the slice implements

- the generator block + item, carried from the jar (models, textures, lang,
  loot, recipe, pickaxe tag);
- a 1,000,000 FE **insert-only** buffer (`Capabilities.Energy.BLOCK`, API 142),
  exactly upstream's `EnergyStorageNoReceive`;
- a one-slot fuel container (automation + harness path);
- a server tick that burns any `minecraft:cooking_fuel` item at 15 FE per burn
  tick, with the fuel's own `speed_multiplier` (vanilla coal: 60 FE/t for 400
  ticks = 24,000 FE; Primal Coal: 120 FE/t for 600 ticks = 72,000 FE), pausing
  when the buffer is full and continuing seamlessly across fuel boundaries;
- `/justdirethings info <x> <y> <z>` and `/justdirethings set <fe> <x> <y> <z>`
  (the latter a harness control).

## Not yet ported

- **Push-out to neighbours** (`providePowerAdjacent`, `fePerTick × 10` per side).
- **No GUI/menu.** Upstream opens `GeneratorT1Screen`; this slice has no menu
- **The Fuel Generator** (`generatorfluidt1`): 5,000,000 FE buffer, 5,000 FE/t,
- **The Energy Transmitter** (`energytransmitter`): the network, per-side
- **The pocket generator** (item FE): `item_energy_buffer` (API 307) plus an
- **Item automation**: `itemcollector`, `sensort1/t2`, `clickert1/t2`,
- **Goo and farming** (`gooblock_tier1..4`, `goosoil_tier1..4`,
- **Time and paradox** (`time_crystal_*`, `time_fluid`, `paradoxmachine`,

## Installing

1. Install the [Horizon launcher](https://github.com/horizon-rust/horizon/releases) (0.2.70 or newer).
2. Add `justdirethings.hmod` to the instance's mods.
3. The package is pinned to a specific Horizon build; the launcher checks that pin.

## Licence and credit

Just Dire Things was created by its upstream authors; the code, models and assets in the original mod
remain theirs and are used here under **MIT**. The Rust in this repository is a clean-room
rewrite for Horizon, not a translation of the original source.

- `LICENSE` — the upstream licence text.
- `LICENSE-NOTICE.md` — what this port carries and where it came from.
- The upstream project: https://github.com/Direwolf20-MC/JustDireThings

## Technical details

```
horizon-mod.toml   package manifest (id, version, api, [native] pin)
Cargo.toml         the crate; depends on `horizon-mod-api` from the fetched SDK
src/               lib.rs (the one `impl Hooks`), runtime.rs (pure logic + tests), glue
assets/ data/      the content the package ships
tools/             content generators and the e2e scenario
```

- Build: `cargo horizon-mod build` (packages; it does not compile against the game).
- Test: `cargo test`, `cargo clippy --all-targets -- -D warnings`.
- In-game: `ports/tools/launcher-game.sh run justdirethings` in the campaign tree, or install the
  `.hmod` and play; the harness scenario is `tools/e2e.sh`.

This repository is generated from the campaign's port at `ports/justdirethings/`; the port's own
`notes.md` is the full record (decisions, traps, evidence).
