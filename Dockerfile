# Edition 2024 needs nightly Rust for now.
FROM rustlang/rust:nightly as builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY .cargo ./.cargo
COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim
WORKDIR /app
COPY --from=builder /app/target/release/tianyi-auto /usr/local/bin/tianyi-auto
ENTRYPOINT ["/usr/local/bin/tianyi-auto"]
