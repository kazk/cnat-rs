FROM clux/muslrust:stable AS chef
RUN cargo install cargo-chef

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /volume/recipe.json recipe.json
# Build dependencies
RUN cargo chef cook --release --target x86_64-unknown-linux-musl --recipe-path recipe.json
# Build controller
COPY . .
RUN cargo build --release --target x86_64-unknown-linux-musl

FROM scratch
COPY --from=builder /volume/target/x86_64-unknown-linux-musl/release/cnat /cnat
USER 1000
ENTRYPOINT ["/cnat"]
