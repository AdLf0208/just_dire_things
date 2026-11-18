# Just Dire Things for Horizon — slice 1: the Simple Coal Generator

A clean-room Rust rewrite of **Just Dire Things 1.5.7**'s Simple Coal Generator
(`justdirethings:generatort1`) and the four coals it burns (`coal_t1..t4`) for
Horizon (Minecraft Java 26.3). Ported from MIT source; see
[`LICENSE-NOTICE.md`](LICENSE-NOTICE.md). Local-only until the maintainer
approves publishing.

## What the slice implements

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

## Not in this slice

Push-out to neighbours, the GUI/menu, the Fuel Generator, the Energy
Transmitter, the pocket generator, item automation, goo, time crystals and tools
are recorded gaps in [`notes.md`](notes.md).

## Build

This is a flat **SDK package** (Horizon's mod-API merge retired `.hnmod`): one
`.hmod` carries the code (`src/lib.rs` + `src/runtime.rs`) and the content
(`assets/`, `data/`), built by the launcher against the game build
`horizon-mod.toml`'s `[native]` pins.

```sh
# the pure logic, without the game
cargo test

# everything else, through the campaign's build slots
ports/tools/build-slot.sh run --job jdt-pack --port ports/justdirethings \
    --step build -- tools/package.sh          # build, check, deliver + hash
ports/tools/build-slot.sh run --job jdt-check --port ports/justdirethings \
    --step check -- cargo horizon-mod check justdirethings.hmod
```

The fetched SDK (`.horizon/sdk/`, from any `cargo horizon-mod` command) is never
packed. Content sources are rebuilt from the jar with
`python3 tools/assemble_content.py`.

The deliverable is **`justdirethings.hmod` in this directory** (the package is
the port: there is no separate content file and no `.hnmod`):
`justdirethings 0.1.0 (native-mod, format native-1/0b14f8a…/e59da90…, for 26.3)`.
Its size, SHA-256, entry count and the reproducibility check are recorded in
[`VALIDATION.md`](VALIDATION.md) — this file travels inside the package, so it
cannot quote the package's own hash and stay true.

`tools/e2e.sh` holds the in-game scenario that proved the slice on the retired
`.hnmod` build. It cannot run against this SDK build yet — no `horizon-ctl` for
`0b14f8a` exists on this machine — and `notes.md` §Migration to the SDK format
records exactly what was and was not exercised.
