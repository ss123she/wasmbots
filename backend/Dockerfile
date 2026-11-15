FROM rust:slim-bookworm AS builder
WORKDIR /app
COPY . .
RUN mkdir -p presets && cargo build --release -j 2

FROM debian:bookworm-slim
WORKDIR /app
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/backend /app/server
COPY --from=builder /app/presets ./presets
EXPOSE 8000
CMD ["/app/server"]
