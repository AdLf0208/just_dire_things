#!/usr/bin/env bash
# Build the flat SDK package and deliver it in the port root.
#
# The package is the port: `horizon-mod.toml` + `src/` + `assets/` + `data/`.
# `cargo horizon-mod build` packs it into `target/horizon-mod/<id>.hmod` (the
# SDK's API source is fetched into `.horizon/sdk/` and never packed); this
# script copies that file to `./justdirethings.hmod`, checks it and prints its
# size and SHA-256, which is what a resubmission quotes.
#
# Run it through a build slot, never with a bare cargo:
#   ports/tools/build-slot.sh run --job jdt-pack --port ports/justdirethings \
#       --step build -- tools/package.sh
set -euo pipefail
cd "$(dirname "$0")/.."

cargo horizon-mod build
cp -f target/horizon-mod/justdirethings.hmod justdirethings.hmod
cargo horizon-mod check justdirethings.hmod
sha256sum justdirethings.hmod
