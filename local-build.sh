#!/bin/bash
# build Mandelbrot server using Rust

echo "building mb-rust-server..."
cd mb-rust-server
cargo build --release

echo "building mb-wasm..."
cd ../mb-wasm
./build.sh   # stamps client/MB.html's ASSET_VERSION after copying the binary
