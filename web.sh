#!/bin/sh
cargo b --release --target wasm32-unknown-unknown && cp target/wasm32-unknown-unknown/release/raycast-rs.wasm docs/raycast.wasm && python -m http.server -d docs
