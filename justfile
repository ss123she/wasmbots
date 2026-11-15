default: run

run: build-bots dev

[parallel]
build-bots: _build-hunter _build-sniper

_build-hunter:
    cargo build --manifest-path bots/hunter/Cargo.toml --target wasm32-unknown-unknown --release
    cp bots/hunter/target/wasm32-unknown-unknown/release/hunter.wasm backend/presets/

_build-sniper:
    cargo build --manifest-path bots/sniper/Cargo.toml --target wasm32-unknown-unknown --release
    cp bots/sniper/target/wasm32-unknown-unknown/release/sniper.wasm backend/presets/

dev:
    bunx concurrently -n BACK,FRONT -c cyan,magenta --kill-others "just dev-back" "just dev-front"

[working-directory('backend')]
dev-back:
    cargo run

[working-directory('frontend')]
dev-front:
    bun run dev
