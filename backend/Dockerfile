FROM rust:1.90-bookworm AS builder

WORKDIR /app

COPY backend/Cargo.toml backend/Cargo.lock ./backend/
COPY backend/src ./backend/src
COPY tests ./tests

WORKDIR /app/backend

RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates git \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/backend/target/release/repoinsight-backend /app/repoinsight-backend

ENV HOST=0.0.0.0
ENV PORT=3000
ENV LOCAL_REPO_ANALYSIS_ENABLED=false

EXPOSE 3000

CMD ["/app/repoinsight-backend"]