FROM rust:slim-bookworm AS builder
WORKDIR /app
COPY . .
RUN mkdir -p presets && \
    cargo build --release -j 2 && \
    BIN=$(find target/release -maxdepth 1 -type f -executable ! -name '*.so' ! -name '*.d' | head -n 1) && \
    cp "$BIN" /app/server

FROM debian:bookworm-slim
WORKDIR /app
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/server /app/server
COPY --from=builder /app/presets ./presets
EXPOSE 8000
CMD ["/app/server"]
