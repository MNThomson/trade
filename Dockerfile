FROM postgres:latest AS postgres
COPY src/init.sql /docker-entrypoint-initdb.d/


################################################
FROM otel/opentelemetry-collector-contrib:0.104.0 AS collector
COPY config.yaml /etc/otelcol-contrib/config.yaml


################################################
FROM clux/muslrust:amd64-1.87.0-nightly-2025-02-20 AS chef
RUN cargo install cargo-chef


FROM chef AS planner
COPY Cargo.* .
RUN cargo chef prepare --recipe-path recipe.json


FROM chef AS cacher
COPY --from=planner /volume/recipe.json recipe.json
RUN cargo chef cook --release --target x86_64-unknown-linux-musl --recipe-path recipe.json


FROM chef AS builder
COPY Cargo.* build.rs rust-toolchain.toml .
COPY .sqlx/ .sqlx/
COPY src/ src/
COPY --from=cacher /volume/target target
COPY --from=cacher /root/.cargo /root/.cargo
RUN cargo build --release --target x86_64-unknown-linux-musl


FROM gcr.io/distroless/static:nonroot AS trade
COPY --from=builder --chown=nonroot:nonroot /volume/target/x86_64-unknown-linux-musl/release/trade /app/trade
ENTRYPOINT ["/app/trade"]
