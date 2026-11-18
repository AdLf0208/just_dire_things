# Working on this Horizon mod (the SDK layout)

> **Build slots (v2):** every build/test/e2e goes through the campaign runner
> (`ports/tools/build-slot.sh` from the campaign root). It **blocks in the
> kernel** until a slot is free — a freed slot is taken in ~0.07 s — so **never
> poll**: no `while ! slot; sleep` loop, no background build. It sets
> `CARGO_BUILD_JOBS` per step and a timeout exits **75**. Read
> `../_shared/build-slots-usage.md` before running anything; observe with
> `status` / `watch` / `events` / `stats`, never with `ps`+`sleep`. Never pass
> `--target-dir`.

This port is a **flat SDK package** (Horizon's mod-API merge, launcher ≥
0.2.70): `horizon-mod.toml` + `Cargo.toml` + `src/lib.rs` (the one
`#[mod_api::horizon_mod] impl Hooks`) + `src/runtime.rs` + `assets/` + `data/`.
The retired `.hnmod` shape — `mixins/<id>/`, `horizon-mixin.toml`,
`horizon-native.json`, per-hook `.rs` helpers, the plan/injection schema,
`native/`, `rust/`, `rust-toolchain.toml` — is **not** part of this package and
must not come back.

Read `README.md`, `notes.md` and `../_shared/AGENT_MOD_STRUCTURE.md` §0 before
editing. `../_shared/pins/horizon-1d63e0a/` and `horizon/` are read-only pin
providers; this port stages nothing from them any more (the SDK is fetched into
`.horizon/sdk/` by any `cargo horizon-mod` command and is never packed).

Keep gameplay logic in `src/runtime.rs` (pure: no `mod_api` types, no host, no
I/O) with its `#[cfg(test)] mod tests`; `src/lib.rs` holds only the typed glue —
`register`, the `BlockEntityPart`, the commands. One behaviour per function, no
business rules in the glue.

Validate with `cargo fmt -- --check` (not `--all`: the fetched SDK's own
`build.rs` files are not ours and are not rustfmt-clean), `cargo clippy
--all-targets -- -D warnings`, `cargo test`, then `cargo horizon-mod build`,
`cargo horizon-mod check justdirethings.hmod` and `cargo horizon-mod fits
justdirethings.hmod`, all inside a build-slot lease. Keep validation claims and
supported revisions honest: the manifest's `[native]` commits are the only pin,
and `justdirethings.hmod` in the port root is the delivered file.
