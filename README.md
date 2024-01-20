# ferrogallic

Clone of skribble.io.

<img width="957" alt="image" src="https://github.com/erikdesjardins/ferrogallic/assets/7673145/8a5ee8c5-a232-4781-8825-857126f0de0d">

## Development

```sh
watchexec -d 1000 -c -r "wasm-pack build --target web --dev ferrogallic_web && cargo run --manifest-path ferrogallic/Cargo.toml -- 127.0.0.1:8080 -v"
```
