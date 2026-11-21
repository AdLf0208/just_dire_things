# Validation — Just Dire Things

## Slice 2 (the redstone modes), version 0.2.3 — current

Pin **`0a875da91dbb0fdcb7ba8c42c11341aa229af51a`** (Nightly 2026-11-20), horizon-mixins
**`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`**, launcher **0.2.92**, SDK `ports/_shared/sdk/0a875da/`.
Every step ran inside a `ports/tools/build-slot.sh` lease (`--job W1-justdirethings`) with
`CARGO_INCREMENTAL=0`, never a bare cargo in the port and never `--target-dir`. The steps that need
`cargo horizon-mod` ran with `BUILD_LOCAL=1` (the far box has no such tool); the rest were forwarded.

| Step | Command | Result |
|---|---|---|
| red (new family) | `cargo test` | **1 failed / 25 passed**: `a_falling_edge_is_kept_even_when_nothing_burned` — *"a falling edge with no burn must still be written back"* (`../_shared/evidence/justdirethings-slice2/red-falling-edge.log`) |
| format | `cargo fmt -- --check` | clean (`../_shared/evidence/justdirethings/sdk-fmt.log`) |
| tests | `cargo test` | **26 passed**, 0 failed (`../_shared/evidence/justdirethings/sdk-test.log`) |
| clippy | `cargo clippy --all-targets -- -D warnings` | exit 0, 0 warnings (`../_shared/evidence/justdirethings/sdk-clippy.log`) |
| package build ×2 | `cargo horizon-mod build` (with `rm -f Cargo.lock` before each) | `8c9be198d40173d28253eb1b6d86ee7c8a6b4bc50d6e8578f28c184e5372a3cd` both times, `cmp` identical (`justdirethings-slice2/build-final-{1,2}.log`; the earlier `build-023-*` pair is the same code with the pre-README docs) |
| package check | `cargo horizon-mod check justdirethings.hmod` | exit 0, `justdirethings 0.2.3 (native-mod, format native-1/0a875da9…/e59da909…, for 26.3)`, 36,240 bytes, SHA-256 `8c9be198…a3cd` (`justdirethings/sdk-check.log`) |
| fits, the pin | `cargo horizon-mod fits justdirethings.hmod` | `0a875da  made for` (`justdirethings/sdk-fits.log`) |
| fits, a different build | `cargo horizon-mod fits --build 0b14f8acfd97fa2c750e42795249dcc981178ca4 justdirethings.hmod` | `0b14f8a  fits` (same log) — the cross-build, so the pin line is not a tautology |
| gate | `python3 ports/tools/verify-sdk-package.py justdirethings` | **1/1**, `26 passed`, 0 clippy warnings, 37 entries, no packed `Cargo.lock` |
| in-game leg | `build-slot.sh run … --step e2e -- bash tools/e2e.sh` on the launcher-built game | **PASS**, `EXIT:0` (`../_shared/evidence/justdirethings-e2e/e2e-slice2-launcher.log`) |
| control | the same leg on the same Nightly **without** the package | **`FAIL`**, `EXIT:1` (`e2e-slice2-control.log`) |
| fail-capability ×2 | `SELFTEST=1` / `SELFTEST=redstone` | **`FAIL`**, `EXIT:1` each (`e2e-slice2-selftest{,-redstone}.log`) |
| e2e log gate | `python3 ports/tools/check-e2e-logs.py justdirethings-e2e --strict` | `logs=7 ok=7 missing-exit=0 bad-exit=0`, exit 0 |
| skills gate | `python3 ports/tools/check-skills.py --notes ports/justdirethings` | `SKILLS GATE: PASS`, exit 0 |

