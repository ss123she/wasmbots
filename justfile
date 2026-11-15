default: run

# Run both frontend and backend concurrently (requires just >= 1.29.0)
[parallel]
run: dev-back dev-front

[working-directory: 'backend']
dev-back:
    cargo run

[working-directory: 'frontend']
dev-front:
    npm run dev

build:
    cargo build --release --manifest-path backend/Cargo.toml
    npm --prefix frontend run build

# Run tests and linters
test:
    cargo test --manifest-path backend/Cargo.toml
    npm --prefix frontend run lint
