FROM ekidd/rust-musl-builder:1.48.0 AS planner
RUN cargo install cargo-chef
COPY . .
RUN cargo chef prepare --recipe-path recipe.json


FROM ekidd/rust-musl-builder:1.48.0 AS cacher
RUN cargo install cargo-chef
COPY --from=planner /home/rust/src/recipe.json recipe.json
RUN cargo chef cook --release --target x86_64-unknown-linux-musl --recipe-path recipe.json


FROM ekidd/rust-musl-builder:1.48.0 AS builder
COPY . .
COPY --from=cacher /home/rust/src/target target
COPY --from=cacher $CARGO_HOME $CARGO_HOME
RUN cargo build --release --target x86_64-unknown-linux-musl


FROM scratch
COPY --from=builder /home/rust/src/target/x86_64-unknown-linux-musl/release/cnat /cnat
USER 1000
ENTRYPOINT ["/cnat"]
