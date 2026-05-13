# --- Build stage ---
FROM rust:1.85-slim AS builder

WORKDIR /app
COPY . .

RUN cargo build --locked --release --bin metasearch

# --- Runtime stage ---
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl && rm -rf /var/lib/apt/lists/*
RUN useradd --system --create-home --home-dir /app --shell /usr/sbin/nologin metasearch

COPY --from=builder /app/target/release/metasearch /usr/local/bin/metasearch
COPY config.toml /app/config.toml
COPY templates/ /app/templates/
COPY static/ /app/static/

WORKDIR /app
RUN chown -R metasearch:metasearch /app

USER metasearch

EXPOSE 8888
STOPSIGNAL SIGTERM

HEALTHCHECK --interval=30s --timeout=5s --start-period=15s --retries=3 CMD curl --fail http://127.0.0.1:8888/readyz || exit 1

CMD ["metasearch", "serve", "--config", "config.toml"]
