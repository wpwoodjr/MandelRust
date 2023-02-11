cargo build --target wasm32-unknown-unknown --release
cp target/wasm32-unknown-unknown/release/mb_wasm.wasm ../client/mb-wasm.wasm
