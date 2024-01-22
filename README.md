## Build
```
cargo build --release --target wasm32-unknown-unknown
wasm-bindgen --no-typescript --target web --out-dir ./build/ ./target/wasm32-unknown-unknown/release/some-game.wasm
```