The game: `ports/tools/launcher-game.sh build justdirethings` with
`HORIZON_E2E_INSTANCE=jdt-e2e` — a **fresh** instance, because `native-build` builds on the
Horizon the named instance plays unless it has none, and the shared `e2e` instance still played
`0b14f8a`; the fresh instance is what makes the build target this package's own pin (with the old
one the launcher refuses the package as *"made by an older tool"*, which is the pin mismatch
speaking, not the package). The launcher then compiled the mod against the Nightly and exported
`~/.cache/horizon-e2e/0a875da/justdirethings/` (`GAME`, `CTL`) plus
`…/0a875da/control/horizon-client` (`CONTROL_GAME`, the differential).

Delivered file, in the port root:

```
justdirethings.hmod  36,240 B  8c9be198d40173d28253eb1b6d86ee7c8a6b4bc50d6e8578f28c184e5372a3cd  37 entries
```

What this slice adds to the ladder: a **red proof in the pure layer** (the family's rule as a
failing test, quoted above), the **first in-game run of the port at the SDK pin** (it had none),
and a **differential inside the leg** — a guest 40 blocks away is refused while the same guest one
block away lands both writes — so the refusal is distance, not a blanket one.

## Slice 1 (the Simple Coal Generator), 0.1.0–0.2.1 — historical

## The SDK ladder (2026-11-18)

