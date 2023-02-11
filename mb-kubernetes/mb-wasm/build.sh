#!/bin/bash
# build Mandelbrot wasm code using Rust

rustup target add wasm32-unknown-unknown
cargo build --target wasm32-unknown-unknown --release
cp target/wasm32-unknown-unknown/release/mb_wasm.wasm ../client/mb-wasm.wasm
