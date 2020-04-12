# ferrogallic

Clone of skribble.io.

## Development

```sh
watchexec -d 1000 -r -c "wasm-pack build --target web --dev ferrogallic_web && cargo run --manifest-path ferrogallic/Cargo.toml -- 127.0.0.1:8080 -v"
```