SDK build **`0b14f8acfd97fa2c750e42795249dcc981178ca4`** (2026-11-18), mixins
**`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`**, launcher 0.2.78; every step ran
inside a `ports/tools/build-slot.sh` lease (`--job sdk-jdt`) with
`CARGO_INCREMENTAL=0`, never a bare cargo and never `--target-dir`. The compile
legs ran locally with `BUILD_LOCAL=1` (a scratch tree under `/tmp` outside the
campaign's stage cache).

| Step | Command | Result |
|---|---|---|
| runtime carried | `sha256sum src/runtime.rs` | `5e381b0b099569c38066ff4ac51d006963dc90d5fce45f8c4c7749736399db5e`, byte-identical to the retired `mixins/justdirethings/runtime.rs` |
| format | `cargo fmt -- --check` | clean (`--all` fails inside the fetched SDK's own `build.rs` files, which are not ours) |
| runtime tests | `cargo test` | **11 passed**, 0 failed (`sdk-runtime-test.log`) |
| clippy | `cargo clippy --all-targets -- -D warnings` | exit 0, 0 warnings (`sdk-clippy.log`) |
| package build | `cargo horizon-mod build` | `Built ./target/horizon-mod/justdirethings.hmod` (`sdk-build-1.log`, `-2`, `-3`) |
| reproducible | build twice + once with the delivered file in the root | `4d1c5f14d49432519daa8dc9321ac6faa41b63618f9d2461b30360842cf58005` all three times; 38 entries; the root copy is **not** packed into itself |
| package check | `cargo horizon-mod check justdirethings.hmod` | exit 0, `justdirethings 0.1.0 (native-mod, format native-1/0b14f8acfd…/e59da909…, for 26.3)`, 31,298 bytes, SHA-256 `4d1c5f14…58005`, unsigned (`sdk-check.log`) |
| surface usage | `cargo horizon-mod check --uses justdirethings.hmod` | exit 0 (`sdk-check-uses.log`): `register`, `register_commands`, `run_mod_command`, `BlockEntityPart::{block_entity, block_entity_ticks, block_entity_tick}` |
| fits | `cargo horizon-mod fits justdirethings.hmod` | `Just Dire Things: 0b14f8a made for`, exit 0 (`sdk-fits.log`) |
| content regenerates | `python3 tools/assemble_content.py` then `diff -r` | 29 asset/data files identical to the carried tree |
| in-game | `tools/e2e.sh` | **not run at this build**: no `horizon-ctl`/game for `0b14f8a` on this machine (the shared pair is the retired `1d63e0a5`). The scenario is the one below; it must be re-run when a ctl for the SDK build exists |
| licence in the package | `zipfile` listing of the `.hmod` | `LICENSE` (1,070 B) and `LICENSE-NOTICE.md` (852 B) inside |

The delivered file, in the port root:

```
justdirethings.hmod  31,298 B  4d1c5f14d49432519daa8dc9321ac6faa41b63618f9d2461b30360842cf58005  38 entries
```

## The retired `.hnmod` ladder (2026-11-12) — historical

Pin `1d63e0a5a37a7a708eb0b32b49bf0dfeb28ed8a9`, mixins
`5a64af938ec86b00d53d16a408aabc0038fb3e57` (schema 6). Every step ran inside a
`ports/tools/build-slot.sh` lease with `CARGO_INCREMENTAL=0`; no bare cargo and no
`--target-dir`. The compile legs were forwarded to the second machine, the game
legs ran there headless; artefacts and evidence were pulled back. The scripts
named below (`tools/horizon.py`, `package.py`, `hmod-load.sh`, `test_stage.py`,
`test_template.py`, `add-notices.py`) were removed with the format; the evidence
logs stay in `../_shared/evidence/justdirethings/`.

| Step | Evidence | Result |
|---|---|---|
| `cargo fmt --all -- --check` | `fmt.log` | clean (empty) |
| `rustfmt --check mixins/justdirethings/embedded_content.rs` | `fmt-helper.log` | clean (empty) |
| `cargo test --locked --offline` | `cargo-test.log` | 11 passed, 0 failed (red first: 6 failed / 5 passed on the stubs) |
| `cargo clippy --all-targets -- -D warnings` | `clippy.log` | clean |
| `tools/horizon.py stage` | `stage.log` | `stage_changed: true` (the new `embedded-content` injection), 2 modules |
| `tools/horizon.py check` | `horizon-check.log`, `horizon-check-cargo.log` | `cargo_exit: 0`, 0 warnings |
| `tools/horizon.py test` | `horizon-test.log`, `horizon-test-cargo.log` | 11 passed, 1240 filtered |
| `tools/horizon.py build` | `horizon-build.log`, `horizon-build-cargo.log` | `cargo_exit: 0`, 0 warnings |
| `tools/package.sh` | `package.log` | .hmod 10,975 B `fd49d5f5…`, .hnmod **18,322 B** `31d5f91f…`, 13 entries, embedded copy identical |
| `cargo horizon-mod check --kind native-mod` | `check-native.log` | exit 0, `native-mod … for 26.3` |
| `tools/package.py` twice | in-session | `REPRODUCIBLE` (`31d5f91f…` both runs) |
| `build-slot.sh e2e … -- tools/e2e.sh` (no `--dev-mod`) | `e2e-embedded.log` | PASS, `EXIT:0`; control leg shows the ids unknown |
| `SELFTEST=1 tools/e2e.sh` | `e2e-embedded-selftest.log` | FAIL, `EXIT:1` |
| pre-embedding history (`--dev-mod content/`) | `e2e-pre-embedding.log`, `e2e-pre-embedding-selftest.log` | PASS `EXIT:0` / FAIL `EXIT:1` |
| `tools/hmod-load.sh` | `hmod-load.log` | PASS (registers with `--dev-mod`, unknown without), `EXIT:0` |
| `check-e2e-logs.py justdirethings --strict` | this run | `logs=4 ok=4 missing-exit=0 bad-exit=0` |
| `upstream_ready.py justdirethings` | this run | `READY justdirethings MIT`, exit 0 |

Good-pack hashes (final sources):

```
content/justdirethings.hmod  10,975 B  fd49d5f523067fa0cedfc77bf3abfb2f5d8b139b0cc4e2f510b285a8dd8b93ff  30 entries
justdirethings.hnmod         18,322 B  31d5f91fe360150692a74df75c4c72fc3e94f5f8efff753761a8bc4a482ecce7  13 entries
mixins/justdirethings/data/justdirethings-content.hmod  10,975 B  fd49d5f5…b93ff  (= the built .hmod)
```

The `.hmod` rebuilt on the second machine is byte-identical to the recorded
`fd49d5f5…` (same sources, same pin), and the `.hnmod` packed twice on one
machine gave the same SHA-256, so both packages are deterministic. The `.hnmod`
is the port's single deliverable: it carries the content it loads.
