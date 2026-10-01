#!/usr/bin/env bash
set -eu
cd "$(dirname "$0")"

RUSTFLAGS="-C target-feature=+atomics,+bulk-memory,+mutable-globals \
-C link-args=--shared-memory -C link-args=--import-memory \
-C link-args=--max-memory=1073741824 \
-C link-args=--export=__heap_base -C link-args=--export=__data_end \
-C link-args=--export=__tls_base -C link-args=--export=__tls_size \
-C link-args=--export=__tls_align -C link-args=--export=__wasm_init_tls" \
CARGO_UNSTABLE_BUILD_STD="panic_abort,std" \
CARGO_TARGET_DIR=../target-mt \
wasm-pack build --target web --out-dir pkg-mt . -- --features threads

CARGO_TARGET_DIR=../target-st \
wasm-pack build --target web --out-dir pkg-st .
