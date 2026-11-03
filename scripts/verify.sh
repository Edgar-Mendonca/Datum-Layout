#!/usr/bin/env bash
# The exchange proofs: every example's PCF read and solved by Datum Pipe's own engine, and every
# example's glTF checked by the Khronos validator. Run from the repository root.
set -euo pipefail
out=$(mktemp -d)
cargo run -q -p datum-core --example export -- "$out" >/dev/null
pipe=$(mktemp -d)/datum-pipe
git clone -q --depth 1 https://github.com/Edgar-Mendonca/Datum-Pipe "$pipe"
cp scripts/pipe-read.rs "$pipe/crates/core/examples/layout_read.rs"
echo "Datum Pipe $(git -C "$pipe" rev-parse --short HEAD) reads Datum Layout's files:"
(cd "$pipe" && cargo run -q --release -p datum-core --example layout_read -- "$out") | tee "$out/pipe.txt"
! grep -q "^FAIL" "$out/pipe.txt"
v=$(mktemp -d); (cd "$v" && npm i -q --no-save gltf-validator@2.0.0-dev.3.10 >/dev/null 2>&1)
echo "glTF:"
NODE_PATH="$v/node_modules" node scripts/gltf-validate.cjs "$out"/*.glb | tee "$out/gltf.txt"
! grep -q "^FAIL" "$out/gltf.txt"
