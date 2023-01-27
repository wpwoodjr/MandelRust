#!/bin/bash
# run Mandelbrot server locally

cd client
../mb-rust-server/target/release/mb-rust-server $@
